use super::*;
use crate::ai::{
    analyze_ci_failure, diagnose_repository, generate_commit_message, generate_pr_draft,
    review_changes, smart_merge_conflict, translate_natural_language_to_git, AiContext,
    ContextProvider,
};
use crate::git::{create_worktree, remove_worktree};
use crate::services::reveal_in_file_manager;
use crate::ui::modals::{ContextMenuAction, HistoryMenuAction, PaletteEnterResult};
use crate::ui::theme::{Theme, ThemeId};
use crate::ui::views::changes::ChangesFocus;

impl App {
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
            ContextMenuAction::CheckoutCommit => {
                self.checkout_selected_commit();
            }
            ContextMenuAction::CreateBranch => {
                let target_commit = self
                    .context_menu
                    .target_commit
                    .clone()
                    .or_else(|| self.history_view.selected_commit().cloned());
                if let Some(commit) = target_commit {
                    self.history_modal.open_branch_input(commit);
                }
            }
            ContextMenuAction::CherryPick => {
                self.cherry_pick_selected_commit();
            }
            ContextMenuAction::RevertCommit => {
                self.revert_selected_commit();
            }
            ContextMenuAction::TimeMachine => {
                let target_commit = self
                    .context_menu
                    .target_commit
                    .clone()
                    .or_else(|| self.history_view.selected_commit().cloned());
                if let Some(commit) = target_commit {
                    self.history_modal.open_time_machine(commit);
                }
            }
            ContextMenuAction::CopyCommitSha => {
                self.copy_selected_commit_sha();
            }
            ContextMenuAction::CopyCommitMessage => {
                self.copy_selected_commit_message();
            }
            ContextMenuAction::AskAiCommit => {
                let ctx = self.capture_ai_context();
                self.ai_modal.open_with_context(&ctx);
            }
        }
    }

    /// Apply the result of Command Palette execution.
    pub fn apply_palette_result(&mut self, result: PaletteEnterResult) {
        match result {
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
                match crate::services::SafeGitRunner::parse_ai_command(&cmd) {
                    Ok((sub, args)) => {
                        let workdir = if let Some(git) = &self.git_service {
                            git.workdir().to_path_buf()
                        } else {
                            std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        };
                        match crate::services::SafeGitRunner::execute_git(&workdir, &sub, &args) {
                            Ok(output) => {
                                if output.status.success() {
                                    self.last_status = Some(format!(
                                        "Executed AI command: git {} {}",
                                        sub,
                                        args.join(" ")
                                    ));
                                } else {
                                    let err = String::from_utf8_lossy(&output.stderr);
                                    self.last_status = Some(format!(
                                        "AI command failed: {}",
                                        err.lines().next().unwrap_or("unknown error")
                                    ));
                                }
                            }
                            Err(e) => {
                                self.last_status = Some(format!("Failed to run AI command: {}", e));
                            }
                        }
                    }
                    Err(err) => {
                        self.last_status =
                            Some(format!("⚠️ AI command rejected by safety guard: {}", err));
                    }
                }
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
                        let res =
                            translate_natural_language_to_git(&req, &branch, client.as_ref(), lang)
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
            "tab_issues" => {
                self.active_tab = AppTab::Issues;
                self.load_issues(true);
                self.last_status = Some("Reloading GitHub issues...".to_string());
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
            "abort_operation" => self.abort_in_progress_operation(),
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

    /// Execute an arbitrary Git CLI command in the repository workspace safely without a shell.
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

        if !crate::services::SafeGitRunner::is_safe_subcommand(subcommand) {
            self.last_status = Some(format!("Invalid Git subcommand: '{}'", subcommand));
            return;
        }

        let parsed_args = crate::services::SafeGitRunner::tokenize_args(trimmed_args);
        let output =
            crate::services::SafeGitRunner::execute_git(&workdir, subcommand, &parsed_args);

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

    /// Safely abort any in-progress Git operation (rebase, merge, cherry-pick, revert).
    pub fn abort_in_progress_operation(&mut self) {
        if let Some(git) = &self.git_service {
            match git.abort_current_operation() {
                Ok(msg) => {
                    self.last_status = Some(if msg.trim().is_empty() {
                        "Operation aborted successfully".to_string()
                    } else {
                        format!("Operation aborted: {}", msg.trim())
                    });
                    self.refresh_git_status();
                }
                Err(e) => {
                    self.last_status = Some(format!("Failed to abort operation: {}", e));
                }
            }
        } else {
            self.last_status = Some("No Git repository active".to_string());
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

    /// Safe discard with automated backup for selected file (or direct discard if backup disabled in config).
    pub fn safe_discard_selected(&mut self) {
        let filtered = self.filtered_files();
        let idx = self.changes_view.file_scroll.selected_index;
        if let Some(item) = filtered.get(idx) {
            let path = item.path.clone();
            let is_staged = item.staged;
            if let Some(git) = &self.git_service {
                if !self.config.git.soft_discard_backup {
                    match git.hard_discard_targeted(&path, is_staged) {
                        Ok(()) => {
                            self.refresh_git_status();
                            self.last_status =
                                Some(format!("Discarded {} (backup disabled)", path));
                        }
                        Err(e) => {
                            self.last_status = Some(format!("Discard failed: {}", e));
                        }
                    }
                } else {
                    match git.soft_discard_targeted(&path, is_staged, &self.backup_service) {
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
                        self.last_status =
                            Some(format!("Checked out commit {} (Detached HEAD)", hash));
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
                        self.last_status =
                            Some(format!("Cherry-picked commit {} successfully", hash));
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
                    self.last_status = Some(
                        "⚠️ No staged changes. Stage specific files or press 'a' to stage all before committing."
                            .to_string(),
                    );
                } else {
                    self.last_status = Some("No changes to commit".to_string());
                }
                return;
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
                        self.last_status =
                            Some(format!("Committed locally: {} (unpushed)", short_oid));
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
            self.pending_external_editor = Some(full_path);
        }
    }

    /// Retrieve and clear any pending external editor request to execute cleanly outside event loop.
    pub fn take_pending_editor(&mut self) -> Option<std::path::PathBuf> {
        self.pending_external_editor.take()
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
                    self.last_status = Some(format!("Could not read file ({}): {}", rel_path, e));
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

    /// Checkout selected branch by name.
    pub fn checkout_selected_branch(&mut self, target_b: &str) {
        if target_b != self.branch_switcher.current_branch {
            let prev = self.repo_status.branch.clone();
            if let Some(git) = &self.git_service {
                match git.checkout_branch(target_b) {
                    Ok(_) => {
                        self.previous_branch = Some(prev);
                        self.last_status = Some(format!("Switched branch to {}", target_b));
                        self.refresh_git_status();
                        self.branch_switcher.close();
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Checkout failed: {}", e));
                    }
                }
            }
        } else {
            self.branch_switcher.close();
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

    /// Perform quick squash of top 2 commits in rebase modal.
    pub fn rebase_quick_squash(&mut self) {
        if let Some(git) = &self.git_service {
            let combined_msg = "squash: combined recent commits".to_string();
            if crate::git::rebase::quick_squash(git.workdir(), 2, &combined_msg).is_ok() {
                self.last_status = Some("Squashed top 2 commits".to_string());
                self.refresh_git_status();
                self.open_rebase_modal();
            }
        }
    }

    /// Perform fixup into HEAD commit in rebase modal.
    pub fn rebase_fixup_head(&mut self) {
        if let Some(git) = &self.git_service {
            if crate::git::rebase::fixup_head_commit(git.workdir()).is_ok() {
                self.last_status = Some("Fixup merged into HEAD commit".to_string());
                self.refresh_git_status();
                self.open_rebase_modal();
            }
        }
    }

    /// Trigger AI smart merge for current conflict hunk.
    pub fn trigger_ai_merge_conflict(&mut self) {
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
                if let Ok(resolved) =
                    block_on_safe(smart_merge_conflict(&path, &hunk, client.as_ref(), lang))
                {
                    self.conflict_resolver.set_ai_resolution(resolved);
                }
            }
        }
    }

    /// Save and stage resolved conflict content.
    pub fn save_and_stage_resolved_conflict(&mut self) {
        let path = self.conflict_resolver.file_path.clone();
        let content = self.conflict_resolver.build_resolved_content();
        if std::fs::write(&path, content).is_ok() {
            if let Some(git) = &self.git_service {
                let _ = git.stage_file(&path);
            }
            self.refresh_git_status();
            self.last_status = Some(format!("Resolved and staged conflicts in {}", path));
        } else {
            self.last_status = Some(format!("Failed to save resolved file {}", path));
        }
        self.conflict_resolver.close();
    }

    /// Check whether any modal or overlay dialog is currently open.
    pub fn is_any_modal_open(&self) -> bool {
        self.command_palette.is_open
            || self.branch_switcher.is_open
            || self.settings_modal.is_open
            || self.open_repo_modal.is_open
            || self.history_modal.is_open
            || self.conflict_resolver.is_open
            || self.ai_modal.is_open
            || self.remote_diff_preview.is_open
            || self.rebase_modal.is_open
            || self.markdown_preview_modal.is_open
            || self.context_menu.is_open
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
                    let _ = sender.send(AppEvent::GitHubWorkflowRerunResult(
                        res.map_err(|e| e.to_string()),
                    ));
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
        let has_key = self.config.ai.effective_api_key().is_some()
            || self.config.ai.provider == crate::ai::AiProvider::Ollama;
        if !has_key {
            let provider_name = format!("{:?}", self.config.ai.provider);
            let response = if self.config.language == crate::i18n::Language::Ja {
                format!(
                    "⚠️ プロバイダー '{}' の AI API キーが設定されていません。\n\n~/.config/prowly/config.toml、環境変数（例: {}_API_KEY）、または設定モーダル (F2) で API キーを設定してください。",
                    provider_name,
                    provider_name.to_uppercase()
                )
            } else {
                format!(
                    "⚠️ AI API key is not configured for provider '{}'.\n\nPlease set your API key in ~/.config/prowly/config.toml, via environment variable (e.g. {}_API_KEY), or in the Settings modal (F2).",
                    provider_name,
                    provider_name.to_uppercase()
                )
            };
            self.ai_modal.set_response(response, None);
            return;
        }

        if let Some(prev) = self.ai_abort_handle.take() {
            prev.abort();
        }
        self.ai_modal.response_text.clear();
        self.ai_modal.proposed_patch = None;

        let sys = "You are Prowly, an expert AI assistant inside a modern Git/GitHub TUI client. Provide concise answers and include unified diff patches in ```diff code blocks when recommending changes.";
        let full_user_prompt = ctx.format_user_prompt(&query);
        let masked_user_prompt = crate::ai::mask_secrets(&full_user_prompt);
        let ai_prompt = crate::ai::AiPrompt::system_and_user(sys, masked_user_prompt);

        if let Some(sender) = self.event_sender.clone() {
            let join_handle = tokio::spawn(async move {
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
            self.ai_abort_handle = Some(join_handle.abort_handle());
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

    /// Cancel an in-flight AI assistant query immediately.
    pub fn cancel_ai_query(&mut self) {
        if let Some(handle) = self.ai_abort_handle.take() {
            handle.abort();
            self.ai_modal.is_loading = false;
            let cancel_msg = if self.config.language == crate::i18n::Language::Ja {
                "🛑 AI リクエストがキャンセルされました。".to_string()
            } else {
                "🛑 AI request was cancelled.".to_string()
            };
            self.ai_modal.set_response(cancel_msg, None);
            self.last_status = Some("AI request cancelled".to_string());
        }
    }

    /// Apply AI modal proposed patch or suggestion.
    pub fn apply_ai_suggestion(&mut self) {
        if let Some(ref patch) = self.ai_modal.proposed_patch {
            if let Some(git) = &self.git_service {
                let mut child = match crate::git::clean_git_command(git.workdir())
                    .args(["apply", "-"])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        self.last_status = Some(format!("Failed to spawn git apply: {}", e));
                        return;
                    }
                };

                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(patch.as_bytes());
                }

                match child.wait_with_output() {
                    Ok(out) if out.status.success() => {
                        self.refresh_git_status();
                        self.last_status = Some("Applied AI patch successfully!".to_string());
                        self.ai_modal.close();
                    }
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        let msg = if err.is_empty() {
                            String::from_utf8_lossy(&out.stdout).trim().to_string()
                        } else {
                            err
                        };
                        self.last_status = Some(format!("Failed to apply patch: {}", msg));
                    }
                    Err(e) => {
                        self.last_status = Some(format!("Error running git apply: {}", e));
                    }
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
        let diff_text = crate::ai::mask_secrets(&self.get_staged_or_current_diff_text());
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
        let diff_text = crate::ai::mask_secrets(&self.get_staged_or_current_diff_text());
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

        let ctx = self.capture_ai_context();
        let diff_text = if let Some(ref snippet) = ctx.diff_snippet {
            snippet.clone()
        } else if let Some(ref git) = self.git_service {
            git.get_diff(None, true)
                .or_else(|_| git.get_diff(None, false))
                .map(|d| {
                    let mut s = String::new();
                    for hunk in &d.hunks {
                        s.push_str(&hunk.header);
                        s.push('\n');
                        for l in &hunk.lines {
                            s.push(l.origin);
                            s.push_str(&l.content);
                            s.push('\n');
                        }
                    }
                    s
                })
                .unwrap_or_default()
        } else {
            String::new()
        };

        let lang = self.config.language;
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
        let (run_name, log_snippet) = match self.actions_view.selected_run() {
            Some(run) => match &run.logs {
                Some(logs) if !logs.trim().is_empty() => (run.name.clone(), logs.clone()),
                _ => {
                    let msg = if self.config.language == crate::i18n::Language::Ja {
                        "⚠️ 選択されたワークフロー実行のログが取得できません。gh CLI または GitHub API でログがダウンロードされているか確認してください。"
                    } else {
                        "⚠️ No logs available for the selected workflow run. Ensure logs have been downloaded via gh CLI or GitHub API."
                    };
                    self.ai_modal.set_response(msg.to_string(), None);
                    return;
                }
            },
            None => {
                let msg = if self.config.language == crate::i18n::Language::Ja {
                    "⚠️ CI ワークフローが選択されていません。"
                } else {
                    "⚠️ No CI workflow run selected."
                };
                self.ai_modal.set_response(msg.to_string(), None);
                return;
            }
        };

        let client = crate::ai::create_ai_client(&self.config.ai);
        let lang = self.config.language;

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
