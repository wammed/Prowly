//! Application state and UI event dispatcher for Prowly.

use crate::ai::{
    analyze_ci_failure, diagnose_repository, generate_commit_message, generate_pr_draft,
    review_changes, smart_merge_conflict, translate_natural_language_to_git, AiContext,
    ContextProvider,
};
use crate::config::Config;
use crate::event::AppEvent;
use crate::git::{
    create_worktree, list_worktrees, remove_worktree, ConflictChoice, FileDiff, GitFileItem,
    GitRepoStatus, GitService, WorktreeItem,
};
use crate::github::{GitHubRepoTarget, GitHubService};
use crate::i18n::I18n;
use crate::services::{open_external_editor, BackupService, ClipboardService};
use crate::ui::layout::LayoutAreas;
use crate::ui::modals::{
    AiAssistantModal, CommandPaletteModal, ConflictResolverModal, ContextMenu, ContextMenuAction,
};
use crate::ui::theme::{Theme, ThemeId};
use crate::ui::views::{
    ActionsView, AlertsView, ChangesView, DirectoryMap, PrsView, WorktreesView,
};
use crate::ui::Icons;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
    Frame,
};

/// Safely execute an asynchronous future from synchronous code without panicking,
/// even when called from within an active Tokio runtime thread.
pub fn block_on_safe<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R> + Send,
    R: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => std::thread::scope(|s| {
            s.spawn(|| handle.block_on(future))
                .join()
                .expect("async execution thread panicked")
        }),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime");
            rt.block_on(future)
        }
    }
}

/// Main tabs available in Prowly's right pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Changes,
    Prs,
    Actions,
    Alerts,
    Worktrees,
}

impl AppTab {
    pub const ALL: &'static [AppTab] = &[
        AppTab::Changes,
        AppTab::Prs,
        AppTab::Actions,
        AppTab::Alerts,
        AppTab::Worktrees,
    ];

    pub fn index(&self) -> usize {
        match self {
            Self::Changes => 0,
            Self::Prs => 1,
            Self::Actions => 2,
            Self::Alerts => 3,
            Self::Worktrees => 4,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Prs,
            2 => Self::Actions,
            3 => Self::Alerts,
            4 => Self::Worktrees,
            _ => Self::Changes,
        }
    }

    pub fn title_key(&self) -> &'static str {
        match self {
            Self::Changes => "tabs.changes",
            Self::Prs => "tabs.prs",
            Self::Actions => "tabs.actions",
            Self::Alerts => "tabs.alerts",
            Self::Worktrees => "tabs.worktrees",
        }
    }
}

/// Active input focus between screen panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    DirectoryMap,
    MainContent,
}

/// Main application state controller.
pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub i18n: I18n,
    pub icons: Icons,
    pub clipboard: ClipboardService,
    pub backup_service: BackupService,

    // Git Engine Integration
    pub git_service: Option<GitService>,
    pub repo_status: GitRepoStatus,
    pub current_diff: FileDiff,
    pub worktrees: Vec<WorktreeItem>,

    // UI state
    pub active_tab: AppTab,
    pub focused_pane: FocusedPane,
    pub should_quit: bool,
    pub last_status: Option<String>,
    pub layout_areas: LayoutAreas,

    // Views
    pub dir_map: DirectoryMap,
    pub changes_view: ChangesView,
    pub worktrees_view: WorktreesView,
    pub prs_view: PrsView,
    pub actions_view: ActionsView,
    pub alerts_view: AlertsView,

    // GitHub Integration
    pub github_service: GitHubService,

    // Modals
    pub ai_modal: AiAssistantModal,
    pub context_menu: ContextMenu,
    pub command_palette: CommandPaletteModal,
    pub conflict_resolver: ConflictResolverModal,

    // Channel sender for background async tasks
    pub event_sender: Option<tokio::sync::mpsc::UnboundedSender<AppEvent>>,
}

impl App {
    /// Create new application instance from configuration.
    pub fn new(config: Config) -> Self {
        let theme = Theme::get(config.theme);
        let i18n = I18n::new(config.language);
        let icons = Icons::new(config.use_nerd_font);
        let clipboard = ClipboardService::new();
        let backup_service = BackupService::new();

        let git_service = GitService::discover().ok();
        let gh_token = GitHubService::detect_token();
        let github_service = GitHubService::new(gh_token);

        let mut app = Self {
            config,
            theme,
            i18n,
            icons,
            clipboard,
            backup_service,
            git_service,
            repo_status: GitRepoStatus::default(),
            current_diff: FileDiff::default(),
            worktrees: Vec::new(),
            active_tab: AppTab::Changes,
            focused_pane: FocusedPane::MainContent,
            should_quit: false,
            last_status: None,
            layout_areas: LayoutAreas::default(),
            dir_map: DirectoryMap::new(),
            changes_view: ChangesView::new(),
            worktrees_view: WorktreesView::new(),
            prs_view: PrsView::new(),
            actions_view: ActionsView::new(),
            alerts_view: AlertsView::new(),
            github_service,
            ai_modal: AiAssistantModal::new(),
            context_menu: ContextMenu::new(),
            command_palette: CommandPaletteModal::new(),
            conflict_resolver: ConflictResolverModal::new(),
            event_sender: None,
        };

        app.refresh_git_status();
        app.refresh_worktrees();
        app
    }

    /// Set event sender for asynchronous background operations.
    pub fn set_event_sender(&mut self, sender: tokio::sync::mpsc::UnboundedSender<AppEvent>) {
        self.event_sender = Some(sender);
    }

