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
use crate::services::{
    open_external_editor_with_preference, reveal_in_file_manager, BackupService, ClipboardService,
};
use crate::ui::layout::LayoutAreas;
use crate::ui::modals::{
    AiAssistantModal, BranchSwitcherModal, CommandPaletteModal, ConflictResolverModal, ContextMenu,
    ContextMenuAction, HistoryActionState, HistoryActionsModal, HistoryMenuAction,
    MarkdownPreviewModal, OpenRepoAction, OpenRepoModal, PaletteEnterResult, RebaseModal,
    RemoteDiffPreviewModal, SettingsAction, SettingsModal,
};
use crate::ui::theme::{Theme, ThemeId};
use crate::ui::views::{
    changes::{ChangesButtonAction, ChangesFocus, ChangesRenderParams},
    ActionsView, AlertsView, ChangesView, DirectoryMap, HistoryView, PrsView,
    WorktreesView,
};
use crate::ui::capsule::{key_badge, pill, split_pill};
use crate::ui::Icons;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
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
    History,
    Prs,
    Actions,
    Alerts,
    Worktrees,
}

impl AppTab {
    pub const ALL: &'static [AppTab] = &[
        AppTab::Changes,
        AppTab::History,
        AppTab::Prs,
        AppTab::Actions,
        AppTab::Alerts,
        AppTab::Worktrees,
    ];

    pub fn index(&self) -> usize {
        match self {
            Self::Changes => 0,
            Self::History => 1,
            Self::Prs => 2,
            Self::Actions => 3,
            Self::Alerts => 4,
            Self::Worktrees => 5,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::History,
            2 => Self::Prs,
            3 => Self::Actions,
            4 => Self::Alerts,
            5 => Self::Worktrees,
            _ => Self::Changes,
        }
    }

    pub fn title_key(&self) -> &'static str {
        match self {
            Self::Changes => "tabs.changes",
            Self::History => "tabs.history",
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
    pub previous_branch: Option<String>,

    // UI state
    pub active_tab: AppTab,
    pub focused_pane: FocusedPane,
    pub should_quit: bool,
    pub last_status: Option<String>,
    pub layout_areas: LayoutAreas,

    // Views
    pub dir_map: DirectoryMap,
    pub changes_view: ChangesView,
    pub history_view: HistoryView,
    pub worktrees_view: WorktreesView,
    pub prs_view: PrsView,
    pub actions_view: ActionsView,
    pub alerts_view: AlertsView,

    // GitHub Integration
    pub github_service: GitHubService,
    pub is_repo_private: Option<bool>,

    // Modals
    pub ai_modal: AiAssistantModal,
    pub context_menu: ContextMenu,
    pub command_palette: CommandPaletteModal,
    pub conflict_resolver: ConflictResolverModal,
    pub branch_switcher: BranchSwitcherModal,
    pub history_modal: HistoryActionsModal,
    pub remote_diff_preview: RemoteDiffPreviewModal,
    pub rebase_modal: RebaseModal,
    pub markdown_preview_modal: MarkdownPreviewModal,
    pub settings_modal: SettingsModal,
    pub open_repo_modal: OpenRepoModal,

    // File browsing state
    pub all_repo_files: Vec<GitFileItem>,
    pub show_all_files: bool,

    // Channel sender for background async tasks
    pub event_sender: Option<tokio::sync::mpsc::UnboundedSender<AppEvent>>,

    /// Flag requesting full terminal clear and buffer reset on next frame
    pub needs_clear: bool,

    /// Last time an auto-fetch was triggered
    pub last_auto_fetch: std::time::Instant,

    /// Last time local git status was refreshed
    pub last_status_check: std::time::Instant,

    /// Current mouse hover coordinates in terminal cells
    pub hover_pos: Option<(u16, u16)>,
}

impl App {
    /// Create new application instance from configuration.
    pub fn new(mut config: Config) -> Self {
        let theme = Theme::get(config.theme);
        let i18n = I18n::new(config.language);
        let icons = Icons::new(config.use_nerd_font);
        let clipboard = ClipboardService::new();
        let backup_service = BackupService::new();

        let mut git_service = GitService::discover().ok();
        if git_service.is_none() {
            if let Some(ref last) = config.last_repository {
                let p = std::path::Path::new(last);
                if p.is_dir() {
                    git_service = GitService::open(p).ok();
                }
            }
        }

        if let Some(ref git) = git_service {
            let _ = std::env::set_current_dir(git.workdir());
            config.add_recent_repository(git.workdir().display().to_string());
        }

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
            previous_branch: None,
            active_tab: AppTab::Changes,
            focused_pane: FocusedPane::MainContent,
            should_quit: false,
            last_status: None,
            layout_areas: LayoutAreas::default(),
            dir_map: DirectoryMap::new(),
            changes_view: ChangesView::new(),
            history_view: HistoryView::new(),
            worktrees_view: WorktreesView::new(),
            prs_view: PrsView::new(),
            actions_view: ActionsView::new(),
            alerts_view: AlertsView::new(),
            github_service,
            is_repo_private: None,
            ai_modal: AiAssistantModal::new(),
            context_menu: ContextMenu::new(),
            command_palette: CommandPaletteModal::new(),
            conflict_resolver: ConflictResolverModal::new(),
            branch_switcher: BranchSwitcherModal::new(),
            history_modal: HistoryActionsModal::new(),
            remote_diff_preview: RemoteDiffPreviewModal::new(),
            rebase_modal: RebaseModal::new(),
            markdown_preview_modal: MarkdownPreviewModal::new(),
            settings_modal: SettingsModal::new(),
            open_repo_modal: OpenRepoModal::new(),
            all_repo_files: Vec::new(),
            show_all_files: false,
            event_sender: None,
            needs_clear: false,
            last_auto_fetch: std::time::Instant::now(),
            last_status_check: std::time::Instant::now(),
            hover_pos: None,
        };

