//! Command Palette with fuzzy search and AI natural language translation.

use crate::ai::SuggestedGitCommand;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Preset action available in the command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteAction {
    pub id: &'static str,
    pub title: &'static str,
    pub shortcut: Option<&'static str>,
}

impl PaletteAction {
    pub const ALL: &'static [PaletteAction] = &[
        PaletteAction {
            id: "stage_all",
            title: "Git: Stage All Changes",
            shortcut: Some("a"),
        },
        PaletteAction {
            id: "unstage_all",
            title: "Git: Unstage All Changes",
            shortcut: Some("u"),
        },
        PaletteAction {
            id: "commit",
            title: "Git: Commit Staged Changes",
            shortcut: Some("c"),
        },
        PaletteAction {
            id: "push",
            title: "Git: Push to Remote",
            shortcut: None,
        },
        PaletteAction {
            id: "pull",
            title: "Git: Pull from Remote",
            shortcut: None,
        },
        PaletteAction {
            id: "stash",
            title: "Git: Stash All Changes",
            shortcut: None,
        },
        PaletteAction {
            id: "stash_pop",
            title: "Git: Pop Stash",
            shortcut: None,
        },
        PaletteAction {
            id: "switch_branch",
            title: "Git: Switch Branch",
            shortcut: Some("b"),
        },
        PaletteAction {
            id: "remote_diff",
            title: "Git: Preview Remote Diff (Unpushed / Unpulled)",
            shortcut: None,
        },
        PaletteAction {
            id: "rebase",
            title: "Git: Interactive Rebase & Squash",
            shortcut: Some("r"),
        },
        PaletteAction {
            id: "resolve_conflicts",
            title: "Git: Resolve Merge Conflicts",
            shortcut: None,
        },
        PaletteAction {
            id: "discard_changes",
            title: "Git: Discard Selected Changes",
            shortcut: Some("d"),
        },
        PaletteAction {
            id: "stage_selected",
            title: "Git: Toggle Stage Selected File",
            shortcut: Some("Space"),
        },
        PaletteAction {
            id: "stash_selected",
            title: "Git: Stash Selected File Only",
            shortcut: None,
        },
        PaletteAction {
            id: "open_editor",
            title: "View: Open Selected in External Editor",
            shortcut: Some("e"),
        },
        PaletteAction {
            id: "reveal_file_manager",
            title: "View: Reveal Selected in File Manager",
            shortcut: None,
        },
        PaletteAction {
            id: "refresh",
            title: "View: Refresh Repository & GitHub Data",
            shortcut: Some("R"),
        },
        PaletteAction {
            id: "tab_changes",
            title: "View: Switch to Changes",
            shortcut: Some("1"),
        },
        PaletteAction {
            id: "tab_prs",
            title: "View: Switch to Pull Requests",
            shortcut: Some("2"),
        },
        PaletteAction {
            id: "tab_actions",
            title: "View: Switch to Actions & CI",
            shortcut: Some("3"),
        },
        PaletteAction {
            id: "tab_alerts",
            title: "View: Switch to Alerts",
            shortcut: Some("4"),
        },
        PaletteAction {
            id: "tab_worktrees",
            title: "View: Switch to Worktrees",
            shortcut: Some("5"),
        },
        PaletteAction {
            id: "toggle_all_files",
            title: "View: Toggle All Files / Changed Only",
            shortcut: Some("A"),
        },
        PaletteAction {
            id: "cycle_theme",
            title: "Preferences: Cycle Color Theme",
            shortcut: Some("t"),
        },
        PaletteAction {
            id: "toggle_lang",
            title: "Preferences: Toggle Language (EN/JA)",
            shortcut: Some("l"),
        },
        PaletteAction {
            id: "settings",
            title: "Preferences: Open Settings Modal",
            shortcut: Some(","),
        },
        PaletteAction {
            id: "open_repo",
            title: "Repository: Open / Switch Repository",
            shortcut: Some("Ctrl+O"),
        },
        PaletteAction {
            id: "ai_diag",
            title: "AI: Diagnose Repository Health",
            shortcut: Some("Ctrl+D"),
        },
        PaletteAction {
            id: "ai_review",
            title: "AI: Pre-commit Code Review",
            shortcut: Some("Ctrl+R"),
        },
        PaletteAction {
            id: "ai_ci_failure",
            title: "AI: Analyze CI / Actions Failure",
            shortcut: Some("Ctrl+F"),
        },
        PaletteAction {
            id: "ai_assistant",
            title: "AI: Ask AI Assistant Chat",
            shortcut: Some("?"),
        },
        PaletteAction {
            id: "ai_pr_draft",
            title: "AI: Generate Pull Request Draft",
            shortcut: None,
        },
    ];
}

/// Command palette modal state.
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteModal {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub ai_suggestion: Option<SuggestedGitCommand>,
}

