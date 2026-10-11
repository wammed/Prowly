//! Changes view combining staged/unstaged file list, Diff preview, commit inputs, and AI assist buttons.

use crate::git::{FileDiff, GitFileItem};
use crate::i18n::I18n;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::views::diff::DiffView;
use crate::ui::virtual_scroll::VirtualScroll;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Sub-focus within Changes view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesFocus {
    FileList,
    DiffPreview,
    CommitSummary,
    CommitDescription,
}

/// Changes view controller.
pub struct ChangesView {
    pub file_scroll: VirtualScroll,
    pub diff_view: DiffView,
    pub focus: ChangesFocus,
    pub commit_summary: String,
    pub commit_description: String,
}

impl ChangesView {
    pub fn new() -> Self {
        Self {
            file_scroll: VirtualScroll::new(0, 10),
            diff_view: DiffView::new(),
            focus: ChangesFocus::FileList,
            commit_summary: String::new(),
            commit_description: String::new(),
        }
    }

    /// Render Changes view layout into frame.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        files: &[GitFileItem],
        active_diff: &FileDiff,
        theme: &Theme,
        i18n: &I18n,
        icons: &Icons,
        area: Rect,
    ) {
        // Vertical layout: Top (Files + Diff preview, Min) -> Bottom (Commit box + Actions, Length 6)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(6)])
            .split(area);

        let top_area = main_chunks[0];
        let bottom_area = main_chunks[1];

        // Horizontal split of top area: File list (35% or 30-40 cols) -> Diff preview (remaining)
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(top_area);

        let file_list_area = top_chunks[0];
        let diff_area = top_chunks[1];

        // 1. Render File List
        self.render_file_list(frame, files, theme, i18n, icons, file_list_area);

        // 2. Render Diff Preview
        self.diff_view.render(frame, active_diff, theme, diff_area);

        // 3. Render Commit box & Actions bar
        self.render_commit_controls(frame, theme, i18n, icons, bottom_area);
    }

    fn render_file_list(
        &mut self,
        frame: &mut Frame,
        files: &[GitFileItem],
        theme: &Theme,
        i18n: &I18n,
        icons: &Icons,
        area: Rect,
    ) {
        let is_focused = self.focus == ChangesFocus::FileList;
        let border_color = if is_focused {
            theme.border_focused
        } else {
            theme.border
        };

        let staged_count = files.iter().filter(|f| f.staged).count();
        let total_count = files.len();

        let title = format!(
            " {} Changes (Staged: {}/{}) ",
            icons.branch(),
            staged_count,
            total_count
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        self.file_scroll.set_total_items(files.len());
        self.file_scroll.set_viewport_height(inner.height as usize);

        if files.is_empty() {
            let empty_p = Paragraph::new(format!(" {}", i18n.t("changes.no_changes")))
                .style(Style::default().fg(theme.dim_fg));
            frame.render_widget(empty_p, inner);
            return;
        }

        let visible = self.file_scroll.visible_range();
        let lines: Vec<Line> = files[visible]
            .iter()
            .enumerate()
            .map(|(rel_idx, item)| {
                let abs_idx = self.file_scroll.viewport_offset + rel_idx;
                let is_sel = abs_idx == self.file_scroll.selected_index;

                let checkbox = if item.staged {
                    icons.checkbox_checked()
                } else {
                    icons.checkbox_unchecked()
                };

                let badge = item.status.badge();
                let status_style = match badge {
                    "M" => Style::default().fg(theme.diff_mod_fg),
                    "+" => Style::default().fg(theme.diff_add_fg),
                    "-" => Style::default().fg(theme.diff_del_fg),
                    "!" => Style::default().fg(theme.conflict_fg),
                    _ => Style::default().fg(theme.untracked_fg),
                };

                let line_style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                Line::from(vec![
                    Span::styled(format!(" {} ", checkbox), line_style),
                    Span::styled(format!("[{}] ", badge), status_style),
                    Span::styled(item.path.clone(), line_style),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_commit_controls(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        i18n: &I18n,
        icons: &Icons,
        area: Rect,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " Commit & Actions ",
                Style::default().fg(theme.primary),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Subdivide inner into input line and actions button bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        // Summary Input
        let summary_text = if self.commit_summary.is_empty() {
            i18n.t("changes.commit_placeholder")
        } else {
            self.commit_summary.clone()
        };
        let summary_style = if self.commit_summary.is_empty() {
            Style::default().fg(theme.dim_fg)
        } else {
            Style::default().fg(theme.fg)
        };
        let summary_line = Line::from(vec![
            Span::styled(
                " Commit: ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(summary_text, summary_style),
        ]);
        frame.render_widget(Paragraph::new(summary_line), chunks[0]);

        // Description Input (optional)
        let desc_text = if self.commit_description.is_empty() {
            i18n.t("changes.commit_desc_placeholder")
        } else {
            self.commit_description.clone()
        };
        let desc_style = if self.commit_description.is_empty() {
            Style::default().fg(theme.dim_fg)
        } else {
            Style::default().fg(theme.fg)
        };
        let desc_line = Line::from(vec![
            Span::styled(" Desc:   ", Style::default().fg(theme.dim_fg)),
            Span::styled(desc_text, desc_style),
        ]);
        frame.render_widget(Paragraph::new(desc_line), chunks[1]);

        // Action buttons bar
        let buttons = vec![
            Span::styled(
                format!(" [ {} Commit & Sync ] ", icons.commit()),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" [ {} AI Generate ] ", icons.ai_sparkle()),
                Style::default().fg(theme.ai_sparkle),
            ),
            Span::styled(
                format!(" [ {} AI Review ] ", icons.ai_sparkle()),
                Style::default().fg(theme.info),
            ),
            Span::styled(
                format!(" [ {} Stash All ] ", icons.stash()),
                Style::default().fg(theme.secondary),
            ),
            Span::styled(
                format!(
                    " [ {} Discard (Safe) ] ",
                    icons.get(crate::ui::IconId::Trash)
                ),
                Style::default().fg(theme.error),
            ),
        ];
        frame.render_widget(Paragraph::new(Line::from(buttons)), chunks[2]);
    }
}

impl Default for ChangesView {
    fn default() -> Self {
        Self::new()
    }
}
