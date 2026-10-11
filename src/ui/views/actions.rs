//! GitHub Actions CI runs and log inspection view.

use crate::github::WorkflowRunItem;
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

/// Focus state within Actions view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionsFocus {
    RunsList,
    LogView,
}

/// GitHub Actions view component.
pub struct ActionsView {
    pub runs: Vec<WorkflowRunItem>,
    pub list_scroll: VirtualScroll,
    pub log_scroll: VirtualScroll,
    pub focus: ActionsFocus,
    pub status_message: Option<String>,
}

impl ActionsView {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            list_scroll: VirtualScroll::new(0, 10),
            log_scroll: VirtualScroll::new(0, 10),
            focus: ActionsFocus::RunsList,
            status_message: None,
        }
    }

    pub fn set_runs(&mut self, runs: Vec<WorkflowRunItem>) {
        self.list_scroll.set_total_items(runs.len());
        self.runs = runs;
    }

    pub fn selected_run(&self) -> Option<&WorkflowRunItem> {
        self.runs.get(self.list_scroll.selected_index)
    }

    pub fn selected_run_mut(&mut self) -> Option<&mut WorkflowRunItem> {
        let idx = self.list_scroll.selected_index;
        self.runs.get_mut(idx)
    }

    pub fn next(&mut self) {
        match self.focus {
            ActionsFocus::RunsList => {
                self.list_scroll.next();
                self.log_scroll.scroll_to_top();
            }
            ActionsFocus::LogView => {
                self.log_scroll.next();
            }
        }
    }

    pub fn previous(&mut self) {
        match self.focus {
            ActionsFocus::RunsList => {
                self.list_scroll.previous();
                self.log_scroll.scroll_to_top();
            }
            ActionsFocus::LogView => {
                self.log_scroll.previous();
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            ActionsFocus::RunsList => ActionsFocus::LogView,
            ActionsFocus::LogView => ActionsFocus::RunsList,
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
                format!(" {} ACTIONS & CI ", icons.check_success()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.runs.is_empty() {
            let msg = if let Some(ref m) = self.status_message {
                m.as_str()
            } else {
                "No GitHub Actions workflow runs found."
            };
            let p = Paragraph::new(format!("\n  {}", msg)).style(Style::default().fg(theme.dim_fg));
            frame.render_widget(p, inner);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner);

        // 1. Left: runs list
        let list_border_color = if is_focused && self.focus == ActionsFocus::RunsList {
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
        for (i, run) in self
            .runs
            .iter()
            .enumerate()
            .skip(visible_range.start)
            .take(visible_range.end - visible_range.start)
        {
            let is_selected = i == self.list_scroll.selected_index;
            let (icon, color) = match run.conclusion.as_deref() {
                Some("success") => ("✓", theme.diff_add_fg),
                Some("failure") => ("✗", theme.diff_del_fg),
                Some("cancelled") => ("⊘", theme.dim_fg),
                _ => {
                    if run.status == "in_progress" {
                        ("●", Color::Rgb(249, 226, 175))
                    } else {
                        ("○", theme.dim_fg)
                    }
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
                    format!("[{}] ", icon),
                    Style::default()
                        .fg(color)
                        .add_modifier(Modifier::BOLD)
                        .bg(line_bg),
                ),
                Span::styled(
                    run.name.clone(),
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

            // Subline: event & branch
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default().bg(line_bg)),
                Span::styled(&run.event, Style::default().fg(theme.primary).bg(line_bg)),
                Span::styled(" on ", Style::default().fg(theme.dim_fg).bg(line_bg)),
                Span::styled(&run.head_branch, Style::default().fg(theme.fg).bg(line_bg)),
                Span::styled(
                    format!(" ({})", run.status),
                    Style::default().fg(theme.dim_fg).bg(line_bg),
                ),
            ]));
        }

        let list_p = Paragraph::new(lines);
        frame.render_widget(list_p, list_inner);

        // 2. Right: run detail & log viewer
        let log_border_color = if is_focused && self.focus == ActionsFocus::LogView {
            theme.border_focused
        } else {
            theme.border
        };

        let log_block = Block::default()
            .borders(Borders::NONE)
            .border_style(Style::default().fg(log_border_color));
        let log_inner = log_block.inner(chunks[1]);
        frame.render_widget(log_block, chunks[1]);

        let selected_idx = self.list_scroll.selected_index;
        if let Some(run) = self.runs.get(selected_idx) {
            let detail_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ])
                .split(log_inner);

            // Metadata header
            let header = vec![
                Line::from(vec![Span::styled(
                    format!("Run #{} - {}", run.id, run.name),
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(
                        run.conclusion.as_deref().unwrap_or(&run.status),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(
                        format!("  Branch: {}", run.head_branch),
                        Style::default().fg(theme.dim_fg),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(header), detail_chunks[0]);

            // Log body or guidance
            let logs_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border));
            let logs_inner = logs_block.inner(detail_chunks[1]);
            frame.render_widget(logs_block, detail_chunks[1]);

            if let Some(ref raw_logs) = run.logs {
                let log_lines: Vec<&str> = raw_logs.lines().collect();
                self.log_scroll.viewport_height = logs_inner.height as usize;
                self.log_scroll.set_total_items(log_lines.len());
                let range = self.log_scroll.visible_range();

                let rendered_logs: Vec<Line> = log_lines
                    .iter()
                    .skip(range.start)
                    .take(range.end - range.start)
                    .map(|l| {
                        let color = if l.contains("error") || l.contains("FAIL") {
                            theme.diff_del_fg
                        } else if l.contains("warn") {
                            Color::Rgb(249, 226, 175)
                        } else {
                            theme.fg
                        };
                        Line::from(Span::styled(*l, Style::default().fg(color)))
                    })
                    .collect();

                frame.render_widget(Paragraph::new(rendered_logs), logs_inner);
            } else {
                let prompt = if run.conclusion.as_deref() == Some("failure") {
                    "Press [ Ctrl+F ] to analyze failure with AI co-pilot."
                } else {
                    "Select a run or press [ R ] to refresh status."
                };
                let placeholder = Paragraph::new(format!("\n  {}", prompt))
                    .style(Style::default().fg(theme.dim_fg))
                    .wrap(Wrap { trim: false });
                frame.render_widget(placeholder, logs_inner);
            }

            // Action bar
            let actions = vec![Line::from(vec![
                Span::styled("[ r: Re-run ]  ", Style::default().fg(theme.primary)),
                Span::styled(
                    "[ Ctrl+F: AI Analyze Failure ]  ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("[ y: Copy URL ]  ", Style::default().fg(theme.dim_fg)),
                Span::styled("[ R: Refresh ]", Style::default().fg(theme.dim_fg)),
            ])];
            frame.render_widget(Paragraph::new(actions), detail_chunks[2]);
        }
    }
}
