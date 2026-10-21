//! Quick Interactive Rebase modal component.
//! Allows inspecting recent commit history, verified badges, squashing, fixups, and rewording.

use crate::git::rebase::{RebaseAction, RebaseCommitItem};
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Result of handling a mouse click on the interactive rebase modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseClickResult {
    /// Clicked outside modal or cancel -> close modal.
    Close,
    /// Select commit item at index.
    SelectCommit(usize),
    /// Quick squash action.
    Squash,
    /// Quick fixup action.
    Fixup,
    /// Reword action.
    Reword,
    /// Clicked inside modal -> consumed.
    Handled,
}

/// Rebase modal state.
#[derive(Debug, Clone, Default)]
pub struct RebaseModal {
    pub is_open: bool,
    pub commits: Vec<RebaseCommitItem>,
    pub selected_index: usize,
    pub is_editing_reword: bool,
    pub reword_input: String,
}

impl RebaseModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, commits: Vec<RebaseCommitItem>) {
        self.is_open = true;
        self.commits = commits;
        self.selected_index = 0;
        self.is_editing_reword = false;
        self.reword_input.clear();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.is_editing_reword = false;
        self.reword_input.clear();
    }

    pub fn next(&mut self) {
        if !self.commits.is_empty() && self.selected_index + 1 < self.commits.len() {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected_commit(&self) -> Option<&RebaseCommitItem> {
        self.commits.get(self.selected_index)
    }

    pub fn set_selected_action(&mut self, action: RebaseAction) {
        if let Some(item) = self.commits.get_mut(self.selected_index) {
            item.action = action;
        }
    }

    /// Calculate modal bounding box.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 76.min(viewport.width.saturating_sub(4));
        let height = 22.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Handle mouse click inside or outside modal.
    pub fn handle_click(&mut self, x: u16, y: u16, viewport: Rect) -> Option<RebaseClickResult> {
        if !self.is_open {
            return None;
        }

        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(RebaseClickResult::Close);
        }

        let inner_x = rect.x + 1;
        let inner_y = rect.y + 1;
        let inner_h = rect.height.saturating_sub(2);

        let commit_start_y = inner_y + 2;
        let commit_count = self.commits.len().min(inner_h.saturating_sub(5) as usize);
        if y >= commit_start_y && (y as usize) < (commit_start_y as usize) + commit_count {
            let idx = (y - commit_start_y) as usize;
            self.selected_index = idx;
            return Some(RebaseClickResult::SelectCommit(idx));
        }

        if !self.is_editing_reword && y == inner_y + inner_h.saturating_sub(1) {
            let rel_x = x.saturating_sub(inner_x);
            if rel_x < 23 {
                return Some(RebaseClickResult::Squash);
            } else if rel_x < 43 {
                return Some(RebaseClickResult::Fixup);
            } else if rel_x < 64 {
                return Some(RebaseClickResult::Reword);
            } else {
                return Some(RebaseClickResult::Close);
            }
        }

        Some(RebaseClickResult::Handled)
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);

        frame.render_widget(Clear, rect);

        let title = format!(
            " {} Quick Interactive Rebase & Commit History ",
            icons.commit()
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

        let mut lines = Vec::new();

        if self.commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No commits found in history",
                Style::default().fg(theme.dim_fg),
            )));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "  Action  Hash    Author     Summary                     Signature",
                Style::default().fg(theme.dim_fg),
            )]));
            lines.push(Line::from(""));

            for (idx, item) in self
                .commits
                .iter()
                .enumerate()
                .take(inner.height as usize - 5)
            {
                let is_sel = idx == self.selected_index;
                let prefix = if is_sel { "▶ " } else { "  " };

                let action_style = match item.action {
                    RebaseAction::Pick => Style::default().fg(theme.info),
                    RebaseAction::Squash => Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                    RebaseAction::Fixup => Style::default().fg(theme.warning),
                    RebaseAction::Reword => Style::default().fg(theme.primary),
                    RebaseAction::Drop => Style::default().fg(theme.error),
                };

                let line_style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                let (sig_icon, sig_style) = if item.is_verified {
                    (
                        icons.verified(),
                        Style::default().fg(theme.badge_verified_fg),
                    )
                } else {
                    (
                        icons.unverified(),
                        Style::default().fg(theme.badge_unverified_fg),
                    )
                };

                let summary_truncated = crate::ui::truncate_display_width(&item.summary, 24);
                let author_truncated = crate::ui::truncate_chars(&item.author, 10);

                lines.push(Line::from(vec![
                    Span::styled(prefix, line_style),
                    Span::styled(format!("[{:<6}] ", item.action.badge()), action_style),
                    Span::styled(
                        format!("{:<7} ", item.hash),
                        Style::default().fg(theme.secondary),
                    ),
                    Span::styled(
                        format!("{:<10} ", author_truncated),
                        Style::default().fg(theme.dim_fg),
                    ),
                    Span::styled(summary_truncated, line_style),
                    Span::raw("  "),
                    Span::styled(format!("{} ", sig_icon), sig_style),
                ]));
            }
        }

        lines.push(Line::from(""));
        if self.is_editing_reword {
            lines.push(Line::from(vec![
                Span::styled(
                    "New message: ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &self.reword_input,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " (Press Enter to confirm, Esc to cancel)",
                    Style::default().fg(theme.dim_fg),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("[ s: Squash top 2 ]  ", Style::default().fg(theme.accent)),
                Span::styled("[ f: Fixup HEAD ]  ", Style::default().fg(theme.warning)),
                Span::styled("[ r: Reword HEAD ]  ", Style::default().fg(theme.primary)),
                Span::styled("[ Esc: Close ]", Style::default().fg(theme.dim_fg)),
            ]));
        }

        let p = Paragraph::new(lines);
        frame.render_widget(p, inner);

        if self.is_editing_reword {
            let label_w = 13u16; // "New message: "
            let text_w = unicode_width::UnicodeWidthStr::width(self.reword_input.as_str()) as u16;
            let cur_x = (inner.x + label_w + text_w).min(inner.x + inner.width.saturating_sub(1));
            // The reword input line was pushed near the end (lines.len() - 1)
            let line_y = inner.y + 1 + self.commits.len().min(8) as u16 + 2;
            frame
                .set_cursor_position((cur_x, line_y.min(inner.y + inner.height.saturating_sub(1))));
        }
    }
}
