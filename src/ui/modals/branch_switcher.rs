//! Quick Branch Switcher popup modal with fuzzy search and fast return.

use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Result of handling a mouse click on the branch switcher modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchSwitcherClickResult {
    /// Clicked outside the modal area -> close modal.
    Close,
    /// Clicked on a branch entry in the list -> switch to this branch.
    Select(String),
    /// Clicked inside modal (input area, borders, etc.) -> consumed, no action.
    Handled,
}

/// Modal for fast branch searching, switching, and returning to the previous branch.
#[derive(Debug, Clone, Default)]
pub struct BranchSwitcherModal {
    pub is_open: bool,
    pub branches: Vec<String>,
    pub current_branch: String,
    pub previous_branch: Option<String>,
    pub query: String,
    pub selected_index: usize,
}

impl BranchSwitcherModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open modal with current branch list.
    pub fn open(&mut self, current: String, branches: Vec<String>, previous: Option<String>) {
        self.is_open = true;
        self.current_branch = current;
        self.branches = branches;
        self.previous_branch = previous;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.selected_index = 0;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.selected_index = 0;
    }

    pub fn next(&mut self) {
        let count = self.filtered_branches().len();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Filter branches by search query.
    pub fn filtered_branches(&self) -> Vec<String> {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            return self.branches.clone();
        }
        self.branches
            .iter()
            .filter(|b| b.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    pub fn selected_branch(&self) -> Option<String> {
        self.filtered_branches().get(self.selected_index).cloned()
    }

    /// Calculate modal bounding box.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 60.min(viewport.width.saturating_sub(4));
        let height = 18.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Handle mouse click inside or outside modal.
    pub fn handle_click(
        &mut self,
        x: u16,
        y: u16,
        viewport: Rect,
    ) -> Option<BranchSwitcherClickResult> {
        if !self.is_open {
            return None;
        }

        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(BranchSwitcherClickResult::Close);
        }

        let inner_x = rect.x + 1;
        let inner_y = rect.y + 1;
        let inner_w = rect.width.saturating_sub(2);
        let inner_h = rect.height.saturating_sub(2);

        // Branch list lines start at inner_y + 2
        if x >= inner_x && x < inner_x + inner_w && y >= inner_y + 2 && y < inner_y + inner_h {
            let visible_height = (inner_h as usize).saturating_sub(3);
            let scroll_top = if visible_height > 0 && self.selected_index >= visible_height {
                self.selected_index + 1 - visible_height
            } else {
                0
            };
            let line_offset = (y - (inner_y + 2)) as usize;
            let clicked_idx = scroll_top + line_offset;
            let filtered = self.filtered_branches();
            if clicked_idx < filtered.len() {
                self.selected_index = clicked_idx;
                return Some(BranchSwitcherClickResult::Select(
                    filtered[clicked_idx].clone(),
                ));
            }
        }

        Some(BranchSwitcherClickResult::Handled)
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);

        frame.render_widget(Clear, rect);

        let prev_hint = if let Some(ref prev) = self.previous_branch {
            format!(" | Previous: {}", prev)
        } else {
            String::new()
        };

        let title = format!(
            " {} Switch Branch (Current: {}{}) ",
            icons.branch(),
            self.current_branch,
            prev_hint
        );

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

        let filtered = self.filtered_branches();
        let mut lines = Vec::new();

        // Query input line
        let input_line = Line::from(vec![
            Span::styled(" Filter: ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                if self.query.is_empty() {
                    "Type to fuzzy search... (Esc to close)"
                } else {
                    &self.query
                },
                if self.query.is_empty() {
                    Style::default().fg(theme.dim_fg)
                } else {
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
                },
            ),
        ]);
        lines.push(input_line);
        lines.push(Line::from(""));

        // Branch list
        if filtered.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No matching branches found",
                Style::default().fg(theme.dim_fg),
            )));
        } else {
            let visible_height = (inner.height as usize).saturating_sub(3);
            let scroll_top = if visible_height > 0 && self.selected_index >= visible_height {
                self.selected_index + 1 - visible_height
            } else {
                0
            };
            for (offset, branch) in filtered
                .iter()
                .skip(scroll_top)
                .take(visible_height)
                .enumerate()
            {
                let actual_idx = scroll_top + offset;
                let is_sel = actual_idx == self.selected_index;
                let is_current = branch == &self.current_branch;
                let prefix = if is_sel { "▶ " } else { "  " };

                let style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(theme.success)
                } else {
                    Style::default().fg(theme.fg)
                };

                let current_marker = if is_current { " (current)" } else { "" };
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{} ", icons.branch()), style),
                    Span::styled(branch.clone(), style),
                    Span::styled(current_marker, Style::default().fg(theme.dim_fg)),
                ]));
            }
        }

        let p = Paragraph::new(lines);
        frame.render_widget(p, inner);

        let label_w = 9u16; // " Filter: "
        let query_w = unicode_width::UnicodeWidthStr::width(self.query.as_str()) as u16;
        let cur_x = (inner.x + label_w + query_w).min(inner.x + inner.width.saturating_sub(1));
        frame.set_cursor_position((cur_x, inner.y));
    }
}
