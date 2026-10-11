//! Right-click context popup menu component.

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
    ToggleStage,
    OpenEditor,
    RevealFileManager,
    StashFile,
    SafeDiscard,
    CopyPath,
    CopyDiff,
    AskAi,
}

impl ContextMenuAction {
    pub const ALL: &'static [ContextMenuAction] = &[
        ContextMenuAction::ToggleStage,
        ContextMenuAction::SafeDiscard,
        ContextMenuAction::CopyPath,
        ContextMenuAction::CopyDiff,
        ContextMenuAction::AskAi,
        ContextMenuAction::OpenEditor,
        ContextMenuAction::RevealFileManager,
        ContextMenuAction::StashFile,
    ];

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
        }
    }
}

/// Floating context menu state.
#[derive(Debug, Clone, Default)]
pub struct ContextMenu {
    pub is_open: bool,
    pub selected_index: usize,
    pub target_file: Option<String>,
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
        self.anchor_x = x;
        self.anchor_y = y;
    }

    /// Close menu.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Move selection down.
    pub fn next(&mut self) {
        if self.selected_index + 1 < ContextMenuAction::ALL.len() {
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
        ContextMenuAction::ALL[self.selected_index]
    }

    /// Render popup menu onto frame.
    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let width = 32.min(viewport.width);
        let height = (ContextMenuAction::ALL.len() as u16 + 2).min(viewport.height);

        // Clamp inside screen bounds
        let x = self.anchor_x.min(viewport.width.saturating_sub(width));
        let y = self.anchor_y.min(viewport.height.saturating_sub(height));

        let rect = Rect {
            x,
            y,
            width,
            height,
        };

        frame.render_widget(Clear, rect);

        let title = self
            .target_file
            .as_deref()
            .map(|f| format!(" {} ", f))
            .unwrap_or_else(|| " Context Menu ".to_string());

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

        let lines: Vec<Line> = ContextMenuAction::ALL
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
                };

                let style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
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
