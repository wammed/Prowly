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

/// Action button clicked in Changes view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesButtonAction {
    CommitAndSync,
    AiGenerate,
    AiReview,
    StashAll,
    DiscardSafe,
}

/// Rendering parameters bundled to avoid clippy::too_many_arguments.
pub struct ChangesRenderParams<'a> {
    pub files: &'a [GitFileItem],
    pub active_diff: &'a FileDiff,
    pub theme: &'a Theme,
    pub i18n: &'a I18n,
    pub icons: &'a Icons,
    pub area: Rect,
}

/// Changes view controller.
pub struct ChangesView {
    pub file_scroll: VirtualScroll,
    pub diff_view: DiffView,
    pub focus: ChangesFocus,
    pub commit_summary: String,
    pub commit_description: String,
    pub file_list_area: Rect,
    pub commit_controls_area: Rect,
    pub summary_input_area: Rect,
    pub desc_input_area: Rect,
    pub actions_bar_area: Rect,
    pub file_list_menu_area: Rect,
}

impl ChangesView {
    pub fn new() -> Self {
        Self {
            file_scroll: VirtualScroll::new(0, 10),
            diff_view: DiffView::new(),
            focus: ChangesFocus::FileList,
            commit_summary: String::new(),
            commit_description: String::new(),
            file_list_area: Rect::default(),
            commit_controls_area: Rect::default(),
            summary_input_area: Rect::default(),
            desc_input_area: Rect::default(),
            actions_bar_area: Rect::default(),
            file_list_menu_area: Rect::default(),
        }
    }

    /// Handle text input when a commit text box is focused.
    pub fn handle_char(&mut self, c: char) {
        match self.focus {
            ChangesFocus::CommitSummary => self.commit_summary.push(c),
            ChangesFocus::CommitDescription => self.commit_description.push(c),
            _ => {}
        }
    }

    /// Handle backspace in commit inputs.
    pub fn handle_backspace(&mut self) {
        match self.focus {
            ChangesFocus::CommitSummary => {
                self.commit_summary.pop();
            }
            ChangesFocus::CommitDescription => {
                self.commit_description.pop();
            }
            _ => {}
        }
    }

    /// Check which action button was clicked at (x, y).
    pub fn hit_test_action_buttons(&self, x: u16, y: u16) -> Option<ChangesButtonAction> {
        let area = self.actions_bar_area;
        if y != area.y || x < area.x || x >= area.x + area.width {
            return None;
        }

        let rel_x = x.saturating_sub(area.x);
        // Buttons layout approx:
        // [ Commit & Sync ] (19) [ AI Generate ] (17) [ AI Review ] (15) [ Stash All ] (15) [ Discard (Safe) ] (18)
        if rel_x < 20 {
            Some(ChangesButtonAction::CommitAndSync)
        } else if rel_x < 38 {
            Some(ChangesButtonAction::AiGenerate)
        } else if rel_x < 54 {
            Some(ChangesButtonAction::AiReview)
        } else if rel_x < 70 {
            Some(ChangesButtonAction::StashAll)
        } else {
            Some(ChangesButtonAction::DiscardSafe)
        }
    }

    /// Check if the file list header menu button was clicked.
    pub fn hit_test_menu_button(&self, x: u16, y: u16) -> bool {
        let r = self.file_list_menu_area;
        x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
    }

    /// Check if an input area or file list was clicked.
    pub fn hit_test_inputs(&self, x: u16, y: u16) -> Option<ChangesFocus> {
        let in_rect = |r: Rect| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height;
        if in_rect(self.summary_input_area) {
            Some(ChangesFocus::CommitSummary)
        } else if in_rect(self.desc_input_area) {
            Some(ChangesFocus::CommitDescription)
        } else if in_rect(self.file_list_area) {
            Some(ChangesFocus::FileList)
        } else {
            None
        }
    }

    /// Render Changes view layout into frame.
    pub fn render(&mut self, frame: &mut Frame, params: ChangesRenderParams) {
        let ChangesRenderParams {
            files,
            active_diff,
            theme,
            i18n,
            icons,
            area,
        } = params;

        // Vertical layout: Top (Files + Diff preview, Min) -> Bottom (Commit box + Actions, Length 6)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(6)])
            .split(area);

        let top_area = main_chunks[0];
        let bottom_area = main_chunks[1];
        self.commit_controls_area = bottom_area;

        // Horizontal split of top area: File list (36%) -> Diff preview (64%)
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(top_area);

        let file_list_area = top_chunks[0];
        let diff_area = top_chunks[1];
        self.file_list_area = file_list_area;

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
            " {} {} (Staged: {}/{}) ",
            icons.branch(),
            i18n.t("tabs.changes"),
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

        // Menu button in top right corner of file list box
        let menu_btn_w = 11;
        let menu_x = area.x + area.width.saturating_sub(menu_btn_w + 1);
        self.file_list_menu_area = Rect {
            x: menu_x,
            y: area.y,
            width: menu_btn_w,
            height: 1,
        };
        let menu_p = Paragraph::new(Span::styled(
            " [ 󰍜 Menu ▾ ] ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(menu_p, self.file_list_menu_area);

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
        &mut self,
        frame: &mut Frame,
        theme: &Theme,
        i18n: &I18n,
        icons: &Icons,
        area: Rect,
    ) {
        let is_any_commit_focused = self.focus == ChangesFocus::CommitSummary
            || self.focus == ChangesFocus::CommitDescription;
        let border_color = if is_any_commit_focused {
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
                    " {} & Actions (Press Tab/Click to Focus) ",
                    i18n.t("btn.commit")
                ),
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

        self.summary_input_area = chunks[0];
        self.desc_input_area = chunks[1];
        self.actions_bar_area = chunks[2];

        // Summary Input
        let is_summary_focused = self.focus == ChangesFocus::CommitSummary;
        let summary_text = if self.commit_summary.is_empty() {
            if is_summary_focused {
                "|".to_string()
            } else {
                i18n.t("changes.commit_placeholder").to_string()
            }
        } else if is_summary_focused {
            format!("{}|", self.commit_summary)
        } else {
            self.commit_summary.clone()
        };

        let summary_style = if self.commit_summary.is_empty() && !is_summary_focused {
            Style::default().fg(theme.dim_fg)
        } else {
            Style::default()
                .fg(theme.fg)
                .add_modifier(if is_summary_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                })
        };

        let summary_label_style = if is_summary_focused {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.primary)
        };

        let summary_line = Line::from(vec![
            Span::styled(" Commit: ", summary_label_style),
            Span::styled(summary_text, summary_style),
        ]);
        frame.render_widget(Paragraph::new(summary_line), chunks[0]);

        // Description Input (optional)
        let is_desc_focused = self.focus == ChangesFocus::CommitDescription;
        let desc_text = if self.commit_description.is_empty() {
            if is_desc_focused {
                "|".to_string()
            } else {
                i18n.t("changes.commit_desc_placeholder").to_string()
            }
        } else if is_desc_focused {
            format!("{}|", self.commit_description)
        } else {
            self.commit_description.clone()
        };

        let desc_style = if self.commit_description.is_empty() && !is_desc_focused {
            Style::default().fg(theme.dim_fg)
        } else {
            Style::default().fg(theme.fg)
        };

        let desc_line = Line::from(vec![
            Span::styled(
                " Desc:   ",
                if is_desc_focused {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default().fg(theme.dim_fg)
                },
            ),
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
