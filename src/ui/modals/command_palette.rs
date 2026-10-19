//! Command Palette with multi-layer architecture, fuzzy search, Git CLI catalog, and AI translation.

use crate::ai::SuggestedGitCommand;
use crate::ui::icons::Icons;
use crate::ui::modals::git_subcommands::GitSubcommandDef;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Preset action available in the command palette Root layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteAction {
    pub id: &'static str,
    pub title: &'static str,
    pub shortcut: Option<&'static str>,
}

impl PaletteAction {
    pub const ALL: &'static [PaletteAction] = &[
        PaletteAction {
            id: "browse_git_cli",
            title: "Git: Browse & Execute Git CLI Subcommands (100+)",
            shortcut: Some("g"),
        },
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
            id: "fetch",
            title: "Git: Fetch from All Remotes",
            shortcut: Some("F5"),
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
            id: "preview_markdown",
            title: "View: Preview Markdown for Selected File",
            shortcut: Some("m"),
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
            id: "tab_history",
            title: "View: Switch to History & Git Graph",
            shortcut: Some("2"),
        },
        PaletteAction {
            id: "tab_issues",
            title: "View: Switch to GitHub Issues",
            shortcut: Some("3"),
        },
        PaletteAction {
            id: "tab_prs",
            title: "View: Switch to Pull Requests",
            shortcut: Some("4"),
        },
        PaletteAction {
            id: "tab_actions",
            title: "View: Switch to Actions & CI",
            shortcut: Some("5"),
        },
        PaletteAction {
            id: "tab_alerts",
            title: "View: Switch to Alerts",
            shortcut: Some("6"),
        },
        PaletteAction {
            id: "tab_worktrees",
            title: "View: Switch to Worktrees",
            shortcut: Some("7"),
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
        PaletteAction {
            id: "abort_operation",
            title: "Git: Abort In-Progress Operation (Merge/Cherry-Pick/Revert)",
            shortcut: Some("A"),
        },
    ];
}

/// Active hierarchical layer within the command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteLayer {
    /// Layer 1: Prowly built-in actions & shortcuts
    Root,
    /// Layer 2: Git CLI subcommands catalog (100+)
    GitSubcommand,
    /// Layer 3: Direct options & argument input for a chosen Git subcommand
    ArgsInput {
        subcommand: String,
        description: String,
        common_flags: Vec<String>,
    },
}

/// Outcome of pressing Enter in the Command Palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteEnterResult {
    /// Internal transition to another layer
    LayerChanged,
    /// Execute built-in action
    ExecuteAction(&'static str),
    /// Execute arbitrary Git CLI command
    ExecuteGitCommand { subcommand: String, args: String },
    /// Execute AI-suggested command
    ExecuteAiCommand(String),
    /// Request natural language AI translation
    RequestAi(String),
    /// Safety guard warning triggered, requiring user confirmation
    DangerWarning(String),
}

/// Result of handling a mouse click inside/outside the Command Palette modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteClickResult {
    /// Modal should be closed (clicked outside).
    Close,
    /// An action or command was selected and should be executed.
    Enter(PaletteEnterResult),
    /// Click was inside modal and handled (prevents event penetration).
    Handled,
}

/// Detect potentially dangerous or destructive Git command invocations.
pub fn is_dangerous_git_args(subcommand: &str, args: &str) -> Option<&'static str> {
    crate::services::SafeGitRunner::check_dangerous_git_str(subcommand, args)
}

/// Command palette modal state supporting multi-layer navigation.
#[derive(Debug, Clone)]
pub struct CommandPaletteModal {
    pub is_open: bool,
    pub layer: PaletteLayer,
    pub query: String,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub ai_suggestion: Option<SuggestedGitCommand>,
    pub confirmed_danger: bool,
    pub confirmed_danger_target: Option<String>,
    pub danger_message: Option<String>,
    pub flag_candidate_index: usize,
}

