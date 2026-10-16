//! GitHub Issues tab view with list, detail preview, and labels.

use crate::github::IssueItem;
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

/// Focus state within Issues view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueFocus {
    List,
    Detail,
}

/// GitHub Issues view component.
pub struct IssuesView {
    pub issues: Vec<IssueItem>,
    pub list_scroll: VirtualScroll,
    pub detail_scroll: VirtualScroll,
    pub focus: IssueFocus,
    pub status_message: Option<String>,
}

impl IssuesView {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            list_scroll: VirtualScroll::new(0, 10),
            detail_scroll: VirtualScroll::new(0, 10),
            focus: IssueFocus::List,
            status_message: None,
        }
    }

    pub fn set_issues(&mut self, issues: Vec<IssueItem>) {
        self.list_scroll.set_total_items(issues.len());
        self.issues = issues;
    }

    pub fn selected_issue(&self) -> Option<&IssueItem> {
        self.issues.get(self.list_scroll.selected_index)
    }

    pub fn next(&mut self) {
        match self.focus {
            IssueFocus::List => {
                self.list_scroll.next();
                self.detail_scroll.scroll_to_top();
            }
            IssueFocus::Detail => {
                self.detail_scroll.next();
            }
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            IssueFocus::List => {
                self.list_scroll.previous();
                self.detail_scroll.scroll_to_top();
            }
            IssueFocus::Detail => {
                self.detail_scroll.previous();
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            IssueFocus::List => IssueFocus::Detail,
            IssueFocus::Detail => IssueFocus::List,
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
                format!(
                    " {} GITHUB ISSUES ",
                    icons.get(crate::ui::IconId::IssueOpen)
                ),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.issues.is_empty() {
            let msg = if let Some(ref m) = self.status_message {
                m.as_str()
            } else {
                "No issues loaded. Click [  Load Issues ] on the tab bar or press 'r' to fetch from GitHub."
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
        let list_border_color = if is_focused && self.focus == IssueFocus::List {
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
        for (i, issue) in self
            .issues
            .iter()
            .enumerate()
            .skip(visible_range.start)
            .take(visible_range.end - visible_range.start)
        {
            let is_selected = i == self.list_scroll.selected_index;
            let (badge_text, badge_color) = match issue.state.as_str() {
                "open" => ("OPEN", theme.diff_add_fg),
                _ => ("CLOSED", Color::Rgb(163, 113, 247)),
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
                    format!("#{} ", issue.number),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD)
                        .bg(line_bg),
                ),
                Span::styled(
                    issue.title.clone(),
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

            // Subline: author & comments count & labels
            let mut sub_spans = vec![
                Span::styled("    by ", Style::default().fg(theme.dim_fg).bg(line_bg)),
                Span::styled(&issue.author, Style::default().fg(theme.fg).bg(line_bg)),
            ];

            if issue.comments_count > 0 {
                sub_spans.push(Span::styled(
                    format!(" 💬 {}", issue.comments_count),
                    Style::default().fg(theme.dim_fg).bg(line_bg),
                ));
            }

            if !issue.labels.is_empty() {
                let labels_str = issue.labels.join(", ");
                let truncated = if labels_str.len() > 24 {
                    format!(" [{:.21}...]", labels_str)
                } else {
                    format!(" [{}]", labels_str)
                };
                sub_spans.push(Span::styled(
                    truncated,
                    Style::default().fg(theme.accent).bg(line_bg),
                ));
            }

            lines.push(Line::from(sub_spans));
        }

        let list_p = Paragraph::new(lines);
        frame.render_widget(list_p, list_inner);

        // 2. Right detail pane
        let detail_border_color = if is_focused && self.focus == IssueFocus::Detail {
            theme.border_focused
        } else {
            theme.border
        };

        let detail_block = Block::default()
            .borders(Borders::NONE)
            .border_style(Style::default().fg(detail_border_color));
        let detail_inner = detail_block.inner(chunks[1]);
        frame.render_widget(detail_block, chunks[1]);

        let selected = self.selected_issue().cloned();
        if let Some(issue) = selected {
            let mut detail_lines = Vec::new();

            // Header info
            detail_lines.push(Line::from(vec![
                Span::styled(
                    format!("#{} ", issue.number),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &issue.title,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                ),
            ]));

            detail_lines.push(Line::from(vec![
                Span::styled("State: ", Style::default().fg(theme.dim_fg)),
                Span::styled(
                    issue.state.to_uppercase(),
                    Style::default()
                        .fg(if issue.state == "open" {
                            theme.diff_add_fg
                        } else {
                            Color::Rgb(163, 113, 247)
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  |  Author: ", Style::default().fg(theme.dim_fg)),
                Span::styled(&issue.author, Style::default().fg(theme.fg)),
                Span::styled("  |  Comments: ", Style::default().fg(theme.dim_fg)),
                Span::styled(
                    issue.comments_count.to_string(),
                    Style::default().fg(theme.fg),
                ),
            ]));

            if !issue.labels.is_empty() {
                detail_lines.push(Line::from(vec![
                    Span::styled("Labels: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(issue.labels.join(" • "), Style::default().fg(theme.accent)),
                ]));
            }

            if !issue.html_url.is_empty() {
                detail_lines.push(Line::from(vec![
                    Span::styled("URL: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(&issue.html_url, Style::default().fg(theme.primary)),
                ]));
            }

            detail_lines.push(Line::from(vec![Span::styled(
                "─".repeat(detail_inner.width.saturating_sub(2) as usize),
                Style::default().fg(theme.border),
            )]));

            // Issue description body
            if let Some(ref body) = issue.body {
                for line in body.lines() {
                    detail_lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(theme.fg),
                    )));
                }
            } else {
                detail_lines.push(Line::from(Span::styled(
                    "No description provided for this issue.",
                    Style::default().fg(theme.dim_fg),
                )));
            }

            // Keybindings help footer
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![
                Span::styled("Shortcuts: ", Style::default().fg(theme.dim_fg)),
                Span::styled(
                    "[o] ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Open in Browser   ", Style::default().fg(theme.fg)),
                Span::styled(
                    "[y] ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Copy URL   ", Style::default().fg(theme.fg)),
                Span::styled(
                    "[r] ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Reload Issues", Style::default().fg(theme.fg)),
            ]));

            self.detail_scroll.set_total_items(detail_lines.len());
            self.detail_scroll.viewport_height = detail_inner.height as usize;

            let detail_range = self.detail_scroll.visible_range();
            let visible_detail: Vec<Line> = detail_lines
                .into_iter()
                .skip(detail_range.start)
                .take(detail_range.end - detail_range.start)
                .collect();

            let detail_p = Paragraph::new(visible_detail).wrap(Wrap { trim: false });
            frame.render_widget(detail_p, detail_inner);
        }
    }
}
