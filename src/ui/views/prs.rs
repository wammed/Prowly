//! Pull Requests tab view with list, detail preview, and local checkout.

use crate::github::PullRequestItem;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::virtual_scroll::VirtualScroll;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

/// Focus state within PR view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrFocus {
    List,
    Detail,
}

/// Pull Requests view component.
pub struct PrsView {
    pub prs: Vec<PullRequestItem>,
    pub list_scroll: VirtualScroll,
    pub detail_scroll: VirtualScroll,
    pub focus: PrFocus,
    pub status_message: Option<String>,
}

impl PrsView {
    pub fn new() -> Self {
        Self {
            prs: Vec::new(),
            list_scroll: VirtualScroll::new(0, 10),
            detail_scroll: VirtualScroll::new(0, 10),
            focus: PrFocus::List,
            status_message: None,
        }
    }

    pub fn set_prs(&mut self, prs: Vec<PullRequestItem>) {
        self.list_scroll.set_total_items(prs.len());
        self.prs = prs;
    }

    pub fn selected_pr(&self) -> Option<&PullRequestItem> {
        self.prs.get(self.list_scroll.selected_index)
    }

    pub fn next(&mut self) {
        match self.focus {
            PrFocus::List => {
                self.list_scroll.next();
                self.detail_scroll.scroll_to_top();
            }
            PrFocus::Detail => {
                self.detail_scroll.next();
            }
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            PrFocus::List => {
                self.list_scroll.previous();
                self.detail_scroll.scroll_to_top();
            }
            PrFocus::Detail => {
                self.detail_scroll.previous();
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            PrFocus::List => PrFocus::Detail,
            PrFocus::Detail => PrFocus::List,
        };
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        is_focused: bool,
        theme: &Theme,
        icons: &Icons,
        area: Rect,
    ) {
        let border_color = if is_focused {
            theme.border_focused
        } else {
            theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(" {} PULL REQUESTS ", icons.pr_open()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.prs.is_empty() {
            let msg = if let Some(ref m) = self.status_message {
                m.as_str()
            } else {
                "No open pull requests found for this repository."
            };
            let p = Paragraph::new(format!("\n  {}", msg)).style(Style::default().fg(theme.dim_fg));
            frame.render_widget(p, inner);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner);

        // 1. Left list pane
        let list_border_color = if is_focused && self.focus == PrFocus::List {
            theme.border_focused
        } else {
            theme.border
        };

        let list_block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(list_border_color));
        let list_inner = list_block.inner(chunks[0]);
        frame.render_widget(list_block, chunks[0]);

        self.list_scroll.viewport_height = list_inner.height as usize;
        let visible_range = self.list_scroll.visible_range();

        let mut lines = Vec::new();
        for (i, pr) in self
            .prs
            .iter()
            .enumerate()
            .skip(visible_range.start)
            .take(visible_range.end - visible_range.start)
        {
            let is_selected = i == self.list_scroll.selected_index;
            let (badge_text, badge_color) = if pr.is_draft {
                ("DRAFT", theme.dim_fg)
            } else {
                match pr.state.as_str() {
                    "open" => ("OPEN", theme.diff_add_fg),
                    "merged" => ("MERGED", Color::Rgb(163, 113, 247)),
                    _ => ("CLOSED", theme.diff_del_fg),
                }
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let line_bg = if is_selected {
                theme.selection_bg
            } else {
                theme.bg
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.primary).bg(line_bg)),
                Span::styled(
                    format!("[{}] ", badge_text),
                    Style::default()
                        .fg(badge_color)
                        .add_modifier(Modifier::BOLD)
                        .bg(line_bg),
                ),
                Span::styled(
                    format!("#{} ", pr.number),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD)
                        .bg(line_bg),
                ),
                Span::styled(
                    pr.title.clone(),
                    Style::default()
                        .fg(theme.fg)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        })
                        .bg(line_bg),
                ),
            ]));

            // Subline: author & branch
            lines.push(Line::from(vec![
                Span::styled("    by ", Style::default().fg(theme.dim_fg).bg(line_bg)),
                Span::styled(&pr.author, Style::default().fg(theme.fg).bg(line_bg)),
                Span::styled(" (", Style::default().fg(theme.dim_fg).bg(line_bg)),
                Span::styled(
                    &pr.head_branch,
                    Style::default().fg(theme.primary).bg(line_bg),
                ),
                Span::styled(")", Style::default().fg(theme.dim_fg).bg(line_bg)),
            ]));
        }

        let list_p = Paragraph::new(lines);
        frame.render_widget(list_p, list_inner);

        // 2. Right detail pane
        let detail_border_color = if is_focused && self.focus == PrFocus::Detail {
            theme.border_focused
        } else {
            theme.border
        };

        let detail_block = Block::default()
            .borders(Borders::NONE)
            .border_style(Style::default().fg(detail_border_color));
        let detail_inner = detail_block.inner(chunks[1]);
        frame.render_widget(detail_block, chunks[1]);

        if let Some(pr) = self.selected_pr() {
            let detail_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ])
                .split(detail_inner);

            // Title & branch metadata
            let header_lines = vec![
                Line::from(vec![Span::styled(
                    format!("#{} {}", pr.number, pr.title),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("Branch: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(
                        &pr.head_branch,
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" -> ", Style::default().fg(theme.dim_fg)),
                    Span::styled(&pr.base_branch, Style::default().fg(theme.dim_fg)),
                ]),
                Line::from(vec![
                    Span::styled("Author: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(&pr.author, Style::default().fg(theme.fg)),
                    Span::styled(
                        format!("  Comments: {}", pr.comments_count),
                        Style::default().fg(theme.dim_fg),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(header_lines), detail_chunks[0]);

            // Body description
            let body_text = pr.body.as_deref().unwrap_or("No description provided.");
            let body_p = Paragraph::new(body_text)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(theme.border)),
                )
                .style(Style::default().fg(theme.fg));
            frame.render_widget(body_p, detail_chunks[1]);

            // Action bar
            let actions = vec![Line::from(vec![
                Span::styled(
                    "[ Enter: Checkout locally ]  ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("[ y: Copy URL ]  ", Style::default().fg(theme.dim_fg)),
                Span::styled("[ R: Refresh ]  ", Style::default().fg(theme.dim_fg)),
                Span::styled("[ Ctrl+P: AI Draft PR ]", Style::default().fg(theme.accent)),
            ])];
            frame.render_widget(Paragraph::new(actions), detail_chunks[2]);
        }
    }
}
