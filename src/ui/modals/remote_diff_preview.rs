//! Remote Diff Preview dropdown modal for unpushed and unpulled commits.

use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Result of handling a mouse click on the remote diff preview modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteDiffClickResult {
    /// Clicked outside modal or on close hint -> close modal.
    Close,
    /// Clicked inside modal -> consumed.
    Handled,
}

/// Modal displaying incoming (unpulled) and outgoing (unpushed) commit summaries.
#[derive(Debug, Clone, Default)]
pub struct RemoteDiffPreviewModal {
    pub is_open: bool,
    pub unpushed_commits: Vec<String>,
    pub unpulled_commits: Vec<String>,
    pub upstream_name: Option<String>,
}

impl RemoteDiffPreviewModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, unpushed: Vec<String>, unpulled: Vec<String>, upstream: Option<String>) {
        self.is_open = true;
        self.unpushed_commits = unpushed;
        self.unpulled_commits = unpulled;
        self.upstream_name = upstream;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Calculate modal bounding rectangle.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 64.min(viewport.width.saturating_sub(4));
        let height = 20.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = 3.min(viewport.height.saturating_sub(height));
        Rect::new(x, y, width, height)
    }

    /// Handle mouse click inside or outside modal.
    pub fn handle_click(&mut self, x: u16, y: u16, viewport: Rect) -> Option<RemoteDiffClickResult> {
        if !self.is_open {
            return None;
        }

        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(RemoteDiffClickResult::Close);
        }

        // Close on clicking bottom instruction line
        if y == rect.y + rect.height.saturating_sub(2) {
            return Some(RemoteDiffClickResult::Close);
        }

        Some(RemoteDiffClickResult::Handled)
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);

        frame.render_widget(Clear, rect);

        let upstream_str = self
            .upstream_name
            .as_deref()
            .unwrap_or("No upstream configured");
        let title = format!(" {} Remote Sync Status ({}) ", icons.sync(), upstream_str);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.info))
            .title(Span::styled(
                title,
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        let mut lines = Vec::new();

        // 1. Unpushed section
        lines.push(Line::from(vec![Span::styled(
            format!(
                "{} Unpushed Commits ({}) :",
                icons.arrow_up(),
                self.unpushed_commits.len()
            ),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )]));
        if self.unpushed_commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "  • Clean (Everything pushed to remote)",
                Style::default().fg(theme.dim_fg),
            )));
        } else {
            for c in self.unpushed_commits.iter().take(6) {
                lines.push(Line::from(vec![
                    Span::styled("  ↑ ", Style::default().fg(theme.success)),
                    Span::styled(c.clone(), Style::default().fg(theme.fg)),
                ]));
            }
        }

        lines.push(Line::from(""));

        // 2. Unpulled section
        lines.push(Line::from(vec![Span::styled(
            format!(
                "{} Unpulled Commits ({}) :",
                icons.arrow_down(),
                self.unpulled_commits.len()
            ),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )]));
        if self.unpulled_commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "  • Clean (Up to date with remote)",
                Style::default().fg(theme.dim_fg),
            )));
        } else {
            for c in self.unpulled_commits.iter().take(6) {
                lines.push(Line::from(vec![
                    Span::styled("  ↓ ", Style::default().fg(theme.warning)),
                    Span::styled(c.clone(), Style::default().fg(theme.fg)),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "[ Press Esc to close | Use Command Palette to Push / Pull ]",
            Style::default().fg(theme.dim_fg),
        )));

        let p = Paragraph::new(lines);
        frame.render_widget(p, inner);
    }
}