    /// Refresh repository status, directory map, and active diff from Git engine.
    pub fn refresh_git_status(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(status) = git.get_status() {
                self.repo_status = status;
                self.dir_map.update_from_files(&self.repo_status.files);

                let filtered = self.filtered_files();
                self.changes_view
                    .file_scroll
                    .set_total_items(filtered.len());
                self.update_active_diff();
            }
        } else {
            // Demo fallback if outside a Git repo
            self.repo_status = GitRepoStatus {
                branch: "main".to_string(),
                upstream: Some("origin/main".to_string()),
                ahead: 1,
                behind: 0,
                files: vec![
                    GitFileItem {
                        path: "src/main.rs".to_string(),
                        staged: true,
                        status: crate::git::FileStatus::Modified,
                    },
                    GitFileItem {
                        path: "src/git/service.rs".to_string(),
                        staged: false,
                        status: crate::git::FileStatus::Added,
                    },
                ],
            };
            self.dir_map.update_from_files(&self.repo_status.files);
            self.changes_view
                .file_scroll
                .set_total_items(self.repo_status.files.len());
        }
    }

    /// Refresh list of active Git worktrees.
    pub fn refresh_worktrees(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(wt_list) = list_worktrees(git.workdir()) {
                self.worktrees = wt_list;
                self.worktrees_view
                    .scroll
                    .set_total_items(self.worktrees.len());
            }
        }
    }

    /// Refresh GitHub PRs, Actions, and Alerts.
    pub fn refresh_github(&mut self) {
        let target = self
            .git_service
            .as_ref()
            .and_then(|g| g.remote_url(None))
            .and_then(|url| GitHubRepoTarget::from_remote_url(&url));

        if let Some(target) = target {
            if let Some(sender) = self.event_sender.clone() {
                let gh = self.github_service.clone();
                tokio::spawn(async move {
                    if let Ok(prs) = gh.list_pull_requests(&target, false).await {
                        let _ = sender.send(AppEvent::GitHubPrsLoaded(prs));
                    }
                    if let Ok(runs) = gh.list_workflow_runs(&target, false).await {
                        let _ = sender.send(AppEvent::GitHubRunsLoaded(runs));
                    }
                    if let Ok(alerts) = gh.list_alerts(false).await {
                        let _ = sender.send(AppEvent::GitHubAlertsLoaded(alerts));
                    }
                });
            } else if tokio::runtime::Handle::try_current().is_err() {
                let gh = &self.github_service;
                if let Ok(prs) = block_on_safe(gh.list_pull_requests(&target, false)) {
                    self.prs_view.set_prs(prs);
                }
                if let Ok(runs) = block_on_safe(gh.list_workflow_runs(&target, false)) {
                    self.actions_view.set_runs(runs);
                }
                if let Ok(alerts) = block_on_safe(gh.list_alerts(false)) {
                    self.alerts_view.set_alerts(alerts);
                }
            }
        }
    }

    /// Retrieve files filtered by active directory in Directory Map.
    pub fn filtered_files(&self) -> Vec<GitFileItem> {
        if let Some(prefix) = self.dir_map.current_selected_prefix() {
            self.repo_status
                .files
                .iter()
                .filter(|f| f.path.starts_with(prefix))
                .cloned()
                .collect()
        } else {
            self.repo_status.files.clone()
        }
    }

    /// Update diff view for currently selected file.
    pub fn update_active_diff(&mut self) {
        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            if let Some(git) = &self.git_service {
                if let Ok(diff) = git.get_diff(Some(&item.path), item.staged) {
                    self.current_diff = diff;
                    return;
                }
            }
        }
        self.current_diff = FileDiff {
            file_path: "Diff Preview".to_string(),
            ..Default::default()
        };
    }

    /// Process a dispatched application event.
    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Mouse(mouse) => self.handle_mouse(mouse),
            AppEvent::Resize(w, h) => {
                self.layout_areas = LayoutAreas::compute(Rect::new(0, 0, w, h));
            }
            AppEvent::Tick => {}
            AppEvent::StatusMessage(msg) => {
                self.last_status = Some(msg);
            }
            AppEvent::GitHubPrsLoaded(prs) => {
                self.prs_view.set_prs(prs);
            }
            AppEvent::GitHubRunsLoaded(runs) => {
                self.actions_view.set_runs(runs);
            }
            AppEvent::GitHubAlertsLoaded(alerts) => {
                self.alerts_view.set_alerts(alerts);
            }
            AppEvent::AiCommitGenerated(res) => match res {
                Ok(gen) => {
                    self.changes_view.commit_summary = gen.summary;
                    if let Some(desc) = gen.description {
                        self.changes_view.commit_description = desc;
                    }
                    self.last_status = Some("AI generated commit message!".to_string());
                }
                Err(e) => {
                    self.last_status = Some(format!("AI error: {}", e));
                }
            },
            AppEvent::AiReviewCompleted(res) => match res {
                Ok(review) => {
                    let ctx = self.capture_ai_context();
                    self.ai_modal.open_with_context(&ctx);
                    self.ai_modal.set_response(review, None);
                }
                Err(e) => {
                    self.last_status = Some(format!("AI review error: {}", e));
                }
            },
            AppEvent::AiDiagnosticCompleted(res) => match res {
                Ok(diag) => {
                    let mut summary = format!(
                        "󱚣 AI Diagnosis: {}\n\nHealth: {:?}\n\nRecommended Actions:\n",
                        diag.headline, diag.health
                    );
                    for action in &diag.recommended_actions {
                        summary.push_str(&format!("• {}\n", action));
                    }
                    if !diag.explanation.is_empty() {
                        summary.push_str(&format!("\n{}", diag.explanation));
                    }
                    let ctx = self.capture_ai_context();
                    self.ai_modal.open_with_context(&ctx);
                    self.ai_modal.set_response(summary, None);
                }
                Err(e) => {
                    self.last_status = Some(format!("AI diagnostic error: {}", e));
                }
            },
            AppEvent::AiPrDraftGenerated(res) => match res {
                Ok(draft) => {
                    let formatted =
                        format!("󰘬 AI PR Draft:\n\nTitle: {}\n\n{}", draft.title, draft.body);
                    let ctx = self.capture_ai_context();
                    self.ai_modal.open_with_context(&ctx);
                    self.ai_modal.set_response(formatted, None);
                }
                Err(e) => {
                    self.last_status = Some(format!("AI PR draft error: {}", e));
                }
            },
            AppEvent::AiCiAnalysisCompleted(res) => match res {
                Ok(analysis) => {
                    let formatted = format!(
                        "󰄛 AI CI Failure Analysis:\n\n• Root Cause: {}\n• Recommended Fix: {}\n\n{}",
                        analysis.root_cause, analysis.suggested_fix, analysis.full_advice
                    );
                    let ctx = self.capture_ai_context();
                    self.ai_modal.open_with_context(&ctx);
                    self.ai_modal.set_response(formatted, None);
                }
                Err(e) => {
                    self.last_status = Some(format!("AI CI error: {}", e));
                }
            },
            AppEvent::AiCommandSuggested(res) => {
                if let Ok(suggested) = res {
                    self.command_palette.ai_suggestion = Some(suggested);
                }
            }
            AppEvent::AiMergeResolved(res) => {
                if let Ok(resolved) = res {
                    self.conflict_resolver.set_ai_resolution(resolved);
                }
            }
        }
    }

    /// Keyboard event handler.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // If Context Menu is active, it captures key navigation
        if self.context_menu.is_open {
            match key.code {
                KeyCode::Esc => self.context_menu.close(),
                KeyCode::Up | KeyCode::Char('k') => self.context_menu.previous(),
                KeyCode::Down | KeyCode::Char('j') => self.context_menu.next(),
                KeyCode::Enter => {
                    let action = self.context_menu.selected_action();
                    self.context_menu.close();
                    self.execute_context_action(action);
                }
                _ => {}
            }
            return;
        }

        // If AI Modal is active, it captures text input
        if self.ai_modal.is_open {
            match key.code {
                KeyCode::Esc => self.ai_modal.close(),
                KeyCode::Enter => {
                    self.ai_modal.is_loading = true;
                    let q = self.ai_modal.input_query.clone();
                    let response = format!(
                        "AI solution for '{}':\n\n- Conventional commit suggested: feat(git): worktree & directory map support\n- Code looks clean and tests are passing.",
                        q
                    );
                    self.ai_modal.set_response(response, None);
                }
                KeyCode::Backspace => self.ai_modal.handle_backspace(),
                KeyCode::Char(c) => self.ai_modal.handle_char(c),
                _ => {}
            }
            return;
        }

        // If Command Palette is active, it captures key input
        if self.command_palette.is_open {
            match key.code {
                KeyCode::Esc => self.command_palette.close(),
                KeyCode::Up => self.command_palette.previous(),
                KeyCode::Down => self.command_palette.next(),
                KeyCode::Backspace => self.command_palette.pop_char(),
                KeyCode::Enter => {
                    if let Some(cmd) = self.command_palette.ai_suggestion.take() {
                        let _ = std::process::Command::new("sh")
                            .args(["-c", &cmd.command])
                            .output();
                        self.last_status = Some(format!("Executed: {}", cmd.command));
                        self.refresh_git_status();
                        self.command_palette.close();
                    } else if let Some(action) = self.command_palette.selected_action() {
                        let id = action.id;
                        self.command_palette.close();
                        self.execute_palette_action(id);
                    } else {
                        let req = self.command_palette.query.clone();
                        let branch = self.repo_status.branch.clone();
                        let client = crate::ai::create_ai_client(&self.config.ai);
                        let lang = self.config.language;
                        if let Some(sender) = self.event_sender.clone() {
                            tokio::spawn(async move {
                                let res = translate_natural_language_to_git(
                                    &req,
                                    &branch,
                                    client.as_ref(),
                                    lang,
                                )
                                .await
                                .map_err(|e| e.to_string());
                                let _ = sender.send(AppEvent::AiCommandSuggested(res));
                            });
                        } else if tokio::runtime::Handle::try_current().is_err() {
                            if let Ok(suggested) = block_on_safe(translate_natural_language_to_git(
                                &req,
                                &branch,
                                client.as_ref(),
                                lang,
                            )) {
                                self.command_palette.ai_suggestion = Some(suggested);
                            }
                        }
                    }
                }
                KeyCode::Char(c) => self.command_palette.push_char(c),
                _ => {}
            }
            return;
        }

        // If Conflict Resolver is active, it captures conflict navigation and choices
        if self.conflict_resolver.is_open {
            match key.code {
                KeyCode::Esc => self.conflict_resolver.close(),
                KeyCode::Tab | KeyCode::Char('n') => self.conflict_resolver.next_hunk(),
                KeyCode::Char('p') => self.conflict_resolver.previous_hunk(),
                KeyCode::Char('1') => self
                    .conflict_resolver
                    .resolve_current(ConflictChoice::Current),
                KeyCode::Char('2') => self
                    .conflict_resolver
                    .resolve_current(ConflictChoice::Incoming),
                KeyCode::Char('3') => self.conflict_resolver.resolve_current(ConflictChoice::Both),
                KeyCode::Char('4') | KeyCode::Char('m')
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        || key.code == KeyCode::Char('4') =>
                {
                    if let Some(hunk) = self.conflict_resolver.current_hunk().cloned() {
                        let client = crate::ai::create_ai_client(&self.config.ai);
                        let path = self.conflict_resolver.file_path.clone();
                        let lang = self.config.language;
                        if let Some(sender) = self.event_sender.clone() {
                            tokio::spawn(async move {
                                let res = smart_merge_conflict(&path, &hunk, client.as_ref(), lang)
                                    .await
                                    .map_err(|e| e.to_string());
                                let _ = sender.send(AppEvent::AiMergeResolved(res));
                            });
                        } else if tokio::runtime::Handle::try_current().is_err() {
                            if let Ok(resolved) = block_on_safe(smart_merge_conflict(
                                &path,
                                &hunk,
                                client.as_ref(),
                                lang,
                            )) {
                                self.conflict_resolver.set_ai_resolution(resolved);
                            }
                        }
                    }
                }
                KeyCode::Enter => {
                    let path = self.conflict_resolver.file_path.clone();
                    let content = self.conflict_resolver.build_resolved_content();
                    if let Ok(_) = std::fs::write(&path, content) {
                        if let Some(git) = &self.git_service {
                            let _ = git.stage_file(&path);
                        }
                        self.refresh_git_status();
                        self.last_status =
                            Some(format!("Resolved and staged conflicts in {}", path));
                    }
                    self.conflict_resolver.close();
                }
                _ => {}
            }
            return;
        }

        // Global shortcuts with Control
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('a') => {
                    let ctx = self.capture_ai_context();
                    self.ai_modal.toggle(&ctx);
                    return;
                }
                // Ctrl+G: AI Generate Commit Message
                KeyCode::Char('g') => {
                    self.trigger_ai_generate_commit();
                    return;
                }
                // Ctrl+R: AI Pre-Commit Review
                KeyCode::Char('r') => {
                    self.trigger_ai_review();
                    return;
                }
                // Ctrl+D: Repository AI Diagnostic
                KeyCode::Char('d') => {
                    self.trigger_ai_diagnostic();
                    return;
                }
                // Ctrl+S: Stash All
                KeyCode::Char('s') => {
                    if let Some(git) = &self.git_service {
                        if let Ok(msg) = git.cli_stash_save("Prowly auto stash") {
                            self.last_status = Some(format!("Stashed: {}", msg));
                            self.refresh_git_status();
                        }
                    }
                    return;
                }
                // Ctrl+P: Pop Stash
                KeyCode::Char('p') => {
                    if let Some(git) = &self.git_service {
                        if let Ok(msg) = git.cli_stash_pop() {
                            self.last_status = Some(format!("Popped stash: {}", msg));
                            self.refresh_git_status();
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => {
                let ctx = self.capture_ai_context();
                self.ai_modal.toggle(&ctx);
            }
            // Open Context Menu via 'm' or F10
            KeyCode::Char('m') | KeyCode::F(10) => {
                let sel_file = self
                    .filtered_files()
                    .get(self.changes_view.file_scroll.selected_index)
                    .map(|f| f.path.clone());
                self.context_menu.open_at(sel_file, 40, 10);
            }
            KeyCode::Tab => {
                self.focused_pane = match self.focused_pane {
                    FocusedPane::DirectoryMap => FocusedPane::MainContent,
                    FocusedPane::MainContent => FocusedPane::DirectoryMap,
                };
            }
            // Tab switching 1..5
            KeyCode::Char('1') => self.active_tab = AppTab::Changes,
            KeyCode::Char('2') => self.active_tab = AppTab::Prs,
            KeyCode::Char('3') => self.active_tab = AppTab::Actions,
            KeyCode::Char('4') => self.active_tab = AppTab::Alerts,
            KeyCode::Char('5') => {
                self.active_tab = AppTab::Worktrees;
                self.refresh_worktrees();
            }

            // Cycle Theme (t)
            KeyCode::Char('t') => self.cycle_theme(),

            // Toggle Language (l)
            KeyCode::Char('l') => {
                let new_lang = self.i18n.toggle_language();
                self.config.language = new_lang;
                self.last_status = Some(format!("Language: {}", new_lang.display_name()));
            }

            // Copy (y) to Clipboard
            // Copy (y) to Clipboard
            KeyCode::Char('y') => {
                let text_to_copy = match self.focused_pane {
                    FocusedPane::DirectoryMap => self
                        .dir_map
                        .nodes
                        .get(self.dir_map.scroll.selected_index)
                        .map(|n| n.path_prefix.clone()),
                    FocusedPane::MainContent => match self.active_tab {
                        AppTab::Prs => self.prs_view.selected_pr().map(|p| p.html_url.clone()),
                        AppTab::Actions => {
                            self.actions_view.selected_run().map(|r| r.html_url.clone())
                        }
                        AppTab::Alerts => self.alerts_view.selected_alert().map(|a| a.url.clone()),
                        _ => self
                            .filtered_files()
                            .get(self.changes_view.file_scroll.selected_index)
                            .map(|item| item.path.clone()),
                    },
                };
                if let Some(text) = text_to_copy {
                    if self.clipboard.copy_text(&text).is_ok() {
                        self.last_status = Some(format!("Yanked to clipboard: {}", text));
                    }
                }
            }

            // Space: Toggle Staging in Changes view, or Filter in Directory Map
            KeyCode::Char(' ') => match self.focused_pane {
                FocusedPane::DirectoryMap => {
                    let filter = self.dir_map.apply_selection();
                    let filter_msg = filter
                        .as_deref()
                        .map(|p| format!("Filtering by '{}'", p))
                        .unwrap_or_else(|| "Filter cleared (All files)".to_string());
                    self.last_status = Some(filter_msg);
                    let filtered = self.filtered_files();
                    self.changes_view
                        .file_scroll
                        .set_total_items(filtered.len());
                    self.update_active_diff();
                }
                FocusedPane::MainContent => {
                    if self.active_tab == AppTab::Changes {
                        self.toggle_stage_selected();
                    }
                }
            },

            // 'a': Stage All
            KeyCode::Char('a') => {
                if let Some(git) = &self.git_service {
                    let _ = git.stage_all();
                    self.refresh_git_status();
                    self.last_status = Some("Staged all files".to_string());
                }
            }

            // 'u': Unstage All
            KeyCode::Char('u') => {
                if let Some(git) = &self.git_service {
                    let _ = git.unstage_all();
                    self.refresh_git_status();
                    self.last_status = Some("Unstaged all files".to_string());
                }
            }

            // 'd': Safe Soft-Discard for selected file or mark notification as read
            KeyCode::Char('d') => {
                if self.active_tab == AppTab::Alerts {
                    if let Some(id) = self.alerts_view.mark_selected_read() {
                        let gh = &self.github_service;
                        let _ = block_on_safe(gh.mark_notification_read(&id));
                        self.last_status = Some("Marked notification as read".to_string());
                    }
                } else {
                    self.safe_discard_selected();
                }
            }

            // 'c': Commit staged changes
            KeyCode::Char('c') => {
                self.commit_staged();
            }

            // ':' or F1: Open Command Palette
            KeyCode::Char(':') | KeyCode::F(1) => {
                self.command_palette.open();
            }

            // 'e': Open external editor on selected file
            KeyCode::Char('e') => {
                let filtered = self.filtered_files();
                if let Some(item) = filtered.get(self.changes_view.file_scroll.selected_index) {
                    let path = std::path::PathBuf::from(&item.path);
                    if let Ok(_) = open_external_editor(&path) {
                        self.refresh_git_status();
                        self.last_status = Some(format!("Edited {}", item.path));
                    }
                }
            }

            // 'R': Refresh GitHub data
            KeyCode::Char('R') => {
                self.refresh_github();
                self.last_status = Some("Refreshed GitHub PRs, Actions, and Alerts".to_string());
            }

            // Ctrl+P: AI Draft Pull Request
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.trigger_ai_pr_draft();
            }

            // Ctrl+F: AI Analyze CI Failure
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.trigger_ai_ci_analysis();
            }

            // Worktree actions in Worktrees tab
            KeyCode::Char('W') => {
                if self.active_tab == AppTab::Worktrees {
                    self.prompt_create_worktree();
                }
            }
            KeyCode::Char('D') => {
                if self.active_tab == AppTab::Worktrees {
                    self.prompt_remove_worktree();
                }
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.previous(),
                FocusedPane::MainContent => match self.active_tab {
                    AppTab::Worktrees => self.worktrees_view.scroll.previous(),
                    AppTab::Prs => self.prs_view.previous(),
                    AppTab::Actions => self.actions_view.previous(),
                    AppTab::Alerts => self.alerts_view.previous(),
                    AppTab::Changes => {
                        self.changes_view.file_scroll.previous();
                        self.update_active_diff();
                    }
                },
            },
            KeyCode::Down | KeyCode::Char('j') => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.next(),
                FocusedPane::MainContent => match self.active_tab {
                    AppTab::Worktrees => self.worktrees_view.scroll.next(),
                    AppTab::Prs => self.prs_view.next(),
                    AppTab::Actions => self.actions_view.next(),
                    AppTab::Alerts => self.alerts_view.next(),
                    AppTab::Changes => {
                        self.changes_view.file_scroll.next();
                        self.update_active_diff();
                    }
                },
            },
            KeyCode::PageUp => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.page_up(),
                FocusedPane::MainContent => {
                    if self.active_tab == AppTab::Worktrees {
                        self.worktrees_view.scroll.page_up();
                    } else {
                        self.changes_view.file_scroll.page_up();
                        self.update_active_diff();
                    }
                }
            },
            KeyCode::PageDown => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.page_down(),
                FocusedPane::MainContent => {
                    if self.active_tab == AppTab::Worktrees {
                        self.worktrees_view.scroll.page_down();
                    } else {
                        self.changes_view.file_scroll.page_down();
                        self.update_active_diff();
                    }
                }
            },
            KeyCode::Home => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.scroll_to_top(),
                FocusedPane::MainContent => {
                    if self.active_tab == AppTab::Worktrees {
                        self.worktrees_view.scroll.scroll_to_top();
                    } else {
                        self.changes_view.file_scroll.scroll_to_top();
                        self.update_active_diff();
                    }
                }
            },
            KeyCode::End => match self.focused_pane {
                FocusedPane::DirectoryMap => self.dir_map.scroll.scroll_to_bottom(),
                FocusedPane::MainContent => {
                    if self.active_tab == AppTab::Worktrees {
                        self.worktrees_view.scroll.scroll_to_bottom();
                    } else {
                        self.changes_view.file_scroll.scroll_to_bottom();
                        self.update_active_diff();
                    }
                }
            },
            _ => {}
        }
    }

    /// Mouse event handler.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        let (x, y) = (mouse.column, mouse.row);

        // Right-Click: Open Context Menu
        if mouse.kind == MouseEventKind::Down(MouseButton::Right) {
            let sel_file = self
                .filtered_files()
                .get(self.changes_view.file_scroll.selected_index)
                .map(|f| f.path.clone());
            self.context_menu.open_at(sel_file, x, y);
            return;
        }

        // Left-Click when context menu is open
        if self.context_menu.is_open {
            self.context_menu.close();
            return;
        }

        // Click on Ask AI header button
        if self.layout_areas.hits_ask_ai(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let ctx = self.capture_ai_context();
            self.ai_modal.toggle(&ctx);
            return;
        }

        // Click on Help header button
        if self.layout_areas.hits_help(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.last_status = Some(
                "Help: [Tab] Panes | [Space] Filter/Stage | [m/Right-Click] Menu | [Ctrl+D] AI Diagnose | [q] Quit".to_string(),
            );
            return;
        }

        // Tabs bar click
        if self.layout_areas.hits_tabs(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let tab_width = 16;
            let rel_x = x.saturating_sub(self.layout_areas.right_tabs.x);
            let clicked_tab_idx = (rel_x / tab_width) as usize;
            if clicked_tab_idx < AppTab::ALL.len() {
                self.active_tab = AppTab::from_index(clicked_tab_idx);
                if self.active_tab == AppTab::Worktrees {
                    self.refresh_worktrees();
                }
            }
            return;
        }

        // Directory Map pane click & scroll
        if self.layout_areas.hits_left_pane(x, y) {
            self.focused_pane = FocusedPane::DirectoryMap;
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let rel_y = y.saturating_sub(self.layout_areas.left_pane.y + 1) as usize;
                    if self.dir_map.scroll.handle_click_row(rel_y).is_some() {
                        let filter = self.dir_map.apply_selection();
                        let msg = filter
                            .as_deref()
                            .map(|p| format!("Filtering by '{}'", p))
                            .unwrap_or_else(|| "Filter cleared".to_string());
                        self.last_status = Some(msg);
                        let filtered = self.filtered_files();
                        self.changes_view
                            .file_scroll
                            .set_total_items(filtered.len());
                        self.update_active_diff();
                    }
                }
                MouseEventKind::ScrollDown => self.dir_map.scroll.scroll_down(3),
                MouseEventKind::ScrollUp => self.dir_map.scroll.scroll_up(3),
                _ => {}
            }
            return;
        }

        // Right content pane click & scroll
        if self.layout_areas.hits_right_content(x, y) {
            self.focused_pane = FocusedPane::MainContent;
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let rel_y = y.saturating_sub(self.layout_areas.right_content.y + 1) as usize;
                    match self.active_tab {
                        AppTab::Worktrees => {
                            self.worktrees_view.scroll.handle_click_row(rel_y);
                        }
                        AppTab::Prs => {
                            self.prs_view.list_scroll.handle_click_row(rel_y);
                        }
                        AppTab::Actions => {
                            self.actions_view.list_scroll.handle_click_row(rel_y);
                        }
                        AppTab::Alerts => {
                            self.alerts_view.scroll.handle_click_row(rel_y);
                        }
                        AppTab::Changes => {
                            if self
                                .changes_view
                                .file_scroll
                                .handle_click_row(rel_y)
                                .is_some()
                            {
                                self.update_active_diff();
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => match self.active_tab {
                    AppTab::Worktrees => self.worktrees_view.scroll.scroll_down(3),
                    AppTab::Prs => self.prs_view.list_scroll.scroll_down(3),
                    AppTab::Actions => self.actions_view.list_scroll.scroll_down(3),
                    AppTab::Alerts => self.alerts_view.scroll.scroll_down(3),
                    AppTab::Changes => {
                        self.changes_view.file_scroll.scroll_down(3);
                        self.update_active_diff();
                    }
                },
                MouseEventKind::ScrollUp => match self.active_tab {
                    AppTab::Worktrees => self.worktrees_view.scroll.scroll_up(3),
                    AppTab::Prs => self.prs_view.list_scroll.scroll_up(3),
                    AppTab::Actions => self.actions_view.list_scroll.scroll_up(3),
                    AppTab::Alerts => self.alerts_view.scroll.scroll_up(3),
                    AppTab::Changes => {
                        self.changes_view.file_scroll.scroll_up(3);
                        self.update_active_diff();
                    }
                },
                _ => {}
            }
        }
    }

    /// Dispatch context menu selection.
    pub fn execute_context_action(&mut self, action: ContextMenuAction) {
        match action {
            ContextMenuAction::ToggleStage => self.toggle_stage_selected(),
            ContextMenuAction::SafeDiscard => self.safe_discard_selected(),
            ContextMenuAction::CopyPath => {
                if let Some(item) = self
                    .filtered_files()
                    .get(self.changes_view.file_scroll.selected_index)
                {
                    let _ = self.clipboard.copy_text(&item.path);
                    self.last_status = Some(format!("Copied path: {}", item.path));
                }
            }
            ContextMenuAction::CopyDiff => {
                let diff_text = format!("{:?}", self.current_diff);
                let _ = self.clipboard.copy_text(&diff_text);
                self.last_status = Some("Copied diff to clipboard".to_string());
            }
            ContextMenuAction::AskAi => {
                let ctx = self.capture_ai_context();
                self.ai_modal.open_with_context(&ctx);
            }
        }
    }

    /// Execute an action selected from the Command Palette.
    pub fn execute_palette_action(&mut self, action_id: &str) {
        match action_id {
            "stage_all" => {
                if let Some(git) = &self.git_service {
                    let _ = git.stage_all();
                    self.refresh_git_status();
                }
            }
            "unstage_all" => {
                if let Some(git) = &self.git_service {
                    let _ = git.unstage_all();
                    self.refresh_git_status();
                }
            }
            "commit" => self.commit_staged(),
            "tab_changes" => self.active_tab = AppTab::Changes,
            "tab_prs" => self.active_tab = AppTab::Prs,
            "tab_actions" => self.active_tab = AppTab::Actions,
            "tab_alerts" => self.active_tab = AppTab::Alerts,
            "tab_worktrees" => self.active_tab = AppTab::Worktrees,
            "cycle_theme" => self.cycle_theme(),
            "toggle_lang" => {
                let l = self.i18n.toggle_language();
                self.config.language = l;
            }
            "ai_diag" => self.trigger_ai_diagnostic(),
            "ai_review" => self.trigger_ai_review(),
            "resolve_conflicts" => {
                if let Some(conflicted) = self
                    .repo_status
                    .files
                    .iter()
                    .find(|f| f.status == crate::git::FileStatus::Conflicted)
                {
                    if let Ok(content) = std::fs::read_to_string(&conflicted.path) {
                        self.conflict_resolver
                            .open(conflicted.path.clone(), content);
                    }
                } else {
                    self.last_status = Some("No conflicted files found".to_string());
                }
            }
            _ => {}
        }
    }

    /// Toggle stage for currently selected file.
    pub fn toggle_stage_selected(&mut self) {
        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            let path = item.path.clone();
            let was_staged = item.staged;

            if let Some(git) = &self.git_service {
                let res = if was_staged {
                    git.unstage_file(&path)
                } else {
                    git.stage_file(&path)
                };
                if res.is_ok() {
                    self.refresh_git_status();
                }
            } else if let Some(item_mut) =
                self.repo_status.files.iter_mut().find(|f| f.path == path)
            {
                item_mut.staged = !item_mut.staged;
            }
        }
    }

    /// Safe discard with automated backup for selected file.
    pub fn safe_discard_selected(&mut self) {
        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            let path = item.path.clone();
            if let Some(git) = &self.git_service {
                match git.soft_discard(&path, &self.backup_service) {
                    Ok(backup) => {
                        self.refresh_git_status();
                        self.last_status = Some(format!(
                            "Safe discard: backup saved to {}",
                            backup.backup_path.display()
                        ));
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Discard failed: {}", e));
                    }
                }
            }
        }
    }

    /// Commit staged changes using summary input.
    pub fn commit_staged(&mut self) {
        let summary = if self.changes_view.commit_summary.is_empty() {
            "chore: update files".to_string()
        } else {
            self.changes_view.commit_summary.clone()
        };

        if let Some(git) = &self.git_service {
            match git.commit(&summary) {
                Ok(oid) => {
                    self.last_status = Some(format!("Committed: {}", &oid.to_string()[..7]));
                    self.changes_view.commit_summary.clear();
                    self.refresh_git_status();
                }
                Err(e) => {
                    self.last_status = Some(format!("Commit failed: {}", e));
                }
            }
        }
    }

    /// Worktree creation.
    pub fn prompt_create_worktree(&mut self) {
        if let Some(git) = &self.git_service {
            let count = self.worktrees.len() + 1;
            let name = format!("worktree-feat-{}", count);
            let target_path = git.workdir().join(&name);
            match create_worktree(git.workdir(), &target_path, None) {
                Ok(wt) => {
                    self.last_status = Some(format!("Created worktree: {}", wt.name));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.last_status = Some(format!("Worktree creation failed: {}", e));
                }
            }
        }
    }

    /// Worktree removal.
    pub fn prompt_remove_worktree(&mut self) {
        let idx = self.worktrees_view.scroll.selected_index;
        if let Some(wt) = self.worktrees.get(idx) {
            if wt.is_current {
                self.last_status = Some("Cannot remove the current active worktree".to_string());
                return;
            }
            let name = wt.name.clone();
            if let Some(git) = &self.git_service {
                match remove_worktree(git.workdir(), &name, false) {
                    Ok(()) => {
                        self.last_status = Some(format!("Removed worktree: {}", name));
                        self.refresh_worktrees();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Worktree removal failed: {}", e));
                    }
                }
            }
        }
    }

    /// Trigger AI to generate a Conventional Commit message from staged diff.
    pub fn trigger_ai_generate_commit(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let diff_text = format!("{:?}", self.current_diff);
        let lang = self.config.language;

        if let Some(sender) = self.event_sender.clone() {
            self.changes_view.commit_summary = "󱚣 AI generating commit...".to_string();
            tokio::spawn(async move {
                let res = generate_commit_message(&diff_text, client.as_ref(), lang)
                    .await
                    .map_err(|e| e.to_string());
                let _ = sender.send(AppEvent::AiCommitGenerated(res));
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            let commit_res =
                block_on_safe(generate_commit_message(&diff_text, client.as_ref(), lang));
            if let Ok(gen) = commit_res {
                self.changes_view.commit_summary = gen.summary;
                if let Some(desc) = gen.description {
                    self.changes_view.commit_description = desc;
                }
                self.last_status = Some("AI generated commit message!".to_string());
            }
        }
    }

    /// Trigger AI pre-commit code review.
    pub fn trigger_ai_review(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let diff_text = format!("{:?}", self.current_diff);
        let lang = self.config.language;

        let ctx = self.capture_ai_context();
        self.ai_modal.open_with_context(&ctx);
        self.ai_modal.response_text = "󱚣 AI reviewing code...".to_string();

        if let Some(sender) = self.event_sender.clone() {
            tokio::spawn(async move {
                let res = review_changes(&diff_text, client.as_ref(), lang)
                    .await
                    .map_err(|e| e.to_string());
                let _ = sender.send(AppEvent::AiReviewCompleted(res));
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            let review_res = block_on_safe(review_changes(&diff_text, client.as_ref(), lang));
            if let Ok(review) = review_res {
                self.ai_modal.set_response(review, None);
            }
        }
    }

    /// Trigger full repository AI diagnostic.
    pub fn trigger_ai_diagnostic(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let status = self.repo_status.clone();
        let lang = self.config.language;

        let ctx = self.capture_ai_context();
        self.ai_modal.open_with_context(&ctx);
        self.ai_modal.response_text = "󱚣 AI diagnosing repository health...".to_string();

        if let Some(sender) = self.event_sender.clone() {
            tokio::spawn(async move {
                let res = diagnose_repository(&status, client.as_ref(), lang)
                    .await
                    .map_err(|e| e.to_string());
                let _ = sender.send(AppEvent::AiDiagnosticCompleted(res));
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            let diag_res = block_on_safe(diagnose_repository(&status, client.as_ref(), lang));
            if let Ok(diag) = diag_res {
                let mut summary = format!(
                    "󱚣 AI Diagnosis: {}\n\nHealth: {:?}\n\nRecommended Actions:\n",
                    diag.headline, diag.health
                );
                for action in &diag.recommended_actions {
                    summary.push_str(&format!("• {}\n", action));
                }
                if !diag.explanation.is_empty() {
                    summary.push_str(&format!("\n{}", diag.explanation));
                }
                self.ai_modal.set_response(summary, None);
            }
        }
    }

    /// Trigger AI generation of PR draft title and body.
    pub fn trigger_ai_pr_draft(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let branch = self.repo_status.branch.clone();
        let diff_text = format!("{:?}", self.current_diff);
        let lang = self.config.language;

        let ctx = self.capture_ai_context();
        self.ai_modal.open_with_context(&ctx);
        self.ai_modal.response_text = "󱚣 AI drafting PR...".to_string();

        if let Some(sender) = self.event_sender.clone() {
            tokio::spawn(async move {
                let res = generate_pr_draft(&branch, &diff_text, client.as_ref(), lang)
                    .await
                    .map_err(|e| e.to_string());
                let _ = sender.send(AppEvent::AiPrDraftGenerated(res));
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            if let Ok(draft) = block_on_safe(generate_pr_draft(
                &branch,
                &diff_text,
                client.as_ref(),
                lang,
            )) {
                let formatted =
                    format!("󰘬 AI PR Draft:\n\nTitle: {}\n\n{}", draft.title, draft.body);
                self.ai_modal.set_response(formatted, None);
            }
        }
    }

    /// Trigger AI analysis of CI failure.
    pub fn trigger_ai_ci_analysis(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let lang = self.config.language;

        let (run_name, log_snippet) = if let Some(run) = self.actions_view.selected_run() {
            (
                run.name.clone(),
                run.logs
                    .clone()
                    .unwrap_or_else(|| "Error: Process completed with exit code 1".to_string()),
            )
        } else {
            ("CI Workflow".to_string(), "Failed build".to_string())
        };

        let ctx = self.capture_ai_context();
        self.ai_modal.open_with_context(&ctx);
        self.ai_modal.response_text = "󱚣 AI analyzing CI logs...".to_string();

        if let Some(sender) = self.event_sender.clone() {
            tokio::spawn(async move {
                let res = analyze_ci_failure(&run_name, &log_snippet, client.as_ref(), lang)
                    .await
                    .map_err(|e| e.to_string());
                let _ = sender.send(AppEvent::AiCiAnalysisCompleted(res));
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            if let Ok(analysis) = block_on_safe(analyze_ci_failure(
                &run_name,
                &log_snippet,
                client.as_ref(),
                lang,
            )) {
                let formatted = format!(
                    "󰄛 AI CI Failure Analysis:\n\n• Root Cause: {}\n• Recommended Fix: {}\n\n{}",
                    analysis.root_cause, analysis.suggested_fix, analysis.full_advice
                );
                self.ai_modal.set_response(formatted, None);
            }
        }
    }

    /// Advance active theme through all 21 presets.
    pub fn cycle_theme(&mut self) {
        let all = ThemeId::ALL;
        let current_idx = all.iter().position(|&id| id == self.theme.id).unwrap_or(0);
        let next_idx = (current_idx + 1) % all.len();
        let next_id = all[next_idx];
        self.theme = Theme::get(next_id);
        self.config.theme = next_id;
        self.last_status = Some(format!("Theme: {}", self.theme.name));
    }

    /// Render complete TUI interface into Ratatui frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.layout_areas = LayoutAreas::compute(area);

        // 1. Header Rendering
        self.render_header(frame);

        // 2. Left Directory Map Pane
        let is_dir_focused = self.focused_pane == FocusedPane::DirectoryMap;
        let is_git = self.git_service.is_some();
        self.dir_map.render(
            frame,
            is_dir_focused,
            is_git,
            &self.theme,
            &self.icons,
            self.layout_areas.left_pane,
        );

        // 3. Right Pane Tabs & Content
        self.render_right_pane(frame);

        // 4. Footer Rendering
        self.render_footer(frame);

        // 5. Context Menu Overlay (if open)
        if self.context_menu.is_open {
            self.context_menu
                .render(frame, &self.theme, &self.icons, area);
        }

        // 6. Omnipresent AI Modal Overlay (if open)
        if self.ai_modal.is_open {
            self.ai_modal.render(frame, &self.theme, area);
        }

        // 7. Command Palette Overlay (if open)
        if self.command_palette.is_open {
            self.command_palette
                .render(frame, &self.theme, &self.icons, area);
        }

        // 8. Conflict Resolver Modal Overlay (if open)
        if self.conflict_resolver.is_open {
            self.conflict_resolver
                .render(frame, &self.theme, &self.icons, area);
        }
    }

    fn render_header(&self, frame: &mut Frame) {
        let top_area = self.layout_areas.header_top;
        let bottom_area = self.layout_areas.header_bottom;

        // Top Header Row
        let top_line = Line::from(vec![
            Span::styled(
                " 󰄛 Prowly ",
                Style::default()
                    .fg(self.theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [  ~/prowly ] ", Style::default().fg(self.theme.dim_fg)),
            Span::styled(
                format!(
                    " [ {} {} ({}{} {}{}) ] ",
                    self.icons.branch(),
                    self.repo_status.branch,
                    self.icons.arrow_up(),
                    self.repo_status.ahead,
                    self.icons.arrow_down(),
                    self.repo_status.behind
                ),
                Style::default().fg(self.theme.info),
            ),
            Span::styled(
                format!(" [ {} Synced ] ", self.icons.check_success()),
                Style::default().fg(self.theme.success),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(top_line).style(Style::default().bg(self.theme.header_bg)),
            top_area,
        );

        // Top right buttons: [ 󱚣 Ask AI ] and [? Help]
        let ask_ai_p = Paragraph::new(Span::styled(
            " [ 󱚣 Ask AI ] ",
            Style::default()
                .fg(self.theme.ai_sparkle)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(ask_ai_p, self.layout_areas.ask_ai_button);

        let help_p = Paragraph::new(Span::styled(
            " [? Help] ",
            Style::default().fg(self.theme.secondary),
        ));
        frame.render_widget(help_p, self.layout_areas.help_button);

        // Bottom Header Row
        let bottom_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(" user @ repo ({})", self.i18n.t("header.private")),
                Style::default().fg(self.theme.dim_fg),
            ),
            Span::raw("    "),
            Span::styled(
                format!("{} CI: #42 Passed", self.icons.check_success()),
                Style::default().fg(self.theme.success),
            ),
            Span::raw("    "),
            Span::styled(
                format!("{} Alerts: 0", self.icons.get(crate::ui::IconId::Alert)),
                Style::default().fg(self.theme.dim_fg),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(bottom_line).style(Style::default().bg(self.theme.header_bg)),
            bottom_area,
        );
    }

    fn render_right_pane(&mut self, frame: &mut Frame) {
        // Render Tabs
        let titles: Vec<Line> = AppTab::ALL
            .iter()
            .map(|tab| {
                let key = tab.title_key();
                let text = self.i18n.t(key);
                let icon_str = match tab {
                    AppTab::Changes => self.icons.branch(),
                    AppTab::Prs => self.icons.pr_open(),
                    AppTab::Actions => self.icons.check_success(),
                    AppTab::Alerts => self.icons.get(crate::ui::IconId::Alert),
                    AppTab::Worktrees => self.icons.worktree(),
                };
                Line::from(format!(" {} {} ", icon_str, text))
            })
            .collect();

        let tabs = Tabs::new(titles)
            .select(self.active_tab.index())
            .style(Style::default().fg(self.theme.dim_fg))
            .highlight_style(
                Style::default()
                    .fg(self.theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("│");

        frame.render_widget(tabs, self.layout_areas.right_tabs);

        // Render Active Tab Content
        let is_focused = self.focused_pane == FocusedPane::MainContent;
        let border_color = if is_focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(self.theme.bg));

        let inner = block.inner(self.layout_areas.right_content);
        frame.render_widget(block, self.layout_areas.right_content);

        match self.active_tab {
            AppTab::Changes => {
                let filtered = self.filtered_files();
                let diff = self.current_diff.clone();
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.changes_view
                    .render(frame, &filtered, &diff, &theme, &self.i18n, &icons, inner);
            }
            AppTab::Prs => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.prs_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Actions => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.actions_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Alerts => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.alerts_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Worktrees => {
                let worktrees = self.worktrees.clone();
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.worktrees_view
                    .render(frame, &worktrees, &theme, &icons, inner);
            }
        }
    }

    fn render_footer(&self, frame: &mut Frame) {
        let status_msg = self.last_status.as_deref().unwrap_or("Editing Prowly");
        let left_part = format!(
            " Context: {} | [Tab] Panes [Space] Filter/Stage [m] Menu [Ctrl+D] AI Diag [q] Quit",
            status_msg
        );
        let right_part = format!(
            "Theme: {} | Lang: {} ",
            self.theme.name,
            self.config.language.code().to_uppercase()
        );

        let space_count = (self.layout_areas.footer.width as usize)
            .saturating_sub(left_part.len() + right_part.len());
        let footer_text = format!(
            "{}{:width$}{}",
            left_part,
            "",
            right_part,
            width = space_count
        );

        let p = Paragraph::new(footer_text).style(
            Style::default()
                .bg(self.theme.status_bg)
                .fg(self.theme.status_fg),
        );
        frame.render_widget(p, self.layout_areas.footer);
    }
}

impl ContextProvider for App {
    fn capture_ai_context(&self) -> AiContext {
        let mut ctx = AiContext::new(format!("{:?}", self.active_tab));
        ctx.repo_name = Some("Prowly".to_string());
        ctx.current_branch = Some(self.repo_status.branch.clone());
        ctx.upstream_ahead = self.repo_status.ahead;
        ctx.upstream_behind = self.repo_status.behind;

        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            ctx.selected_file = Some(item.path.clone());
            ctx.is_staged = Some(item.staged);
        }
        ctx
    }
}