impl Default for CommandPaletteModal {
    fn default() -> Self {
        Self {
            is_open: false,
            layer: PaletteLayer::Root,
            query: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            ai_suggestion: None,
            confirmed_danger: false,
            confirmed_danger_target: None,
            danger_message: None,
            flag_candidate_index: 0,
        }
    }
}

impl CommandPaletteModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open modal in Root layer.
    pub fn open(&mut self) {
        self.is_open = true;
        self.layer = PaletteLayer::Root;
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.ai_suggestion = None;
        self.confirmed_danger = false;
        self.danger_message = None;
        self.flag_candidate_index = 0;
    }

    pub fn is_confirmed_danger(&self) -> bool {
        self.confirmed_danger
    }

    /// Open modal directly into Layer 2 (Git Subcommands catalog).
    pub fn open_git_subcommands(&mut self) {
        self.open();
        self.layer = PaletteLayer::GitSubcommand;
    }

    /// Open modal directly into Layer 3 (Arguments input for a subcommand).
    pub fn open_args_input(&mut self, subcommand: &str) {
        self.open();
        let def = GitSubcommandDef::find(subcommand);
        let desc = def.map(|d| d.description).unwrap_or("Execute Git command");
        let flags = def
            .map(|d| d.common_flags.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        self.layer = PaletteLayer::ArgsInput {
            subcommand: subcommand.to_string(),
            description: desc.to_string(),
            common_flags: flags,
        };
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.layer = PaletteLayer::Root;
        self.query.clear();
        self.ai_suggestion = None;
        self.confirmed_danger = false;
        self.danger_message = None;
    }

    /// Retrieve filtered actions matching current query in Root layer.
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

    /// Retrieve filtered Git subcommands matching current query in Layer 2.
    pub fn filtered_subcommands(&self) -> Vec<&'static GitSubcommandDef> {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            return GitSubcommandDef::ALL.iter().collect();
        }
        GitSubcommandDef::ALL
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.category.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn next(&mut self) {
        match &self.layer {
            PaletteLayer::Root => {
                let total = self.filtered_actions().len();
                if total > 0 && self.selected_index + 1 < total {
                    self.selected_index += 1;
                }
            }
            PaletteLayer::GitSubcommand => {
                let total = self.filtered_subcommands().len();
                if total > 0 && self.selected_index + 1 < total {
                    self.selected_index += 1;
                }
            }
            PaletteLayer::ArgsInput { common_flags, .. } => {
                if !common_flags.is_empty() {
                    self.flag_candidate_index =
                        (self.flag_candidate_index + 1) % common_flags.len();
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match &self.layer {
            PaletteLayer::Root | PaletteLayer::GitSubcommand => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            PaletteLayer::ArgsInput { common_flags, .. } => {
                if !common_flags.is_empty() {
                    if self.flag_candidate_index == 0 {
                        self.flag_candidate_index = common_flags.len() - 1;
                    } else {
                        self.flag_candidate_index -= 1;
                    }
                }
            }
        }
    }

    pub fn page_down(&mut self, step: usize) {
        match &self.layer {
            PaletteLayer::Root => {
                let total = self.filtered_actions().len();
                if total > 0 {
                    self.selected_index = (self.selected_index + step).min(total - 1);
                }
            }
            PaletteLayer::GitSubcommand => {
                let total = self.filtered_subcommands().len();
                if total > 0 {
                    self.selected_index = (self.selected_index + step).min(total - 1);
                }
            }
            PaletteLayer::ArgsInput { .. } => {}
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
        self.confirmed_danger = false;
        self.confirmed_danger_target = None;
        self.danger_message = None;
    }

    /// Handle backspace key: pop character or return to parent layer if empty.
    pub fn pop_char(&mut self) {
        if !self.query.is_empty() {
            self.query.pop();
            self.selected_index = 0;
            self.scroll_offset = 0;
            self.confirmed_danger = false;
            self.confirmed_danger_target = None;
            self.danger_message = None;
        } else {
            // When query is empty, navigate back up the hierarchy
            match &self.layer {
                PaletteLayer::ArgsInput { .. } => {
                    self.layer = PaletteLayer::GitSubcommand;
                    self.query.clear();
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    self.confirmed_danger = false;
                    self.confirmed_danger_target = None;
                    self.danger_message = None;
                }
                PaletteLayer::GitSubcommand => {
                    self.layer = PaletteLayer::Root;
                    self.query.clear();
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    self.confirmed_danger = false;
                    self.confirmed_danger_target = None;
                    self.danger_message = None;
                }
                PaletteLayer::Root => {}
            }
        }
    }

    /// Handle Esc key: navigate back one layer, or return false if already at Root.
    pub fn handle_esc(&mut self) -> bool {
        match &self.layer {
            PaletteLayer::ArgsInput { .. } => {
                self.layer = PaletteLayer::GitSubcommand;
                self.query.clear();
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.confirmed_danger = false;
                self.confirmed_danger_target = None;
                self.danger_message = None;
                true
            }
            PaletteLayer::GitSubcommand => {
                self.layer = PaletteLayer::Root;
                self.query.clear();
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.confirmed_danger = false;
                self.confirmed_danger_target = None;
                self.danger_message = None;
                true
            }
            PaletteLayer::Root => false,
        }
    }

    /// Handle Tab key: autocomplete flag in Layer 3.
    pub fn handle_tab(&mut self) {
        if let PaletteLayer::ArgsInput {
            ref common_flags, ..
        } = self.layer
        {
            if !common_flags.is_empty() {
                let flag = &common_flags[self.flag_candidate_index % common_flags.len()];
                self.flag_candidate_index = (self.flag_candidate_index + 1) % common_flags.len();
                if self.query.is_empty() {
                    self.query = flag.clone();
                } else if self.query.ends_with(' ') {
                    self.query.push_str(flag);
                } else {
                    self.query.push(' ');
                    self.query.push_str(flag);
                }
                self.confirmed_danger = false;
                self.danger_message = None;
            }
        }
    }

    /// Process Enter key across hierarchical layers with direct shortcuts & safety guard.
    pub fn handle_enter(&mut self) -> PaletteEnterResult {
        match &self.layer {
            PaletteLayer::Root => {
                // 1. If AI generated command is active
                if let Some(cmd) = self.ai_suggestion.take() {
                    if let Ok((sub, args)) =
                        crate::services::SafeGitRunner::parse_ai_command(&cmd.command)
                    {
                        if let Some(warn) =
                            crate::services::SafeGitRunner::check_dangerous_git_args(&sub, &args)
                        {
                            if self.confirmed_danger_target.as_deref() != Some(&cmd.command) {
                                self.confirmed_danger = true;
                                self.confirmed_danger_target = Some(cmd.command.clone());
                                self.danger_message = Some(warn.to_string());
                                self.ai_suggestion = Some(cmd);
                                return PaletteEnterResult::DangerWarning(warn.to_string());
                            }
                        }
                    }
                    self.confirmed_danger = false;
                    self.confirmed_danger_target = None;
                    self.danger_message = None;
                    return PaletteEnterResult::ExecuteAiCommand(cmd.command);
                }

                // 2. Direct shortcut jump check (e.g. `git cherry-pick ...` or `rebase -i ...`)
                let trimmed = self.query.trim();
                let cmd_part = if let Some(stripped) = trimmed.strip_prefix("git ") {
                    stripped.trim()
                } else {
                    trimmed
                };
                let words: Vec<&str> = cmd_part.split_whitespace().collect();

                if !words.is_empty() && GitSubcommandDef::is_known_subcommand(words[0]) {
                    let sub_def = GitSubcommandDef::find(words[0]).unwrap();
                    let remaining_args = if words.len() > 1 {
                        words[1..].join(" ")
                    } else {
                        String::new()
                    };

                    if remaining_args.is_empty() {
                        // Jump straight to Layer 3 for options/flags
                        self.layer = PaletteLayer::ArgsInput {
                            subcommand: sub_def.name.to_string(),
                            description: sub_def.description.to_string(),
                            common_flags: sub_def
                                .common_flags
                                .iter()
                                .map(|s| s.to_string())
                                .collect(),
                        };
                        self.query.clear();
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                        self.confirmed_danger = false;
                        self.danger_message = None;
                        return PaletteEnterResult::LayerChanged;
                    } else {
                        // Check dangerous flags before direct execution
                        let target_str = format!("{} {}", sub_def.name, remaining_args);
                        if let Some(warn) = is_dangerous_git_args(sub_def.name, &remaining_args) {
                            if self.confirmed_danger_target.as_deref() != Some(&target_str) {
                                self.confirmed_danger = true;
                                self.confirmed_danger_target = Some(target_str);
                                self.danger_message = Some(warn.to_string());
                                self.layer = PaletteLayer::ArgsInput {
                                    subcommand: sub_def.name.to_string(),
                                    description: sub_def.description.to_string(),
                                    common_flags: sub_def
                                        .common_flags
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect(),
                                };
                                self.query = remaining_args;
                                return PaletteEnterResult::DangerWarning(warn.to_string());
                            }
                        }
                        self.confirmed_danger = false;
                        self.confirmed_danger_target = None;
                        return PaletteEnterResult::ExecuteGitCommand {
                            subcommand: sub_def.name.to_string(),
                            args: remaining_args,
                        };
                    }
                }

                // 3. Built-in action selection
                if let Some(action) = self.selected_action() {
                    if action.id == "browse_git_cli" {
                        self.layer = PaletteLayer::GitSubcommand;
                        self.query.clear();
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                        self.confirmed_danger = false;
                        self.danger_message = None;
                        return PaletteEnterResult::LayerChanged;
                    }
                    return PaletteEnterResult::ExecuteAction(action.id);
                }

                // 4. No matching built-in -> request AI natural language translation
                PaletteEnterResult::RequestAi(self.query.clone())
            }

            PaletteLayer::GitSubcommand => {
                let subs = self.filtered_subcommands();
                if let Some(sub) = subs.get(self.selected_index) {
                    self.layer = PaletteLayer::ArgsInput {
                        subcommand: sub.name.to_string(),
                        description: sub.description.to_string(),
                        common_flags: sub.common_flags.iter().map(|s| s.to_string()).collect(),
                    };
                    self.query.clear();
                    self.selected_index = 0;
                    self.scroll_offset = 0;
                    self.confirmed_danger = false;
                    self.danger_message = None;
                    self.flag_candidate_index = 0;
                    PaletteEnterResult::LayerChanged
                } else {
                    PaletteEnterResult::LayerChanged
                }
            }

            PaletteLayer::ArgsInput { subcommand, .. } => {
                let sub = subcommand.clone();
                let args = self.query.trim().to_string();
                let target_str = format!("{} {}", sub, args);

                // Check safety guard
                if let Some(warn) = is_dangerous_git_args(&sub, &args) {
                    if self.confirmed_danger_target.as_deref() != Some(&target_str) {
                        self.confirmed_danger = true;
                        self.confirmed_danger_target = Some(target_str);
                        self.danger_message = Some(warn.to_string());
                        return PaletteEnterResult::DangerWarning(warn.to_string());
                    }
                }

                self.confirmed_danger = false;
                self.confirmed_danger_target = None;
                PaletteEnterResult::ExecuteGitCommand {
                    subcommand: sub,
                    args,
                }
            }
        }
    }

    /// Calculate modal bounding rectangle in viewport.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 78.min(viewport.width.saturating_sub(4));
        let height = 22.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 3;
        Rect::new(x, y, width, height)
    }

    /// Calculate modal layout chunks (header/input, body list, footer).
    pub fn content_chunks(&self, viewport: Rect) -> (Rect, std::rc::Rc<[Rect]>) {
        let area = self.modal_rect(viewport);
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(inner);
        (area, chunks)
    }

    /// Handle mouse click in the Command Palette modal.
    pub fn handle_click(&mut self, x: u16, y: u16, viewport: Rect) -> Option<PaletteClickResult> {
        if !self.is_open {
            return None;
        }

        let (area, chunks) = self.content_chunks(viewport);

        // Click outside closes the modal
        if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
            return Some(PaletteClickResult::Close);
        }

        let body_area = chunks[1];
        if x >= body_area.x
            && x < body_area.x + body_area.width
            && y >= body_area.y
            && y < body_area.y + body_area.height
        {
            let rel_y = (y - body_area.y) as usize;
            match &self.layer {
                PaletteLayer::Root => {
                    if self.ai_suggestion.is_some() {
                        return Some(PaletteClickResult::Enter(self.handle_enter()));
                    }

                    let filtered = self.filtered_actions();
                    let clicked_idx = self.scroll_offset + rel_y;
                    if clicked_idx < filtered.len() {
                        self.selected_index = clicked_idx;
                        return Some(PaletteClickResult::Enter(self.handle_enter()));
                    }
                }
                PaletteLayer::GitSubcommand => {
                    let subs = self.filtered_subcommands();
                    let clicked_idx = self.scroll_offset + rel_y;
                    if clicked_idx < subs.len() {
                        self.selected_index = clicked_idx;
                        return Some(PaletteClickResult::Enter(self.handle_enter()));
                    }
                }
                PaletteLayer::ArgsInput { common_flags, .. } => {
                    // Line 0: "", Line 1: Description, Line 2: "", Line 3: "Common Options...", Line 4: pills
                    if rel_y == 4 && !common_flags.is_empty() {
                        let rel_x = x.saturating_sub(body_area.x);
                        let mut cur_x = 3; // Initial indent "   "
                        for (i, f) in common_flags.iter().enumerate() {
                            let pill_w = (f.len() + 2) as u16; // " {} "
                            if rel_x >= cur_x && rel_x < cur_x + pill_w {
                                self.flag_candidate_index = i;
                                if self.query.is_empty() {
                                    self.query = f.clone();
                                } else if self.query.ends_with(' ') {
                                    self.query.push_str(f);
                                } else {
                                    self.query.push(' ');
                                    self.query.push_str(f);
                                }
                                self.confirmed_danger = false;
                                self.danger_message = None;
                                return Some(PaletteClickResult::Handled);
                            }
                            cur_x += pill_w + 1; // plus " " separator
                        }
                    } else if self.confirmed_danger && rel_y >= 5 {
                        return Some(PaletteClickResult::Enter(self.handle_enter()));
                    }
                }
            }
        }

        // Click inside modal (e.g. search query box, footer, or blank space) is handled
        Some(PaletteClickResult::Handled)
    }

    /// Render command palette modal supporting Layer 1, 2, and 3.
    pub fn render(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let (area, chunks) = self.content_chunks(viewport);

        frame.render_widget(Clear, area);

        // Header Breadcrumbs Title
        let title_line = match &self.layer {
            PaletteLayer::Root => Line::from(vec![Span::styled(
                format!(" {} COMMAND PALETTE ", icons.ai_sparkle()),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            PaletteLayer::GitSubcommand => Line::from(vec![
                Span::styled(" Palette ", Style::default().fg(theme.dim_fg)),
                Span::styled("❯ ", Style::default().fg(theme.dim_fg)),
                Span::styled(
                    format!("{} Git CLI Subcommands (100+) ", icons.branch()),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            PaletteLayer::ArgsInput { subcommand, .. } => Line::from(vec![
                Span::styled(" Palette ", Style::default().fg(theme.dim_fg)),
                Span::styled("❯ ", Style::default().fg(theme.dim_fg)),
                Span::styled("Git ", Style::default().fg(theme.dim_fg)),
                Span::styled("❯ ", Style::default().fg(theme.primary)),
                Span::styled(
                    format!("{} git {} ", icons.branch(), subcommand),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(title_line)
            .style(Style::default().bg(theme.bg));

        frame.render_widget(block, area);

        // 1. Search Query Input Box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused));
        let input_inner = input_block.inner(chunks[0]);
        frame.render_widget(input_block, chunks[0]);

        let prompt_prefix = match &self.layer {
            PaletteLayer::Root => " > ",
            PaletteLayer::GitSubcommand => " git > ",
            PaletteLayer::ArgsInput { subcommand, .. } => {
                // Return dynamic prompt prefix
                Box::leak(format!(" git {} > ", subcommand).into_boxed_str())
            }
        };

        let prompt_line = Line::from(vec![
            Span::styled(
                prompt_prefix,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.query, Style::default().fg(theme.fg)),
        ]);
        frame.render_widget(Paragraph::new(prompt_line), input_inner);

        let prefix_w = unicode_width::UnicodeWidthStr::width(prompt_prefix) as u16;
        let query_w = unicode_width::UnicodeWidthStr::width(self.query.as_str()) as u16;
        let cur_x = (input_inner.x + prefix_w + query_w)
            .min(input_inner.x + input_inner.width.saturating_sub(1));
        frame.set_cursor_position((cur_x, input_inner.y));

        // 2. Body Viewport based on Layer
        match &self.layer {
            PaletteLayer::Root => {
                let filtered = self.filtered_actions();
                if let Some(ref ai_cmd) = self.ai_suggestion {
                    let (risk_color, risk_text) = match ai_cmd.risk_level {
                        crate::ai::CommandRiskLevel::Safe => (theme.diff_add_fg, "SAFE"),
                        crate::ai::CommandRiskLevel::Caution => {
                            (Color::Rgb(249, 226, 175), "CAUTION")
                        }
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
                    } else if visible_height > 0
                        && self.selected_index >= scroll_top + visible_height
                    {
                        scroll_top = self.selected_index + 1 - visible_height;
                    }
                    let max_scroll = filtered.len().saturating_sub(visible_height);
                    scroll_top = scroll_top.min(max_scroll);
                    self.scroll_offset = scroll_top;

                    let mut lines = Vec::new();
                    for (offset, action) in filtered
                        .iter()
                        .skip(scroll_top)
                        .take(visible_height)
                        .enumerate()
                    {
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

                // Footer Help for Root
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

            PaletteLayer::GitSubcommand => {
                let subs = self.filtered_subcommands();
                let visible_height = chunks[1].height as usize;
                let mut scroll_top = self.scroll_offset;
                if self.selected_index < scroll_top {
                    scroll_top = self.selected_index;
                } else if visible_height > 0 && self.selected_index >= scroll_top + visible_height {
                    scroll_top = self.selected_index + 1 - visible_height;
                }
                let max_scroll = subs.len().saturating_sub(visible_height);
                scroll_top = scroll_top.min(max_scroll);
                self.scroll_offset = scroll_top;

                let mut lines = Vec::new();
                for (offset, sub) in subs
                    .iter()
                    .skip(scroll_top)
                    .take(visible_height)
                    .enumerate()
                {
                    let actual_idx = scroll_top + offset;
                    let is_selected = actual_idx == self.selected_index;
                    let line_bg = if is_selected {
                        theme.selection_bg
                    } else {
                        theme.bg
                    };
                    let prefix = if is_selected { " ▶ " } else { "   " };

                    let cat_color = match sub.category {
                        "Main" => theme.success,
                        "Ancillary" => theme.info,
                        _ => theme.dim_fg,
                    };

                    let ui_badge = if sub.has_dedicated_ui() {
                        Span::styled(
                            "[UI] ",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD)
                                .bg(line_bg),
                        )
                    } else {
                        Span::styled("[CLI] ", Style::default().fg(theme.dim_fg).bg(line_bg))
                    };

                    let spans = vec![
                        Span::styled(prefix, Style::default().fg(theme.primary).bg(line_bg)),
                        ui_badge,
                        Span::styled(
                            format!("[{}] ", sub.category),
                            Style::default().fg(cat_color).bg(line_bg),
                        ),
                        Span::styled(
                            format!("{:<15}", sub.name),
                            Style::default()
                                .fg(theme.fg)
                                .add_modifier(Modifier::BOLD)
                                .bg(line_bg),
                        ),
                        Span::styled(" — ", Style::default().fg(theme.dim_fg).bg(line_bg)),
                        Span::styled(
                            sub.description,
                            Style::default().fg(theme.dim_fg).bg(line_bg),
                        ),
                    ];
                    lines.push(Line::from(spans));
                }
                frame.render_widget(Paragraph::new(lines), chunks[1]);

                // Footer Help for GitSubcommand
                let count_str = format!("({}/{}) ", self.selected_index + 1, subs.len());
                let help = Line::from(vec![
                    Span::styled(
                        "[UI]: Dedicated TUI  [CLI]: Direct Execution  ",
                        Style::default().fg(theme.secondary),
                    ),
                    Span::styled(
                        "[ Enter: Select Subcommand ]  ",
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(
                        format!("[ ↑/↓: Navigate {}]  ", count_str),
                        Style::default().fg(theme.dim_fg),
                    ),
                    Span::styled(
                        "[ Esc/Backspace: Back to Palette ]",
                        Style::default().fg(theme.dim_fg),
                    ),
                ]);
                frame.render_widget(Paragraph::new(help), chunks[2]);
            }

            PaletteLayer::ArgsInput {
                subcommand,
                description,
                common_flags,
            } => {
                let mut lines = Vec::new();
                lines.push(Line::from(""));

                // Subcommand description
                lines.push(Line::from(vec![
                    Span::styled(
                        "  󰘬 Description: ",
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(description.as_str(), Style::default().fg(theme.fg)),
                ]));
                lines.push(Line::from(""));

                // Suggested flags pills
                if !common_flags.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "  Common Options & Flags: ",
                            Style::default().fg(theme.primary),
                        ),
                        Span::styled(
                            "(Press [ Tab ] to cycle/insert)",
                            Style::default().fg(theme.dim_fg),
                        ),
                    ]));

                    let mut flag_spans = vec![Span::raw("   ")];
                    for (i, f) in common_flags.iter().enumerate() {
                        let is_active_flag = i == (self.flag_candidate_index % common_flags.len());
                        let flag_style = if is_active_flag {
                            Style::default()
                                .fg(theme.bg)
                                .bg(theme.primary)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.fg).bg(theme.selection_bg)
                        };
                        flag_spans.push(Span::styled(format!(" {} ", f), flag_style));
                        flag_spans.push(Span::raw(" "));
                    }
                    lines.push(Line::from(flag_spans));
                    lines.push(Line::from(""));
                }

                // Safety guard warning alert box
                if self.confirmed_danger {
                    let msg = self
                        .danger_message
                        .as_deref()
                        .unwrap_or("Potentially dangerous command!");
                    lines.push(Line::from(vec![Span::styled(
                        format!("  ⚠ CAUTION: {} ", msg),
                        Style::default()
                            .fg(Color::Rgb(249, 226, 175))
                            .bg(theme.diff_del_bg)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    lines.push(Line::from(vec![Span::styled(
                        "    Press [ Enter ] again to confirm execution, or [ Esc ] to edit.",
                        Style::default().fg(theme.warning),
                    )]));
                } else {
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "  󱚣 Ready to execute: git {} {}",
                            subcommand,
                            self.query.trim()
                        ),
                        Style::default().fg(theme.dim_fg),
                    )]));
                }

                frame.render_widget(Paragraph::new(lines), chunks[1]);

                // Footer Help for ArgsInput
                let help = Line::from(vec![
                    Span::styled(
                        format!("[ Enter: Run git {} ]  ", subcommand),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(
                        "[ Tab: Insert Option ]  ",
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(
                        "[ Esc/Backspace: Back to Subcommands ]",
                        Style::default().fg(theme.dim_fg),
                    ),
                ]);
                frame.render_widget(Paragraph::new(help), chunks[2]);
            }
        }
    }
}
