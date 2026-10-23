//! Full-featured mouse and keyboard operable Settings modal.
//! Writes user settings directly to ~/.config/prowly/config.toml.

use crate::ai::AiProvider;
use crate::config::Config;
use crate::i18n::{I18n, Language};
use crate::ui::icons::Icons;
use crate::ui::theme::{Theme, ThemeId};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

/// Active input field focused for text editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsInputFocus {
    AiModel,
    AiApiKey,
    AiApiBase,
    EditorCommand,
}

/// Active item focused for navigation cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsNavFocus {
    #[default]
    Theme,
    Language,
    NerdFont,
    AiProvider,
    AiModelPresets,
    AiModelInput,
    AiApiKeyInput,
    AiApiBaseInput,
    GitSoftDiscard,
    GitAutoFetch,
    GitRestoreRepo,
    EditorCommand,
    SaveButton,
    CancelButton,
}

impl SettingsNavFocus {
    pub const ALL: &'static [SettingsNavFocus] = &[
        SettingsNavFocus::Theme,
        SettingsNavFocus::Language,
        SettingsNavFocus::NerdFont,
        SettingsNavFocus::AiProvider,
        SettingsNavFocus::AiModelPresets,
        SettingsNavFocus::AiModelInput,
        SettingsNavFocus::AiApiKeyInput,
        SettingsNavFocus::AiApiBaseInput,
        SettingsNavFocus::GitSoftDiscard,
        SettingsNavFocus::GitAutoFetch,
        SettingsNavFocus::GitRestoreRepo,
        SettingsNavFocus::EditorCommand,
        SettingsNavFocus::SaveButton,
        SettingsNavFocus::CancelButton,
    ];

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|x| x == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(&self) -> Self {
        let idx = Self::ALL.iter().position(|x| x == self).unwrap_or(0);
        if idx == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[idx - 1]
        }
    }
}

/// Action resulting from settings interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    SaveAndApply,
    Close,
}

/// Settings modal dialog state.
pub struct SettingsModal {
    pub is_open: bool,
    pub theme: ThemeId,
    pub language: Language,
    pub use_nerd_font: bool,
    pub ai_provider: AiProvider,
    pub ai_model: String,
    pub ai_api_key: String,
    pub ai_api_base: String,
    pub git_soft_discard: bool,
    pub git_auto_fetch_secs: u64,
    pub git_restore_last_repo: bool,
    pub editor_command: String,

    pub show_api_key: bool,
    pub focused_input: Option<SettingsInputFocus>,
    pub focused_nav: SettingsNavFocus,
    pub status_banner: Option<String>,
    pub scroll_offset: u16,
    pub hover_pos: Option<(u16, u16)>,
}

