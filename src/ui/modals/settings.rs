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
    pub editor_command: String,

    pub show_api_key: bool,
    pub focused_input: Option<SettingsInputFocus>,
    pub status_banner: Option<String>,
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
            editor_command: String::new(),
            show_api_key: false,
            focused_input: None,
            status_banner: None,
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
        self.editor_command = config.editor.command.clone().unwrap_or_default();
        self.show_api_key = false;
        self.focused_input = None;
        self.status_banner = None;
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
        config.editor.command = if self.editor_command.trim().is_empty() {
            None
        } else {
            Some(self.editor_command.trim().to_string())
        };
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
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 76.min(viewport.width.saturating_sub(4));
        let height = 28.min(viewport.height.saturating_sub(2));
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

        // Header Action Buttons (Row 0): [ 💾 Save ] and [ ❌ Cancel ]
        if inner_y == 0 {
            if inner_x >= rect.width.saturating_sub(24) && inner_x < rect.width.saturating_sub(12) {
                return Some(SettingsAction::SaveAndApply);
            }
            if inner_x >= rect.width.saturating_sub(12) {
                return Some(SettingsAction::Close);
            }
        }

        // Row 2: Theme selection
        if inner_y == 2 {
            let themes = [
                (ThemeId::CatppuccinMocha, "Mocha"),
                (ThemeId::TokyoNight, "TokyoNight"),
                (ThemeId::Dracula, "Dracula"),
                (ThemeId::GruvboxDark, "Gruvbox"),
                (ThemeId::OneDark, "OneDark"),
                (ThemeId::Nord, "Nord"),
            ];
            let mut cur_x = 10;
            for (tid, name) in themes {
                let btn_w = name.len() as u16 + 4;
                if inner_x >= cur_x && inner_x < cur_x + btn_w {
                    self.theme = tid;
                    return None;
                }
                cur_x += btn_w + 1;
            }
        }

        // Row 3: Language & Nerd Font
        if inner_y == 3 {
            // [ English ] (10..22), [ 日本語 ] (23..34)
            if inner_x >= 10 && inner_x < 22 {
                self.language = Language::En;
            } else if inner_x >= 23 && inner_x < 34 {
                self.language = Language::Ja;
            }
            // Nerd Font: [ ON ] (50..57), [ OFF ] (58..66)
            if inner_x >= 50 && inner_x < 57 {
                self.use_nerd_font = true;
            } else if inner_x >= 58 && inner_x < 66 {
                self.use_nerd_font = false;
            }
        }

        // Row 5: AI Provider Chips
        if inner_y == 5 {
            let providers = [
                (AiProvider::OpenAi, "OpenAI"),
                (AiProvider::Anthropic, "Anthropic"),
                (AiProvider::Gemini, "Gemini"),
                (AiProvider::Ollama, "Ollama"),
                (AiProvider::Custom, "Custom"),
            ];
            let mut cur_x = 12;
            for (prov, label) in providers {
                let w = label.len() as u16 + 4;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_provider = prov;
                    self.ai_model = prov.default_model().to_string();
                    return None;
                }
                cur_x += w + 1;
            }
        }

        // Row 7: Model Presets
        if inner_y == 7 {
            let presets: &[&str] = match self.ai_provider {
                AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "o3-mini"],
                AiProvider::Anthropic => {
                    &["claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"]
                }
                AiProvider::Gemini => &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"],
                AiProvider::Ollama => &["llama3.2:latest", "deepseek-r1:latest", "codellama"],
                AiProvider::Custom => &["default", "local-model"],
            };
            let mut cur_x = 12;
            for model_name in presets {
                let w = model_name.len() as u16 + 4;
                if inner_x >= cur_x && inner_x < cur_x + w {
                    self.ai_model = model_name.to_string();
                    return None;
                }
                cur_x += w + 1;
            }
        }

        // Row 8: Model Input
        if inner_y == 8 && inner_x >= 12 && inner_x < rect.width.saturating_sub(4) {
            self.focused_input = Some(SettingsInputFocus::AiModel);
            return None;
        }

        // Row 10: API Key Input & Buttons
        if inner_y == 10 {
            // Mask toggle button [ Show / Hide ] at end of row
            let show_btn_x = rect.width.saturating_sub(18);
            if inner_x >= show_btn_x && inner_x < show_btn_x + 8 {
                self.show_api_key = !self.show_api_key;
                return None;
            }
            let clear_btn_x = rect.width.saturating_sub(9);
            if inner_x >= clear_btn_x {
                self.ai_api_key.clear();
                return None;
            }
            if inner_x >= 12 && inner_x < show_btn_x {
                self.focused_input = Some(SettingsInputFocus::AiApiKey);
                return None;
            }
        }

        // Row 12: Endpoint Base URL & Reset Default
        if inner_y == 12 {
            let reset_btn_x = rect.width.saturating_sub(16);
            if inner_x >= reset_btn_x {
                self.ai_api_base.clear();
                return None;
            }
            if inner_x >= 12 && inner_x < reset_btn_x {
                self.focused_input = Some(SettingsInputFocus::AiApiBase);
                return None;
            }
        }

        // Row 15: Git Soft Discard & Auto Fetch
        if inner_y == 15 {
            // Soft Discard: [ ON ] (16..23), [ OFF ] (24..32)
            if inner_x >= 16 && inner_x < 23 {
                self.git_soft_discard = true;
            } else if inner_x >= 24 && inner_x < 32 {
                self.git_soft_discard = false;
            }
            // Auto Fetch: [ Off ] (48..55), [ 30s ] (56..63), [ 60s ] (64..71)
            if inner_x >= 48 && inner_x < 55 {
                self.git_auto_fetch_secs = 0;
            } else if inner_x >= 56 && inner_x < 63 {
                self.git_auto_fetch_secs = 30;
            } else if inner_x >= 64 && inner_x < 71 {
                self.git_auto_fetch_secs = 60;
            }
        }

        // Row 17: Editor Command
        if inner_y == 17 {
            // Presets: [ nvim ] (34..43), [ code ] (44..53), [ nano ] (54..63)
            if inner_x >= 34 && inner_x < 43 {
                self.editor_command = "nvim".to_string();
                return None;
            } else if inner_x >= 44 && inner_x < 53 {
                self.editor_command = "code".to_string();
                return None;
            } else if inner_x >= 54 && inner_x < 63 {
                self.editor_command = "nano".to_string();
                return None;
            }
            if inner_x >= 12 && inner_x < 32 {
                self.focused_input = Some(SettingsInputFocus::EditorCommand);
                return None;
            }
        }

        // Bottom Action Bar: Row height - 3
        if inner_y >= rect.height.saturating_sub(4) {
            if inner_x >= 4 && inner_x < 34 {
                return Some(SettingsAction::SaveAndApply);
            }
            if inner_x >= 36 && inner_x < 56 {
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
        frame.render_widget(Clear, rect);

        let title = format!(" {} SETTINGS / 設定 ", icons.settings());
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

        let mut lines = Vec::new();

        // 1. Header action buttons
        let save_style = Style::default()
            .bg(theme.success)
            .fg(theme.bg)
            .add_modifier(Modifier::BOLD);
        let cancel_style = Style::default().fg(theme.dim_fg);
        lines.push(Line::from(vec![
            Span::styled(
                "Preferences & AI Integration",
                Style::default().fg(theme.dim_fg),
            ),
            Span::raw(" ".repeat((inner.width as usize).saturating_sub(50))),
            Span::styled(" [ 💾 Save ] ", save_style),
            Span::raw(" "),
            Span::styled(" [ ❌ Cancel ] ", cancel_style),
        ]));

        lines.push(Line::from(""));

        // 2. Theme row
        let themes = [
            (ThemeId::CatppuccinMocha, "Mocha"),
            (ThemeId::TokyoNight, "TokyoNight"),
            (ThemeId::Dracula, "Dracula"),
            (ThemeId::GruvboxDark, "Gruvbox"),
            (ThemeId::OneDark, "OneDark"),
            (ThemeId::Nord, "Nord"),
        ];
        let mut theme_spans = vec![Span::styled(
            "Theme:    ",
            Style::default().fg(theme.primary),
        )];
        for (tid, name) in themes {
            let is_sel = self.theme == tid;
            let style = if is_sel {
                Style::default()
                    .bg(theme.primary)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            theme_spans.push(Span::styled(format!(" [ {} ] ", name), style));
            theme_spans.push(Span::raw(" "));
        }
        lines.push(Line::from(theme_spans));

        // 3. Language & Nerd Font row
        let lang_en_style = if self.language == Language::En {
            Style::default()
                .bg(theme.primary)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let lang_ja_style = if self.language == Language::Ja {
            Style::default()
                .bg(theme.primary)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        let nf_on_style = if self.use_nerd_font {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let nf_off_style = if !self.use_nerd_font {
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
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

        lines.push(Line::from(Span::styled(
            "────────────────────────────────────────────────────────────────────────",
            Style::default().fg(theme.border),
        )));

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

        lines.push(Line::from(""));

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
            "(No API Key configured)".to_string()
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

        lines.push(Line::from(""));

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

        lines.push(Line::from(Span::styled(
            "────────────────────────────────────────────────────────────────────────",
            Style::default().fg(theme.border),
        )));

        // 9. Git Engine Preferences
        let soft_on = if self.git_soft_discard {
            Style::default()
                .bg(theme.success)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let soft_off = if !self.git_soft_discard {
            Style::default()
                .bg(theme.warning)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_off = if self.git_auto_fetch_secs == 0 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_30 = if self.git_auto_fetch_secs == 30 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };
        let af_60 = if self.git_auto_fetch_secs == 60 {
            Style::default()
                .bg(theme.info)
                .fg(theme.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.dim_fg)
        };

        lines.push(Line::from(vec![
            Span::styled("Soft Discard: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ ON ] ", soft_on),
            Span::styled(" [ OFF ] ", soft_off),
            Span::raw("     "),
            Span::styled("Auto-Fetch: ", Style::default().fg(theme.secondary)),
            Span::styled(" [ Off ] ", af_off),
            Span::styled(" [ 30s ] ", af_30),
            Span::styled(" [ 60s ] ", af_60),
        ]));

        lines.push(Line::from(""));

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

        lines.push(Line::from(""));

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
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
