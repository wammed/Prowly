//! Modal for Git commit history actions (checkout, branch creation, cherry-pick, revert, time machine reset).

use crate::git::HistoryCommitItem;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Available action choices in the commit action menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryMenuAction {
    Checkout,
    CreateBranch,
    CherryPick,
    Revert,
    TimeMachine,
    CopySha,
    CopyMessage,
}

impl HistoryMenuAction {
    pub const ALL: &'static [HistoryMenuAction] = &[
        HistoryMenuAction::Checkout,
        HistoryMenuAction::CreateBranch,
        HistoryMenuAction::CherryPick,
        HistoryMenuAction::Revert,
        HistoryMenuAction::TimeMachine,
        HistoryMenuAction::CopySha,
        HistoryMenuAction::CopyMessage,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Checkout => "Checkout Commit (Detached HEAD)",
            Self::CreateBranch => "Create New Branch from here...",
            Self::CherryPick => "Cherry-pick into current HEAD",
            Self::Revert => "Revert Commit (Create inverse commit)",
            Self::TimeMachine => "Time Machine: Reset branch here (Discard newer)",
            Self::CopySha => "Copy Full Commit SHA",
            Self::CopyMessage => "Copy Commit Message",
        }
    }

    pub fn shortcut(&self) -> &'static str {
        match self {
            Self::Checkout => "[c]",
            Self::CreateBranch => "[b]",
            Self::CherryPick => "[p]",
            Self::Revert => "[r]",
            Self::TimeMachine => "[t]",
            Self::CopySha => "[y]",
            Self::CopyMessage => "[m]",
        }
    }
}

/// State of the commit actions modal dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryActionState {
    Menu,
    BranchInput,
    TimeMachineConfirm,
}

/// Modal controller for interacting with a selected commit.
#[derive(Debug, Clone)]
pub struct HistoryActionsModal {
    pub is_open: bool,
    pub state: HistoryActionState,
    pub commit: Option<HistoryCommitItem>,
    pub selected_menu_idx: usize,
    pub new_branch_name: String,
}

impl Default for HistoryActionsModal {
    fn default() -> Self {
        Self {
            is_open: false,
            state: HistoryActionState::Menu,
            commit: None,
            selected_menu_idx: 0,
            new_branch_name: String::new(),
        }
    }
}