impl CommandPaletteModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.ai_suggestion = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.ai_suggestion = None;
    }

    /// Retrieve filtered actions matching current query.
    pub fn filtered_actions(&self) -> Vec<&'static PaletteAction> {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            return PaletteAction::ALL.iter().collect();
        }
        PaletteAction::ALL
            .iter()
            .filter(|a| a.title.to_lowercase().contains(&q) || a.id.contains(&q))
            .collect()
    }

    pub fn next(&mut self) {
        let total = self.filtered_actions().len();
        if total > 0 && self.selected_index + 1 < total {
            self.selected_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn page_down(&mut self, step: usize) {
        let total = self.filtered_actions().len();
        if total > 0 {
            self.selected_index = (self.selected_index + step).min(total - 1);
        }
    }

    pub fn page_up(&mut self, step: usize) {
        self.selected_index = self.selected_index.saturating_sub(step);
    }

    pub fn selected_action(&self) -> Option<&'static PaletteAction> {
        let list = self.filtered_actions();
        list.get(self.selected_index).copied()
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.ai_suggestion = None;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.ai_suggestion = None;
    }

    pub fn render(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let width = 76.min(viewport.width.saturating_sub(4));
        let height = 20.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 3; // Position slightly above center
        let area = Rect::new(x, y, width, height);

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                format!(" {} COMMAND PALETTE ", icons.ai_sparkle()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(inner);

        // 1. Search Query Input Box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused));
        let input_inner = input_block.inner(chunks[0]);
        frame.render_widget(input_block, chunks[0]);

        let prompt_line = Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.query, Style::default().fg(theme.fg)),
            Span::styled("█", Style::default().fg(theme.cursor)),
        ]);
        frame.render_widget(Paragraph::new(prompt_line), input_inner);

        // 2. Results List or AI Suggestion
        let filtered = self.filtered_actions();
        if let Some(ref ai_cmd) = self.ai_suggestion {
            let (risk_color, risk_text) = match ai_cmd.risk_level {
                crate::ai::CommandRiskLevel::Safe => (theme.diff_add_fg, "SAFE"),
                crate::ai::CommandRiskLevel::Caution => (Color::Rgb(249, 226, 175), "CAUTION"),
                crate::ai::CommandRiskLevel::Dangerous => (theme.diff_del_fg, "DANGEROUS"),
            };

            let ai_card = vec![
                Line::from(vec![
                    Span::styled(
                        "󱚣 AI Recommended Command: ",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("[{}]", risk_text),
                        Style::default().fg(risk_color).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "  $ ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &ai_cmd.command,
                        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Explanation: ", Style::default().fg(theme.dim_fg)),
                    Span::styled(&ai_cmd.explanation, Style::default().fg(theme.fg)),
                ]),
            ];
            frame.render_widget(Paragraph::new(ai_card), chunks[1]);
        } else if filtered.is_empty() {
                let empty_line = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            "  No matching built-in commands. ",
                            Style::default().fg(theme.dim_fg),
                        ),
                        Span::styled(
                            "Press [ Enter ] to ask AI to generate command.",
                            Style::default().fg(theme.accent),
                        ),
                    ]),
                ];
                frame.render_widget(Paragraph::new(empty_line), chunks[1]);
            } else {
                let visible_height = chunks[1].height as usize;
                let mut scroll_top = self.scroll_offset;
                if self.selected_index < scroll_top {
                    scroll_top = self.selected_index;
                } else if visible_height > 0 && self.selected_index >= scroll_top + visible_height {
                    scroll_top = self.selected_index + 1 - visible_height;
                }
                let max_scroll = filtered.len().saturating_sub(visible_height);
                scroll_top = scroll_top.min(max_scroll);
                self.scroll_offset = scroll_top;

                let mut lines = Vec::new();
                for (offset, action) in filtered.iter().skip(scroll_top).take(visible_height).enumerate() {
                    let actual_idx = scroll_top + offset;
                    let is_selected = actual_idx == self.selected_index;
                    let line_bg = if is_selected {
                        theme.selection_bg
                    } else {
                        theme.bg
                    };
                    let prefix = if is_selected { " ▶ " } else { "   " };

                    let mut spans = vec![
                        Span::styled(prefix, Style::default().fg(theme.primary).bg(line_bg)),
                        Span::styled(
                            action.title,
                            Style::default()
                                .fg(theme.fg)
                                .add_modifier(if is_selected {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                })
                                .bg(line_bg),
                        ),
                    ];

                    if let Some(sc) = action.shortcut {
                        spans.push(Span::styled(
                            format!(" [{}]", sc),
                            Style::default().fg(theme.dim_fg).bg(line_bg),
                        ));
                    }

                    lines.push(Line::from(spans));
                }
                frame.render_widget(Paragraph::new(lines), chunks[1]);
            }

        // 3. Footer Help
        let count_str = if filtered.is_empty() {
            String::new()
        } else {
            format!("({}/{}) ", self.selected_index + 1, filtered.len())
        };
        let help = Line::from(vec![
            Span::styled("[ Enter: Execute ]  ", Style::default().fg(theme.primary)),
            Span::styled(
                format!("[ ↑/↓: Navigate {}]  ", count_str),
                Style::default().fg(theme.dim_fg),
            ),
            Span::styled("[ Esc: Dismiss ]", Style::default().fg(theme.dim_fg)),
        ]);
        frame.render_widget(Paragraph::new(help), chunks[2]);
    }
}
