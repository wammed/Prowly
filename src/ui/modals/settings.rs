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
        let total_lines = 16 + offset;
        let max_scroll = total_lines.saturating_sub(inner_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
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

    /// Handle text input when a text field is focused.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SettingsAction> {
        match key.code {
            KeyCode::Esc => {
                if self.focused_input.is_some() {
                    self.focused_input = None;
                    None
                } else {
                    Some(SettingsAction::Close)
                }
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL) || self.focused_input.is_none() {
                    Some(SettingsAction::SaveAndApply)
                } else {
                    self.focused_input = None;
                    None
                }
            }
            KeyCode::Up | KeyCode::Char('k') if self.focused_input.is_none() => {
                self.scroll_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') if self.focused_input.is_none() => {
                self.scroll_offset = self.scroll_offset.saturating_add(1).min(18);
                None
            }
            KeyCode::PageUp if self.focused_input.is_none() => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                None
            }
            KeyCode::PageDown if self.focused_input.is_none() => {
                self.scroll_offset = self.scroll_offset.saturating_add(5).min(18);
                None
            }
            KeyCode::Tab => {
                self.focused_input = match self.focused_input {
                    None => Some(SettingsInputFocus::AiModel),
                    Some(SettingsInputFocus::AiModel) => Some(SettingsInputFocus::AiApiKey),
                    Some(SettingsInputFocus::AiApiKey) => Some(SettingsInputFocus::AiApiBase),
                    Some(SettingsInputFocus::AiApiBase) => Some(SettingsInputFocus::EditorCommand),
                    Some(SettingsInputFocus::EditorCommand) => None,
                };
                None
            }
            KeyCode::Backspace => {
                match self.focused_input {
                    Some(SettingsInputFocus::AiModel) => {
                        self.ai_model.pop();
                    }
                    Some(SettingsInputFocus::AiApiKey) => {
                        self.ai_api_key.pop();
                    }
                    Some(SettingsInputFocus::AiApiBase) => {
                        self.ai_api_base.pop();
                    }
                    Some(SettingsInputFocus::EditorCommand) => {
                        self.editor_command.pop();
                    }
                    None => {}
                }
                None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                match self.focused_input {
                    Some(SettingsInputFocus::AiModel) => self.ai_model.push(c),
                    Some(SettingsInputFocus::AiApiKey) => self.ai_api_key.push(c),
                    Some(SettingsInputFocus::AiApiBase) => self.ai_api_base.push(c),
                    Some(SettingsInputFocus::EditorCommand) => self.editor_command.push(c),
                    None => {}
                }
                None
            }
            _ => None,
        }
    }

    /// Calculate modal bounding box in viewport.
    /// Adapts generously to larger terminal sizes up to 96 columns wide and 34 rows high.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let ideal_w = (viewport.width * 88 / 100).clamp(76, 96);
        let width = ideal_w.min(viewport.width.saturating_sub(2));

        let ideal_h = (viewport.height * 92 / 100).clamp(24, 34);
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

        // Header Action Buttons (Row 0 or Screen Top): [ 💾 Save ] and [ ❌ Cancel ]
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

        // Row 2 .. 2 + theme_row_count: Theme selection
        let theme_start_y = 2;
        let theme_end_y = theme_start_y + theme_row_count;
        if content_y >= theme_start_y && content_y < theme_end_y {
            let row_idx = (content_y - theme_start_y) as usize;
            if let Some(row) = theme_rows.get(row_idx) {
                for &(tid, _, start_x, end_x) in row {
                    if inner_x >= start_x && inner_x < end_x {
                        self.theme = tid;
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
            } else if (23..34).contains(&inner_x) {
                self.language = Language::Ja;
            }
            // Nerd Font: [ ON ] (52..60), [ OFF ] (60..69)
            if (52..60).contains(&inner_x) {
                self.use_nerd_font = true;
            } else if (60..69).contains(&inner_x) {
                self.use_nerd_font = false;
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
                let w = label.len() as u16 + 6;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_provider = prov;
                    self.ai_model = prov.default_model().to_string();
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
                AiProvider::Ollama => &["llama3.2:latest", "deepseek-r1:latest", "codellama"],
                AiProvider::Custom => &["default", "local-model"],
            };
            let mut cur_x = 10;
            for model_name in presets {
                let w = model_name.len() as u16 + 6;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_model = model_name.to_string();
                    return None;
                }
                cur_x += w + 1;
            }
        }

        // Model Input
        if content_y == 8 + offset && inner_x >= 10 && inner_x < inner_w.saturating_sub(4) {
            self.focused_input = Some(SettingsInputFocus::AiModel);
            return None;
        }

        // API Key Input & Buttons
        if content_y == 9 + offset {
            // [ Show / Hide ] (40..50), [ Clear ] (50..63)
            if (40..50).contains(&inner_x) {
                self.show_api_key = !self.show_api_key;
                return None;
            }
            if (50..63).contains(&inner_x) {
                self.ai_api_key.clear();
                return None;
            }
            if (10..40).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::AiApiKey);
                return None;
            }
        }

        // Endpoint Base URL & Reset Default
        if content_y == 10 + offset {
            // [ Default ] (50..63)
            if (50..63).contains(&inner_x) {
                self.ai_api_base.clear();
                return None;
            }
            if (10..50).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::AiApiBase);
                return None;
            }
        }

        // Git Soft Discard & Auto Fetch
        if content_y == 12 + offset {
            // Soft Discard: [ ON ] (14..22), [ OFF ] (22..31)
            if (14..22).contains(&inner_x) {
                self.git_soft_discard = true;
            } else if (22..31).contains(&inner_x) {
                self.git_soft_discard = false;
            }
            // Auto Fetch: [ Off ] (48..57), [ 30s ] (57..66), [ 60s ] (66..75)
            if (48..57).contains(&inner_x) {
                self.git_auto_fetch_secs = 0;
            } else if (57..66).contains(&inner_x) {
                self.git_auto_fetch_secs = 30;
            } else if (66..75).contains(&inner_x) {
                self.git_auto_fetch_secs = 60;
            }
            // Restore Repo: [ ON ] (90..98), [ OFF ] (98..107)
            if (90..98).contains(&inner_x) {
                self.git_restore_last_repo = true;
            } else if (98..107).contains(&inner_x) {
                self.git_restore_last_repo = false;
            }
        }

        // Editor Command
        if content_y == 13 + offset {
            // Presets: [ nvim ] (40..50), [ code ] (50..60), [ nano ] (60..70)
            if (40..50).contains(&inner_x) {
                self.editor_command = "nvim".to_string();
                return None;
            } else if (50..60).contains(&inner_x) {
                self.editor_command = "code".to_string();
                return None;
            } else if (60..70).contains(&inner_x) {
                self.editor_command = "nano".to_string();
                return None;
            }
            if (10..30).contains(&inner_x) {
                self.focused_input = Some(SettingsInputFocus::EditorCommand);
                return None;
            }
        }

        // Bottom Action Bar: content_y == 15 + offset or inner_y near bottom
        if content_y == 15 + offset || inner_y >= rect.height.saturating_sub(4) {
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

        // 1. Drop Shadow Effect (Pseudo-3D elevation)
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
        let total_content_lines = 16 + offset;
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

        let mut lines = Vec::new();

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
        for (r_idx, row) in theme_rows.iter().enumerate() {
            let mut theme_spans = vec![if r_idx == 0 {
                Span::styled("Theme:    ", Style::default().fg(theme.primary))
            } else {
                Span::raw(" ".repeat(10))
            }];
            for &(tid, name, start_x, end_x) in row {
                let is_sel = self.theme == tid;
                let is_chip_hovered = is_hovered(start_x, end_x, 1 + r_idx as u16);
                let style = if is_sel {
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
        let is_en_hovered = is_hovered(10, 23, 2 + offset);
        let is_ja_hovered = is_hovered(23, 36, 2 + offset);
        let is_nf_on_hovered = is_hovered(54, 62, 2 + offset);
        let is_nf_off_hovered = is_hovered(62, 71, 2 + offset);

        let lang_en_style = if self.language == Language::En {
            Style::default()
                .bg(theme.primary)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_en_hovered {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let lang_ja_style = if self.language == Language::Ja {
            Style::default()
                .bg(theme.primary)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_ja_hovered {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        let nf_on_style = if self.use_nerd_font {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_nf_on_hovered {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let nf_off_style = if !self.use_nerd_font {
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_nf_off_hovered {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            Span::styled("Lang:     ", Style::default().fg(theme.primary)),
            Span::styled(" [ English ] ", lang_en_style),
            Span::styled(" [ 日本語 ] ", lang_ja_style),
            Span::raw("       "),
            Span::styled("Nerd Font: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ ON ] ", nf_on_style),
            Span::styled(" [ OFF ] ", nf_off_style),
        ]));

        lines.push(card_header("󱚣 AI CO-PILOT INTEGRATION / AI連携", false));

        // 4. AI Provider row
        let providers = [
            (AiProvider::OpenAi, "OpenAI"),
            (AiProvider::Anthropic, "Anthropic"),
            (AiProvider::Gemini, "Gemini"),
            (AiProvider::Ollama, "Ollama"),
            (AiProvider::Custom, "Custom"),
        ];
        let mut prov_spans = vec![Span::styled(
            "Provider: ",
            Style::default().fg(theme.ai_sparkle),
        )];
        for (p, name) in providers {
            let is_sel = self.ai_provider == p;
            let style = if is_sel {
                Style::default()
                    .bg(theme.ai_sparkle)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            prov_spans.push(Span::styled(format!(" [ {} ] ", name), style));
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
            Span::styled("Status:   ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                format!("{} {}", conn_icon, conn_text),
                Style::default().fg(conn_color).add_modifier(Modifier::BOLD),
            ),
        ]));

        // 6. Model Presets & Custom Input
        let presets: &[&str] = match self.ai_provider {
            AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "o3-mini"],
            AiProvider::Anthropic => &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"],
            AiProvider::Gemini => &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"],
            AiProvider::Ollama => &["llama3.2:latest", "deepseek-r1:latest", "codellama"],
            AiProvider::Custom => &["default", "local-model"],
        };
        let mut model_chips = vec![Span::styled(
            "Presets:  ",
            Style::default().fg(theme.dim_fg),
        )];
        for &m in presets {
            let is_active = self.ai_model == m;
            let style = if is_active {
                Style::default()
                    .bg(theme.info)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.dim_fg)
            };
            model_chips.push(Span::styled(format!(" [ {} ] ", m), style));
            model_chips.push(Span::raw(" "));
        }
        lines.push(Line::from(model_chips));

        let model_focus_style = if self.focused_input == Some(SettingsInputFocus::AiModel) {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        lines.push(Line::from(vec![
            Span::styled("Model:    ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<30} ", self.ai_model), model_focus_style),
            Span::styled(" (Click or Tab to edit)", Style::default().fg(theme.dim_fg)),
        ]));

        // 7. API Key Input
        let key_focus_style = if self.focused_input == Some(SettingsInputFocus::AiApiKey) {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
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
            format!("•••••••••••••••• ({})", len)
        };

        lines.push(Line::from(vec![
            Span::styled("API Key:  ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<28} ", displayed_key), key_focus_style),
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

        // 8. Custom Base URL
        let base_focus_style = if self.focused_input == Some(SettingsInputFocus::AiApiBase) {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
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
            Span::styled("Base URL: ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<38} ", displayed_base), base_focus_style),
            Span::styled(" [ Default ] ", Style::default().fg(theme.dim_fg)),
        ]));

        lines.push(card_header("󰊢 GIT ENGINE & TOOLS / Git・外部ツール", false));

        // 9. Git Engine Preferences
        let is_soft_on_hovered = is_hovered(14, 22, 12 + offset);
        let is_soft_off_hovered = is_hovered(22, 31, 12 + offset);
        let is_af_off_hovered = is_hovered(48, 57, 12 + offset);
        let is_af_30_hovered = is_hovered(57, 66, 12 + offset);
        let is_af_60_hovered = is_hovered(66, 75, 12 + offset);

        let soft_on = if self.git_soft_discard {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_soft_on_hovered {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let soft_off = if !self.git_soft_discard {
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_soft_off_hovered {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_off = if self.git_auto_fetch_secs == 0 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_af_off_hovered {
            Style::default()
                .fg(theme.info)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_30 = if self.git_auto_fetch_secs == 30 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_af_30_hovered {
            Style::default()
                .fg(theme.info)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_60 = if self.git_auto_fetch_secs == 60 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_af_60_hovered {
            Style::default()
                .fg(theme.info)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        let is_rest_on_hovered = is_hovered(90, 98, 12 + offset);
        let is_rest_off_hovered = is_hovered(98, 107, 12 + offset);
        let rest_on = if self.git_restore_last_repo {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_rest_on_hovered {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let rest_off = if !self.git_restore_last_repo {
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else if is_rest_off_hovered {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            Span::styled("Soft Discard: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ ON ] ", soft_on),
            Span::styled(" [ OFF ] ", soft_off),
            Span::raw("   "),
            Span::styled("Auto-Fetch: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ Off ] ", af_off),
            Span::styled(" [ 30s ] ", af_30),
            Span::styled(" [ 60s ] ", af_60),
            Span::raw("   "),
            Span::styled("Restore Repo: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ ON ] ", rest_on),
            Span::styled(" [ OFF ] ", rest_off),
        ]));

        // 10. Editor Preferences
        let editor_focus_style = if self.focused_input == Some(SettingsInputFocus::EditorCommand) {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
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
            Span::styled("Editor:   ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<18} ", disp_editor), editor_focus_style),
            Span::styled(" Presets: ", Style::default().fg(theme.dim_fg)),
            Span::styled(" [ nvim ] ", Style::default().fg(theme.secondary)),
            Span::styled(" [ code ] ", Style::default().fg(theme.secondary)),
            Span::styled(" [ nano ] ", Style::default().fg(theme.secondary)),
        ]));

        lines.push(card_footer());

        // 11. Bottom Action Buttons
        lines.push(Line::from(vec![
            Span::styled(
                " [ 💾 Save & Apply to config.toml ] ",
                Style::default()
                    .bg(theme.success)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                " [ ❌ Cancel (Esc) ] ",
                Style::default().bg(theme.status_bg).fg(theme.dim_fg),
            ),
            Span::raw("   "),
            Span::styled(
                " Hint: Tab to cycle • j/k to scroll ",
                Style::default().fg(theme.dim_fg),
            ),
        ]));

        frame.render_widget(Paragraph::new(lines).scroll((self.scroll_offset, 0)), inner);
    }
}
