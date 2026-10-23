use super::*;
use crate::event::AppEvent;
use crate::git::{ConflictChoice, GitService};
use crate::github::GitHubRepoTarget;
use crate::ui::layout::LayoutAreas;
use crate::ui::modals::{
    AiAssistantClickResult, BranchSwitcherClickResult, ConflictResolverClickResult,
    HistoryActionState, HistoryModalClickResult, MarkdownPreviewClickResult, OpenRepoAction,
    PaletteClickResult, RebaseClickResult, RemoteDiffClickResult, SettingsAction,
};
use crate::ui::views::changes::{ChangesButtonAction, ChangesFocus};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

impl App {
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
                // Auto-refresh git status every 1.5s using lightweight check to avoid blocking UI thread
                if self.last_status_check.elapsed() >= std::time::Duration::from_millis(1500) {
                    self.last_status_check = std::time::Instant::now();
                    self.refresh_git_status_light();
                }

                let interval = self.config.git.auto_fetch_interval_secs;
                if interval > 0
                    && self.last_auto_fetch.elapsed() >= std::time::Duration::from_secs(interval)
                {
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
            AppEvent::GitHubIssuesLoaded(issues) => {
                self.issues_view.set_issues(issues);
                self.issues_view.status_message = None;
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
            AppEvent::AiAssistantResponseReceived(res) => {
                self.ai_abort_handle = None;
                match res {
                    Ok((ans, patch)) => {
                        self.ai_modal.set_response(ans, patch);
                    }
                    Err(e) => {
                        self.ai_modal.is_loading = false;
                        self.ai_modal.set_response(format!("AI Error: {}", e), None);
                    }
                }
            }
            AppEvent::AiAssistantChunkReceived(chunk) => {
                self.ai_modal.append_chunk(&chunk);
            }
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
                        self.config.auto_save_enabled = true;
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
                    OpenRepoAction::RemoveRecent(path) => {
                        self.config.remove_recent_repository(&path);
                        let _ = self.config.save();
                        self.open_repo_modal
                            .recent_repos
                            .retain(|p| p.trim_end_matches('/') != path.trim_end_matches('/'));
                        self.open_repo_modal
                            .recent_scroll
                            .set_total_items(self.open_repo_modal.recent_repos.len());
                        if self.open_repo_modal.recent_scroll.selected_index
                            >= self.open_repo_modal.recent_repos.len()
                        {
                            self.open_repo_modal.recent_scroll.selected_index =
                                self.open_repo_modal.recent_repos.len().saturating_sub(1);
                        }
                        self.last_status =
                            Some(format!("Removed {} from recent repositories", path));
                    }
                    OpenRepoAction::InitRepo(path) => {
                        self.open_repo_modal.close();
                        match GitService::init_repo(&path) {
                            Ok(()) => {
                                self.last_status = Some(format!(
                                    "Initialized empty Git repository in {}",
                                    path.display()
                                ));
                                self.switch_repository(&path);
                            }
                            Err(e) => {
                                self.last_status =
                                    Some(format!("Failed to initialize repository: {}", e));
                            }
                        }
                    }
                    OpenRepoAction::CloneRepo { url, target_dir } => {
                        self.open_repo_modal.close();
                        self.last_status = Some(format!("Cloning {} ...", url));
                        match GitService::clone_repo(&url, &target_dir) {
                            Ok(dest) => {
                                self.last_status =
                                    Some(format!("Successfully cloned into {}", dest.display()));
                                self.switch_repository(&dest);
                            }
                            Err(e) => {
                                self.last_status =
                                    Some(format!("Failed to clone repository: {}", e));
                            }
                        }
                    }
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
                        self.checkout_selected_branch(&target_b);
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
                    let _ = self
                        .clipboard
                        .copy_text(&self.markdown_preview_modal.raw_content);
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
                KeyCode::Char('s') => self.rebase_quick_squash(),
                KeyCode::Char('f') => self.rebase_fixup_head(),
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
                KeyCode::Esc => {
                    if self.ai_modal.is_loading {
                        self.cancel_ai_query();
                    } else {
                        self.ai_modal.close();
                    }
                }
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
                    let res = self.command_palette.handle_enter();
                    self.apply_palette_result(res);
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
                KeyCode::Char('A') => {
                    self.abort_in_progress_operation();
                    self.conflict_resolver.close();
                }
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
                    self.trigger_ai_merge_conflict();
                }
                KeyCode::Enter => {
                    self.save_and_stage_resolved_conflict();
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
            KeyCode::Char('o') => {
                let url = match self.active_tab {
                    AppTab::Issues => self
                        .issues_view
                        .selected_issue()
                        .map(|i| i.html_url.clone()),
                    AppTab::Prs => self.prs_view.selected_pr().map(|p| p.html_url.clone()),
                    AppTab::Actions => self.actions_view.selected_run().map(|r| r.html_url.clone()),
                    AppTab::Alerts => self.alerts_view.selected_alert().map(|a| a.url.clone()),
                    _ => None,
                };
                if let Some(u) = url {
                    if !u.is_empty() {
                        #[cfg(target_os = "linux")]
                        let _ = std::process::Command::new("xdg-open").arg(&u).spawn();
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").arg(&u).spawn();
                        #[cfg(target_os = "windows")]
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "start", &u])
                            .spawn();
                        self.last_status = Some(format!("Opened in browser: {}", u));
                    }
                } else {
                    self.open_open_repo();
                }
            }
            KeyCode::Char('A') => {
                let op_in_progress =
                    self.repo_status.operation_state != crate::git::RepoOperationState::None;
                if op_in_progress {
                    self.abort_in_progress_operation();
                } else {
                    self.show_all_files = !self.show_all_files;
                    let msg = if self.show_all_files {
                        "Showing all repository files"
                    } else {
                        "Showing changed files only"
                    };
                    self.last_status = Some(msg.to_string());
                    self.refresh_git_status();
                }
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
                        self.context_menu.open_for_commit(commit_item, 40, 10);
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
                if self.active_tab == AppTab::Issues
                    && self.focused_pane == FocusedPane::MainContent
                {
                    self.issues_view.toggle_focus();
                } else {
                    self.focused_pane = match self.focused_pane {
                        FocusedPane::DirectoryMap => FocusedPane::MainContent,
                        FocusedPane::MainContent => FocusedPane::DirectoryMap,
                    };
                }
            }
            // Tab switching 1..7 & 'i'
            KeyCode::Char('1') => self.active_tab = AppTab::Changes,
            KeyCode::Char('2') => {
                self.active_tab = AppTab::History;
                self.refresh_history();
            }
            KeyCode::Char('3') => {
                self.active_tab = AppTab::Issues;
                self.load_issues(true);
                self.last_status = Some("Reloading GitHub issues...".to_string());
            }
            KeyCode::Char('4') => self.active_tab = AppTab::Prs,
            KeyCode::Char('5') => self.active_tab = AppTab::Actions,
            KeyCode::Char('6') => self.active_tab = AppTab::Alerts,
            KeyCode::Char('7') => {
                self.active_tab = AppTab::Worktrees;
                self.refresh_worktrees();
            }
            KeyCode::Char('i') => {
                self.active_tab = AppTab::Issues;
                self.load_issues(true);
                self.last_status = Some("Reloading GitHub issues...".to_string());
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
                        AppTab::History => self
                            .history_view
                            .selected_commit()
                            .map(|c| c.full_hash.clone()),
                        AppTab::Issues => self
                            .issues_view
                            .selected_issue()
                            .map(|i| i.html_url.clone()),
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
                        let gh = self.github_service.clone();
                        if self.event_sender.is_some() {
                            tokio::spawn(async move {
                                let _ = gh.mark_notification_read(&id).await;
                            });
                        } else if tokio::runtime::Handle::try_current().is_err() {
                            let _ = block_on_safe(gh.mark_notification_read(&id));
                        }
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

            // 'r': Revert in History tab, Re-run failed CI in Actions, or Reload in Issues
            KeyCode::Char('r') => {
                if self.active_tab == AppTab::History {
                    self.revert_selected_commit();
                } else if self.active_tab == AppTab::Actions {
                    self.rerun_selected_action_job();
                } else if self.active_tab == AppTab::Issues {
                    self.load_issues(true);
                    self.last_status = Some("Reloading GitHub issues...".to_string());
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
                self.last_status = Some(
                    "Refreshed repository status and GitHub data (fetching remote...)".to_string(),
                );
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
                    AppTab::Issues => self.issues_view.previous(),
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
                    AppTab::Issues => self.issues_view.next(),
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

        // 0. Command Palette clicks & wheel
        if self.command_palette.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.command_palette.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.command_palette.next();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.command_palette
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            PaletteClickResult::Close => {
                                self.command_palette.close();
                            }
                            PaletteClickResult::Enter(enter_res) => {
                                self.apply_palette_result(enter_res);
                            }
                            PaletteClickResult::Handled => {}
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 1. Branch Switcher Modal clicks & wheel
        if self.branch_switcher.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.branch_switcher.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.branch_switcher.next();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.branch_switcher
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            BranchSwitcherClickResult::Close => {
                                self.branch_switcher.close();
                            }
                            BranchSwitcherClickResult::Select(target_b) => {
                                self.checkout_selected_branch(&target_b);
                            }
                            BranchSwitcherClickResult::Handled => {}
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 2. Settings Modal clicks & wheel
        if self.settings_modal.is_open {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.settings_modal
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            SettingsAction::SaveAndApply => {
                                self.settings_modal.apply_to_config(&mut self.config);
                                self.config.auto_save_enabled = true;
                                if let Err(e) = self.config.save() {
                                    self.last_status = Some(format!("Save config error: {}", e));
                                } else {
                                    self.theme = Theme::get(self.config.theme);
                                    self.i18n = I18n::new(self.config.language);
                                    self.icons = Icons::new(self.config.use_nerd_font);
                                    self.last_status = Some(
                                        "Saved settings to ~/.config/prowly/config.toml"
                                            .to_string(),
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

        // 3. Open Repo Modal clicks & wheel
        if self.open_repo_modal.is_open {
            match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
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
                            OpenRepoAction::RemoveRecent(path) => {
                                self.config.remove_recent_repository(&path);
                                let _ = self.config.save();
                                self.open_repo_modal.recent_repos.retain(|p| {
                                    p.trim_end_matches('/') != path.trim_end_matches('/')
                                });
                                self.open_repo_modal
                                    .recent_scroll
                                    .set_total_items(self.open_repo_modal.recent_repos.len());
                                if self.open_repo_modal.recent_scroll.selected_index
                                    >= self.open_repo_modal.recent_repos.len()
                                {
                                    self.open_repo_modal.recent_scroll.selected_index =
                                        self.open_repo_modal.recent_repos.len().saturating_sub(1);
                                }
                                self.last_status =
                                    Some(format!("Removed {} from recent repositories", path));
                            }
                            OpenRepoAction::InitRepo(path) => {
                                self.open_repo_modal.close();
                                match GitService::init_repo(&path) {
                                    Ok(()) => {
                                        self.last_status = Some(format!(
                                            "Initialized empty Git repository in {}",
                                            path.display()
                                        ));
                                        self.switch_repository(&path);
                                    }
                                    Err(e) => {
                                        self.last_status =
                                            Some(format!("Failed to initialize repository: {}", e));
                                    }
                                }
                            }
                            OpenRepoAction::CloneRepo { url, target_dir } => {
                                self.open_repo_modal.close();
                                self.last_status = Some(format!("Cloning {} ...", url));
                                match GitService::clone_repo(&url, &target_dir) {
                                    Ok(dest) => {
                                        self.last_status = Some(format!(
                                            "Successfully cloned into {}",
                                            dest.display()
                                        ));
                                        self.switch_repository(&dest);
                                    }
                                    Err(e) => {
                                        self.last_status =
                                            Some(format!("Failed to clone repository: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollUp => {
                    self.open_repo_modal.scroll_up(x, y, self.layout_areas.root);
                }
                MouseEventKind::ScrollDown => {
                    self.open_repo_modal
                        .scroll_down(x, y, self.layout_areas.root);
                }
                _ => {}
            }
            return;
        }

        // 4. History Actions Modal clicks & wheel
        if self.history_modal.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.history_modal.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.history_modal.next();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(result) =
                        self.history_modal
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match result {
                            HistoryModalClickResult::ExecuteAction(action) => {
                                self.execute_history_menu_action(action);
                            }
                            HistoryModalClickResult::ConfirmBranch => {
                                let branch_name =
                                    self.history_modal.new_branch_name.trim().to_string();
                                self.history_modal.close();
                                if !branch_name.is_empty() {
                                    self.create_branch_at_selected_commit(&branch_name);
                                }
                            }
                            HistoryModalClickResult::ConfirmTimeMachine => {
                                self.history_modal.close();
                                self.time_machine_reset_to_selected_commit();
                            }
                            HistoryModalClickResult::Close => {
                                self.history_modal.close();
                            }
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 5. Conflict Resolver Modal clicks & wheel
        if self.conflict_resolver.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.conflict_resolver.previous_hunk();
                }
                MouseEventKind::ScrollDown => {
                    self.conflict_resolver.next_hunk();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.conflict_resolver
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            ConflictResolverClickResult::Close => {
                                self.conflict_resolver.close();
                            }
                            ConflictResolverClickResult::Resolve(_) => {}
                            ConflictResolverClickResult::RequestAi => {
                                self.trigger_ai_merge_conflict();
                            }
                            ConflictResolverClickResult::Save => {
                                self.save_and_stage_resolved_conflict();
                            }
                            ConflictResolverClickResult::Handled => {}
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 6. AI Assistant Modal clicks & wheel
        if self.ai_modal.is_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                if let Some(action) = self.ai_modal.handle_click(x, y, self.layout_areas.root) {
                    match action {
                        AiAssistantClickResult::Close => {
                            self.ai_modal.close();
                        }
                        AiAssistantClickResult::ApplyOrSubmit => {
                            if self.ai_modal.proposed_patch.is_some() && !self.ai_modal.is_loading {
                                self.apply_ai_suggestion();
                            } else if !self.ai_modal.input_query.trim().is_empty() {
                                self.trigger_ai_assistant_query();
                            }
                        }
                        AiAssistantClickResult::Handled => {}
                    }
                }
            }
            return;
        }

        // 7. Remote Diff Preview Modal clicks & wheel
        if self.remote_diff_preview.is_open {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                if let Some(action) =
                    self.remote_diff_preview
                        .handle_click(x, y, self.layout_areas.root)
                {
                    match action {
                        RemoteDiffClickResult::Close => {
                            self.remote_diff_preview.close();
                        }
                        RemoteDiffClickResult::Handled => {}
                    }
                }
            }
            return;
        }

        // 8. Quick Interactive Rebase Modal clicks & wheel
        if self.rebase_modal.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.rebase_modal.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.rebase_modal.next();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.rebase_modal.handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            RebaseClickResult::Close => {
                                self.rebase_modal.close();
                            }
                            RebaseClickResult::SelectCommit(_) => {}
                            RebaseClickResult::Squash => {
                                self.rebase_quick_squash();
                            }
                            RebaseClickResult::Fixup => {
                                self.rebase_fixup_head();
                            }
                            RebaseClickResult::Reword => {
                                self.rebase_modal.is_editing_reword = true;
                                if let Some(c) = self.rebase_modal.selected_commit() {
                                    self.rebase_modal.reword_input = c.summary.clone();
                                }
                            }
                            RebaseClickResult::Handled => {}
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 9. Markdown Preview Modal clicks & wheel
        if self.markdown_preview_modal.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.markdown_preview_modal.scroll_up(3);
                }
                MouseEventKind::ScrollDown => {
                    self.markdown_preview_modal.scroll_down(3);
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) =
                        self.markdown_preview_modal
                            .handle_click(x, y, self.layout_areas.root)
                    {
                        match action {
                            MarkdownPreviewClickResult::Close => {
                                self.markdown_preview_modal.close();
                            }
                            MarkdownPreviewClickResult::Copy => {
                                let _ = self
                                    .clipboard
                                    .copy_text(&self.markdown_preview_modal.raw_content);
                                self.last_status =
                                    Some("Copied Markdown content to clipboard".to_string());
                            }
                            MarkdownPreviewClickResult::Handled => {}
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 10. Context Menu clicks & wheel
        if self.context_menu.is_open {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.context_menu.previous();
                }
                MouseEventKind::ScrollDown => {
                    self.context_menu.next();
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some(action) = self.context_menu.hit_test(x, y, self.layout_areas.root) {
                        self.context_menu.close();
                        self.execute_context_action(action);
                    } else {
                        self.context_menu.close();
                    }
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    self.context_menu.close();
                }
                _ => {}
            }
            return;
        }

        // 11. Right-Click: Open Context Menu
        if mouse.kind == MouseEventKind::Down(MouseButton::Right) {
            if self.active_tab == AppTab::History {
                if self.history_view.handle_list_click(x, y) {
                    self.update_active_history_diff();
                }
                if let Some(commit) = self.history_view.selected_commit().cloned() {
                    self.context_menu.open_for_commit(commit, x, y);
                    return;
                }
            }
            let sel_file = self
                .filtered_files()
                .get(self.changes_view.file_scroll.selected_index)
                .map(|f| f.path.clone());
            self.context_menu.open_at(sel_file, x, y);
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
            if self.active_tab == AppTab::History {
                if let Some(commit) = self.history_view.selected_commit().cloned() {
                    self.context_menu.open_for_commit(commit, x, y + 1);
                    return;
                }
            }
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
                    AppTab::Issues => self.icons.get(crate::ui::IconId::IssueOpen),
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
                    } else if self.active_tab == AppTab::Issues {
                        self.load_issues(true);
                        self.last_status = Some("Reloading GitHub issues...".to_string());
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
                            if self.history_view.handle_list_click(x, y) {
                                self.update_active_history_diff();
                            } else if self.history_view.hits_diff_pane(x, y) {
                                self.history_view.focus =
                                    crate::ui::views::history::HistoryFocus::DiffPreview;
                            }
                        }
                        AppTab::Worktrees => {
                            self.worktrees_view.scroll.handle_click_row(rel_y);
                        }
                        AppTab::Prs => {
                            self.prs_view.list_scroll.handle_click_row(rel_y);
                        }
                        AppTab::Issues => {
                            self.issues_view.list_scroll.handle_click_row(rel_y);
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
                            } else if let Some(btn_action) = self
                                .changes_view
                                .hit_test_action_buttons(x, y, self.icons.use_nerd_font)
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
                                if self
                                    .changes_view
                                    .file_scroll
                                    .handle_click_row(rel_y)
                                    .is_some()
                                {
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
                        if self.history_view.hits_diff_pane(x, y) {
                            self.history_view.scroll_diff_down(3);
                        } else {
                            self.history_view.scroll.scroll_down(3);
                            self.update_active_history_diff();
                        }
                    }
                    AppTab::Worktrees => self.worktrees_view.scroll.scroll_down(3),
                    AppTab::Prs => self.prs_view.list_scroll.scroll_down(3),
                    AppTab::Issues => self.issues_view.list_scroll.scroll_down(3),
                    AppTab::Actions => self.actions_view.list_scroll.scroll_down(3),
                    AppTab::Alerts => self.alerts_view.scroll.scroll_down(3),
                    AppTab::Changes => {
                        self.changes_view.file_scroll.scroll_down(3);
                        self.update_active_diff();
                    }
                },
                MouseEventKind::ScrollUp => match self.active_tab {
                    AppTab::History => {
                        if self.history_view.hits_diff_pane(x, y) {
                            self.history_view.scroll_diff_up(3);
                        } else {
                            self.history_view.scroll.scroll_up(3);
                            self.update_active_history_diff();
                        }
                    }
                    AppTab::Worktrees => self.worktrees_view.scroll.scroll_up(3),
                    AppTab::Prs => self.prs_view.list_scroll.scroll_up(3),
                    AppTab::Issues => self.issues_view.list_scroll.scroll_up(3),
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
}
