//! Markdown preview modal component.
//! Provides a scrollable, richly rendered Markdown preview within the terminal UI.

use crate::ui::icons::Icons;
use crate::ui::markdown::render_markdown;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Result of handling a mouse click on the markdown preview modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownPreviewClickResult {
    /// Clicked outside modal or close button -> close modal.
    Close,
    /// Clicked copy button -> copy content to clipboard.
    Copy,
    /// Clicked inside modal area -> consumed.
    Handled,
}

/// Floating full/large popup modal for rich Markdown preview.
#[derive(Debug, Clone, Default)]
pub struct MarkdownPreviewModal {
    pub is_open: bool,
    pub file_path: String,
    pub raw_content: String,
    pub rendered_lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub viewport_height: usize,
}

impl MarkdownPreviewModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open modal with raw markdown content and render it using the active theme.
    pub fn open(&mut self, file_path: String, content: String, theme: &Theme) {
        self.is_open = true;
        self.file_path = file_path;
        self.rendered_lines = render_markdown(&content, theme);
        self.raw_content = content;
        self.scroll_offset = 0;
    }

    /// Close modal.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Scroll down by n lines.
    pub fn scroll_down(&mut self, n: usize) {
        let max_scroll = self
            .rendered_lines
            .len()
            .saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + n).min(max_scroll);
    }

    /// Scroll up by n lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Jump to the beginning of the document.
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to the end of the document.
    pub fn scroll_to_bottom(&mut self) {
        let max_scroll = self
            .rendered_lines
            .len()
            .saturating_sub(self.viewport_height);
        self.scroll_offset = max_scroll;
    }

    /// Calculate modal bounding rectangle.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = (viewport.width.saturating_sub(8))
            .max(40)
            .min(viewport.width);
        let height = (viewport.height.saturating_sub(4))
            .max(15)
            .min(viewport.height);
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Handle mouse click inside or outside modal.
    pub fn handle_click(
        &mut self,
        x: u16,
        y: u16,
        viewport: Rect,
    ) -> Option<MarkdownPreviewClickResult> {
        if !self.is_open {
            return None;
        }

        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(MarkdownPreviewClickResult::Close);
        }

        let inner_y = rect.y + 1;
        let inner_h = rect.height.saturating_sub(2);
        let footer_y = inner_y + inner_h.saturating_sub(1);

        if y == footer_y {
            let inner_x = rect.x + 1;
            let rel_x = x.saturating_sub(inner_x);
            if (51..63).contains(&rel_x) {
                return Some(MarkdownPreviewClickResult::Copy);
            } else if rel_x >= 63 {
                return Some(MarkdownPreviewClickResult::Close);
            }
        }

        Some(MarkdownPreviewClickResult::Handled)
    }

    /// Render modal into the terminal frame.
    pub fn render(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);

        // Clear backdrop
        frame.render_widget(Clear, rect);

        let total_lines = self.rendered_lines.len();
        // Inner height accounts for top border and bottom action bar border
        let inner_height = rect.height.saturating_sub(3) as usize;
        self.viewport_height = inner_height;

        let visible_start = self.scroll_offset;
        let visible_end = (visible_start + inner_height).min(total_lines);
        let percent = if total_lines == 0 {
            100
        } else {
            ((visible_end as f32 / total_lines as f32) * 100.0) as usize
        };

        let file_name = std::path::Path::new(&self.file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.file_path);

        let title = format!(" {} Markdown Preview: {} ", icons.file(), file_name);

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
            .title_bottom(Span::styled(
                format!(
                    " [ Lines {}-{} / {} ({}%) ] ",
                    if total_lines == 0 {
                        0
                    } else {
                        visible_start + 1
                    },
                    visible_end,
                    total_lines,
                    percent
                ),
                Style::default().fg(theme.dim_fg),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        // Content area layout: Main scroll area (inner_height) -> Action helper footer (1 line)
        if inner.height <= 1 {
            return;
        }

        let content_rect = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };

        let footer_rect = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };

        // Slice visible lines
        let visible_lines: Vec<Line<'static>> = self
            .rendered_lines
            .iter()
            .skip(visible_start)
            .take(inner_height)
            .cloned()
            .collect();

        let content_widget = Paragraph::new(visible_lines).style(Style::default().fg(theme.fg));
        frame.render_widget(content_widget, content_rect);

        // Action footer
        let footer = Line::from(vec![
            Span::styled(
                "[ ↑/↓/j/k ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Scroll ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                "| PgUp/PgDn ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Page ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                "| g/G ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Top/Bottom ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                "| y ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Copy ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                "| Esc/q ",
                Style::default()
                    .fg(theme.diff_del_fg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Close ]", Style::default().fg(theme.dim_fg)),
        ]);
        frame.render_widget(Paragraph::new(vec![footer]), footer_rect);
    }
}