/// Helper to calculate layout of theme chips wrapped across multiple lines.
/// Returns a list of rows, each containing (ThemeId, chip_name, start_x, end_x).
pub fn layout_theme_chips(inner_w: u16) -> Vec<Vec<(ThemeId, &'static str, u16, u16)>> {
    let mut rows: Vec<Vec<(ThemeId, &'static str, u16, u16)>> = Vec::new();
    let mut current_row: Vec<(ThemeId, &'static str, u16, u16)> = Vec::new();
    let indent: u16 = 10;
    let mut cur_x = indent;

    for &tid in ThemeId::ALL {
        let name = tid.chip_name();
        let btn_w = name.len() as u16 + 6;
        if !current_row.is_empty() && cur_x + btn_w > inner_w {
            rows.push(current_row);
            current_row = Vec::new();
            cur_x = indent;
        }
        let start_x = cur_x;
        let end_x = cur_x + btn_w;
        current_row.push((tid, name, start_x, end_x));
        cur_x = end_x + 1;
    }
    if !current_row.is_empty() {
        rows.push(current_row);
    }
    rows
}

impl Default for SettingsModal {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsModal {
    pub fn new() -> Self {
        Self {
            is_open: false,
            theme: ThemeId::CatppuccinMocha,
            language: Language::En,
            use_nerd_font: true,
            ai_provider: AiProvider::OpenAi,
            ai_model: "gpt-4o-mini".to_string(),
            ai_api_key: String::new(),
            ai_api_base: String::new(),
            git_soft_discard: true,
            git_auto_fetch_secs: 60,
            git_restore_last_repo: true,
            editor_command: String::new(),
            show_api_key: false,
            focused_input: None,
            focused_nav: SettingsNavFocus::default(),
            status_banner: None,
            scroll_offset: 0,
            hover_pos: None,
        }
    }

    /// Open settings modal populating from active configuration.
    pub fn open_with_config(&mut self, config: &Config) {
        self.is_open = true;
        self.theme = config.theme;
        self.language = config.language;
        self.use_nerd_font = config.use_nerd_font;
        self.ai_provider = config.ai.provider;
        self.ai_model = config.ai.model.clone();
        self.ai_api_key = config.ai.api_key.clone().unwrap_or_default();
        self.ai_api_base = config.ai.api_base.clone().unwrap_or_default();
        self.git_soft_discard = config.git.soft_discard_backup;
        self.git_auto_fetch_secs = config.git.auto_fetch_interval_secs;
        self.git_restore_last_repo = config.git.restore_last_repository;
        self.editor_command = config.editor.command.clone().unwrap_or_default();
        self.show_api_key = false;
        self.focused_input = None;
        self.focused_nav = SettingsNavFocus::default();
        self.status_banner = None;
        self.scroll_offset = 0;
    }

    /// Scroll up one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down one line, constrained by modal viewport height.
    pub fn scroll_down(&mut self, viewport: Rect) {
        let rect = self.modal_rect(viewport);
        let inner_height = rect.height.saturating_sub(2);
        let inner_w = rect.width.saturating_sub(2);
        let theme_rows = layout_theme_chips(inner_w);
        let offset = (theme_rows.len() as u16).saturating_sub(1);
        let total_lines = 18 + offset;
        let max_scroll = total_lines.saturating_sub(inner_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Calculate the line row offset (0-indexed relative to top of scrolled content) for a nav item.
    pub fn item_row_y(&self, focus: SettingsNavFocus, inner_w: u16) -> u16 {
        let theme_rows = layout_theme_chips(inner_w);
        let offset = (theme_rows.len() as u16).saturating_sub(1);
        match focus {
            SettingsNavFocus::Theme => 2,
            SettingsNavFocus::Language | SettingsNavFocus::NerdFont => 3 + offset,
            SettingsNavFocus::AiProvider => 5 + offset,
            SettingsNavFocus::AiModelPresets => 7 + offset,
            SettingsNavFocus::AiModelInput => 8 + offset,
            SettingsNavFocus::AiApiKeyInput => 9 + offset,
            SettingsNavFocus::AiApiBaseInput => 10 + offset,
            SettingsNavFocus::GitSoftDiscard | SettingsNavFocus::GitAutoFetch => 12 + offset,
            SettingsNavFocus::GitRestoreRepo => 13 + offset,
            SettingsNavFocus::EditorCommand => 14 + offset,
            SettingsNavFocus::SaveButton | SettingsNavFocus::CancelButton => 16 + offset,
        }
    }

    /// Ensure the currently focused nav item is visible within the viewport.
    pub fn ensure_visible(&mut self, viewport: Rect) {
        let rect = self.modal_rect(viewport);
        let inner_h = rect.height.saturating_sub(2);
        let inner_w = rect.width.saturating_sub(2);
        let row_y = self.item_row_y(self.focused_nav, inner_w);
        if row_y < self.scroll_offset {
            self.scroll_offset = row_y;
        } else if row_y >= self.scroll_offset + inner_h {
            self.scroll_offset = row_y.saturating_sub(inner_h) + 1;
        }
    }

    /// Close settings dialog.
    pub fn close(&mut self) {
        self.is_open = false;
        self.focused_input = None;
    }

    /// Apply updated settings onto configuration struct.
    pub fn apply_to_config(&self, config: &mut Config) {
        config.theme = self.theme;
        config.language = self.language;
        config.use_nerd_font = self.use_nerd_font;
        config.ai.provider = self.ai_provider;
        config.ai.model = self.ai_model.clone();
        config.ai.api_key = if self.ai_api_key.trim().is_empty() {
            None
        } else {
            Some(self.ai_api_key.trim().to_string())
        };
        config.ai.api_base = if self.ai_api_base.trim().is_empty() {
            None
        } else {
            Some(self.ai_api_base.trim().to_string())
        };
        config.git.soft_discard_backup = self.git_soft_discard;
        config.git.auto_fetch_interval_secs = self.git_auto_fetch_secs;
        config.git.restore_last_repository = self.git_restore_last_repo;
        config.editor.command = if self.editor_command.trim().is_empty() {
            None
        } else {
            Some(self.editor_command.trim().to_string())
        };
        config.populate_api_keys_from_env();
    }

    /// Handle text input and keyboard navigation.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SettingsAction> {
        // Ctrl+S or Ctrl+Enter saves immediately from anywhere
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') | KeyCode::Enter => {
                    return Some(SettingsAction::SaveAndApply);
                }
                _ => {}
            }
        }

        // When actively typing in a text input
        if let Some(focus) = self.focused_input {
            match key.code {
                KeyCode::Esc => {
                    self.focused_input = None;
                    return None;
                }
                KeyCode::Enter => {
                    self.focused_input = None;
                    return None;
                }
                KeyCode::Tab => {
                    self.focused_input = match focus {
                        SettingsInputFocus::AiModel => Some(SettingsInputFocus::AiApiKey),
                        SettingsInputFocus::AiApiKey => Some(SettingsInputFocus::AiApiBase),
                        SettingsInputFocus::AiApiBase => Some(SettingsInputFocus::EditorCommand),
                        SettingsInputFocus::EditorCommand => None,
                    };
                    self.focused_nav = match self.focused_input {
                        Some(SettingsInputFocus::AiModel) => SettingsNavFocus::AiModelInput,
                        Some(SettingsInputFocus::AiApiKey) => SettingsNavFocus::AiApiKeyInput,
                        Some(SettingsInputFocus::AiApiBase) => SettingsNavFocus::AiApiBaseInput,
                        Some(SettingsInputFocus::EditorCommand) => SettingsNavFocus::EditorCommand,
                        None => SettingsNavFocus::SaveButton,
                    };
                    return None;
                }
                KeyCode::Backspace => {
                    match focus {
                        SettingsInputFocus::AiModel => {
                            self.ai_model.pop();
                        }
                        SettingsInputFocus::AiApiKey => {
                            self.ai_api_key.pop();
                        }
                        SettingsInputFocus::AiApiBase => {
                            self.ai_api_base.pop();
                        }
                        SettingsInputFocus::EditorCommand => {
                            self.editor_command.pop();
                        }
                    }
                    return None;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match focus {
                        SettingsInputFocus::AiModel => self.ai_model.push(c),
                        SettingsInputFocus::AiApiKey => self.ai_api_key.push(c),
                        SettingsInputFocus::AiApiBase => self.ai_api_base.push(c),
                        SettingsInputFocus::EditorCommand => self.editor_command.push(c),
                    }
                    return None;
                }
                _ => return None,
            }
        }

        // Navigation Mode (when no text input is actively being edited)
        match key.code {
            KeyCode::Esc => Some(SettingsAction::Close),
            KeyCode::Enter => match self.focused_nav {
                SettingsNavFocus::SaveButton => Some(SettingsAction::SaveAndApply),
                SettingsNavFocus::CancelButton => Some(SettingsAction::Close),
                SettingsNavFocus::AiModelInput => {
                    self.focused_input = Some(SettingsInputFocus::AiModel);
                    None
                }
                SettingsNavFocus::AiApiKeyInput => {
                    self.focused_input = Some(SettingsInputFocus::AiApiKey);
                    None
                }
                SettingsNavFocus::AiApiBaseInput => {
                    self.focused_input = Some(SettingsInputFocus::AiApiBase);
                    None
                }
                SettingsNavFocus::EditorCommand => {
                    self.focused_input = Some(SettingsInputFocus::EditorCommand);
                    None
                }
                _ => {
                    self.toggle_focused_setting(true);
                    None
                }
            },
            KeyCode::Char(' ') => match self.focused_nav {
                SettingsNavFocus::SaveButton => Some(SettingsAction::SaveAndApply),
                SettingsNavFocus::CancelButton => Some(SettingsAction::Close),
                SettingsNavFocus::AiModelInput => {
                    self.focused_input = Some(SettingsInputFocus::AiModel);
                    None
                }
                SettingsNavFocus::AiApiKeyInput => {
                    self.focused_input = Some(SettingsInputFocus::AiApiKey);
                    None
                }
                SettingsNavFocus::AiApiBaseInput => {
                    self.focused_input = Some(SettingsInputFocus::AiApiBase);
                    None
                }
                SettingsNavFocus::EditorCommand => {
                    self.focused_input = Some(SettingsInputFocus::EditorCommand);
                    None
                }
                _ => {
                    self.toggle_focused_setting(true);
                    None
                }
            },
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                self.focused_nav = self.focused_nav.prev();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset = self.scroll_offset.saturating_add(1).min(18);
                self.focused_nav = self.focused_nav.next();
                None
            }
            KeyCode::Tab => {
                self.focused_nav = self.focused_nav.next();
                None
            }
            KeyCode::BackTab => {
                self.focused_nav = self.focused_nav.prev();
                None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.toggle_focused_setting(false);
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.toggle_focused_setting(true);
                None
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                None
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(5).min(18);
                None
            }
            _ => None,
        }
    }

    /// Helper to toggle or cycle setting option.
    fn toggle_focused_setting(&mut self, forward: bool) {
        match self.focused_nav {
            SettingsNavFocus::Theme => {
                let themes = ThemeId::ALL;
                let idx = themes.iter().position(|&t| t == self.theme).unwrap_or(0);
                let next_idx = if forward {
                    (idx + 1) % themes.len()
                } else if idx == 0 {
                    themes.len() - 1
                } else {
                    idx - 1
                };
                self.theme = themes[next_idx];
            }
            SettingsNavFocus::Language => {
                self.language = match self.language {
                    Language::En => Language::Ja,
                    Language::Ja => Language::En,
                };
            }
            SettingsNavFocus::NerdFont => {
                self.use_nerd_font = !self.use_nerd_font;
            }
            SettingsNavFocus::AiProvider => {
                let providers = AiProvider::ALL;
                let idx = providers
                    .iter()
                    .position(|&p| p == self.ai_provider)
                    .unwrap_or(0);
                let next_idx = if forward {
                    (idx + 1) % providers.len()
                } else if idx == 0 {
                    providers.len() - 1
                } else {
                    idx - 1
                };
                self.ai_provider = providers[next_idx];
                self.ai_model = self.ai_provider.default_model().to_string();
            }
            SettingsNavFocus::AiModelPresets => {
                let presets: &[&str] = match self.ai_provider {
                    AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "o3-mini"],
                    AiProvider::Anthropic => {
                        &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"]
                    }
                    AiProvider::Gemini => {
                        &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"]
                    }
                    AiProvider::Ollama => &["cyberTielCorder:35B-A3B", "bonsai-2:27b", "qwen3-30b"],
                    AiProvider::Custom => &["default", "local-model"],
                };
                let idx = presets
                    .iter()
                    .position(|&m| m == self.ai_model)
                    .unwrap_or(0);
                let next_idx = if forward {
                    (idx + 1) % presets.len()
                } else if idx == 0 {
                    presets.len() - 1
                } else {
                    idx - 1
                };
                self.ai_model = presets[next_idx].to_string();
            }
            SettingsNavFocus::GitSoftDiscard => {
                self.git_soft_discard = !self.git_soft_discard;
            }
            SettingsNavFocus::GitAutoFetch => {
                self.git_auto_fetch_secs = match self.git_auto_fetch_secs {
                    0 => 30,
                    30 => 60,
                    _ => 0,
                };
            }
            SettingsNavFocus::GitRestoreRepo => {
                self.git_restore_last_repo = !self.git_restore_last_repo;
            }
            SettingsNavFocus::SaveButton | SettingsNavFocus::CancelButton => {
                self.focused_nav = match self.focused_nav {
                    SettingsNavFocus::SaveButton => SettingsNavFocus::CancelButton,
                    _ => SettingsNavFocus::SaveButton,
                };
            }
            _ => {}
        }
    }

    /// Calculate modal bounding box in viewport.
    /// Expands generously to larger terminal sizes up to 104 columns wide and 36 rows high.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let ideal_w = (viewport.width * 88 / 100).clamp(76, 104);
        let width = ideal_w.min(viewport.width.saturating_sub(2));

        let ideal_h = (viewport.height * 92 / 100).clamp(24, 36);
        let height = ideal_h.min(viewport.height.saturating_sub(2));

        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Handle mouse click inside settings modal.
    pub fn handle_click(&mut self, x: u16, y: u16, viewport: Rect) -> Option<SettingsAction> {
        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(SettingsAction::Close);
        }

        let inner_x = x.saturating_sub(rect.x + 1);
        let inner_y = y.saturating_sub(rect.y + 1);
        let content_y = inner_y + self.scroll_offset;
        let inner_w = rect.width.saturating_sub(2);

        // Header Action Buttons: row 0
        if inner_y == 0 || content_y == 0 {
            let cancel_x = inner_w.saturating_sub(15);
            let save_x = cancel_x.saturating_sub(14);
            if inner_x >= save_x && inner_x < cancel_x {
                return Some(SettingsAction::SaveAndApply);
            }
            if inner_x >= cancel_x {
                return Some(SettingsAction::Close);
            }
        }

        let theme_rows = layout_theme_chips(inner_w);
        let theme_row_count = theme_rows.len() as u16;
        let offset = theme_row_count.saturating_sub(1);

        // Theme selection
        let theme_start_y = 2;
        let theme_end_y = theme_start_y + theme_row_count;
        if content_y >= theme_start_y && content_y < theme_end_y {
            let row_idx = (content_y - theme_start_y) as usize;
            if let Some(row) = theme_rows.get(row_idx) {
                for &(tid, _, start_x, end_x) in row {
                    if inner_x >= start_x && inner_x < end_x {
                        self.theme = tid;
                        self.focused_nav = SettingsNavFocus::Theme;
                        return None;
                    }
                }
            }
        }

        // Language & Nerd Font
        if content_y == 3 + offset {
            // [ English ] (10..23), [ 日本語 ] (23..34)
            if (10..23).contains(&inner_x) {
                self.language = Language::En;
                self.focused_nav = SettingsNavFocus::Language;
            } else if (23..34).contains(&inner_x) {
                self.language = Language::Ja;
                self.focused_nav = SettingsNavFocus::Language;
            }
            // Nerd Font: [ ON ] (48..60), [ OFF ] (60..72)
            if (48..60).contains(&inner_x) {
                self.use_nerd_font = true;
                self.focused_nav = SettingsNavFocus::NerdFont;
            } else if (60..72).contains(&inner_x) {
                self.use_nerd_font = false;
                self.focused_nav = SettingsNavFocus::NerdFont;
            }
        }

        // AI Provider Chips
        if content_y == 5 + offset {
            let providers = [
                (AiProvider::OpenAi, "OpenAI"),
                (AiProvider::Anthropic, "Anthropic"),
                (AiProvider::Gemini, "Gemini"),
                (AiProvider::Ollama, "Ollama"),
                (AiProvider::Custom, "Custom"),
            ];
            let mut cur_x = 10;
            for (prov, label) in providers {
                let w = label.len() as u16 + 5;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_provider = prov;
                    self.ai_model = prov.default_model().to_string();
                    self.focused_nav = SettingsNavFocus::AiProvider;
                    return None;
                }
                cur_x += w + 1;
            }
        }

        // Model Presets
        if content_y == 7 + offset {
            let presets: &[&str] = match self.ai_provider {
                AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "o3-mini"],
                AiProvider::Anthropic => {
                    &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"]
                }
                AiProvider::Gemini => &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"],
                AiProvider::Ollama => &["CyberTielCorder:35B-A3B", "bonsai-2:27b", "codellama"],
                AiProvider::Custom => &["default", "local-model"],
            };
            let mut cur_x = 10;
            for model_name in presets {
                let w = model_name.len() as u16 + 4;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_model = model_name.to_string();
                    self.focused_nav = SettingsNavFocus::AiModelPresets;
                    return None;
                }
                cur_x += w + 1;
            }
        }

        // Model Input
        if content_y == 8 + offset && inner_x >= 10 && inner_x < inner_w.saturating_sub(4) {
            self.focused_input = Some(SettingsInputFocus::AiModel);
            self.focused_nav = SettingsNavFocus::AiModelInput;
            return None;
        }

        // API Key Input & Buttons
        if content_y == 9 + offset {
            if (36..48).contains(&inner_x) {
                self.show_api_key = !self.show_api_key;
                self.focused_nav = SettingsNavFocus::AiApiKeyInput;
                return None;
            }
            if (48..63).contains(&inner_x) {
                self.ai_api_key.clear();
                self.focused_nav = SettingsNavFocus::AiApiKeyInput;
                return None;
            }
            if (10..36).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::AiApiKey);
                self.focused_nav = SettingsNavFocus::AiApiKeyInput;
                return None;
            }
        }

        // Endpoint Base URL & Reset Default
        if content_y == 10 + offset {
            if (46..63).contains(&inner_x) {
                self.ai_api_base.clear();
                self.focused_nav = SettingsNavFocus::AiApiBaseInput;
                return None;
            }
            if (10..46).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::AiApiBase);
                self.focused_nav = SettingsNavFocus::AiApiBaseInput;
                return None;
            }
        }

        // Git Soft Discard & Auto Fetch (row 12 + offset)
        if content_y == 12 + offset {
            if (14..22).contains(&inner_x) {
                self.git_soft_discard = true;
                self.focused_nav = SettingsNavFocus::GitSoftDiscard;
            } else if (22..31).contains(&inner_x) {
                self.git_soft_discard = false;
                self.focused_nav = SettingsNavFocus::GitSoftDiscard;
            }
            if (48..57).contains(&inner_x) {
                self.git_auto_fetch_secs = 0;
                self.focused_nav = SettingsNavFocus::GitAutoFetch;
            } else if (57..66).contains(&inner_x) {
                self.git_auto_fetch_secs = 30;
                self.focused_nav = SettingsNavFocus::GitAutoFetch;
            } else if (66..76).contains(&inner_x) {
                self.git_auto_fetch_secs = 60;
                self.focused_nav = SettingsNavFocus::GitAutoFetch;
            }
            // Compatibility for old single-line layout Restore Repo click
            if (85..98).contains(&inner_x) {
                self.git_restore_last_repo = true;
                self.focused_nav = SettingsNavFocus::GitRestoreRepo;
            } else if (98..110).contains(&inner_x) {
                self.git_restore_last_repo = false;
                self.focused_nav = SettingsNavFocus::GitRestoreRepo;
            }
        }

        // Git Restore Repo (row 13 + offset)
        if content_y == 13 + offset {
            if (14..22).contains(&inner_x) {
                self.git_restore_last_repo = true;
                self.focused_nav = SettingsNavFocus::GitRestoreRepo;
            } else if (22..31).contains(&inner_x) {
                self.git_restore_last_repo = false;
                self.focused_nav = SettingsNavFocus::GitRestoreRepo;
            }
        }

        // Editor Command (row 14 + offset or 13 + offset)
        if content_y == 14 + offset || content_y == 13 + offset {
            if (38..48).contains(&inner_x) {
                self.editor_command = "nvim".to_string();
                self.focused_nav = SettingsNavFocus::EditorCommand;
                return None;
            } else if (48..58).contains(&inner_x) {
                self.editor_command = "code".to_string();
                self.focused_nav = SettingsNavFocus::EditorCommand;
                return None;
            } else if (58..68).contains(&inner_x) {
                self.editor_command = "nano".to_string();
                self.focused_nav = SettingsNavFocus::EditorCommand;
                return None;
            }
            if (10..30).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::EditorCommand);
                self.focused_nav = SettingsNavFocus::EditorCommand;
                return None;
            }
        }

        // Bottom Action Bar
        if content_y >= 15 + offset || inner_y >= rect.height.saturating_sub(4) {
            if (1..38).contains(&inner_x) {
                return Some(SettingsAction::SaveAndApply);
            }
            if (38..65).contains(&inner_x) {
                return Some(SettingsAction::Close);
            }
        }

        None
    }

    /// Render settings modal onto frame.
    pub fn render(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        icons: &Icons,
        _i18n: &I18n,
        viewport: Rect,
    ) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);

        // 1. Drop Shadow Effect
        if viewport.width > rect.width + 1 && viewport.height > rect.height + 1 {
            let shadow_rect = Rect {
                x: (rect.x + 1).min(viewport.width.saturating_sub(1)),
                y: (rect.y + 1).min(viewport.height.saturating_sub(1)),
                width: rect.width.min(viewport.width.saturating_sub(rect.x + 1)),
                height: rect.height.min(viewport.height.saturating_sub(rect.y + 1)),
            };
            let shadow_block =
                Block::default().style(Style::default().bg(ratatui::style::Color::Rgb(12, 14, 20)));
            frame.render_widget(shadow_block, shadow_rect);
        }

        // 2. Clear background for modal window
        frame.render_widget(Clear, rect);

        let inner_height = rect.height.saturating_sub(2);
        let inner_w = rect.width.saturating_sub(2);
        let theme_rows = layout_theme_chips(inner_w);
        let offset = (theme_rows.len() as u16).saturating_sub(1);
        let total_content_lines = 18 + offset;
        let scroll_hint = if inner_height < total_content_lines {
            " [ ▲▼ Scroll: j/k ]".to_string()
        } else {
            String::new()
        };
        let title = format!(" {} SETTINGS / 設定{} ", icons.settings(), scroll_hint);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        let card_header = |icon_and_title: &str, is_top: bool| -> Line<'static> {
            let prefix = if is_top { "╭─ " } else { "├─ " };
            let title_len = UnicodeWidthStr::width(icon_and_title);
            let prefix_w = UnicodeWidthStr::width(prefix);
            let total_w = inner.width as usize;
            let bar_len = total_w.saturating_sub(prefix_w + title_len + 4);
            let bar = "─".repeat(bar_len);
            let suffix = if is_top { "╮" } else { "┤" };
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.primary)),
                Span::styled(
                    icon_and_title.to_string(),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {} ", bar), Style::default().fg(theme.border)),
                Span::styled(suffix, Style::default().fg(theme.border)),
            ])
        };

        let card_footer = || -> Line<'static> {
            let total_w = inner.width as usize;
            let bar = "─".repeat(total_w.saturating_sub(2));
            Line::from(vec![
                Span::styled("╰", Style::default().fg(theme.border)),
                Span::styled(bar, Style::default().fg(theme.border)),
                Span::styled("╯", Style::default().fg(theme.border)),
            ])
        };

        let cursor_span = |is_cur: bool| -> Span<'static> {
            if is_cur {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            }
        };

        let is_hovered = |x_start: u16, x_end: u16, y: u16| -> bool {
            if let Some((mx, my)) = self.hover_pos {
                if mx >= inner.x
                    && mx < inner.x + inner.width
                    && my >= inner.y
                    && my < inner.y + inner.height
                {
                    let rx = mx - inner.x;
                    let cy = (my - inner.y) + self.scroll_offset;
                    cy == y && rx >= x_start && rx < x_end
                } else {
                    false
                }
            } else {
                false
            }
        };

        let mut lines = Vec::new();

        // 1. Header action buttons
        let title_w = UnicodeWidthStr::width("Preferences & AI Integration");
        let save_w = UnicodeWidthStr::width(" [ 💾 Save ] ");
        let cancel_w = UnicodeWidthStr::width(" [ ❌ Cancel ] ");
        let pad = (inner.width as usize).saturating_sub(title_w + save_w + cancel_w + 1);

        let is_save_hovered = is_hovered(pad as u16, (pad + save_w) as u16, 0);
        let save_style = if is_save_hovered {
            Style::default()
                .bg(theme.border_focused)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        };

        let is_cancel_hovered = is_hovered(
            (pad + save_w + 1) as u16,
            (pad + save_w + 1 + cancel_w) as u16,
            0,
        );
        let cancel_style = if is_cancel_hovered {
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            Span::styled(
                "Preferences & AI Integration",
                Style::default().fg(theme.dim_fg),
            ),
            Span::raw(" ".repeat(pad)),
            Span::styled(" [ 💾 Save ] ", save_style),
            Span::raw(" "),
            Span::styled(" [ ❌ Cancel ] ", cancel_style),
        ]));

        lines.push(card_header(
            "🎨 APPEARANCE & DISPLAY / 外観・基本設定",
            true,
        ));

        // 2. Theme rows
        let is_theme_nav = self.focused_nav == SettingsNavFocus::Theme;
        for (r_idx, row) in theme_rows.iter().enumerate() {
            let mut theme_spans = vec![
                if r_idx == 0 {
                    cursor_span(is_theme_nav)
                } else {
                    Span::raw("  ")
                },
                if r_idx == 0 {
                    Span::styled(
                        "Theme:    ",
                        if is_theme_nav {
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.primary)
                        },
                    )
                } else {
                    Span::raw("          ")
                },
            ];
            for &(tid, name, start_x, end_x) in row {
                let is_sel = self.theme == tid;
                let is_chip_hovered = is_hovered(start_x, end_x, 1 + r_idx as u16);
                let style = if is_sel && is_theme_nav {
                    Style::default()
                        .bg(theme.accent)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_sel {
                    Style::default()
                        .bg(theme.primary)
                        .fg(theme.bg)
                        .add_modifier(Modifier::BOLD)
                } else if is_chip_hovered {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.fg)
                };
                theme_spans.push(Span::styled(format!(" [ {} ] ", name), style));
                theme_spans.push(Span::raw(" "));
            }
            lines.push(Line::from(theme_spans));
        }

        // 3. Language & Nerd Font row
        let is_lang_nav = self.focused_nav == SettingsNavFocus::Language;
        let is_nf_nav = self.focused_nav == SettingsNavFocus::NerdFont;

        let lang_en_style = if self.language == Language::En {
            Style::default()
                .bg(if is_lang_nav {
                    theme.accent
                } else {
                    theme.primary
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let lang_ja_style = if self.language == Language::Ja {
            Style::default()
                .bg(if is_lang_nav {
                    theme.accent
                } else {
                    theme.primary
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        let nf_on_style = if self.use_nerd_font {
            Style::default()
                .bg(if is_nf_nav {
                    theme.accent
                } else {
                    theme.success
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let nf_off_style = if !self.use_nerd_font {
            Style::default()
                .bg(if is_nf_nav {
                    theme.accent
                } else {
                    theme.warning
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            cursor_span(is_lang_nav || is_nf_nav),
            Span::styled(
                "Lang:     ",
                if is_lang_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                },
            ),
            Span::styled(" [ English ] ", lang_en_style),
            Span::styled(" [ 日本語 ] ", lang_ja_style),
            Span::raw("    "),
            Span::styled(
                "Nerd Font: ",
                if is_nf_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.secondary)
                },
            ),
            Span::styled(" [ ON ] ", nf_on_style),
            Span::styled(" [ OFF ] ", nf_off_style),
        ]));

        lines.push(card_header("󱚣 AI CO-PILOT INTEGRATION / AI連携", false));

        // 4. AI Provider row
        let is_prov_nav = self.focused_nav == SettingsNavFocus::AiProvider;
        let providers = [
            (AiProvider::OpenAi, "OpenAI"),
            (AiProvider::Anthropic, "Anthropic"),
            (AiProvider::Gemini, "Gemini"),
            (AiProvider::Ollama, "Ollama"),
            (AiProvider::Custom, "Custom"),
        ];
        let mut prov_spans = vec![
            cursor_span(is_prov_nav),
            Span::styled(
                "Provider: ",
                if is_prov_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.ai_sparkle)
                },
            ),
        ];
        for (p, name) in providers {
            let is_sel = self.ai_provider == p;
            let style = if is_sel {
                Style::default()
                    .bg(if is_prov_nav {
                        theme.accent
                    } else {
                        theme.ai_sparkle
                    })
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            prov_spans.push(Span::styled(format!(" [ {} ]", name), style));
            prov_spans.push(Span::raw(" "));
        }
        lines.push(Line::from(prov_spans));

        // 5. Connection Status Badge
        let has_key = !self.ai_api_key.trim().is_empty();
        let is_connected = match self.ai_provider {
            AiProvider::OpenAi | AiProvider::Anthropic | AiProvider::Gemini => has_key,
            AiProvider::Ollama | AiProvider::Custom => true,
        };
        let (conn_icon, conn_text, conn_color) = if is_connected {
            (icons.check_success(), "Connected & Ready", theme.success)
        } else {
            (
                icons.get(crate::ui::IconId::CheckWarning),
                "Disconnected (API Key required)",
                theme.warning,
            )
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Status:   ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                format!("{} {}", conn_icon, conn_text),
                Style::default().fg(conn_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        // 6. Model Presets
        let is_preset_nav = self.focused_nav == SettingsNavFocus::AiModelPresets;
        let presets: &[&str] = match self.ai_provider {
            AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "o3-mini"],
            AiProvider::Anthropic => &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"],
            AiProvider::Gemini => &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"],
            AiProvider::Ollama => &["CyberTielCorder:35B-A3B", "bonsai-2:27b", "codellama"],
            AiProvider::Custom => &["default", "local-model"],
        };
        let mut model_chips = vec![
            cursor_span(is_preset_nav),
            Span::styled(
                "Presets:  ",
                if is_preset_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.dim_fg)
                },
            ),
        ];
        for &m in presets {
            let is_active = self.ai_model == m;
            let style = if is_active {
                Style::default()
                    .bg(if is_preset_nav {
                        theme.accent
                    } else {
                        theme.info
                    })
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.dim_fg)
            };
            model_chips.push(Span::styled(format!(" [ {} ]", m), style));
            model_chips.push(Span::raw(" "));
        }
        lines.push(Line::from(model_chips));

        // 7. Custom Model Input
        let is_model_nav = self.focused_nav == SettingsNavFocus::AiModelInput;
        let is_model_editing = self.focused_input == Some(SettingsInputFocus::AiModel);
        let model_focus_style = if is_model_editing {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_model_nav {
            Style::default()
                .bg(theme.header_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        lines.push(Line::from(vec![
            cursor_span(is_model_nav),
            Span::styled(
                "Model:    ",
                if is_model_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                },
            ),
            Span::styled(format!(" {:<24} ", self.ai_model), model_focus_style),
            Span::styled(
                if is_model_editing {
                    " (Typing... Press Enter to finish)"
                } else {
                    " (Enter to edit)"
                },
                Style::default().fg(theme.dim_fg),
            ),
        ]));

        // 8. API Key Input
        let is_key_nav = self.focused_nav == SettingsNavFocus::AiApiKeyInput;
        let is_key_editing = self.focused_input == Some(SettingsInputFocus::AiApiKey);
        let key_focus_style = if is_key_editing {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_key_nav {
            Style::default()
                .bg(theme.header_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        let displayed_key = if self.show_api_key {
            self.ai_api_key.clone()
        } else if self.ai_api_key.is_empty() {
            if self.ai_provider == AiProvider::Ollama {
                "(Not required for Ollama)".to_string()
            } else {
                "(No API Key configured)".to_string()
            }
        } else {
            let len = self.ai_api_key.len();
            format!("•••••••••••• ({})", len)
        };

        lines.push(Line::from(vec![
            cursor_span(is_key_nav),
            Span::styled(
                "API Key:  ",
                if is_key_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                },
            ),
            Span::styled(format!(" {:<22} ", displayed_key), key_focus_style),
            Span::styled(
                if self.show_api_key {
                    " [ Hide ] "
                } else {
                    " [ Show ] "
                },
                Style::default().fg(theme.secondary),
            ),
            Span::styled(" [ 󰆴 Clear ] ", Style::default().fg(theme.error)),
        ]));

        // 9. Custom Base URL
        let is_base_nav = self.focused_nav == SettingsNavFocus::AiApiBaseInput;
        let is_base_editing = self.focused_input == Some(SettingsInputFocus::AiApiBase);
        let base_focus_style = if is_base_editing {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_base_nav {
            Style::default()
                .bg(theme.header_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        let displayed_base = if self.ai_api_base.is_empty() {
            format!("(Default: {})", self.ai_provider.default_endpoint())
        } else {
            self.ai_api_base.clone()
        };
        lines.push(Line::from(vec![
            cursor_span(is_base_nav),
            Span::styled(
                "Base URL: ",
                if is_base_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                },
            ),
            Span::styled(format!(" {:<28} ", displayed_base), base_focus_style),
            Span::styled(" [ Default ] ", Style::default().fg(theme.dim_fg)),
        ]));

        lines.push(card_header("󰊢 GIT ENGINE & TOOLS / Git・外部ツール", false));

        // 10. Git Preferences Row 1: Soft Discard & Auto-Fetch
        let is_soft_nav = self.focused_nav == SettingsNavFocus::GitSoftDiscard;
        let is_af_nav = self.focused_nav == SettingsNavFocus::GitAutoFetch;

        let soft_on = if self.git_soft_discard {
            Style::default()
                .bg(if is_soft_nav {
                    theme.accent
                } else {
                    theme.success
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let soft_off = if !self.git_soft_discard {
            Style::default()
                .bg(if is_soft_nav {
                    theme.accent
                } else {
                    theme.warning
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        let af_off = if self.git_auto_fetch_secs == 0 {
            Style::default()
                .bg(if is_af_nav { theme.accent } else { theme.info })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_30 = if self.git_auto_fetch_secs == 30 {
            Style::default()
                .bg(if is_af_nav { theme.accent } else { theme.info })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_60 = if self.git_auto_fetch_secs == 60 {
            Style::default()
                .bg(if is_af_nav { theme.accent } else { theme.info })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            cursor_span(is_soft_nav || is_af_nav),
            Span::styled(
                "Soft Discard: ",
                if is_soft_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.secondary)
                },
            ),
            Span::styled(" [ ON ] ", soft_on),
            Span::styled(" [ OFF ] ", soft_off),
            Span::raw("  "),
            Span::styled(
                "Auto-Fetch: ",
                if is_af_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.secondary)
                },
            ),
            Span::styled(" [ Off ] ", af_off),
            Span::styled(" [ 30s ] ", af_30),
            Span::styled(" [ 60s ] ", af_60),
        ]));

        // 11. Git Preferences Row 2: Restore Repo (Isolated row to prevent right-edge clipping)
        let is_rest_nav = self.focused_nav == SettingsNavFocus::GitRestoreRepo;
        let rest_on = if self.git_restore_last_repo {
            Style::default()
                .bg(if is_rest_nav {
                    theme.accent
                } else {
                    theme.success
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let rest_off = if !self.git_restore_last_repo {
            Style::default()
                .bg(if is_rest_nav {
                    theme.accent
                } else {
                    theme.warning
                })
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            cursor_span(is_rest_nav),
            Span::styled(
                "Restore Repo: ",
                if is_rest_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.secondary)
                },
            ),
            Span::styled(" [ ON ] ", rest_on),
            Span::styled(" [ OFF ] ", rest_off),
            Span::raw("    "),
            Span::styled(
                "(Restore last opened repo on startup)",
                Style::default().fg(theme.dim_fg),
            ),
        ]));

        // 12. Editor Preferences
        let is_ed_nav = self.focused_nav == SettingsNavFocus::EditorCommand;
        let is_ed_editing = self.focused_input == Some(SettingsInputFocus::EditorCommand);
        let editor_focus_style = if is_ed_editing {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if is_ed_nav {
            Style::default()
                .bg(theme.header_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        let disp_editor = if self.editor_command.is_empty() {
            "($EDITOR)".to_string()
        } else {
            self.editor_command.clone()
        };
        lines.push(Line::from(vec![
            cursor_span(is_ed_nav),
            Span::styled(
                "Editor:   ",
                if is_ed_nav {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.primary)
                },
            ),
            Span::styled(format!(" {:<14} ", disp_editor), editor_focus_style),
            Span::styled(" Presets: ", Style::default().fg(theme.dim_fg)),
            Span::styled(" [ nvim ] ", Style::default().fg(theme.secondary)),
            Span::styled(" [ code ] ", Style::default().fg(theme.secondary)),
            Span::styled(" [ nano ] ", Style::default().fg(theme.secondary)),
        ]));

        lines.push(card_footer());

        // 13. Bottom Action Buttons
        let is_save_focused = self.focused_nav == SettingsNavFocus::SaveButton;
        let is_cancel_focused = self.focused_nav == SettingsNavFocus::CancelButton;

        let bottom_save_style = if is_save_focused {
            Style::default()
                .bg(theme.border_focused)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        };
        let bottom_cancel_style = if is_cancel_focused {
            Style::default()
                .bg(theme.error)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.status_bg).fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            cursor_span(is_save_focused || is_cancel_focused),
            Span::styled(" [ 💾 Save & Apply to config.toml ] ", bottom_save_style),
            Span::raw("  "),
            Span::styled(" [ ❌ Cancel (Esc) ] ", bottom_cancel_style),
        ]));

        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "Hint: ↑↓/Tab: Move • Space/←→: Change • Enter: Edit/Save • Ctrl+S: Save",
                Style::default().fg(theme.dim_fg),
            ),
        ]));

        frame.render_widget(Paragraph::new(lines).scroll((self.scroll_offset, 0)), inner);

        // 14. Terminal Hardware Cursor Positioning for Text Input
        if let Some(focus) = self.focused_input {
            let row_y = self.item_row_y(
                match focus {
                    SettingsInputFocus::AiModel => SettingsNavFocus::AiModelInput,
                    SettingsInputFocus::AiApiKey => SettingsNavFocus::AiApiKeyInput,
                    SettingsInputFocus::AiApiBase => SettingsNavFocus::AiApiBaseInput,
                    SettingsInputFocus::EditorCommand => SettingsNavFocus::EditorCommand,
                },
                inner.width,
            );

            if row_y >= self.scroll_offset && row_y < self.scroll_offset + inner.height {
                let screen_y = inner.y + (row_y - self.scroll_offset);
                // cursor starts after "▶ Label:    " (12 cols) + 1 space padding
                let base_x = inner.x + 13;
                let cur_x = match focus {
                    SettingsInputFocus::AiModel => {
                        let text_w = UnicodeWidthStr::width(self.ai_model.as_str()) as u16;
                        base_x + text_w.min(24)
                    }
                    SettingsInputFocus::AiApiKey => {
                        let disp_len = if self.show_api_key {
                            self.ai_api_key.len() as u16
                        } else {
                            0
                        };
                        base_x + disp_len.min(22)
                    }
                    SettingsInputFocus::AiApiBase => {
                        let text_w = UnicodeWidthStr::width(self.ai_api_base.as_str()) as u16;
                        base_x + text_w.min(28)
                    }
                    SettingsInputFocus::EditorCommand => {
                        let text_w = UnicodeWidthStr::width(self.editor_command.as_str()) as u16;
                        base_x + text_w.min(14)
                    }
                };
                let clamped_x = cur_x.min(inner.x + inner.width.saturating_sub(1));
                frame.set_cursor_position((clamped_x, screen_y));
            }
        }
    }
}
