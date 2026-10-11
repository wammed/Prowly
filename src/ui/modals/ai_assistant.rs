//! Omnipresent AI Assistant modal overlay component.

use crate::ai::AiContext;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// State of the interactive AI Assistant modal overlay.
#[derive(Debug, Clone, Default)]
pub struct AiAssistantModal {
    pub is_open: bool,
    pub input_query: String,
    pub cursor_position: usize,
    pub is_loading: bool,
    pub response_text: String,
    pub context_summary: String,
    pub proposed_patch: Option<String>,
}

impl AiAssistantModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open modal with current screen context.
    pub fn open_with_context(&mut self, context: &AiContext) {
        self.is_open = true;
        self.context_summary = context.summary();
        if self.response_text.is_empty() {
            self.input_query.clear();
            self.cursor_position = 0;
        }
    }

    /// Close modal.
    pub fn close(&mut self) {
        self.is_open = false;
        self.is_loading = false;
    }

    /// Toggle visibility.
    pub fn toggle(&mut self, context: &AiContext) {
        if self.is_open {
            self.close();
        } else {
            self.open_with_context(context);
        }
    }

    /// Handle text input characters.
    pub fn handle_char(&mut self, c: char) {
        self.input_query.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Handle backspace.
    pub fn handle_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_query.remove(self.cursor_position);
        }
    }

    /// Set simulated or completed response.
    pub fn set_response(&mut self, text: String, patch: Option<String>) {
        self.is_loading = false;
        self.response_text = text;
        self.proposed_patch = patch;
    }

    /// Render modal onto frame if open.
    pub fn render(&self, frame: &mut Frame, theme: &Theme, area: Rect) {
        if !self.is_open {
            return;
        }

        // Center a 75% x 75% popup box
        let popup_width = (area.width * 75 / 100).max(50).min(area.width);
        let popup_height = (area.height * 75 / 100).max(18).min(area.height);

        let popup_rect = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear backdrop
        frame.render_widget(Clear, popup_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.ai_sparkle))
            .title(Span::styled(
                " 󱚣 Omnipresent AI Assistant ",
                Style::default()
                    .fg(theme.ai_sparkle)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.ai_bg).fg(theme.ai_fg));

        let inner = block.inner(popup_rect);
        frame.render_widget(block, popup_rect);

        // Layout inside popup: Context bar (1 line) -> Response area (Min) -> Input bar (3 lines) -> Action buttons (1 line)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(inner);

        // 1. Context indicator
        let context_text = if self.context_summary.is_empty() {
            "Context: General screen".to_string()
        } else {
            format!("Context: {}", self.context_summary)
        };
        let context_p = Paragraph::new(context_text).style(Style::default().fg(theme.dim_fg));
        frame.render_widget(context_p, chunks[0]);

        // 2. Response / Thinking area
        let response_content = if self.is_loading {
            "󱚣 Thinking... Generating solution based on screen context...".to_string()
        } else if self.response_text.is_empty() {
            "Ask AI anything about your changes, PR review, CI errors, or enter instructions to generate commits / patches.\n\nExamples:\n• 'Explain the failure in CI logs'\n• 'Draft conventional commit message for staged changes'\n• 'Review diff for potential bugs'".to_string()
        } else {
            self.response_text.clone()
        };

        let response_p = Paragraph::new(response_content)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.fg));
        frame.render_widget(response_p, chunks[1]);

        // 3. User query input field
        let input_title = if self.input_query.is_empty() {
            " Instruction / Question (Press Enter to send) "
        } else {
            " Query "
        };
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(input_title);

        let input_p = Paragraph::new(self.input_query.as_str())
            .block(input_block)
            .style(Style::default().fg(theme.fg));
        frame.render_widget(input_p, chunks[2]);

        // 4. Action bar: [ Apply ] [ Discard / Esc ]
        let action_spans = vec![
            Span::styled(
                "[ Apply / Enter ]",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("[ Discard / Esc ]", Style::default().fg(theme.dim_fg)),
        ];
        let action_p = Paragraph::new(Line::from(action_spans));
        frame.render_widget(action_p, chunks[3]);
    }
}