impl HistoryActionsModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open modal for the specified commit in menu mode.
    pub fn open(&mut self, commit: HistoryCommitItem) {
        self.is_open = true;
        self.state = HistoryActionState::Menu;
        self.commit = Some(commit);
        self.selected_menu_idx = 0;
        self.new_branch_name.clear();
    }

    /// Open modal directly to branch name input mode.
    pub fn open_branch_input(&mut self, commit: HistoryCommitItem) {
        self.is_open = true;
        self.state = HistoryActionState::BranchInput;
        self.commit = Some(commit);
        self.new_branch_name.clear();
    }

    /// Open modal directly to Time Machine confirmation mode.
    pub fn open_time_machine(&mut self, commit: HistoryCommitItem) {
        self.is_open = true;
        self.state = HistoryActionState::TimeMachineConfirm;
        self.commit = Some(commit);
    }

    /// Close modal and reset state.
    pub fn close(&mut self) {
        self.is_open = false;
        self.state = HistoryActionState::Menu;
        self.commit = None;
        self.new_branch_name.clear();
    }

    /// Move menu cursor down.
    pub fn next(&mut self) {
        if self.state == HistoryActionState::Menu
            && self.selected_menu_idx + 1 < HistoryMenuAction::ALL.len()
        {
            self.selected_menu_idx += 1;
        }
    }

    /// Move menu cursor up.
    pub fn previous(&mut self) {
        if self.state == HistoryActionState::Menu && self.selected_menu_idx > 0 {
            self.selected_menu_idx -= 1;
        }
    }

    /// Retrieve currently selected menu action.
    pub fn selected_action(&self) -> HistoryMenuAction {
        HistoryMenuAction::ALL[self.selected_menu_idx]
    }

    /// Push character to branch input.
    pub fn push_char(&mut self, c: char) {
        if self.state == HistoryActionState::BranchInput {
            self.new_branch_name.push(c);
        }
    }

    /// Pop character from branch input.
    pub fn pop_char(&mut self) {
        if self.state == HistoryActionState::BranchInput {
            self.new_branch_name.pop();
        }
    }

    /// Render popup modal overlay based on current state.
    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, screen: Rect) {
        if !self.is_open {
            return;
        }

        let Some(commit) = &self.commit else {
            return;
        };

        match self.state {
            HistoryActionState::Menu => self.render_menu(frame, theme, icons, screen, commit),
            HistoryActionState::BranchInput => {
                self.render_branch_input(frame, theme, icons, screen, commit)
            }
            HistoryActionState::TimeMachineConfirm => {
                self.render_time_machine_confirm(frame, theme, icons, screen, commit)
            }
        }
    }

    fn render_menu(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        icons: &Icons,
        screen: Rect,
        commit: &HistoryCommitItem,
    ) {
        let width = 64.min(screen.width.saturating_sub(4));
        let height = 15.min(screen.height.saturating_sub(4));
        let x = (screen.width.saturating_sub(width)) / 2;
        let y = (screen.height.saturating_sub(height)) / 2;
        let area = Rect {
            x,
            y,
            width,
            height,
        };

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                format!(" {} Actions for commit {} ", icons.commit(), commit.hash),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();
        // Commit subject preview line
        lines.push(Line::from(vec![
            Span::styled("Subject: ", Style::default().fg(theme.dim_fg)),
            Span::styled(
                &commit.summary,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        for (idx, action) in HistoryMenuAction::ALL.iter().enumerate() {
            let is_sel = idx == self.selected_menu_idx;
            let marker = if is_sel { "▶ " } else { "  " };

            let label_style = if is_sel {
                Style::default()
                    .bg(theme.selection_bg)
                    .fg(theme.selection_fg)
                    .add_modifier(Modifier::BOLD)
            } else if *action == HistoryMenuAction::TimeMachine {
                Style::default().fg(theme.warning)
            } else {
                Style::default().fg(theme.fg)
            };

            let shortcut_style = Style::default().fg(theme.dim_fg);

            lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.accent)),
                Span::styled(format!("{:<46} ", action.label()), label_style),
                Span::styled(action.shortcut(), shortcut_style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            " [↑/↓] Select  [Enter] Execute  [Esc] Cancel",
            Style::default().fg(theme.dim_fg),
        )]));

        let widget = Paragraph::new(lines).style(Style::default().bg(theme.bg));
        frame.render_widget(widget, inner);
    }

    fn render_branch_input(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        icons: &Icons,
        screen: Rect,
        commit: &HistoryCommitItem,
    ) {
        let width = 60.min(screen.width.saturating_sub(4));
        let height = 9.min(screen.height.saturating_sub(4));
        let x = (screen.width.saturating_sub(width)) / 2;
        let y = (screen.height.saturating_sub(height)) / 2;
        let area = Rect {
            x,
            y,
            width,
            height,
        };

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(
                format!(" {} Create Branch at {} ", icons.branch(), commit.hash),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(vec![
                Span::styled("Target Commit: ", Style::default().fg(theme.dim_fg)),
                Span::styled(
                    format!("{} - {}", commit.hash, commit.summary),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "New Branch Name: ",
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if self.new_branch_name.is_empty() {
                        "type-branch-name..."
                    } else {
                        &self.new_branch_name
                    },
                    if self.new_branch_name.is_empty() {
                        Style::default().fg(theme.dim_fg)
                    } else {
                        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
                    },
                ),
                Span::styled("█", Style::default().fg(theme.accent)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                " [Enter] Create Branch  [Esc] Cancel",
                Style::default().fg(theme.dim_fg),
            )]),
        ];

        let widget = Paragraph::new(lines).style(Style::default().bg(theme.bg));
        frame.render_widget(widget, inner);
    }

    fn render_time_machine_confirm(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        icons: &Icons,
        screen: Rect,
        commit: &HistoryCommitItem,
    ) {
        let width = 68.min(screen.width.saturating_sub(4));
        let height = 12.min(screen.height.saturating_sub(4));
        let x = (screen.width.saturating_sub(width)) / 2;
        let y = (screen.height.saturating_sub(height)) / 2;
        let area = Rect {
            x,
            y,
            width,
            height,
        };

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                format!(" ⚠️ {} Time Machine: Reset to this commit ", icons.commit()),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "You are about to rewind your current branch to commit: ",
                    Style::default().fg(theme.fg),
                ),
                Span::styled(
                    &commit.hash,
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Message: ", Style::default().fg(theme.dim_fg)),
                Span::styled(&commit.summary, Style::default().fg(theme.fg)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Notice: ",
                    Style::default()
                        .fg(theme.error)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "All commits created AFTER this commit will be discarded from HEAD.",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![Span::styled(
                "A safety backup branch (backup/timemachine-...) will be created automatically.",
                Style::default().fg(theme.success),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    " [y / Enter] ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Confirm Reset    ", Style::default().fg(theme.fg)),
                Span::styled(" [n / Esc] ", Style::default().fg(theme.dim_fg)),
                Span::styled("Cancel", Style::default().fg(theme.fg)),
            ]),
        ];

        let widget = Paragraph::new(lines).style(Style::default().bg(theme.bg));
        frame.render_widget(widget, inner);
    }
}