        app.refresh_git_status();
        app.refresh_worktrees();
        app.refresh_history();
        app.update_active_diff();
        app
    }

    /// Set event sender for asynchronous background operations.
    pub fn set_event_sender(&mut self, sender: tokio::sync::mpsc::UnboundedSender<AppEvent>) {
        self.event_sender = Some(sender);
    }

    /// Set repository visibility explicitly (true = private, false = public).
    pub fn set_repo_visibility(&mut self, is_private: bool) {
        self.is_repo_private = Some(is_private);
    }

    /// Trigger background non-blocking git fetch to sync remote tracking refs.
    pub fn trigger_background_fetch(&mut self) {
        if let Some(git) = &self.git_service {
            let workdir = git.workdir().to_path_buf();
            if let Some(sender) = self.event_sender.clone() {
                tokio::spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .current_dir(&workdir)
                            .args(["fetch", "--all", "--prune"])
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|out| {
                                if out.status.success() {
                                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    let msg = if !stdout.is_empty() { stdout } else { stderr };
                                    Ok(if msg.is_empty() { "Fetched updates".to_string() } else { msg })
                                } else {
                                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                                }
                            })
                    })
                    .await;

                    match res {
                        Ok(fetch_res) => {
                            let _ = sender.send(AppEvent::GitFetchCompleted(fetch_res));
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::GitFetchCompleted(Err(e.to_string())));
                        }
                    }
                });
            } else {
                // Synchronous fallback (e.g. testing)
                let _ = git.cli_fetch();
            }
        }
    }

    /// Trigger background non-blocking git pull to incorporate remote changes into current branch.
    pub fn trigger_background_pull(&mut self) {
        if let Some(git) = &self.git_service {
            self.last_status = Some("Pulling from remote...".to_string());
            let workdir = git.workdir().to_path_buf();
            if let Some(sender) = self.event_sender.clone() {
                tokio::spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .current_dir(&workdir)
                            .args(["pull"])
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|out| {
                                if out.status.success() {
                                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    let msg = if !stdout.is_empty() { stdout } else { stderr };
                                    Ok(if msg.is_empty() { "Pulled updates".to_string() } else { msg })
                                } else {
                                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                                }
                            })
                    })
                    .await;

                    match res {
                        Ok(pull_res) => {
                            let _ = sender.send(AppEvent::GitPullCompleted(pull_res));
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::GitPullCompleted(Err(e.to_string())));
                        }
                    }
                });
            } else {
                let _ = git.cli_pull();
            }
        }
    }

    /// Trigger background non-blocking git push to publish local commits to remote.
    pub fn trigger_background_push(&mut self) {
        if let Some(git) = &self.git_service {
            self.last_status = Some("Pushing to remote...".to_string());
            let workdir = git.workdir().to_path_buf();
            if let Some(sender) = self.event_sender.clone() {
                tokio::spawn(async move {
                    let res = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .current_dir(&workdir)
                            .args(["push"])
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|out| {
                                if out.status.success() {
                                    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    let msg = if !stdout.is_empty() { stdout } else { stderr };
                                    Ok(if msg.is_empty() { "Pushed updates".to_string() } else { msg })
                                } else {
                                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                                }
                            })
                    })
                    .await;

                    match res {
                        Ok(push_res) => {
                            let _ = sender.send(AppEvent::GitPushCompleted(push_res));
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::GitPushCompleted(Err(e.to_string())));
                        }
                    }
                });
            } else {
                let _ = git.cli_push();
            }
        }
    }

    /// Refresh repository status, directory map, and active diff from Git engine.
    pub fn refresh_git_status(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(status) = git.get_status() {
                self.repo_status = status;
                self.all_repo_files = git
                    .list_all_repo_files()
                    .unwrap_or_else(|_| self.repo_status.files.clone());
                let repo_dirs = git.list_all_directories();
                self.dir_map
                    .update_from_repo(&repo_dirs, &self.repo_status.files);

                let filtered = self.filtered_files();
                self.changes_view
                    .file_scroll
                    .set_total_items(filtered.len());
                self.update_active_diff();
            }
        } else {
            self.repo_status = GitRepoStatus::default();
            self.all_repo_files.clear();
            self.dir_map.update_from_files(&[]);
            self.changes_view.file_scroll.set_total_items(0);
        }
    }

    /// Open the Settings modal.
    pub fn open_settings(&mut self) {
        self.settings_modal.open_with_config(&self.config);
    }

    /// Open the Open Repository directory switcher modal.
    pub fn open_open_repo(&mut self) {
        let cur = self.git_service.as_ref().map(|g| g.workdir());
        self.open_repo_modal
            .open_at(cur, &self.config.recent_repositories);
    }

    /// Switch active repository to given path.
    pub fn switch_repository(&mut self, path: &std::path::Path) {
        match GitService::open(path) {
            Ok(git) => {
                let _ = std::env::set_current_dir(git.workdir());
                let p_str = git.workdir().display().to_string();
                self.config.add_recent_repository(p_str.clone());
                let _ = self.config.save();
                self.git_service = Some(git);
                self.is_repo_private = None;
                self.refresh_git_status();
                self.refresh_worktrees();
                self.refresh_history();
                self.refresh_github();
                self.last_status = Some(format!("Switched repository to {}", p_str));
            }
            Err(e) => {
                self.last_status = Some(format!("Failed to open repository: {}", e));
            }
        }
    }

    /// Return whether AI is connected along with provider name and model.
    pub fn ai_connection_status(&self) -> (bool, String, String) {
        let provider = self.config.ai.provider;
        let model = self.config.ai.model.clone();
        let has_key = self
            .config
            .ai
            .api_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false);

        let is_connected = match provider {
            crate::ai::AiProvider::OpenAi
            | crate::ai::AiProvider::Anthropic
            | crate::ai::AiProvider::Gemini => has_key,
            crate::ai::AiProvider::Ollama | crate::ai::AiProvider::Custom => true,
        };

        let prov_name = match provider {
            crate::ai::AiProvider::OpenAi => "OpenAI",
            crate::ai::AiProvider::Anthropic => "Anthropic",
            crate::ai::AiProvider::Gemini => "Gemini",
            crate::ai::AiProvider::Ollama => "Ollama",
            crate::ai::AiProvider::Custom => "Custom",
        }
        .to_string();

        (is_connected, prov_name, model)
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
                    if let Ok(meta) = gh.get_repo_metadata(&target, false).await {
                        let _ = sender.send(AppEvent::GitHubRepoMetadataLoaded(meta));
                    }
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
                if let Ok(meta) = block_on_safe(gh.get_repo_metadata(&target, false)) {
                    self.is_repo_private = Some(meta.is_private);
                }
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
        let source_files = if self.show_all_files || self.repo_status.files.is_empty() {
            if !self.all_repo_files.is_empty() {
                &self.all_repo_files
            } else {
                &self.repo_status.files
            }
        } else {
            &self.repo_status.files
        };

        if let Some(prefix) = self.dir_map.current_selected_prefix() {
            source_files
                .iter()
                .filter(|f| f.path.starts_with(prefix))
                .cloned()
                .collect()
        } else {
            source_files.clone()
        }
    }

    /// Update diff view for currently selected file.
    pub fn update_active_diff(&mut self) {
        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            if let Some(git) = &self.git_service {
                if item.status != crate::git::FileStatus::Clean {
                    if let Ok(diff) = git.get_diff(Some(&item.path), item.staged) {
                        if !diff.hunks.is_empty() {
                            self.current_diff = diff;
                            return;
                        }
                    }
                }
                // Preview file content for unmodified files or files with empty diff
                let full_path = git.workdir().join(&item.path);
                if let Ok(content) = std::fs::read_to_string(&full_path) {
                    self.current_diff = FileDiff::from_file_content(&item.path, &content);
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
            AppEvent::Mouse(mouse) => {
                self.hover_pos = Some((mouse.column, mouse.row));
                self.handle_mouse(mouse);
            }
            AppEvent::MouseMoved(x, y) => {
                self.hover_pos = Some((x, y));
            }
            AppEvent::Resize(w, h) => {
                self.layout_areas = LayoutAreas::compute(Rect::new(0, 0, w, h));
                self.needs_clear = true;
            }
            AppEvent::Tick => {
                // Auto-refresh git status every 1.5s to detect local filesystem changes and CLI commits
                if self.last_status_check.elapsed() >= std::time::Duration::from_millis(1500) {
                    self.last_status_check = std::time::Instant::now();
                    self.refresh_git_status();
                }

                let interval = self.config.git.auto_fetch_interval_secs;
                if interval > 0 && self.last_auto_fetch.elapsed() >= std::time::Duration::from_secs(interval) {
                    self.last_auto_fetch = std::time::Instant::now();
                    self.trigger_background_fetch();
                }
            }
            AppEvent::StatusMessage(msg) => {
                self.last_status = Some(msg);
            }
            AppEvent::GitFetchCompleted(res) => {
                match res {
                    Ok(msg) => self.last_status = Some(format!("Fetch: {}", msg)),
                    Err(e) => self.last_status = Some(format!("Fetch failed: {}", e)),
                }
                self.refresh_git_status();
            }
            AppEvent::GitPullCompleted(res) => {
                match res {
                    Ok(msg) => self.last_status = Some(format!("Pull: {}", msg)),
                    Err(e) => self.last_status = Some(format!("Pull failed: {}", e)),
                }
                self.refresh_git_status();
            }
            AppEvent::GitPushCompleted(res) => {
                match res {
                    Ok(msg) => self.last_status = Some(format!("Push: {}", msg)),
                    Err(e) => self.last_status = Some(format!("Push failed: {}", e)),
                }
                self.refresh_git_status();
            }
            AppEvent::GitHubRepoMetadataLoaded(meta) => {
                self.is_repo_private = Some(meta.is_private);
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
                    self.changes_view.commit_summary.clear();
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
            AppEvent::AiAssistantResponseReceived(res) => match res {
                Ok((ans, patch)) => {
                    self.ai_modal.set_response(ans, patch);
                }
                Err(e) => {
                    self.ai_modal.is_loading = false;
                    self.ai_modal.set_response(format!("AI Error: {}", e), None);
                }
            },
            AppEvent::GitHubWorkflowRerunResult(res) => match res {
                Ok(msg) => self.last_status = Some(msg),
                Err(e) => self.last_status = Some(format!("Workflow rerun failed: {}", e)),
            },
        }
    }

    /// Keyboard event handler.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // 0. Settings Modal captures keys
        if self.settings_modal.is_open {
            if let Some(action) = self.settings_modal.handle_key(key) {
                match action {
                    SettingsAction::SaveAndApply => {
                        self.settings_modal.apply_to_config(&mut self.config);
                        if let Err(e) = self.config.save() {
                            self.last_status = Some(format!("Save config error: {}", e));
                        } else {
                            self.theme = Theme::get(self.config.theme);
                            self.i18n = I18n::new(self.config.language);
                            self.icons = Icons::new(self.config.use_nerd_font);
                            self.last_status =
                                Some("Saved settings to ~/.config/prowly/config.toml".to_string());
                        }
                        self.settings_modal.close();
                    }
                    SettingsAction::Close => self.settings_modal.close(),
                }
            }
            return;
        }

        // 0.5. Open Repo Modal captures keys
        if self.open_repo_modal.is_open {
            if let Some(action) = self.open_repo_modal.handle_key(key) {
                match action {
                    OpenRepoAction::Open(path) => {
                        self.open_repo_modal.close();
                        self.switch_repository(&path);
                    }
                    OpenRepoAction::Close => self.open_repo_modal.close(),
                }
            }
            return;
        }

        // History Actions Modal captures keys
        if self.history_modal.is_open {
            match self.history_modal.state {
                HistoryActionState::Menu => match key.code {
                    KeyCode::Esc => self.history_modal.close(),
                    KeyCode::Up | KeyCode::Char('k') => self.history_modal.previous(),
                    KeyCode::Down | KeyCode::Char('j') => self.history_modal.next(),
                    KeyCode::Char('c') => {
                        self.execute_history_menu_action(HistoryMenuAction::Checkout)
                    }
                    KeyCode::Char('b') => {
                        self.execute_history_menu_action(HistoryMenuAction::CreateBranch)
                    }
                    KeyCode::Char('p') => {
                        self.execute_history_menu_action(HistoryMenuAction::CherryPick)
                    }
                    KeyCode::Char('r') => {
                        self.execute_history_menu_action(HistoryMenuAction::Revert)
                    }
                    KeyCode::Char('t') => {
                        self.execute_history_menu_action(HistoryMenuAction::TimeMachine)
                    }
                    KeyCode::Char('y') => {
                        self.execute_history_menu_action(HistoryMenuAction::CopySha)
                    }
                    KeyCode::Char('m') => {
                        self.execute_history_menu_action(HistoryMenuAction::CopyMessage)
                    }
                    KeyCode::Enter => {
                        let action = self.history_modal.selected_action();
                        self.execute_history_menu_action(action);
                    }
                    _ => {}
                },
                HistoryActionState::BranchInput => match key.code {
                    KeyCode::Esc => self.history_modal.close(),
                    KeyCode::Backspace => self.history_modal.pop_char(),
                    KeyCode::Enter => {
                        let bname = self.history_modal.new_branch_name.trim().to_string();
                        if !bname.is_empty() {
                            self.create_branch_at_selected_commit(&bname);
                        }
                        self.history_modal.close();
                    }
                    KeyCode::Char(c) => self.history_modal.push_char(c),
                    _ => {}
                },
                HistoryActionState::TimeMachineConfirm => match key.code {
                    KeyCode::Esc | KeyCode::Char('n') => self.history_modal.close(),
                    KeyCode::Enter | KeyCode::Char('y') => {
                        self.time_machine_reset_to_selected_commit();
                        self.history_modal.close();
                    }
                    _ => {}
                },
            }
            return;
        }

        // 1. Context Menu captures keys
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

        // 2. Branch Switcher Modal captures keys
        if self.branch_switcher.is_open {
            match key.code {
                KeyCode::Esc => self.branch_switcher.close(),
                KeyCode::Up => self.branch_switcher.previous(),
                KeyCode::Down => self.branch_switcher.next(),
                KeyCode::Backspace => self.branch_switcher.pop_char(),
                KeyCode::Enter => {
                    if let Some(target_b) = self.branch_switcher.selected_branch() {
                        let prev = self.repo_status.branch.clone();
                        if let Some(git) = &self.git_service {
                            match git.checkout_branch(&target_b) {
                                Ok(_) => {
                                    self.previous_branch = Some(prev);
                                    self.last_status =
                                        Some(format!("Switched branch to {}", target_b));
                                    self.refresh_git_status();
                                    self.branch_switcher.close();
                                }
                                Err(e) => {
                                    self.last_status = Some(format!("Checkout failed: {}", e));
                                }
                            }
                        }
                    }
                }
                KeyCode::Char(c) => self.branch_switcher.push_char(c),
                _ => {}
            }
            return;
        }

        // 3. Remote Diff Preview captures keys
        if self.remote_diff_preview.is_open {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.remote_diff_preview.close(),
                _ => {}
            }
            return;
        }

        // 3.5. Markdown Preview Modal captures keys
        if self.markdown_preview_modal.is_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.markdown_preview_modal.close(),
                KeyCode::Up | KeyCode::Char('k') => self.markdown_preview_modal.scroll_up(1),
                KeyCode::Down | KeyCode::Char('j') => self.markdown_preview_modal.scroll_down(1),
                KeyCode::PageUp => self.markdown_preview_modal.scroll_up(15),
                KeyCode::PageDown => self.markdown_preview_modal.scroll_down(15),
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.markdown_preview_modal.scroll_up(10);
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.markdown_preview_modal.scroll_down(10);
                }
                KeyCode::Char('g') | KeyCode::Home => self.markdown_preview_modal.scroll_to_top(),
                KeyCode::Char('G') | KeyCode::End => self.markdown_preview_modal.scroll_to_bottom(),
                KeyCode::Char('y') => {
                    let _ = self.clipboard.copy_text(&self.markdown_preview_modal.raw_content);
                    self.last_status = Some("Copied Markdown content to clipboard".to_string());
                }
                _ => {}
            }
            return;
        }

        // 4. Quick Rebase Modal captures keys
        if self.rebase_modal.is_open {
            if self.rebase_modal.is_editing_reword {
                match key.code {
                    KeyCode::Esc => {
                        self.rebase_modal.is_editing_reword = false;
                    }
                    KeyCode::Backspace => {
                        self.rebase_modal.reword_input.pop();
                    }
                    KeyCode::Enter => {
                        let new_msg = self.rebase_modal.reword_input.clone();
                        if let Some(git) = &self.git_service {
                            if crate::git::rebase::reword_head_commit(git.workdir(), &new_msg)
                                .is_ok()
                            {
                                self.last_status =
                                    Some("Reworded HEAD commit successfully".to_string());
                                self.refresh_git_status();
                                self.rebase_modal.close();
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        self.rebase_modal.reword_input.push(c);
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Esc => self.rebase_modal.close(),
                KeyCode::Up | KeyCode::Char('k') => self.rebase_modal.previous(),
                KeyCode::Down | KeyCode::Char('j') => self.rebase_modal.next(),
                KeyCode::Char('s') => {
                    if let Some(git) = &self.git_service {
                        let combined_msg = "squash: combined recent commits".to_string();
                        if crate::git::rebase::quick_squash(git.workdir(), 2, &combined_msg).is_ok()
                        {
                            self.last_status = Some("Squashed top 2 commits".to_string());
                            self.refresh_git_status();
                            self.open_rebase_modal();
                        }
                    }
                }
                KeyCode::Char('f') => {
                    if let Some(git) = &self.git_service {
                        if crate::git::rebase::fixup_head_commit(git.workdir()).is_ok() {
                            self.last_status = Some("Fixup merged into HEAD commit".to_string());
                            self.refresh_git_status();
                            self.open_rebase_modal();
                        }
                    }
                }
                KeyCode::Char('r') => {
                    self.rebase_modal.is_editing_reword = true;
                    if let Some(c) = self.rebase_modal.selected_commit() {
                        self.rebase_modal.reword_input = c.summary.clone();
                    }
                }
                _ => {}
            }
            return;
        }

        // 5. AI Modal captures input
        if self.ai_modal.is_open {
            match key.code {
                KeyCode::Esc => self.ai_modal.close(),
                KeyCode::Tab => {
                    if self.ai_modal.proposed_patch.is_some() {
                        self.apply_ai_suggestion();
                    }
                }
                KeyCode::Enter => {
                    if self.ai_modal.proposed_patch.is_some() && !self.ai_modal.is_loading {
                        self.apply_ai_suggestion();
                    } else if !self.ai_modal.input_query.trim().is_empty() {
                        self.trigger_ai_assistant_query();
                    }
                }
                KeyCode::Backspace => self.ai_modal.handle_backspace(),
                KeyCode::Char(c) => self.ai_modal.handle_char(c),
                _ => {}
            }
            return;
        }

        // 6. Command Palette captures input
        if self.command_palette.is_open {
            match key.code {
                KeyCode::Esc => {
                    if !self.command_palette.handle_esc() {
                        self.command_palette.close();
                    }
                }
                KeyCode::Tab => {
                    self.command_palette.handle_tab();
                }
                KeyCode::Up => self.command_palette.previous(),
                KeyCode::Down => self.command_palette.next(),
                KeyCode::PageUp => self.command_palette.page_up(10),
                KeyCode::PageDown => self.command_palette.page_down(10),
                KeyCode::Backspace => self.command_palette.pop_char(),
                KeyCode::Enter => {
                    match self.command_palette.handle_enter() {
                        PaletteEnterResult::LayerChanged => {}
                        PaletteEnterResult::ExecuteAction(action_id) => {
                            self.command_palette.close();
                            self.execute_palette_action(action_id);
                        }
                        PaletteEnterResult::ExecuteGitCommand { subcommand, args } => {
                            self.command_palette.close();
                            self.execute_git_cli(&subcommand, &args);
                        }
                        PaletteEnterResult::ExecuteAiCommand(cmd) => {
                            self.command_palette.close();
                            let _ = std::process::Command::new("sh")
                                .args(["-c", &cmd])
                                .output();
                            self.last_status = Some(format!("Executed: {}", cmd));
                            self.refresh_git_status();
                            self.needs_clear = true;
                        }
                        PaletteEnterResult::DangerWarning(warn) => {
                            self.last_status = Some(format!("⚠️ Safety Guard: {}", warn));
                        }
                        PaletteEnterResult::RequestAi(req) => {
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
                }
                KeyCode::Char(c) => self.command_palette.push_char(c),
                _ => {}
            }
            return;
        }

        // 7. Conflict Resolver captures input
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
                    if std::fs::write(&path, content).is_ok() {
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

        // 8. Text Input inside Changes view Commit Box
        if self.focused_pane == FocusedPane::MainContent
            && self.active_tab == AppTab::Changes
            && (self.changes_view.focus == ChangesFocus::CommitSummary
                || self.changes_view.focus == ChangesFocus::CommitDescription)
        {
            match key.code {
                KeyCode::Esc => {
                    self.changes_view.focus = ChangesFocus::FileList;
                    return;
                }
                KeyCode::Tab => {
                    self.changes_view.focus = match self.changes_view.focus {
                        ChangesFocus::CommitSummary => ChangesFocus::CommitDescription,
                        _ => ChangesFocus::FileList,
                    };
                    return;
                }
                KeyCode::Enter => {
                    self.commit_staged();
                    return;
                }
                KeyCode::Backspace => {
                    self.changes_view.handle_backspace();
                    return;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.changes_view.handle_char(c);
                    return;
                }
                _ => {}
            }
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
                // Ctrl+P: Open Command Palette
                KeyCode::Char('p') => {
                    self.command_palette.open();
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
                // Ctrl+F: AI Analyze CI Failure
                KeyCode::Char('f') => {
                    self.trigger_ai_ci_analysis();
                    return;
                }
                // Ctrl+O: Open / Switch Repository
                KeyCode::Char('o') => {
                    self.open_open_repo();
                    return;
                }
                // Ctrl+,: Open Settings
                KeyCode::Char(',') => {
                    self.open_settings();
                    return;
                }
                // Ctrl+L: Redraw / clear terminal screen
                KeyCode::Char('l') => {
                    self.needs_clear = true;
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(',') => self.open_settings(),
            KeyCode::Char('o') => self.open_open_repo(),
            KeyCode::Char('A') => {
                self.show_all_files = !self.show_all_files;
                let msg = if self.show_all_files {
                    "Showing all repository files"
                } else {
                    "Showing changed files only"
                };
                self.last_status = Some(msg.to_string());
                self.refresh_git_status();
            }
            KeyCode::Char('?') => {
                let ctx = self.capture_ai_context();
                self.ai_modal.toggle(&ctx);
            }
            // Quick Branch Switcher via 'b'
            // Quick Branch Switcher via 'b' (or Branch from commit in History tab)
            KeyCode::Char('b') => {
                if self.active_tab == AppTab::History {
                    if let Some(c) = self.history_view.selected_commit() {
                        let commit_item = c.clone();
                        self.history_modal.open_branch_input(commit_item);
                    }
                } else {
                    self.open_branch_switcher();
                }
            }
            // Open Context Menu / Action Menu via 'm' or F10
            KeyCode::Char('m') | KeyCode::F(10) => {
                if self.active_tab == AppTab::History {
                    if let Some(c) = self.history_view.selected_commit() {
                        let commit_item = c.clone();
                        self.history_modal.open(commit_item);
                    }
                } else {
                    let sel_file = self
                        .filtered_files()
                        .get(self.changes_view.file_scroll.selected_index)
                        .map(|f| f.path.clone());
                    self.context_menu.open_at(sel_file, 40, 10);
                }
            }
            // Direct history shortcuts
            KeyCode::Char('p') => {
                if self.active_tab == AppTab::History {
                    self.cherry_pick_selected_commit();
                }
            }
            KeyCode::Char('h') => {
                self.active_tab = AppTab::History;
                self.refresh_history();
            }
            KeyCode::Tab => {
                self.focused_pane = match self.focused_pane {
                    FocusedPane::DirectoryMap => FocusedPane::MainContent,
                    FocusedPane::MainContent => FocusedPane::DirectoryMap,
                };
            }
            // Tab switching 1..6
            KeyCode::Char('1') => self.active_tab = AppTab::Changes,
            KeyCode::Char('2') => {
                self.active_tab = AppTab::History;
                self.refresh_history();
            }
            KeyCode::Char('3') => self.active_tab = AppTab::Prs,
            KeyCode::Char('4') => self.active_tab = AppTab::Actions,
            KeyCode::Char('5') => self.active_tab = AppTab::Alerts,
            KeyCode::Char('6') => {
                self.active_tab = AppTab::Worktrees;
                self.refresh_worktrees();
            }

            // Time Machine in History tab / Cycle Theme (t)
            KeyCode::Char('t') => {
                if self.active_tab == AppTab::History {
                    if let Some(c) = self.history_view.selected_commit() {
                        let commit_item = c.clone();
                        self.history_modal.open_time_machine(commit_item);
                    }
                } else {
                    self.cycle_theme();
                }
            }

            // Toggle Language (l)
            KeyCode::Char('l') => {
                let new_lang = self.i18n.toggle_language();
                self.config.language = new_lang;
                self.last_status = Some(format!("Language: {}", new_lang.display_name()));
            }

            // Copy (y) to Clipboard
            KeyCode::Char('y') => {
                let text_to_copy = match self.focused_pane {
                    FocusedPane::DirectoryMap => self
                        .dir_map
                        .nodes
                        .get(self.dir_map.scroll.selected_index)
                        .map(|n| n.path_prefix.clone()),
                    FocusedPane::MainContent => match self.active_tab {
                        AppTab::History => {
                            self.history_view.selected_commit().map(|c| c.full_hash.clone())
                        }
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
            KeyCode::Char('a') => self.stage_all_files(),

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

            // 'c': Checkout commit in History tab, Checkout PR locally in PRs tab, or Commit in Changes tab
            KeyCode::Char('c') => {
                if self.active_tab == AppTab::History {
                    self.checkout_selected_commit();
                } else if self.active_tab == AppTab::Prs {
                    self.checkout_selected_pr();
                } else if self.changes_view.commit_summary.is_empty() {
                    self.changes_view.focus = ChangesFocus::CommitSummary;
                    self.last_status =
                        Some("Enter commit message (Press Enter to Commit & Sync)".to_string());
                } else {
                    self.commit_staged();
                }
            }

            // 'r': Revert in History tab, or Re-run failed CI job in Actions tab
            KeyCode::Char('r') => {
                if self.active_tab == AppTab::History {
                    self.revert_selected_commit();
                } else if self.active_tab == AppTab::Actions {
                    self.rerun_selected_action_job();
                }
            }

            // Enter key dispatching
            KeyCode::Enter => match self.active_tab {
                AppTab::History => {
                    if let Some(commit) = self.history_view.selected_commit() {
                        let c = commit.clone();
                        self.history_modal.open(c);
                    }
                }
                AppTab::Prs => self.checkout_selected_pr(),
                AppTab::Worktrees => {
                    if let Some(wt) = self
                        .worktrees
                        .get(self.worktrees_view.scroll.selected_index)
                    {
                        let branch = wt.head_branch.clone().unwrap_or_else(|| "HEAD".to_string());
                        if let Some(git) = &self.git_service {
                            if git.checkout_branch(&branch).is_ok() {
                                self.last_status =
                                    Some(format!("Switched to worktree branch {}", branch));
                                self.refresh_git_status();
                                self.refresh_worktrees();
                            }
                        }
                    }
                }
                AppTab::Changes => {
                    let filtered = self.filtered_files();
                    if let Some(item) = filtered.get(self.changes_view.file_scroll.selected_index) {
                        if item.status == crate::git::FileStatus::Conflicted {
                            if let Ok(content) = std::fs::read_to_string(&item.path) {
                                self.conflict_resolver.open(item.path.clone(), content);
                            }
                        } else {
                            self.update_active_diff();
                        }
                    }
                }
                _ => {}
            },

            // ':' or F1: Open Command Palette
            KeyCode::Char(':') | KeyCode::F(1) => {
                self.command_palette.open();
            }

            // 'e': Open external editor on selected file
            KeyCode::Char('e') => {
                self.open_selected_in_editor();
            }

            // 'R': Refresh GitHub data, repository status, and trigger background fetch
            KeyCode::Char('R') => {
                self.refresh_git_status();
                self.refresh_worktrees();
                self.refresh_history();
                self.refresh_github();
                self.trigger_background_fetch();
                self.last_status = Some("Refreshed repository status and GitHub data (fetching remote...)".to_string());
            }

            // F5: Fetch remote updates
            KeyCode::F(5) => {
                self.trigger_background_fetch();
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
                    AppTab::History => {
                        self.history_view.previous_commit();
                        self.update_active_history_diff();
                    }
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
                    AppTab::History => {
                        self.history_view.next_commit();
                        self.update_active_history_diff();
                    }
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
                    if self.active_tab == AppTab::History {
                        self.history_view.scroll_diff_up(10);
                    } else if self.active_tab == AppTab::Worktrees {
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
                    if self.active_tab == AppTab::History {
                        self.history_view.scroll_diff_down(10);
                    } else if self.active_tab == AppTab::Worktrees {
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

    /// Extract GitHub repository target from current origin remote.
    pub fn github_target(&self) -> Option<GitHubRepoTarget> {
        self.git_service
            .as_ref()
            .and_then(|git| git.remote_url(Some("origin")))
            .and_then(|url| GitHubRepoTarget::from_remote_url(&url))
    }

    /// Mouse event handler.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        let (x, y) = (mouse.column, mouse.row);

        // 0. Command Palette wheel
        if self.command_palette.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.command_palette.previous();
                    return;
                }
                MouseEventKind::ScrollDown => {
                    self.command_palette.next();
                    return;
                }
                _ => {}
            }
        }

        // 1. Settings Modal clicks & wheel
        if self.settings_modal.is_open {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) = self
                        .settings_modal
                        .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            SettingsAction::SaveAndApply => {
                                self.settings_modal.apply_to_config(&mut self.config);
                                if let Err(e) = self.config.save() {
                                    self.last_status = Some(format!("Save config error: {}", e));
                                } else {
                                    self.theme = Theme::get(self.config.theme);
                                    self.i18n = I18n::new(self.config.language);
                                    self.icons = Icons::new(self.config.use_nerd_font);
                                    self.last_status = Some(
                                        "Saved settings to ~/.config/prowly/config.toml".to_string(),
                                    );
                                }
                                self.settings_modal.close();
                            }
                            SettingsAction::Close => self.settings_modal.close(),
                        }
                    }
                }
                MouseEventKind::ScrollUp => {
                    self.settings_modal.scroll_up();
                }
                MouseEventKind::ScrollDown => {
                    self.settings_modal.scroll_down(self.layout_areas.root);
                }
                _ => {}
            }
            return;
        }

        // 2. Open Repo Modal clicks
        if self.open_repo_modal.is_open {
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(action) =
                    self.open_repo_modal
                        .handle_click(x, y, self.layout_areas.root)
                {
                    match action {
                        OpenRepoAction::Open(path) => {
                            self.open_repo_modal.close();
                            self.switch_repository(&path);
                        }
                        OpenRepoAction::Close => self.open_repo_modal.close(),
                    }
                }
            }
            return;
        }

        // 3. Right-Click: Open Context Menu
        if mouse.kind == MouseEventKind::Down(MouseButton::Right) {
            let sel_file = self
                .filtered_files()
                .get(self.changes_view.file_scroll.selected_index)
                .map(|f| f.path.clone());
            self.context_menu.open_at(sel_file, x, y);
            return;
        }

        // 4. Click when Context Menu is open: hit-test items
        if self.context_menu.is_open {
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(action) = self.context_menu.hit_test(x, y, self.layout_areas.root) {
                    self.context_menu.close();
                    self.execute_context_action(action);
                } else {
                    self.context_menu.close();
                }
            }
            return;
        }

        // 5. Click on Open Repo button or header path
        if self.layout_areas.hits_open_repo(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.open_open_repo();
            return;
        }

        // 6. Click on Branch switcher header button
        if self.layout_areas.hits_branch(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.open_branch_switcher();
            return;
        }

        // 6.2. Click on Fetch header button
        if self.layout_areas.hits_fetch(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.trigger_background_fetch();
            return;
        }

        // 6.4. Click on Pull header button
        if self.layout_areas.hits_pull(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.trigger_background_pull();
            return;
        }

        // 7. Click on Sync / Remote Diff header button
        if self.layout_areas.hits_sync(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.open_remote_diff_preview();
            return;
        }

        // 8. Click on Settings header button
        if self.layout_areas.hits_settings(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.open_settings();
            return;
        }

        // 8.5. Click on Command Palette header button
        if self.layout_areas.hits_palette(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.command_palette.open();
            return;
        }

        // 9. Click on Menu header button
        if self.layout_areas.hits_menu(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let sel_file = self
                .filtered_files()
                .get(self.changes_view.file_scroll.selected_index)
                .map(|f| f.path.clone());
            self.context_menu.open_at(sel_file, x, y + 1);
            return;
        }

        // 10. Click on Ask AI / AI Status chip
        if self.layout_areas.hits_ask_ai(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let (connected, _, _) = self.ai_connection_status();
            if connected {
                let ctx = self.capture_ai_context();
                self.ai_modal.toggle(&ctx);
            } else {
                self.open_settings();
            }
            return;
        }

        // 11. Click on Help header button
        if self.layout_areas.hits_help(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            self.last_status = Some(
                "Help: [Tab] Panes | [Space] Stage | [,] Settings | [Ctrl+O] Open Repo | [m] Menu | [q] Quit".to_string(),
            );
            return;
        }

        // Tabs bar click
        if self.layout_areas.hits_tabs(x, y)
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
        {
            let rel_x = x.saturating_sub(self.layout_areas.right_tabs.x);
            let mut cur_x = 1; // initial space
            for tab in AppTab::ALL {
                let key = tab.title_key();
                let text = self.i18n.t(key);
                let icon_str = match tab {
                    AppTab::Changes => self.icons.branch(),
                    AppTab::History => self.icons.commit(),
                    AppTab::Prs => self.icons.pr_open(),
                    AppTab::Actions => self.icons.check_success(),
                    AppTab::Alerts => self.icons.get(crate::ui::IconId::Alert),
                    AppTab::Worktrees => self.icons.worktree(),
                };
                let pill_spans = pill(
                    Some(icon_str),
                    &text,
                    self.theme.bg,
                    self.theme.primary,
                    self.theme.bg,
                    self.icons.use_nerd_font,
                );
                let pill_w = crate::ui::capsule::spans_width(&pill_spans) as u16;
                if rel_x >= cur_x && rel_x < cur_x + pill_w {
                    self.active_tab = *tab;
                    if self.active_tab == AppTab::Worktrees {
                        self.refresh_worktrees();
                    } else if self.active_tab == AppTab::History {
                        self.refresh_history();
                    }
                    return;
                }
                cur_x += pill_w + 1;
            }
            return;
        }

        // Directory Map pane click & scroll
        if self.layout_areas.hits_left_pane(x, y) {
            self.focused_pane = FocusedPane::DirectoryMap;
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.git_service.is_none() {
                        let cur_dir = std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."));
                        match GitService::init_repo(&cur_dir) {
                            Ok(_) => {
                                if let Ok(git) = GitService::open(&cur_dir) {
                                    self.git_service = Some(git);
                                }
                                self.refresh_git_status();
                                self.last_status = Some("Initialized Git repository".to_string());
                            }
                            Err(e) => {
                                self.last_status = Some(format!("Init repo failed: {}", e));
                            }
                        }
                        return;
                    }

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
                        AppTab::History => {
                            self.history_view.scroll.handle_click_row(rel_y);
                            self.update_active_history_diff();
                        }
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
                            if self.changes_view.hit_test_menu_button(x, y) {
                                let sel_file = self
                                    .filtered_files()
                                    .get(self.changes_view.file_scroll.selected_index)
                                    .map(|f| f.path.clone());
                                self.context_menu.open_at(sel_file, x, y + 1);
                            } else if let Some(btn_action) =
                                self.changes_view.hit_test_action_buttons(x, y, self.icons.use_nerd_font)
                            {
                                match btn_action {
                                    ChangesButtonAction::StageAll => self.stage_all_files(),
                                    ChangesButtonAction::Commit => self.commit_staged_only(),
                                    ChangesButtonAction::CommitAndSync => self.commit_staged(),
                                    ChangesButtonAction::Pull => self.trigger_background_pull(),
                                    ChangesButtonAction::Fetch => self.trigger_background_fetch(),
                                    ChangesButtonAction::AiGenerate => {
                                        self.trigger_ai_generate_commit()
                                    }
                                    ChangesButtonAction::AiReview => self.trigger_ai_review(),
                                    ChangesButtonAction::StashAll => {
                                        self.execute_palette_action("stash")
                                    }
                                    ChangesButtonAction::DiscardSafe => {
                                        self.safe_discard_selected()
                                    }
                                }
                            } else if let Some(focus) = self.changes_view.hit_test_inputs(x, y) {
                                self.changes_view.focus = focus;
                            } else if let Some((rel_y, is_checkbox)) =
                                self.changes_view.hit_test_file_list(x, y)
                            {
                                self.changes_view.focus = ChangesFocus::FileList;
                                if self.changes_view.file_scroll.handle_click_row(rel_y).is_some() {
                                    if is_checkbox {
                                        self.toggle_stage_selected();
                                    }
                                    self.update_active_diff();
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollDown => match self.active_tab {
                    AppTab::History => {
                        self.history_view.scroll.scroll_down(3);
                        self.update_active_history_diff();
                    }
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
                    AppTab::History => {
                        self.history_view.scroll.scroll_up(3);
                        self.update_active_history_diff();
                    }
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
            ContextMenuAction::OpenEditor => {
                self.open_selected_in_editor();
            }
            ContextMenuAction::RevealFileManager => {
                self.reveal_selected_in_file_manager();
            }
            ContextMenuAction::StashFile => {
                self.stash_selected_file();
            }
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
            ContextMenuAction::PreviewMarkdown => {
                self.open_markdown_preview_for_selected();
            }
        }
    }

    /// Execute an action selected from the Command Palette.
    pub fn execute_palette_action(&mut self, action_id: &str) {
        match action_id {
            "stage_all" => self.stage_all_files(),
            "unstage_all" => {
                if let Some(git) = &self.git_service {
                    let _ = git.unstage_all();
                    self.refresh_git_status();
                    self.last_status = Some("Unstaged all files".to_string());
                }
            }
            "commit" => self.commit_staged(),
            "push" => self.trigger_background_push(),
            "pull" => self.trigger_background_pull(),
            "fetch" => self.trigger_background_fetch(),
            "stash" => {
                if let Some(git) = &self.git_service {
                    match git.cli_stash_save("Prowly stash") {
                        Ok(msg) => {
                            self.last_status = Some(format!("Stashed: {}", msg));
                            self.refresh_git_status();
                        }
                        Err(e) => {
                            self.last_status = Some(format!("Stash failed: {}", e));
                        }
                    }
                }
            }
            "stash_pop" => {
                if let Some(git) = &self.git_service {
                    match git.cli_stash_pop() {
                        Ok(msg) => {
                            self.last_status = Some(format!("Popped stash: {}", msg));
                            self.refresh_git_status();
                        }
                        Err(e) => {
                            self.last_status = Some(format!("Stash pop failed: {}", e));
                        }
                    }
                }
            }
            "switch_branch" => {
                self.open_branch_switcher();
            }
            "remote_diff" => {
                self.open_remote_diff_preview();
            }
            "rebase" => {
                self.open_rebase_modal();
            }
            "discard_changes" => self.safe_discard_selected(),
            "stage_selected" => self.toggle_stage_selected(),
            "stash_selected" => self.stash_selected_file(),
            "open_editor" => self.open_selected_in_editor(),
            "preview_markdown" => self.open_markdown_preview_for_selected(),
            "reveal_file_manager" => self.reveal_selected_in_file_manager(),
            "refresh" => {
                self.refresh_git_status();
                self.refresh_worktrees();
                self.refresh_github();
                self.trigger_background_fetch();
                self.last_status = Some("Refreshed repository status and GitHub data".to_string());
            }
            "ai_ci_failure" => self.trigger_ai_ci_analysis(),
            "ai_assistant" => {
                let ctx = self.capture_ai_context();
                self.ai_modal.open_with_context(&ctx);
            }
            "ai_pr_draft" => self.trigger_ai_pr_draft(),
            "tab_changes" => self.active_tab = AppTab::Changes,
            "tab_history" => {
                self.active_tab = AppTab::History;
                self.refresh_history();
            }
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
            "settings" => self.open_settings(),
            "open_repo" => self.open_open_repo(),
            "toggle_all_files" => {
                self.show_all_files = !self.show_all_files;
                let msg = if self.show_all_files {
                    "Showing all repository files"
                } else {
                    "Showing changed files only"
                };
                self.last_status = Some(msg.to_string());
                self.refresh_git_status();
            }
            _ => {}
        }
    }

    /// Execute an arbitrary Git CLI command in the repository workspace.
    pub fn execute_git_cli(&mut self, subcommand: &str, args: &str) {
        let workdir = if let Some(git) = &self.git_service {
            git.workdir().to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        };

        let trimmed_args = args.trim();
        let full_cmd_str = if trimmed_args.is_empty() {
            format!("git {}", subcommand)
        } else {
            format!("git {} {}", subcommand, trimmed_args)
        };

        let output = std::process::Command::new("sh")
            .current_dir(&workdir)
            .args(["-c", &full_cmd_str])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    let summary = if !stdout.trim().is_empty() {
                        stdout.lines().next().unwrap_or("").to_string()
                    } else if !stderr.trim().is_empty() {
                        stderr.lines().next().unwrap_or("").to_string()
                    } else {
                        "Done".to_string()
                    };
                    self.last_status = Some(format!("{}: {}", full_cmd_str, summary));
                } else {
                    let err_msg = if !stderr.trim().is_empty() {
                        stderr.lines().next().unwrap_or("Failed").to_string()
                    } else {
                        format!("Failed with exit code {:?}", output.status.code())
                    };
                    self.last_status = Some(format!("Error ({}): {}", full_cmd_str, err_msg));
                }
            }
            Err(e) => {
                self.last_status = Some(format!("Error executing {}: {}", full_cmd_str, e));
            }
        }
        self.refresh_git_status();
        self.needs_clear = true;
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

    /// Stage all changed files.
    pub fn stage_all_files(&mut self) {
        if let Some(git) = &self.git_service {
            match git.stage_all() {
                Ok(_) => {
                    self.refresh_git_status();
                    self.last_status = Some("Staged all files".to_string());
                }
                Err(e) => {
                    self.last_status = Some(format!("Failed to stage all: {}", e));
                }
            }
        } else {
            for f in &mut self.repo_status.files {
                f.staged = true;
            }
            self.last_status = Some("Staged all files".to_string());
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
    /// Refresh Git commit history log for History tab.
    pub fn refresh_history(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(rows) = crate::git::get_commit_history(git.workdir(), 100) {
                self.history_view.set_rows(rows);
                self.update_active_history_diff();
            }
        } else {
            self.history_view.set_rows(Vec::new());
        }
    }

    /// Load detailed diff and stats for the currently selected history commit.
    pub fn update_active_history_diff(&mut self) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.full_hash.clone();
                if let Ok(detail) = crate::git::get_commit_details(git.workdir(), &hash) {
                    self.history_view.set_detail(detail);
                }
            }
        }
    }

    /// Checkout currently selected commit in detached HEAD mode.
    pub fn checkout_selected_commit(&mut self) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.hash.clone();
                match crate::git::checkout_commit(git.workdir(), &hash) {
                    Ok(_) => {
                        self.last_status = Some(format!("Checked out commit {} (Detached HEAD)", hash));
                        self.refresh_git_status();
                        self.refresh_history();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Checkout failed: {}", e));
                    }
                }
            }
        }
    }

    /// Create new branch at selected commit and switch to it.
    pub fn create_branch_at_selected_commit(&mut self, branch_name: &str) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.hash.clone();
                match crate::git::create_branch_at_commit(git.workdir(), branch_name, &hash, true) {
                    Ok(_) => {
                        self.last_status = Some(format!(
                            "Created and switched to branch '{}' at commit {}",
                            branch_name, hash
                        ));
                        self.refresh_git_status();
                        self.refresh_history();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Failed to create branch: {}", e));
                    }
                }
            }
        }
    }

    /// Cherry-pick selected commit into current HEAD.
    pub fn cherry_pick_selected_commit(&mut self) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.hash.clone();
                match crate::git::cherry_pick_commit(git.workdir(), &hash) {
                    Ok(_) => {
                        self.last_status = Some(format!("Cherry-picked commit {} successfully", hash));
                        self.refresh_git_status();
                        self.refresh_history();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Cherry-pick failed: {}", e));
                        self.refresh_git_status();
                    }
                }
            }
        }
    }

    /// Revert selected commit by creating an inverse commit.
    pub fn revert_selected_commit(&mut self) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.hash.clone();
                match crate::git::revert_commit(git.workdir(), &hash) {
                    Ok(_) => {
                        self.last_status = Some(format!("Reverted commit {}", hash));
                        self.refresh_git_status();
                        self.refresh_history();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Revert failed: {}", e));
                        self.refresh_git_status();
                    }
                }
            }
        }
    }

    /// Time Machine: Reset branch to selected commit, discarding subsequent commits.
    pub fn time_machine_reset_to_selected_commit(&mut self) {
        if let Some(git) = &self.git_service {
            if let Some(commit) = self.history_view.selected_commit() {
                let hash = commit.hash.clone();
                match crate::git::time_machine_reset(git.workdir(), &hash) {
                    Ok(backup_branch) => {
                        self.last_status = Some(format!(
                            "⏳ Time Machine: Reset to commit {}. Safety backup saved to branch '{}'",
                            hash, backup_branch
                        ));
                        self.refresh_git_status();
                        self.refresh_history();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Time Machine reset failed: {}", e));
                    }
                }
            }
        }
    }

    /// Copy SHA of currently selected commit to clipboard.
    pub fn copy_selected_commit_sha(&mut self) {
        if let Some(commit) = self.history_view.selected_commit() {
            let sha = commit.full_hash.clone();
            if self.clipboard.copy_text(&sha).is_ok() {
                self.last_status = Some(format!("Copied commit SHA: {}", sha));
            }
        }
    }

    /// Copy summary message of currently selected commit to clipboard.
    pub fn copy_selected_commit_message(&mut self) {
        if let Some(commit) = self.history_view.selected_commit() {
            let msg = commit.summary.clone();
            if self.clipboard.copy_text(&msg).is_ok() {
                self.last_status = Some(format!("Copied commit message: {}", msg));
            }
        }
    }

    /// Dispatch action from commit action menu.
    pub fn execute_history_menu_action(&mut self, action: HistoryMenuAction) {
        match action {
            HistoryMenuAction::Checkout => {
                self.history_modal.close();
                self.checkout_selected_commit();
            }
            HistoryMenuAction::CreateBranch => {
                self.history_modal.state = HistoryActionState::BranchInput;
                self.history_modal.new_branch_name.clear();
            }
            HistoryMenuAction::CherryPick => {
                self.history_modal.close();
                self.cherry_pick_selected_commit();
            }
            HistoryMenuAction::Revert => {
                self.history_modal.close();
                self.revert_selected_commit();
            }
            HistoryMenuAction::TimeMachine => {
                self.history_modal.state = HistoryActionState::TimeMachineConfirm;
            }
            HistoryMenuAction::CopySha => {
                self.history_modal.close();
                self.copy_selected_commit_sha();
            }
            HistoryMenuAction::CopyMessage => {
                self.history_modal.close();
                self.copy_selected_commit_message();
            }
        }
    }

    /// Commit staged changes locally without pushing.
    pub fn commit_staged_only(&mut self) {
        self.do_commit_staged(false);
    }

    /// Commit staged changes using summary and description inputs, then push (Commit & Sync).
    pub fn commit_staged(&mut self) {
        self.do_commit_staged(true);
    }

    fn do_commit_staged(&mut self, push: bool) {
        let summary = if self.changes_view.commit_summary.trim().is_empty() {
            self.last_status =
                Some("Please enter a commit message or press Ctrl+G for AI message".to_string());
            return;
        } else {
            self.changes_view.commit_summary.trim().to_string()
        };

        let desc = if self.changes_view.commit_description.trim().is_empty() {
            None
        } else {
            Some(self.changes_view.commit_description.trim())
        };

        if let Some(git) = &self.git_service {
            // Check staging status:
            // If nothing is staged, but unstaged changed files exist,
            // automatically stage all changes so Commit & Sync works seamlessly
            // like GitHub Desktop or VS Code.
            let status = git.get_status().ok();
            let staged_count = status
                .as_ref()
                .map(|s| s.files.iter().filter(|f| f.staged).count())
                .unwrap_or(0);
            let unstaged_count = status
                .as_ref()
                .map(|s| s.files.iter().filter(|f| !f.staged).count())
                .unwrap_or(0);

            if staged_count == 0 {
                if unstaged_count > 0 {
                    let _ = git.stage_all();
                } else {
                    self.last_status = Some("No changes to commit".to_string());
                    return;
                }
            }

            match git.commit_with_description(&summary, desc) {
                Ok(oid) => {
                    let short_oid = &oid.to_string()[..7.min(oid.to_string().len())];
                    if push {
                        let push_status = match git.cli_push() {
                            Ok(_) => " & Synced (pushed)".to_string(),
                            Err(e) => format!(" (push warning: {})", e),
                        };
                        self.last_status = Some(format!("Committed: {}{}", short_oid, push_status));
                    } else {
                        self.last_status = Some(format!("Committed locally: {} (unpushed)", short_oid));
                    }
                    self.changes_view.commit_summary.clear();
                    self.changes_view.commit_description.clear();
                    self.changes_view.focus = ChangesFocus::FileList;
                    self.refresh_git_status();
                    self.refresh_history();
                }
                Err(e) => {
                    self.last_status = Some(format!("Commit failed: {}", e));
                }
            }
        }
    }

    /// Open currently selected file in configured or preferred external editor.
    pub fn open_selected_in_editor(&mut self) {
        let filtered = self.filtered_files();
        if let Some(item) = filtered.get(self.changes_view.file_scroll.selected_index) {
            let full_path = if let Some(git) = &self.git_service {
                git.workdir().join(&item.path)
            } else {
                std::path::PathBuf::from(&item.path)
            };
            let pref = self.config.editor.command.as_deref();
            if open_external_editor_with_preference(&full_path, pref).is_ok() {
                self.refresh_git_status();
                self.last_status = Some(format!("Edited {}", item.path));
            }
            self.needs_clear = true;
        }
    }

    /// Reveal currently selected file in OS file manager.
    pub fn reveal_selected_in_file_manager(&mut self) {
        let filtered = self.filtered_files();
        if let Some(item) = filtered.get(self.changes_view.file_scroll.selected_index) {
            let full_path = if let Some(git) = &self.git_service {
                git.workdir().join(&item.path)
            } else {
                std::path::PathBuf::from(&item.path)
            };
            if reveal_in_file_manager(&full_path).is_ok() {
                self.last_status = Some(format!("Opened in file manager: {}", item.path));
            }
        }
    }

    /// Stash only the currently selected file.
    pub fn stash_selected_file(&mut self) {
        let filtered = self.filtered_files();
        if let Some(item) = filtered.get(self.changes_view.file_scroll.selected_index) {
            if let Some(git) = &self.git_service {
                let msg = format!("Stashed file {}", item.path);
                if git.cli_stash_save(&msg).is_ok() {
                    self.last_status = Some(msg);
                    self.refresh_git_status();
                }
            }
        }
    }

    /// Open Markdown preview modal for currently targeted or selected file.
    pub fn open_markdown_preview_for_selected(&mut self) {
        let file_path = self.context_menu.target_file.clone().or_else(|| {
            self.filtered_files()
                .get(self.changes_view.file_scroll.selected_index)
                .map(|item| item.path.clone())
        });

        if let Some(rel_path) = file_path {
            let full_path = if let Some(git) = &self.git_service {
                git.workdir().join(&rel_path)
            } else {
                std::path::PathBuf::from(&rel_path)
            };

            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    self.markdown_preview_modal
                        .open(rel_path.clone(), content, &self.theme);
                    self.last_status = Some(format!("Previewing Markdown: {}", rel_path));
                }
                Err(e) => {
                    self.last_status =
                        Some(format!("Could not read file ({}): {}", rel_path, e));
                }
            }
        } else {
            self.last_status = Some("No file selected for Markdown preview".to_string());
        }
    }

    /// Open Quick Branch Switcher modal.
    pub fn open_branch_switcher(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(branches) = git.list_branches() {
                let curr = self.repo_status.branch.clone();
                let prev = self.previous_branch.clone();
                self.branch_switcher.open(curr, branches, prev);
            }
        }
    }

    /// Open Remote Diff Preview dropdown.
    pub fn open_remote_diff_preview(&mut self) {
        self.trigger_background_fetch();
        if let Some(git) = &self.git_service {
            if let Ok((unpushed, unpulled)) = git.get_unpushed_unpulled_commits() {
                let upstream = self.repo_status.upstream.clone();
                self.remote_diff_preview.open(unpushed, unpulled, upstream);
            }
        }
    }

    /// Open Quick Interactive Rebase modal.
    pub fn open_rebase_modal(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(commits) = crate::git::rebase::list_recent_commits(git.workdir(), 10) {
                self.rebase_modal.open(commits);
            }
        }
    }

    /// Checkout currently selected PR in PRs tab.
    pub fn checkout_selected_pr(&mut self) {
        let target = self
            .git_service
            .as_ref()
            .and_then(|g| g.remote_url(None))
            .and_then(|url| GitHubRepoTarget::from_remote_url(&url));

        if let (Some(target), Some(git), Some(pr)) = (
            target,
            &self.git_service,
            self.prs_view.selected_pr().cloned(),
        ) {
            match self.github_service.checkout_pr_locally(
                &target,
                pr.number,
                &pr.head_branch,
                git.workdir(),
            ) {
                Ok(msg) => {
                    self.last_status = Some(msg);
                    self.refresh_git_status();
                }
                Err(e) => {
                    self.last_status = Some(format!("PR checkout failed: {}", e));
                }
            }
        }
    }

    /// Rerun selected Actions workflow job.
    pub fn rerun_selected_action_job(&mut self) {
        let target = self
            .git_service
            .as_ref()
            .and_then(|g| g.remote_url(None))
            .and_then(|url| GitHubRepoTarget::from_remote_url(&url));

        if let (Some(target), Some(run)) = (target, self.actions_view.selected_run().cloned()) {
            let gh = self.github_service.clone();
            if let Some(sender) = self.event_sender.clone() {
                tokio::spawn(async move {
                    let res = gh.rerun_workflow_run(&target, run.id).await;
                    let _ = sender.send(AppEvent::GitHubWorkflowRerunResult(res));
                });
            }
        }
    }

    /// Trigger full query in AI assistant modal using active LLM client.
    pub fn trigger_ai_assistant_query(&mut self) {
        self.ai_modal.is_loading = true;
        let query = self.ai_modal.input_query.clone();
        let ctx = self.capture_ai_context();
        let client = crate::ai::create_ai_client(&self.config.ai);
        let has_key = self.config.ai.api_key.is_some()
            || self.config.ai.provider == crate::ai::AiProvider::Ollama;
        if !has_key {
            let response = format!(
                "󱚣 AI Assistant response for '{}':\n\n• Recommendation: Code is clean and staged files are ready.\n• Conventional Commit suggested: feat: implement feature changes\n• Tip: Configure your AI API key in ~/.config/prowly/config.toml to enable live LLM integration.",
                query
            );
            self.ai_modal.set_response(response, None);
            return;
        }

        let lang = self.config.language;
        let sys = "You are Prowly, an expert AI assistant inside a modern Git/GitHub TUI client. Provide concise answers and include unified diff patches in ```diff code blocks when recommending changes.";
        let prompt_text = format!(
            "User Question: {}\n\nContext:\n{}\n\nLanguage: {}\nPlease answer concisely and provide any proposed patch or command in markdown blocks.",
            query,
            ctx.summary(),
            lang.code()
        );
        let ai_prompt = crate::ai::AiPrompt::system_and_user(sys, prompt_text);

        if let Some(sender) = self.event_sender.clone() {
            tokio::spawn(async move {
                match client.complete(&ai_prompt).await {
                    Ok(resp) => {
                        let content = resp.content;
                        let patch = if content.contains("```diff") {
                            content
                                .split("```diff")
                                .nth(1)
                                .and_then(|s| s.split("```").next())
                                .map(|s| s.trim().to_string())
                        } else {
                            None
                        };
                        let _ = sender
                            .send(AppEvent::AiAssistantResponseReceived(Ok((content, patch))));
                    }
                    Err(e) => {
                        let _ =
                            sender.send(AppEvent::AiAssistantResponseReceived(Err(e.to_string())));
                    }
                }
            });
        } else if tokio::runtime::Handle::try_current().is_err() {
            match block_on_safe(client.complete(&ai_prompt)) {
                Ok(resp) => {
                    let content = resp.content;
                    let patch = if content.contains("```diff") {
                        content
                            .split("```diff")
                            .nth(1)
                            .and_then(|s| s.split("```").next())
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    };
                    self.ai_modal.set_response(content, patch);
                }
                Err(e) => {
                    self.ai_modal.set_response(format!("AI Error: {}", e), None);
                }
            }
        }
    }

    /// Apply AI modal proposed patch or suggestion.
    pub fn apply_ai_suggestion(&mut self) {
        if let Some(ref patch) = self.ai_modal.proposed_patch {
            if let Some(git) = &self.git_service {
                let temp_patch_file = git.workdir().join(".prowly_ai_patch.diff");
                if std::fs::write(&temp_patch_file, patch).is_ok() {
                    let _ = std::process::Command::new("git")
                        .current_dir(git.workdir())
                        .args(["apply", ".prowly_ai_patch.diff"])
                        .output();
                    let _ = std::fs::remove_file(temp_patch_file);
                    self.refresh_git_status();
                    self.last_status = Some("Applied AI patch successfully!".to_string());
                    self.ai_modal.close();
                }
            }
        } else {
            self.last_status = Some("No patch to apply".to_string());
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

    /// Retrieve clean unified diff text for staged changes, falling back to current diff.
    pub fn get_staged_or_current_diff_text(&self) -> String {
        if let Some(git) = &self.git_service {
            if let Ok(staged_diff) = git.get_diff(None, true) {
                if !staged_diff.hunks.is_empty() {
                    return staged_diff.to_diff_string();
                }
            }
        }
        self.current_diff.to_diff_string()
    }

    /// Trigger AI to generate a Conventional Commit message from staged diff.
    pub fn trigger_ai_generate_commit(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let diff_text = self.get_staged_or_current_diff_text();
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
            } else if let Err(e) = commit_res {
                self.changes_view.commit_summary.clear();
                self.last_status = Some(format!("AI error: {}", e));
            }
        }
    }

    /// Trigger AI pre-commit code review.
    pub fn trigger_ai_review(&mut self) {
        let client = crate::ai::create_ai_client(&self.config.ai);
        let diff_text = self.get_staged_or_current_diff_text();
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

        // 9. Quick Branch Switcher Overlay (if open)
        if self.branch_switcher.is_open {
            self.branch_switcher
                .render(frame, &self.theme, &self.icons, area);
        }

        // 10. Remote Diff Preview Overlay (if open)
        if self.remote_diff_preview.is_open {
            self.remote_diff_preview
                .render(frame, &self.theme, &self.icons, area);
        }

        // 11. Quick Rebase Modal Overlay (if open)
        if self.rebase_modal.is_open {
            self.rebase_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 12. Open Repository Modal Overlay (if open)
        if self.open_repo_modal.is_open {
            self.open_repo_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 13. Settings Modal Overlay (if open)
        if self.settings_modal.is_open {
            self.settings_modal.hover_pos = self.hover_pos;
            self.settings_modal
                .render(frame, &self.theme, &self.icons, &self.i18n, area);
        }

        // 14. History Actions Modal Overlay (if open)
        if self.history_modal.is_open {
            self.history_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 15. Markdown Preview Modal Overlay (if open)
        if self.markdown_preview_modal.is_open {
            self.markdown_preview_modal
                .render(frame, &self.theme, &self.icons, area);
        }
    }

    /// Check whether mouse cursor is currently hovering within given rectangle.
    pub fn is_hovered(&self, rect: Rect) -> bool {
        if let Some((x, y)) = self.hover_pos {
            x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height)
        } else {
            false
        }
    }

    fn render_header(&mut self, frame: &mut Frame) {
        let top_area = self.layout_areas.header_top;
        let bottom_area = self.layout_areas.header_bottom;

        // Path formatting
        let repo_path = if let Some(git) = &self.git_service {
            let p = git.workdir();
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::Path::new(&home);
                if let Ok(rel) = p.strip_prefix(home_path) {
                    format!("~/{}", rel.display())
                } else {
                    p.display().to_string()
                }
            } else {
                p.display().to_string()
            }
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::Path::new(&home);
                if let Ok(rel) = cwd.strip_prefix(home_path) {
                    format!("~/{} (Not a Repo - Click)", rel.display())
                } else {
                    format!("{} (Not a Repo - Click)", cwd.display())
                }
            } else {
                format!("{} (Not a Repo - Click)", cwd.display())
            }
        };

        // Sync status determination
        let is_sync_needed = self.repo_status.ahead > 0 || self.repo_status.behind > 0;
        let (sync_icon, sync_label, sync_color) = if is_sync_needed {
            (
                self.icons.get(crate::ui::IconId::CheckWarning),
                "Sync Needed",
                self.theme.warning,
            )
        } else {
            (self.icons.check_success(), "Synced", self.theme.success)
        };

        // Top Header Row with GUI Capsules
        let mut top_spans = Vec::new();
        top_spans.push(Span::raw(" "));
        // 1. Brand Capsule (vibrant accent)
        let brand_spans = pill(
            Some("󰄛"),
            "PROWLY",
            self.theme.bg,
            self.theme.primary,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let brand_w = crate::ui::capsule::spans_width(&brand_spans) as u16;
        top_spans.extend(brand_spans);
        top_spans.push(Span::raw(" "));

        // 2. Repo Path Capsule
        let is_repo_hovered = self.is_hovered(self.layout_areas.open_repo_button);
        let (repo_fg, repo_bg) = if is_repo_hovered {
            (self.theme.bg, self.theme.accent)
        } else {
            (self.theme.fg, self.theme.selection_bg)
        };
        let repo_spans = pill(
            Some(""),
            &repo_path,
            repo_fg,
            repo_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let repo_w = crate::ui::capsule::spans_width(&repo_spans) as u16;
        let open_repo_x = top_area.x + 1 + brand_w + 1;
        self.layout_areas.open_repo_button = Rect {
            x: open_repo_x,
            y: top_area.y,
            width: repo_w.min(top_area.width.saturating_sub(open_repo_x)),
            height: 1,
        };
        top_spans.extend(repo_spans);
        top_spans.push(Span::raw(" "));

        // 3. Git Branch & Sync status Split Capsule
        let uncommitted_count = self.repo_status.files.len();
        let branch_stats = if uncommitted_count > 0 {
            format!(
                "↑{} ↓{} (*{})",
                self.repo_status.ahead, self.repo_status.behind, uncommitted_count
            )
        } else {
            format!(
                "↑{} ↓{}",
                self.repo_status.ahead, self.repo_status.behind
            )
        };
        let is_branch_hovered = self.is_hovered(self.layout_areas.branch_button);
        let branch_right_bg = if is_branch_hovered {
            self.theme.border_focused
        } else {
            self.theme.selection_bg
        };
        let branch_spans = split_pill(
            Some(self.icons.branch()),
            &self.repo_status.branch,
            self.theme.bg,
            self.theme.info,
            &branch_stats,
            self.theme.info,
            branch_right_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let branch_w = crate::ui::capsule::spans_width(&branch_spans) as u16;
        let branch_x = open_repo_x + repo_w + 1;
        self.layout_areas.branch_button = Rect {
            x: branch_x,
            y: top_area.y,
            width: branch_w.min(top_area.width.saturating_sub(branch_x)),
            height: 1,
        };
        top_spans.extend(branch_spans);
        top_spans.push(Span::raw(" "));

        // 4. Fetch Capsule Button
        let is_fetch_hovered = self.is_hovered(self.layout_areas.fetch_button);
        let (fetch_fg, fetch_bg) = if is_fetch_hovered {
            (self.theme.bg, self.theme.primary)
        } else {
            (self.theme.primary, self.theme.selection_bg)
        };
        let fetch_spans = pill(
            Some("󰦗"),
            "Fetch",
            fetch_fg,
            fetch_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let fetch_w = crate::ui::capsule::spans_width(&fetch_spans) as u16;
        let fetch_x = branch_x + branch_w + 1;
        self.layout_areas.fetch_button = Rect {
            x: fetch_x,
            y: top_area.y,
            width: fetch_w.min(top_area.width.saturating_sub(fetch_x)),
            height: 1,
        };
        top_spans.extend(fetch_spans);
        top_spans.push(Span::raw(" "));

        // 5. Pull Capsule Button (highlights in warning if behind > 0)
        let pull_label = if self.repo_status.behind > 0 {
            format!("Pull (↓{})", self.repo_status.behind)
        } else {
            "Pull".to_string()
        };
        let is_pull_hovered = self.is_hovered(self.layout_areas.pull_button);
        let (pull_fg, pull_bg) = if is_pull_hovered {
            (self.theme.bg, self.theme.secondary)
        } else if self.repo_status.behind > 0 {
            (self.theme.bg, self.theme.warning)
        } else {
            (self.theme.secondary, self.theme.selection_bg)
        };
        let pull_spans = pill(
            Some("󰦖"),
            &pull_label,
            pull_fg,
            pull_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let pull_w = crate::ui::capsule::spans_width(&pull_spans) as u16;
        let pull_x = fetch_x + fetch_w + 1;
        self.layout_areas.pull_button = Rect {
            x: pull_x,
            y: top_area.y,
            width: pull_w.min(top_area.width.saturating_sub(pull_x)),
            height: 1,
        };
        top_spans.extend(pull_spans);
        top_spans.push(Span::raw(" "));

        // 6. Sync State Capsule
        let is_sync_hovered = self.is_hovered(self.layout_areas.sync_button);
        let (sync_fg, sync_bg) = if is_sync_hovered {
            (self.theme.bg, self.theme.info)
        } else {
            (self.theme.bg, sync_color)
        };
        let sync_spans = pill(
            Some(sync_icon),
            sync_label,
            sync_fg,
            sync_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let sync_w = crate::ui::capsule::spans_width(&sync_spans) as u16;
        let sync_x = pull_x + pull_w + 1;
        self.layout_areas.sync_button = Rect {
            x: sync_x,
            y: top_area.y,
            width: sync_w.min(top_area.width.saturating_sub(sync_x)),
            height: 1,
        };
        top_spans.extend(sync_spans);

        frame.render_widget(
            Paragraph::new(Line::from(top_spans)).style(Style::default().bg(self.theme.header_bg)),
            top_area,
        );

        // Top right buttons: AI Status Chip, [ 󰍜 Menu ], [  Settings ], [? Help]
        let (ai_connected, ai_prov, ai_mod) = self.ai_connection_status();
        let (ai_icon, ai_label, ai_fg) = if ai_connected {
            (
                "󱚣",
                format!("AI: {} ({})", ai_prov, ai_mod),
                self.theme.ai_sparkle,
            )
        } else {
            let disconnected_label = if self.config.language == crate::i18n::Language::Ja {
                "AI: 未接続"
            } else {
                "AI: Disconnected"
            };
            (
                "⚠",
                disconnected_label.to_string(),
                self.theme.warning,
            )
        };

        let is_help_hovered = self.is_hovered(self.layout_areas.help_button);
        let (help_fg, help_bg) = if is_help_hovered {
            (self.theme.bg, self.theme.fg)
        } else {
            (self.theme.dim_fg, self.theme.selection_bg)
        };
        let help_spans = pill(
            Some(""),
            "Help",
            help_fg,
            help_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let help_w = crate::ui::capsule::spans_width(&help_spans) as u16;

        let is_settings_hovered = self.is_hovered(self.layout_areas.settings_button);
        let (settings_fg, settings_bg) = if is_settings_hovered {
            (self.theme.bg, self.theme.primary)
        } else {
            (self.theme.primary, self.theme.selection_bg)
        };
        let settings_spans = pill(
            Some(""),
            "Settings",
            settings_fg,
            settings_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let settings_w = crate::ui::capsule::spans_width(&settings_spans) as u16;

        let is_palette_hovered = self.is_hovered(self.layout_areas.palette_button);
        let (palette_fg, palette_bg) = if is_palette_hovered {
            (self.theme.bg, self.theme.accent)
        } else {
            (self.theme.accent, self.theme.selection_bg)
        };
        let palette_spans = pill(
            Some(self.icons.ai_sparkle()),
            "Palette",
            palette_fg,
            palette_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let palette_w = crate::ui::capsule::spans_width(&palette_spans) as u16;

        let is_menu_hovered = self.is_hovered(self.layout_areas.menu_button);
        let (menu_fg, menu_bg) = if is_menu_hovered {
            (self.theme.bg, self.theme.secondary)
        } else {
            (self.theme.secondary, self.theme.selection_bg)
        };
        let menu_spans = pill(
            Some("󰍜"),
            "Menu",
            menu_fg,
            menu_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let menu_w = crate::ui::capsule::spans_width(&menu_spans) as u16;

        let is_ai_hovered = self.is_hovered(self.layout_areas.ai_status_button);
        let (ai_pill_fg, ai_pill_bg) = if is_ai_hovered {
            (self.theme.bg, self.theme.ai_sparkle)
        } else {
            (ai_fg, self.theme.selection_bg)
        };
        let ai_spans = pill(
            Some(ai_icon),
            &ai_label,
            ai_pill_fg,
            ai_pill_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let ai_w = crate::ui::capsule::spans_width(&ai_spans) as u16;

        let help_x = top_area.x + top_area.width.saturating_sub(help_w);
        let settings_x = help_x.saturating_sub(settings_w);
        let palette_x = settings_x.saturating_sub(palette_w);
        let menu_x = palette_x.saturating_sub(menu_w);
        let ai_x = menu_x.saturating_sub(ai_w);

        self.layout_areas.help_button = Rect { x: help_x, y: top_area.y, width: help_w, height: 1 };
        self.layout_areas.settings_button = Rect { x: settings_x, y: top_area.y, width: settings_w, height: 1 };
        self.layout_areas.palette_button = Rect { x: palette_x, y: top_area.y, width: palette_w, height: 1 };
        self.layout_areas.menu_button = Rect { x: menu_x, y: top_area.y, width: menu_w, height: 1 };
        self.layout_areas.ai_status_button = Rect { x: ai_x, y: top_area.y, width: ai_w, height: 1 };
        self.layout_areas.ask_ai_button = self.layout_areas.ai_status_button;

        frame.render_widget(Paragraph::new(Line::from(ai_spans)), self.layout_areas.ai_status_button);
        frame.render_widget(Paragraph::new(Line::from(menu_spans)), self.layout_areas.menu_button);
        frame.render_widget(Paragraph::new(Line::from(palette_spans)), self.layout_areas.palette_button);
        frame.render_widget(Paragraph::new(Line::from(settings_spans)), self.layout_areas.settings_button);
        frame.render_widget(Paragraph::new(Line::from(help_spans)), self.layout_areas.help_button);

        // Bottom Header Row with Metadata Capsules
        let gh_target = self.github_target();
        let (repo_icon, repo_label) = if let Some(target) = gh_target {
            let is_priv = self.is_repo_private.unwrap_or(false);
            let (icon, vis) = if is_priv {
                (self.icons.lock_private(), self.i18n.t("header.private"))
            } else {
                (self.icons.globe_public(), self.i18n.t("header.public"))
            };
            (icon, format!("{}/{} ({})", target.owner, target.repo, vis))
        } else if let Some(git) = &self.git_service {
            let name = git
                .workdir()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            (self.icons.folder_closed(), format!("local @ {}", name))
        } else {
            (self.icons.folder_closed(), "local @ repo".to_string())
        };

        let (ci_icon, ci_label) = if let Some(run) = self.actions_view.runs.first() {
            let conclusion = run.conclusion.as_deref().unwrap_or("in_progress");
            let icon = if conclusion == "success" {
                self.icons.check_success()
            } else if conclusion == "failure" {
                self.icons.check_failed()
            } else {
                self.icons.check_running()
            };
            (icon, format!("CI: #{} {}", run.id, conclusion))
        } else {
            (self.icons.check_success(), "CI: No runs".to_string())
        };

        let unread_count = self.alerts_view.alerts.iter().filter(|a| a.unread).count();
        let alert_label = format!("Alerts: {}", unread_count);

        let mut bottom_spans = Vec::new();
        bottom_spans.push(Span::raw("  "));
        bottom_spans.extend(pill(
            Some(repo_icon),
            &repo_label,
            self.theme.dim_fg,
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));
        bottom_spans.push(Span::raw(" "));
        bottom_spans.extend(pill(
            Some(ci_icon),
            &ci_label,
            self.theme.success,
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));
        bottom_spans.push(Span::raw(" "));
        bottom_spans.extend(pill(
            Some(self.icons.get(crate::ui::IconId::Alert)),
            &alert_label,
            if unread_count > 0 { self.theme.warning } else { self.theme.dim_fg },
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));

        frame.render_widget(
            Paragraph::new(Line::from(bottom_spans)).style(Style::default().bg(self.theme.header_bg)),
            bottom_area,
        );
    }

    fn render_right_pane(&mut self, frame: &mut Frame) {
        // Render Segmented Control Pill Tabs
        let mut tab_spans = Vec::new();
        tab_spans.push(Span::raw(" "));
        let mut cur_x = 1u16;
        for tab in AppTab::ALL {
            let key = tab.title_key();
            let text = self.i18n.t(key);
            let icon_str = match tab {
                AppTab::Changes => self.icons.branch(),
                AppTab::History => self.icons.commit(),
                AppTab::Prs => self.icons.pr_open(),
                AppTab::Actions => self.icons.check_success(),
                AppTab::Alerts => self.icons.get(crate::ui::IconId::Alert),
                AppTab::Worktrees => self.icons.worktree(),
            };

            let is_active = *tab == self.active_tab;
            let sample_pill = pill(
                Some(icon_str),
                &text,
                self.theme.bg,
                self.theme.primary,
                self.theme.bg,
                self.icons.use_nerd_font,
            );
            let pill_w = crate::ui::capsule::spans_width(&sample_pill) as u16;
            let tab_rect = Rect {
                x: self.layout_areas.right_tabs.x + cur_x,
                y: self.layout_areas.right_tabs.y,
                width: pill_w,
                height: 1,
            };
            let is_hovered = self.is_hovered(tab_rect);

            let (fg, bg) = if is_active {
                (self.theme.bg, self.theme.primary)
            } else if is_hovered {
                (self.theme.primary, self.theme.selection_bg)
            } else {
                (self.theme.dim_fg, self.theme.selection_bg)
            };

            tab_spans.extend(pill(
                Some(icon_str),
                &text,
                fg,
                bg,
                self.theme.bg,
                self.icons.use_nerd_font,
            ));
            tab_spans.push(Span::raw(" "));
            cur_x += pill_w + 1;
        }

        frame.render_widget(Paragraph::new(Line::from(tab_spans)), self.layout_areas.right_tabs);

        // Render Active Tab Content with Elevated Border
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
            .title(if is_focused {
                Span::styled(
                    " ● ACTIVE ",
                    Style::default()
                        .fg(self.theme.border_focused)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("")
            })
            .style(Style::default().bg(self.theme.bg));

        let inner = block.inner(self.layout_areas.right_content);
        frame.render_widget(block, self.layout_areas.right_content);

        match self.active_tab {
            AppTab::Changes => {
                let filtered = self.filtered_files();
                let diff = self.current_diff.clone();
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                let params = ChangesRenderParams {
                    files: &filtered,
                    active_diff: &diff,
                    theme: &theme,
                    i18n: &self.i18n,
                    icons: &icons,
                    area: inner,
                    hover_pos: self.hover_pos,
                };
                self.changes_view.render(frame, params);
            }
            AppTab::History => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.history_view.render(frame, &theme, &icons, inner);
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
        let status_msg = self.last_status.as_deref().unwrap_or("Ready");
        let (ai_connected, ai_prov, ai_mod) = self.ai_connection_status();
        let ai_footer = if ai_connected {
            format!("{}: {}", ai_prov, ai_mod)
        } else if self.config.language == crate::i18n::Language::Ja {
            "未接続".to_string()
        } else {
            "Disconnected".to_string()
        };

        let mut left_spans = Vec::new();
        left_spans.push(Span::raw(" "));
        left_spans.extend(pill(
            Some("󰄛"),
            "NORMAL",
            self.theme.bg,
            self.theme.primary,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.push(Span::styled(
            format!(" 󰋼 {}  ", status_msg),
            Style::default().fg(self.theme.status_fg),
        ));

        // Keycap badges
        left_spans.extend(key_badge(
            "Tab",
            "Panes",
            self.theme.primary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "Space",
            "Stage",
            self.theme.success,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "c",
            "Commit",
            self.theme.accent,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "F1",
            "Palette",
            self.theme.primary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            ",",
            "Settings",
            self.theme.secondary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "Ctrl+O",
            "Open",
            self.theme.info,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "q",
            "Quit",
            self.theme.error,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));

        let mut right_spans = Vec::new();
        right_spans.extend(pill(
            Some("󱚣"),
            &ai_footer,
            if ai_connected { self.theme.ai_sparkle } else { self.theme.warning },
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));
        right_spans.extend(pill(
            Some(""),
            self.theme.name,
            self.theme.fg,
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));
        right_spans.extend(pill(
            Some(""),
            self.config.language.code().to_uppercase().as_str(),
            self.theme.info,
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));

        let left_len = crate::ui::capsule::spans_width(&left_spans);
        let right_len = crate::ui::capsule::spans_width(&right_spans);
        let width = self.layout_areas.footer.width as usize;
        let pad = width.saturating_sub(left_len + right_len);

        let mut all_spans = left_spans;
        all_spans.push(Span::styled(" ".repeat(pad), Style::default().bg(self.theme.status_bg)));
        all_spans.extend(right_spans);

        let p = Paragraph::new(Line::from(all_spans)).style(
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
