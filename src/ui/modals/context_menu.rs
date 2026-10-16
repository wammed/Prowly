//! Right-click context popup menu component.

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

/// Actions available in the right-click context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuAction {
    // File actions (Changes tab)
    ToggleStage,
    OpenEditor,
    RevealFileManager,
    StashFile,
    SafeDiscard,
    CopyPath,
    CopyDiff,
    AskAi,
    PreviewMarkdown,

    // Commit actions (History tab)
    CheckoutCommit,
    CreateBranch,
    CherryPick,
    RevertCommit,
    TimeMachine,
    CopyCommitSha,
    CopyCommitMessage,
    AskAiCommit,
}

impl ContextMenuAction {
    pub const FILE_ACTIONS: &'static [ContextMenuAction] = &[
        ContextMenuAction::ToggleStage,
        ContextMenuAction::SafeDiscard,
        ContextMenuAction::CopyPath,
        ContextMenuAction::CopyDiff,
        ContextMenuAction::AskAi,
        ContextMenuAction::PreviewMarkdown,
        ContextMenuAction::OpenEditor,
        ContextMenuAction::RevealFileManager,
        ContextMenuAction::StashFile,
    ];

    pub const COMMIT_ACTIONS: &'static [ContextMenuAction] = &[
        ContextMenuAction::CheckoutCommit,
        ContextMenuAction::CreateBranch,
        ContextMenuAction::CherryPick,
        ContextMenuAction::RevertCommit,
        ContextMenuAction::TimeMachine,
        ContextMenuAction::CopyCommitSha,
        ContextMenuAction::CopyCommitMessage,
        ContextMenuAction::AskAiCommit,
    ];

    pub const ALL: &'static [ContextMenuAction] = Self::FILE_ACTIONS;

    pub fn label(&self) -> &'static str {
        match self {
            Self::ToggleStage => "Stage / Unstage (Space)",
            Self::OpenEditor => "Open in Editor (e)",
            Self::RevealFileManager => "Reveal in File Manager",
            Self::StashFile => "Stash this file",
            Self::SafeDiscard => "Discard changes (Safe)",
            Self::CopyPath => "Copy File Path (y)",
            Self::CopyDiff => "Copy Diff Snippet",
            Self::AskAi => "Ask AI about this file (?)",
            Self::PreviewMarkdown => "Preview Markdown (m)",

            Self::CheckoutCommit => "Checkout Commit (c)",
            Self::CreateBranch => "Create Branch here... (b)",
            Self::CherryPick => "Cherry-pick into HEAD (p)",
            Self::RevertCommit => "Revert Commit (r)",
            Self::TimeMachine => "Time Machine: Reset here (t)",
            Self::CopyCommitSha => "Copy Commit SHA (y)",
            Self::CopyCommitMessage => "Copy Commit Message (m)",
            Self::AskAiCommit => "Ask AI about this commit (?)",
        }
    }
}

/// Floating context menu state.
#[derive(Debug, Clone, Default)]
pub struct ContextMenu {
    pub is_open: bool,
    pub selected_index: usize,
    pub target_file: Option<String>,
    pub target_commit: Option<HistoryCommitItem>,
    pub anchor_x: u16,
    pub anchor_y: u16,
}

