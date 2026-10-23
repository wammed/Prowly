//! Application state and UI event dispatcher for Prowly.

mod actions;
mod input;
mod render;

use crate::ai::ContextProvider;
use crate::config::Config;
use crate::event::AppEvent;
use crate::git::{list_worktrees, FileDiff, GitFileItem, GitRepoStatus, GitService, WorktreeItem};
use crate::github::{GitHubRepoTarget, GitHubService};
use crate::i18n::I18n;
use crate::services::{BackupService, ClipboardService};
use crate::ui::capsule::pill;
use crate::ui::layout::LayoutAreas;
use crate::ui::modals::{
    AiAssistantModal, BranchSwitcherModal, CommandPaletteModal, ConflictResolverModal, ContextMenu,
    HistoryActionState, HistoryActionsModal, HistoryMenuAction, MarkdownPreviewModal,
    OpenRepoModal, RebaseModal, RemoteDiffPreviewModal, SettingsModal,
};
use crate::ui::theme::Theme;
use crate::ui::views::{
    ActionsView, AlertsView, ChangesView, DirectoryMap, HistoryView, IssuesView, PrsView,
    WorktreesView,
};
use crate::ui::Icons;

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
    Issues,
    Prs,
    Actions,
    Alerts,
    Worktrees,
}

impl AppTab {
    pub const ALL: &'static [AppTab] = &[
        AppTab::Changes,
        AppTab::History,
        AppTab::Issues,
        AppTab::Prs,
        AppTab::Actions,
        AppTab::Alerts,
        AppTab::Worktrees,
    ];

    pub fn index(&self) -> usize {
        match self {
            Self::Changes => 0,
            Self::History => 1,
            Self::Issues => 2,
            Self::Prs => 3,
            Self::Actions => 4,
            Self::Alerts => 5,
            Self::Worktrees => 6,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::History,
            2 => Self::Issues,
            3 => Self::Prs,
            4 => Self::Actions,
            5 => Self::Alerts,
            6 => Self::Worktrees,
            _ => Self::Changes,
        }
    }

    pub fn title_key(&self) -> &'static str {
        match self {
            Self::Changes => "tabs.changes",
            Self::History => "tabs.history",
            Self::Issues => "tabs.issues",
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
    pub issues_view: IssuesView,
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

    /// Pending file requested to open in external editor outside event loop
    pub pending_external_editor: Option<std::path::PathBuf>,

    /// Active abort handle for in-flight cancellable AI queries
    pub ai_abort_handle: Option<tokio::task::AbortHandle>,
}

impl App {
    /// Create new application instance from configuration.
    pub fn new(mut config: Config) -> Self {
        let theme = Theme::get(config.theme);
        let i18n = I18n::new(config.language);
        let icons = Icons::new(config.use_nerd_font);
        let clipboard = ClipboardService::new();
        let backup_service = BackupService::new();

        let home = dirs::home_dir();
        let mut git_service = GitService::discover().ok();
        if git_service.as_ref().map(|g| g.workdir()) == home.as_deref()
            && config.git.restore_last_repository
        {
            if let Some(ref last) = config.last_repository {
                let p = std::path::Path::new(last);
                if p.is_dir() && Some(p) != home.as_deref() {
                    if let Ok(svc) = GitService::open(p) {
                        git_service = Some(svc);
                    }
                }
            }
        }
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
            let p_str = git.workdir().display().to_string();
            if Some(git.workdir()) != home.as_deref() || config.last_repository.is_none() {
                config.add_recent_repository(p_str.clone());
                config.last_repository = Some(p_str);
            }
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
            issues_view: IssuesView::new(),
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
            pending_external_editor: None,
            ai_abort_handle: None,
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
                        crate::git::clean_git_command(&workdir)
                            .args(["fetch", "--all", "--prune"])
                            .output()
                            .map_err(|e| e.to_string())
                            .and_then(|out| {
                                if out.status.success() {
                                    let stdout =
                                        String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    let stderr =
                                        String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    let msg = if !stdout.is_empty() { stdout } else { stderr };
                                    Ok(if msg.is_empty() {
                                        "Fetched updates".to_string()
                                    } else {
                                        msg
                                    })
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
                        crate::git::GitService::open(&workdir)
                            .map_err(|e| e.to_string())
                            .and_then(|g| g.cli_pull().map_err(|e| e.to_string()))
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
                        crate::git::GitService::open(&workdir)
                            .map_err(|e| e.to_string())
                            .and_then(|g| g.cli_push().map_err(|e| e.to_string()))
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

    /// Lightweight status refresh for timer ticks: avoids expensive directory and file-tree walks
    /// when repository status has not changed.
    pub fn refresh_git_status_light(&mut self) {
        if let Some(git) = &self.git_service {
            if let Ok(status) = git.get_status() {
                // If repository status matches what we already have, skip expensive deep walk
                if status == self.repo_status {
                    return;
                }
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
                self.last_status = Some(format!(
                    "Not a Git repository: {}. (Press 'o' -> 'i' to init or 'c' to clone)",
                    e
                ));
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
            .effective_api_key()
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
                    if let Ok(issues) = gh.list_issues(&target, false).await {
                        let _ = sender.send(AppEvent::GitHubIssuesLoaded(issues));
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
                if let Ok(issues) = block_on_safe(gh.list_issues(&target, false)) {
                    self.issues_view.set_issues(issues);
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

    /// Load or force refresh GitHub issues.
    pub fn load_issues(&mut self, force: bool) {
        let target = self
            .git_service
            .as_ref()
            .and_then(|g| g.remote_url(None))
            .and_then(|url| GitHubRepoTarget::from_remote_url(&url));

        if let Some(target) = target {
            self.issues_view.status_message = Some("Loading issues from GitHub...".to_string());
            if let Some(sender) = self.event_sender.clone() {
                let gh = self.github_service.clone();
                tokio::spawn(async move {
                    if let Ok(issues) = gh.list_issues(&target, force).await {
                        let _ = sender.send(AppEvent::GitHubIssuesLoaded(issues));
                    }
                });
            } else if tokio::runtime::Handle::try_current().is_err() {
                let gh = &self.github_service;
                if let Ok(issues) = block_on_safe(gh.list_issues(&target, force)) {
                    self.issues_view.set_issues(issues);
                    self.issues_view.status_message = None;
                }
            }
        } else {
            self.issues_view.status_message =
                Some("No GitHub remote repository found.".to_string());
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
}
