//! Git Worktrees view component.

use crate::git::WorktreeItem;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::virtual_scroll::VirtualScroll;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Worktrees tab view controller.
pub struct WorktreesView {
    pub scroll: VirtualScroll,
}

impl WorktreesView {
    pub fn new() -> Self {
        Self {
            scroll: VirtualScroll::new(0, 10),
        }
    }

    /// Render Worktrees list and action buttons.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        worktrees: &[WorktreeItem],
        theme: &Theme,
        icons: &Icons,
        area: Rect,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                format!(" {} Git Worktrees ({}) ", icons.worktree(), worktrees.len()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner into: Top list (Min) -> Bottom buttons bar (Length 2)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(4), Constraint::Length(2)])
            .split(inner);

        let list_area = chunks[0];
        let buttons_area = chunks[1];

        self.scroll.set_total_items(worktrees.len());
        self.scroll.set_viewport_height(list_area.height as usize);

        let visible = self.scroll.visible_range();
        let lines: Vec<Line> = worktrees[visible]
            .iter()
            .enumerate()
            .map(|(rel_idx, item)| {
                let abs_idx = self.scroll.viewport_offset + rel_idx;
                let is_sel = abs_idx == self.scroll.selected_index;

                let marker = if item.is_current {
                    "(current)"
                } else {
                    "         "
                };

                let marker_style = if item.is_current {
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.dim_fg)
                };

                let branch_str = item.head_branch.as_deref().unwrap_or("detached");
                let line_style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                Line::from(vec![
                    Span::styled(format!(" {} ", icons.worktree()), line_style),
                    Span::styled(format!("{} ", marker), marker_style),
                    Span::styled(format!("{:<16} ", item.name), line_style),
                    Span::styled(
                        format!("({} {}) ", icons.branch(), branch_str),
                        Style::default().fg(theme.info),
                    ),
                    Span::styled(
                        format!("-> {}", item.path.display()),
                        Style::default().fg(theme.dim_fg),
                    ),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), list_area);

        // Action buttons
        let buttons = vec![
            Span::styled(
                format!(" [ {} Add Worktree (W) ] ", icons.worktree()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                format!(
                    " [ {} Remove Worktree (D) ] ",
                    icons.get(crate::ui::IconId::Trash)
                ),
                Style::default().fg(theme.error),
            ),
            Span::raw("   "),
            Span::styled(
                format!(" [ {} Switch Worktree (Enter) ] ", icons.sync()),
                Style::default().fg(theme.secondary),
            ),
        ];
        frame.render_widget(Paragraph::new(Line::from(buttons)), buttons_area);
    }
}

impl Default for WorktreesView {
    fn default() -> Self {
        Self::new()
    }
}
