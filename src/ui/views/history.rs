//! Git Commit History and Graph view component with diff preview and commit inspection.

use crate::git::{CommitDetail, HistoryCommitItem, HistoryRow};
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

/// Sub-focus within Commit History view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFocus {
    CommitList,
    DiffPreview,
}

/// Commit History tab view controller.
pub struct HistoryView {
    pub rows: Vec<HistoryRow>,
    pub scroll: VirtualScroll,
    pub focus: HistoryFocus,
    pub selected_commit_hash: Option<String>,
    pub active_detail: Option<CommitDetail>,
    pub diff_scroll_offset: usize,
    pub list_area: Rect,
    pub diff_area: Rect,
}

impl HistoryView {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            scroll: VirtualScroll::new(0, 20),
            focus: HistoryFocus::CommitList,
            selected_commit_hash: None,
            active_detail: None,
            diff_scroll_offset: 0,
            list_area: Rect::default(),
            diff_area: Rect::default(),
        }
    }

    /// Set current history rows and reset scroll if empty or out of bounds.
    pub fn set_rows(&mut self, rows: Vec<HistoryRow>) {
        self.rows = rows;
        self.scroll.set_total_items(self.rows.len());
        if self.scroll.selected_index >= self.rows.len() {
            self.scroll.selected_index = 0;
        }
        self.update_selected_commit_hash();
    }

    /// Update detail for the currently selected commit.
    pub fn set_detail(&mut self, detail: CommitDetail) {
        self.active_detail = Some(detail);
        self.diff_scroll_offset = 0;
    }

    /// Retrieve currently selected commit item if selected row is a commit.
    pub fn selected_commit(&self) -> Option<&HistoryCommitItem> {
        if self.rows.is_empty() {
            return None;
        }
        let idx = self
            .scroll
            .selected_index
            .min(self.rows.len().saturating_sub(1));
        self.rows.get(idx).and_then(|r| r.commit())
    }

    /// Move to next commit, skipping pure graph connector lines.
    pub fn next_commit(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let mut idx = self.scroll.selected_index;
        while idx + 1 < self.rows.len() {
            idx += 1;
            if matches!(self.rows.get(idx), Some(HistoryRow::Commit(_))) {
                self.scroll.selected_index = idx;
                self.scroll.ensure_visible();
                self.update_selected_commit_hash();
                return;
            }
        }
    }

    /// Move to previous commit, skipping pure graph connector lines.
    pub fn previous_commit(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        let mut idx = self.scroll.selected_index;
        while idx > 0 {
            idx -= 1;
            if matches!(self.rows.get(idx), Some(HistoryRow::Commit(_))) {
                self.scroll.selected_index = idx;
                self.scroll.ensure_visible();
                self.update_selected_commit_hash();
                return;
            }
        }
    }

    /// Scroll diff preview down.
    pub fn scroll_diff_down(&mut self, lines: usize) {
        if let Some(detail) = &self.active_detail {
            let max_offset = detail.diff_lines.len().saturating_sub(5);
            self.diff_scroll_offset = (self.diff_scroll_offset + lines).min(max_offset);
        }
    }

    /// Scroll diff preview up.
    pub fn scroll_diff_up(&mut self, lines: usize) {
        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(lines);
    }

    /// Ensure selected commit hash matches current cursor.
    fn update_selected_commit_hash(&mut self) {
        let hash = self.selected_commit().map(|c| c.hash.clone());
        if hash != self.selected_commit_hash {
            self.selected_commit_hash = hash;
            // Clear current detail so caller can reload if changed
            self.active_detail = None;
            self.diff_scroll_offset = 0;
        }
    }

    /// Render History view split into left Graph list and right Detail/Diff preview.
    pub fn render(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, area: Rect) {
        // Horizontal split: 52% Graph & Commits, 48% Diff Preview & Details
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(area);

        self.list_area = main_chunks[0];
        self.diff_area = main_chunks[1];

        self.render_commit_list(frame, theme, icons, self.list_area);
        self.render_diff_preview(frame, theme, icons, self.diff_area);
    }

    fn render_commit_list(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, area: Rect) {
        let is_focused = self.focus == HistoryFocus::CommitList;
        let border_color = if is_focused {
            theme.border_focused
        } else {
            theme.border
        };

        let commit_count = self
            .rows
            .iter()
            .filter(|r| matches!(r, HistoryRow::Commit(_)))
            .count();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(
                    " {} Git Graph & History ({} commits) ",
                    icons.commit(),
                    commit_count
                ),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.rows.is_empty() {
            let empty_text = Paragraph::new(Line::from(vec![Span::styled(
                " No commit history available.",
                Style::default().fg(theme.dim_fg),
            )]))
            .style(Style::default().bg(theme.bg));
            frame.render_widget(empty_text, inner);
            return;
        }

        self.scroll.set_total_items(self.rows.len());
        self.scroll.set_viewport_height(inner.height as usize);

        let visible = self.scroll.visible_range();
        let mut lines = Vec::new();

        for (rel_idx, row) in self.rows[visible].iter().enumerate() {
            let abs_idx = self.scroll.viewport_offset + rel_idx;
            let is_sel = abs_idx == self.scroll.selected_index;

            match row {
                HistoryRow::Commit(commit) => {
                    let mut spans = Vec::new();

                    // Selection marker
                    if is_sel {
                        spans.push(Span::styled(
                            "▶ ",
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::raw("  "));
                    }

                    // Render Graph characters with distinct styling
                    for ch in commit.graph.chars() {
                        let graph_style = match ch {
                            '*' | 'o' => Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                            '|' => Style::default().fg(theme.primary),
                            '/' | '\\' => Style::default().fg(theme.warning),
                            _ => Style::default().fg(theme.dim_fg),
                        };
                        spans.push(Span::styled(ch.to_string(), graph_style));
                    }

                    // Short SHA
                    let hash_style = if is_sel {
                        Style::default()
                            .fg(theme.warning)
                            .bg(theme.selection_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.warning)
                    };
                    spans.push(Span::styled(format!(" {} ", commit.hash), hash_style));

                    // Refs (Branch / Tag badge)
                    if !commit.refs.is_empty() {
                        let ref_style = Style::default()
                            .fg(theme.success)
                            .add_modifier(Modifier::BOLD);
                        spans.push(Span::styled(format!("({}) ", commit.refs), ref_style));
                    }

                    // Summary message
                    let summary_style = if is_sel {
                        Style::default()
                            .fg(theme.selection_fg)
                            .bg(theme.selection_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    spans.push(Span::styled(commit.summary.clone(), summary_style));

                    // Author & Date
                    spans.push(Span::styled(
                        format!(" • {} ({})", commit.author, commit.date),
                        Style::default().fg(theme.dim_fg),
                    ));

                    let line_style = if is_sel {
                        Style::default().bg(theme.selection_bg)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(spans).style(line_style));
                }
                HistoryRow::GraphOnly(graph_str) => {
                    let mut spans = Vec::new();
                    spans.push(Span::raw("  "));
                    for ch in graph_str.chars() {
                        let graph_style = match ch {
                            '*' | 'o' => Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                            '|' => Style::default().fg(theme.primary),
                            '/' | '\\' => Style::default().fg(theme.warning),
                            _ => Style::default().fg(theme.dim_fg),
                        };
                        spans.push(Span::styled(ch.to_string(), graph_style));
                    }
                    lines.push(Line::from(spans));
                }
            }
        }

        let widget = Paragraph::new(lines).style(Style::default().bg(theme.bg));
        frame.render_widget(widget, inner);
    }

    fn render_diff_preview(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, area: Rect) {
        let is_focused = self.focus == HistoryFocus::DiffPreview;
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
                format!(" {} Commit Details & Diff Preview ", icons.file()),
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(detail) = &self.active_detail else {
            let placeholder = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        " Select a commit on the left to inspect changes & diff.",
                        Style::default().fg(theme.dim_fg),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Quick Actions: ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                ]),
                Line::from("  [Enter / m] Action Menu (Checkout, Branch, Cherry-pick, Revert, Time Machine)"),
                Line::from("  [c] Checkout Commit (Detached HEAD)"),
                Line::from("  [b] Create Branch from here"),
                Line::from("  [p] Cherry-pick to current HEAD"),
                Line::from("  [r] Revert commit"),
                Line::from("  [t] Time Machine: Reset branch to this commit"),
                Line::from("  [y] Copy commit SHA"),
            ])
            .style(Style::default().bg(theme.bg));
            frame.render_widget(placeholder, inner);
            return;
        };

        // Split inner into Header/Meta (Length 6..8) -> Diff content (Min 5) -> Help footer (Length 2)
        let header_height = if detail.stats.is_empty() {
            5
        } else {
            7.min(inner.height.saturating_sub(6))
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Min(4),
                Constraint::Length(2),
            ])
            .split(inner);

        let header_area = chunks[0];
        let diff_content_area = chunks[1];
        let footer_area = chunks[2];

        // 1. Meta / Summary header
        let mut header_lines = Vec::new();
        header_lines.push(Line::from(vec![
            Span::styled(
                "Commit: ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&detail.hash, Style::default().fg(theme.warning)),
        ]));
        header_lines.push(Line::from(vec![
            Span::styled("Author: ", Style::default().fg(theme.dim_fg)),
            Span::styled(&detail.author, Style::default().fg(theme.fg)),
            Span::raw("  "),
            Span::styled("Date: ", Style::default().fg(theme.dim_fg)),
            Span::styled(&detail.date, Style::default().fg(theme.fg)),
        ]));
        header_lines.push(Line::from(vec![
            Span::styled("Message: ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                &detail.summary,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
        ]));

        if !detail.stats.is_empty() {
            let stats_summary = detail.stats.join("  |  ");
            header_lines.push(Line::from(vec![
                Span::styled("Files: ", Style::default().fg(theme.dim_fg)),
                Span::styled(stats_summary, Style::default().fg(theme.accent)),
            ]));
        }

        let header_widget = Paragraph::new(header_lines).style(Style::default().bg(theme.bg));
        frame.render_widget(header_widget, header_area);

        // 2. Diff preview content (showing first ~20 lines by default or scrolled)
        let diff_lines_count = detail.diff_lines.len();
        let visible_diff_lines = if diff_lines_count == 0 {
            vec![Line::from(vec![Span::styled(
                " (No text diff or merge commit without patch changes)",
                Style::default().fg(theme.dim_fg),
            )])]
        } else {
            let start = self.diff_scroll_offset.min(diff_lines_count);
            let take_len =
                (diff_content_area.height as usize).min(diff_lines_count.saturating_sub(start));
            detail.diff_lines[start..start + take_len]
                .iter()
                .map(|line_str| {
                    if line_str.starts_with('+') && !line_str.starts_with("+++") {
                        Line::from(Span::styled(
                            line_str.clone(),
                            Style::default().fg(theme.diff_add_fg),
                        ))
                    } else if line_str.starts_with('-') && !line_str.starts_with("---") {
                        Line::from(Span::styled(
                            line_str.clone(),
                            Style::default().fg(theme.diff_del_fg),
                        ))
                    } else if line_str.starts_with("@@") {
                        Line::from(Span::styled(
                            line_str.clone(),
                            Style::default()
                                .fg(theme.diff_hunk_fg)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if line_str.starts_with("diff --git") {
                        Line::from(Span::styled(
                            line_str.clone(),
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(Span::styled(
                            line_str.clone(),
                            Style::default().fg(theme.dim_fg),
                        ))
                    }
                })
                .collect()
        };

        let diff_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                format!(
                    " Diff ({} lines, offset {}) ",
                    diff_lines_count, self.diff_scroll_offset
                ),
                Style::default().fg(theme.dim_fg),
            ));
        let diff_inner = diff_block.inner(diff_content_area);
        frame.render_widget(diff_block, diff_content_area);

        let diff_widget = Paragraph::new(visible_diff_lines).style(Style::default().bg(theme.bg));
        frame.render_widget(diff_widget, diff_inner);

        // 3. Footer shortcut bar
        let footer_spans = vec![
            Span::styled(
                "[Enter/m] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Action Menu  "),
            Span::styled("[c] ", Style::default().fg(theme.accent)),
            Span::raw("Checkout  "),
            Span::styled("[b] ", Style::default().fg(theme.accent)),
            Span::raw("Branch  "),
            Span::styled("[p] ", Style::default().fg(theme.accent)),
            Span::raw("Cherry-pick  "),
            Span::styled("[r] ", Style::default().fg(theme.accent)),
            Span::raw("Revert  "),
            Span::styled(
                "[t] ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Time Machine"),
        ];
        let footer_widget =
            Paragraph::new(Line::from(footer_spans)).style(Style::default().bg(theme.bg));
        frame.render_widget(footer_widget, footer_area);
    }
}

impl Default for HistoryView {
    fn default() -> Self {
        Self::new()
    }
}