impl ContextMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open context menu for a specific target file at anchor coordinates.
    pub fn open_at(&mut self, file: Option<String>, x: u16, y: u16) {
        self.is_open = true;
        self.selected_index = 0;
        self.target_file = file;
        self.target_commit = None;
        self.anchor_x = x;
        self.anchor_y = y;
    }

    /// Open context menu for a specific commit item at anchor coordinates.
    pub fn open_for_commit(&mut self, commit: HistoryCommitItem, x: u16, y: u16) {
        self.is_open = true;
        self.selected_index = 0;
        self.target_file = None;
        self.target_commit = Some(commit);
        self.anchor_x = x;
        self.anchor_y = y;
    }

    /// Close menu.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Determine active list of actions based on target type.
    pub fn current_actions(&self) -> &'static [ContextMenuAction] {
        if self.target_commit.is_some() {
            ContextMenuAction::COMMIT_ACTIONS
        } else {
            ContextMenuAction::FILE_ACTIONS
        }
    }

    /// Move selection down.
    pub fn next(&mut self) {
        let actions = self.current_actions();
        if self.selected_index + 1 < actions.len() {
            self.selected_index += 1;
        }
    }

    /// Move selection up.
    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Retrieve selected action.
    pub fn selected_action(&self) -> ContextMenuAction {
        let actions = self.current_actions();
        actions[self.selected_index.min(actions.len().saturating_sub(1))]
    }

    /// Calculate bounding rectangle of popup menu.
    pub fn menu_rect(&self, viewport: Rect) -> Rect {
        let actions = self.current_actions();
        let width = if self.target_commit.is_some() {
            38.min(viewport.width)
        } else {
            32.min(viewport.width)
        };
        let height = (actions.len() as u16 + 2).min(viewport.height);
        let x = self.anchor_x.min(viewport.width.saturating_sub(width));
        let y = self.anchor_y.min(viewport.height.saturating_sub(height));
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Hit-test mouse coordinate against context menu items.
    pub fn hit_test(&self, x: u16, y: u16, viewport: Rect) -> Option<ContextMenuAction> {
        if !self.is_open {
            return None;
        }
        let actions = self.current_actions();
        let rect = self.menu_rect(viewport);
        if x > rect.x
            && x < rect.x + rect.width.saturating_sub(1)
            && y > rect.y
            && y < rect.y + rect.height.saturating_sub(1)
        {
            let row = (y.saturating_sub(rect.y + 1)) as usize;
            if row < actions.len() {
                return Some(actions[row]);
            }
        }
        None
    }

    /// Render popup menu onto frame.
    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let actions = self.current_actions();
        let rect = self.menu_rect(viewport);

        frame.render_widget(Clear, rect);

        let title = if let Some(ref commit) = self.target_commit {
            let short_sha = &commit.hash[..7.min(commit.hash.len())];
            format!(" {} Commit {} ", icons.commit(), short_sha)
        } else if let Some(ref f) = self.target_file {
            format!(" {} ", f)
        } else {
            " Context Menu ".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        let lines: Vec<Line> = actions
            .iter()
            .enumerate()
            .map(|(idx, action)| {
                let is_sel = idx == self.selected_index;
                let icon_str = match action {
                    ContextMenuAction::ToggleStage => icons.checkbox_checked(),
                    ContextMenuAction::OpenEditor => "󰅩",
                    ContextMenuAction::RevealFileManager => icons.folder_open(),
                    ContextMenuAction::StashFile => icons.stash(),
                    ContextMenuAction::SafeDiscard => icons.get(crate::ui::IconId::Trash),
                    ContextMenuAction::CopyPath => icons.get(crate::ui::IconId::Copy),
                    ContextMenuAction::CopyDiff => icons.get(crate::ui::IconId::Copy),
                    ContextMenuAction::AskAi => icons.ai_sparkle(),
                    ContextMenuAction::PreviewMarkdown => icons.file(),

                    ContextMenuAction::CheckoutCommit => icons.commit(),
                    ContextMenuAction::CreateBranch => icons.branch(),
                    ContextMenuAction::CherryPick => "󰐱",
                    ContextMenuAction::RevertCommit => "󰁯",
                    ContextMenuAction::TimeMachine => "⏳",
                    ContextMenuAction::CopyCommitSha => icons.get(crate::ui::IconId::Copy),
                    ContextMenuAction::CopyCommitMessage => icons.get(crate::ui::IconId::Copy),
                    ContextMenuAction::AskAiCommit => icons.ai_sparkle(),
                };

                let style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else if *action == ContextMenuAction::TimeMachine {
                    Style::default().fg(theme.warning)
                } else {
                    Style::default().fg(theme.fg)
                };

                Line::from(vec![
                    Span::styled(format!(" {} ", icon_str), style),
                    Span::styled(action.label(), style),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
