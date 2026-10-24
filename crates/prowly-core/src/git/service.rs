//! Git hybrid service engine combining in-process libgit2 queries with Git CLI.

use crate::git::clean_git_command;
use crate::git::types::{
    DiffHunk, DiffLine, DiffLineType, FileDiff, FileStatus, GitError, GitFileItem, GitRepoStatus,
    RepoOperationState,
};
use crate::services::{BackupRecord, BackupService};
use git2::{DiffOptions, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

/// Main Git hybrid controller.
pub struct GitService {
    repo: Repository,
    workdir: PathBuf,
}

impl GitService {
    /// Open a repository at the specified path.
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::open(path)?;
        let workdir = repo
            .workdir()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.to_path_buf());
        Ok(Self { repo, workdir })
    }

    /// Discover and open the nearest Git repository starting from current working directory.
    pub fn discover() -> Result<Self, GitError> {
        let current_dir = std::env::current_dir()?;
        let repo = Repository::discover(&current_dir)?;
        let workdir = repo
            .workdir()
            .map(|p| p.to_path_buf())
            .unwrap_or(current_dir);
        Ok(Self { repo, workdir })
    }

    /// Working directory path of the repository.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Retrieve URL of specified remote (defaults to "origin").
    pub fn remote_url(&self, name: Option<&str>) -> Option<String> {
        let remote_name = name.unwrap_or("origin");
        self.repo
            .find_remote(remote_name)
            .ok()?
            .url()
            .map(|s| s.to_string())
    }

    /// Retrieve full repository status including branch, upstream sync, and modified files.
    pub fn get_status(&self) -> Result<GitRepoStatus, GitError> {
        // 1. Current Branch & Upstream Sync
        let mut branch_name = "HEAD".to_string();
        let mut upstream_name = None;
        let mut ahead = 0;
        let mut behind = 0;

        if let Ok(head) = self.repo.head() {
            if let Some(name) = head.shorthand() {
                branch_name = name.to_string();
            }

            let local_oid = head
                .peel_to_commit()
                .map(|c| c.id())
                .ok()
                .or_else(|| head.target());

            if let Ok(local_branch) = self.repo.find_branch(&branch_name, git2::BranchType::Local) {
                let upstream_branch = local_branch.upstream().ok().or_else(|| {
                    self.repo
                        .find_branch(&format!("origin/{}", branch_name), git2::BranchType::Remote)
                        .ok()
                });

                if let Some(upstream) = upstream_branch {
                    if let Ok(Some(up_name)) = upstream.name() {
                        upstream_name = Some(up_name.to_string());
                    }

                    let up_oid = upstream
                        .get()
                        .peel_to_commit()
                        .map(|c| c.id())
                        .ok()
                        .or_else(|| upstream.get().target());

                    if let (Some(l_oid), Some(u_oid)) = (local_oid, up_oid) {
                        if let Ok((a, b)) = self.repo.graph_ahead_behind(l_oid, u_oid) {
                            ahead = a;
                            behind = b;
                        }
                    }
                }
            }

            // CLI Fallback: If ahead/behind couldn't be determined or upstream was not found,
            // query git CLI which natively understands tracking configurations
            if upstream_name.is_none() || (ahead == 0 && behind == 0) {
                if let Ok(cli_res) =
                    self.run_git_cli(&["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
                {
                    let parts: Vec<&str> = cli_res.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) =
                            (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                        {
                            ahead = a;
                            behind = b;
                        }
                    }
                } else if let Ok(cli_res) = self.run_git_cli(&[
                    "rev-list",
                    "--left-right",
                    "--count",
                    &format!("HEAD...origin/{}", branch_name),
                ]) {
                    let parts: Vec<&str> = cli_res.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) =
                            (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                        {
                            ahead = a;
                            behind = b;
                            if upstream_name.is_none() {
                                upstream_name = Some(format!("origin/{}", branch_name));
                            }
                        }
                    }
                }
            }
        }

        // 2. File Statuses
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(false)
            .include_ignored(false);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut files = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status_flags = entry.status();

            for item in classify_git_status_items(&path, status_flags) {
                files.push(item);
            }
        }

        let operation_state = self.get_operation_state();

        Ok(GitRepoStatus {
            branch: branch_name,
            upstream: upstream_name,
            ahead,
            behind,
            files,
            operation_state,
        })
    }

    /// List all repository files (both clean and modified).
    pub fn list_all_repo_files(&self) -> Result<Vec<GitFileItem>, GitError> {
        let status = self.get_status()?;
        let status_map: std::collections::HashMap<String, (bool, FileStatus)> = status
            .files
            .into_iter()
            .map(|f| (f.path, (f.staged, f.status)))
            .collect();

        let mut paths = std::collections::BTreeSet::new();

        // 1. Files in git index
        if let Ok(index) = self.repo.index() {
            for entry in index.iter() {
                if let Ok(path) = std::str::from_utf8(&entry.path) {
                    paths.insert(path.to_string());
                }
            }
        }

        // 2. Any changed or untracked files
        for k in status_map.keys() {
            paths.insert(k.clone());
        }

        let mut items = Vec::new();
        for path in paths {
            if let Some(&(staged, s)) = status_map.get(&path) {
                items.push(GitFileItem {
                    path,
                    staged,
                    status: s,
                });
            } else {
                items.push(GitFileItem {
                    path,
                    staged: false,
                    status: FileStatus::Clean,
                });
            }
        }
        Ok(items)
    }

    /// Recursively discover project directories (up to depth 4, ignoring .git, target, node_modules).
    pub fn list_all_directories(&self) -> Vec<String> {
        let mut dirs = std::collections::BTreeSet::new();
        Self::scan_dirs_recursive(&self.workdir, "", 0, &mut dirs);
        dirs.into_iter().collect()
    }

    fn scan_dirs_recursive(
        base: &Path,
        rel: &str,
        depth: usize,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        if depth > 4 {
            return;
        }
        let current_path = if rel.is_empty() {
            base.to_path_buf()
        } else {
            base.join(rel)
        };
        if let Ok(entries) = std::fs::read_dir(current_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.')
                            || name == "target"
                            || name == "node_modules"
                            || name == "vendor"
                        {
                            continue;
                        }
                        let next_rel = if rel.is_empty() {
                            format!("{}/", name)
                        } else {
                            format!("{}{}/", rel, name)
                        };
                        out.insert(next_rel.clone());
                        Self::scan_dirs_recursive(base, &next_rel, depth + 1, out);
                    }
                }
            }
        }
    }

    /// Stage a file into the index.
    pub fn stage_file(&self, path: &str) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        let file_path = Path::new(path);
        if self.workdir.join(file_path).exists() {
            index.add_path(file_path)?;
        } else {
            // Deleted file
            index.remove_path(file_path)?;
        }
        index.write()?;
        Ok(())
    }

    /// Unstage a file from the index.
    pub fn unstage_file(&self, path: &str) -> Result<(), GitError> {
        if let Ok(head) = self.repo.head() {
            if let Ok(obj) = head.peel(git2::ObjectType::Commit) {
                self.repo.reset_default(Some(&obj), [path])?;
                return Ok(());
            }
        }
        // Fallback for initial commit without HEAD
        let mut index = self.repo.index()?;
        index.remove_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    /// Stage all modified and untracked files.
    pub fn stage_all(&self) -> Result<(), GitError> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"].iter(), None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage all files.
    pub fn unstage_all(&self) -> Result<(), GitError> {
        if let Ok(head) = self.repo.head() {
            if let Ok(obj) = head.peel(git2::ObjectType::Commit) {
                self.repo.reset_default(Some(&obj), ["*"])?;
                return Ok(());
            }
        }
        let mut index = self.repo.index()?;
        index.clear()?;
        index.write()?;
        Ok(())
    }

    /// Validate that a path is a safe relative path strictly contained within the repository worktree.
    /// Rejects parent directory traversal (`..`), absolute paths, and paths resolving outside the workdir.
    pub fn validate_relative_repo_path(&self, rel_path: &str) -> Result<PathBuf, GitError> {
        let p = Path::new(rel_path);
        if rel_path.trim().is_empty() {
            return Err(GitError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "File path cannot be empty",
            )));
        }
        if p.is_absolute() {
            return Err(GitError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Absolute paths outside repository are not permitted",
            )));
        }
        for component in p.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(GitError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Parent directory traversal ('..') is not permitted",
                    )));
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err(GitError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Root directory or volume prefix is not permitted",
                    )));
                }
                _ => {}
            }
        }

        let canonical_workdir = self.workdir.canonicalize().map_err(GitError::Io)?;
        let full_path = self.workdir.join(p);

        // Fail-closed verification:
        // 1. If file exists, ensure canonical path is strictly inside canonical workdir.
        // 2. If path is a symlink (including broken/dangling), fail closed (reject).
        // 3. If file does not exist, resolve nearest existing ancestor directory and verify it is inside canonical workdir.
        // 4. Any error in canonicalization fails closed immediately.
        if full_path.exists() {
            let canonical_full = full_path.canonicalize().map_err(GitError::Io)?;
            if !canonical_full.starts_with(&canonical_workdir) {
                return Err(GitError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Path escapes repository workdir via symlink or redirection",
                )));
            }
        } else if full_path.is_symlink() {
            return Err(GitError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Broken or dangling symlink cannot be safely verified within repository workdir",
            )));
        } else {
            let mut ancestor = full_path.as_path();
            while !ancestor.exists() {
                if let Some(parent) = ancestor.parent() {
                    ancestor = parent;
                } else {
                    break;
                }
            }
            if ancestor.exists() {
                let canonical_ancestor = ancestor.canonicalize().map_err(GitError::Io)?;
                if !canonical_ancestor.starts_with(&canonical_workdir) {
                    return Err(GitError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Parent directory escapes repository workdir via symlink",
                    )));
                }
            }
        }

        Ok(full_path)
    }

    /// Discard modifications for a file safely with an automated soft backup.
    pub fn soft_discard(
        &self,
        path: &str,
        backup_service: &BackupService,
    ) -> Result<BackupRecord, GitError> {
        let full_path = self.validate_relative_repo_path(path)?;
        let record = backup_service.backup_file(&full_path)?;
        self.apply_discard_to_worktree_and_index(path, None)?;
        Ok(record)
    }

    /// Discard targeted staged or unstaged modifications for a file safely with an automated soft backup.
    pub fn soft_discard_targeted(
        &self,
        path: &str,
        staged: bool,
        backup_service: &BackupService,
    ) -> Result<BackupRecord, GitError> {
        let full_path = self.validate_relative_repo_path(path)?;
        let record = backup_service.backup_file(&full_path)?;
        self.apply_discard_to_worktree_and_index(path, Some(staged))?;
        Ok(record)
    }

    /// Discard modifications for a file directly without creating a backup.
    pub fn hard_discard(&self, path: &str) -> Result<(), GitError> {
        self.apply_discard_to_worktree_and_index(path, None)
    }

    /// Discard targeted staged or unstaged modifications for a file directly.
    pub fn hard_discard_targeted(&self, path: &str, staged: bool) -> Result<(), GitError> {
        self.apply_discard_to_worktree_and_index(path, Some(staged))
    }

    /// Internal helper that cleanly discards changes based on Git status (untracked, staged, unstaged, deleted).
    fn apply_discard_to_worktree_and_index(
        &self,
        path: &str,
        staged_target: Option<bool>,
    ) -> Result<(), GitError> {
        let full_path = self.validate_relative_repo_path(path)?;
        let status = self
            .repo
            .status_file(Path::new(path))
            .unwrap_or(git2::Status::empty());

        let in_head = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_tree().ok())
            .map(|t| t.get_path(Path::new(path)).is_ok())
            .unwrap_or(false);

        let mut opts = git2::build::CheckoutBuilder::new();
        opts.force().path(path);

        match staged_target {
            Some(false) => {
                // User explicitly requested to discard UNSTAGED modifications only
                if status.contains(git2::Status::WT_NEW)
                    && !status.intersects(git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED)
                {
                    // Untracked file
                    if full_path.is_file() {
                        std::fs::remove_file(&full_path)?;
                    } else if full_path.is_dir() {
                        std::fs::remove_dir_all(&full_path)?;
                    }
                } else if status.intersects(
                    git2::Status::WT_MODIFIED
                        | git2::Status::WT_DELETED
                        | git2::Status::WT_RENAMED
                        | git2::Status::WT_TYPECHANGE,
                ) {
                    // Restore worktree from index to preserve whatever was staged
                    self.repo.checkout_index(None, Some(&mut opts))?;
                }
                Ok(())
            }
            Some(true) => {
                // User explicitly requested to discard STAGED modifications
                if in_head {
                    self.unstage_file(path)?;
                    self.repo.checkout_head(Some(&mut opts))?;
                } else {
                    self.unstage_file(path)?;
                    if full_path.is_file() {
                        std::fs::remove_file(&full_path)?;
                    } else if full_path.is_dir() {
                        std::fs::remove_dir_all(&full_path)?;
                    }
                }
                Ok(())
            }
            None => {
                // Full discard (both staged and unstaged)
                if !in_head {
                    if status.intersects(
                        git2::Status::INDEX_NEW
                            | git2::Status::INDEX_MODIFIED
                            | git2::Status::INDEX_DELETED
                            | git2::Status::INDEX_RENAMED
                            | git2::Status::INDEX_TYPECHANGE,
                    ) {
                        self.unstage_file(path)?;
                    }

                    if full_path.is_file() {
                        std::fs::remove_file(&full_path)?;
                    } else if full_path.is_dir() {
                        std::fs::remove_dir_all(&full_path)?;
                    }
                    return Ok(());
                }

                if status.intersects(
                    git2::Status::INDEX_NEW
                        | git2::Status::INDEX_MODIFIED
                        | git2::Status::INDEX_DELETED
                        | git2::Status::INDEX_RENAMED
                        | git2::Status::INDEX_TYPECHANGE,
                ) {
                    self.unstage_file(path)?;
                }

                self.repo.checkout_head(Some(&mut opts))?;
                Ok(())
            }
        }
    }

    /// Compute structured diff for a file (or whole repo if path is None).
    pub fn get_diff(&self, file_path: Option<&str>, staged: bool) -> Result<FileDiff, GitError> {
        let mut opts = DiffOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(false);
        if let Some(p) = file_path {
            opts.pathspec(p);
        }

        let diff = if staged {
            let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            self.repo
                .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?
        } else {
            self.repo.diff_index_to_workdir(None, Some(&mut opts))?
        };

        let mut file_diff = FileDiff {
            file_path: file_path.unwrap_or("diff").to_string(),
            ..Default::default()
        };

        let mut current_hunk: Option<DiffHunk> = None;

        diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
            if let Some(h) = hunk {
                let header = String::from_utf8_lossy(h.header()).trim().to_string();
                if current_hunk.as_ref().map(|curr| &curr.header) != Some(&header) {
                    if let Some(prev) = current_hunk.take() {
                        file_diff.hunks.push(prev);
                    }
                    current_hunk = Some(DiffHunk {
                        header,
                        lines: Vec::new(),
                    });
                }
            }

            let origin = line.origin();
            let line_type = match origin {
                '+' => {
                    file_diff.stats_added += 1;
                    DiffLineType::Added
                }
                '-' => {
                    file_diff.stats_deleted += 1;
                    DiffLineType::Deleted
                }
                'F' | 'H' => DiffLineType::HunkHeader,
                _ => DiffLineType::Context,
            };

            let content = String::from_utf8_lossy(line.content())
                .trim_end_matches(['\n', '\r'])
                .to_string();

            let diff_line = DiffLine {
                origin,
                content,
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
                line_type,
            };

            if let Some(hunk_ref) = current_hunk.as_mut() {
                hunk_ref.lines.push(diff_line);
            }

            true
        })?;

        if let Some(h) = current_hunk {
            file_diff.hunks.push(h);
        }

        Ok(file_diff)
    }

    /// Create a commit with the staged index using the Git CLI, ensuring hooks (pre-commit, commit-msg)
    /// and commit.gpgsign signatures are fully respected.
    pub fn commit(&self, message: &str) -> Result<git2::Oid, GitError> {
        let trimmed_msg = message.trim();
        if trimmed_msg.is_empty() {
            return Err(GitError::Cli("Commit message cannot be empty".to_string()));
        }

        let mut child = crate::git::clean_git_command(self.workdir())
            .args(["commit", "-F", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(message.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let msg = if !stderr.is_empty() { stderr } else { stdout };
            return Err(GitError::Cli(msg));
        }

        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id())
    }

    /// Commit staged changes with summary and optional description.
    pub fn commit_with_description(
        &self,
        summary: &str,
        description: Option<&str>,
    ) -> Result<git2::Oid, GitError> {
        let full_message = match description {
            Some(desc) if !desc.trim().is_empty() => {
                format!("{}\n\n{}", summary.trim(), desc.trim())
            }
            _ => summary.trim().to_string(),
        };
        self.commit(&full_message)
    }

    /// List all local branch names.
    pub fn list_branches(&self) -> Result<Vec<String>, GitError> {
        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        let mut list = Vec::new();
        for b in branches {
            let (branch, _) = b?;
            if let Some(name) = branch.name()? {
                list.push(name.to_string());
            }
        }
        Ok(list)
    }

    /// Checkout a local or remote tracking branch.
    pub fn checkout_branch(&self, branch_name: &str) -> Result<(), GitError> {
        self.run_git_cli(&["checkout", branch_name])?;
        Ok(())
    }

    /// Get unpushed and unpulled commit subject lines relative to upstream.
    pub fn get_unpushed_unpulled_commits(&self) -> Result<(Vec<String>, Vec<String>), GitError> {
        let status = self.get_status()?;
        let unpushed = if status.ahead > 0 {
            if let Some(ref upstream) = status.upstream {
                let range = format!("{}..HEAD", upstream);
                let out = self.run_git_cli(&["log", "--oneline", "-n", "10", &range])?;
                out.lines().map(|s| s.to_string()).collect()
            } else {
                let out = self.run_git_cli(&["log", "--oneline", "-n", "10"])?;
                out.lines().map(|s| s.to_string()).collect()
            }
        } else {
            Vec::new()
        };

        let unpulled = if status.behind > 0 {
            if let Some(ref upstream) = status.upstream {
                let range = format!("HEAD..{}", upstream);
                let out = self.run_git_cli(&["log", "--oneline", "-n", "10", &range])?;
                out.lines().map(|s| s.to_string()).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok((unpushed, unpulled))
    }

    /// Initialize a new Git repository in the working directory.
    pub fn init_repo(dir: &std::path::Path) -> Result<(), GitError> {
        let output = clean_git_command(dir).args(["init"]).output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::Cli(err))
        }
    }

    /// Clone a remote repository into `target_dir`.
    /// Derives the directory name from the remote URL and returns the cloned repository PathBuf.
    pub fn clone_repo(
        url: &str,
        target_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, GitError> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(GitError::Cli("Remote URL cannot be empty".to_string()));
        }

        let name = trimmed
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit(['/', ':'])
            .next()
            .unwrap_or("repo");

        let dest = target_dir.join(name);

        let output = clean_git_command(target_dir)
            .args(["clone", trimmed, name])
            .output()?;

        if output.status.success() {
            Ok(dest)
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::Cli(err))
        }
    }

    /// Retrieve configured default remote name (first remote, falling back to "origin").
    pub fn default_remote_name(&self) -> String {
        if let Ok(remotes) = self.repo.remotes() {
            if let Some(first) = remotes.get(0) {
                return first.to_string();
            }
        }
        "origin".to_string()
    }

    /// Check whether an interactive operation (Rebase, Merge, Cherry-pick, Revert) is currently in progress.
    pub fn get_operation_state(&self) -> RepoOperationState {
        let git_dir = self.repo.path();
        if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
            RepoOperationState::RebaseInProgress
        } else if git_dir.join("MERGE_HEAD").exists() {
            RepoOperationState::MergeInProgress
        } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
            RepoOperationState::CherryPickInProgress
        } else if git_dir.join("REVERT_HEAD").exists() {
            RepoOperationState::RevertInProgress
        } else {
            RepoOperationState::None
        }
    }

    /// Safely abort any in-progress operation (rebase, merge, cherry-pick, revert) restoring original state.
    pub fn abort_current_operation(&self) -> Result<String, GitError> {
        match self.get_operation_state() {
            RepoOperationState::RebaseInProgress => self.run_git_cli(&["rebase", "--abort"]),
            RepoOperationState::MergeInProgress => self.run_git_cli(&["merge", "--abort"]),
            RepoOperationState::CherryPickInProgress => {
                self.run_git_cli(&["cherry-pick", "--abort"])
            }
            RepoOperationState::RevertInProgress => self.run_git_cli(&["revert", "--abort"]),
            RepoOperationState::None => Err(GitError::Cli(
                "No in-progress operation to abort".to_string(),
            )),
        }
    }

    /// Invoke Git CLI for push (ensuring SSH agent / Credential Helper / GPG compatibility).
    pub fn cli_push(&self) -> Result<String, GitError> {
        match self.run_git_cli(&["push"]) {
            Ok(output) => Ok(output),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("no upstream") || err_str.contains("set-upstream") {
                    if let Ok(head) = self.repo.head() {
                        if let Some(branch) = head.shorthand() {
                            let remote = self.default_remote_name();
                            return self.run_git_cli(&["push", "-u", &remote, branch]);
                        }
                    }
                }
                Err(e)
            }
        }
    }

    /// Invoke Git CLI to fetch all remotes.
    pub fn cli_fetch(&self) -> Result<String, GitError> {
        self.run_git_cli(&["fetch", "--all", "--prune"])
    }

    /// Invoke Git CLI for pull.
    pub fn cli_pull(&self) -> Result<String, GitError> {
        match self.run_git_cli(&["pull"]) {
            Ok(output) => Ok(output),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("no tracking information") || err_str.contains("set-upstream") {
                    if let Ok(head) = self.repo.head() {
                        if let Some(branch) = head.shorthand() {
                            let remote = self.default_remote_name();
                            return self.run_git_cli(&["pull", &remote, branch]);
                        }
                    }
                }
                Err(e)
            }
        }
    }

    /// Save working tree changes to stash via Git CLI.
    pub fn cli_stash_save(&self, message: &str) -> Result<String, GitError> {
        self.run_git_cli(&["stash", "push", "-m", message])
    }

    /// Restore and remove the latest stash via Git CLI.
    pub fn cli_stash_pop(&self) -> Result<String, GitError> {
        self.run_git_cli(&["stash", "pop"])
    }

    /// Helper to run git command in the repository working directory with timeout guard.
    fn run_git_cli(&self, args: &[&str]) -> Result<String, GitError> {
        let output = crate::git::clean_git_run(&self.workdir, args)?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let msg = if !stdout.is_empty() {
                stdout
            } else if !stderr.is_empty() {
                stderr
            } else {
                "Success".to_string()
            };
            Ok(msg)
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::Cli(err))
        }
    }
}

/// Classify git status flags into discrete staged and unstaged items, supporting dual-state (partially staged) files.
pub fn classify_git_status_items(path: &str, status: Status) -> Vec<GitFileItem> {
    if status.is_conflicted() {
        return vec![GitFileItem {
            path: path.to_string(),
            staged: false,
            status: FileStatus::Conflicted,
        }];
    }

    let has_index = status.is_index_new()
        || status.is_index_modified()
        || status.is_index_deleted()
        || status.is_index_renamed()
        || status.is_index_typechange();

    let has_worktree = status.is_wt_new()
        || status.is_wt_modified()
        || status.is_wt_deleted()
        || status.is_wt_renamed()
        || status.is_wt_typechange();

    let mut items = Vec::with_capacity(2);

    if has_index {
        let file_status = if status.is_index_new() {
            FileStatus::Added
        } else if status.is_index_deleted() {
            FileStatus::Deleted
        } else if status.is_index_renamed() {
            FileStatus::Renamed
        } else {
            FileStatus::Modified
        };
        items.push(GitFileItem {
            path: path.to_string(),
            staged: true,
            status: file_status,
        });
    }

    if has_worktree {
        let file_status = if status.is_wt_new() {
            FileStatus::Untracked
        } else if status.is_wt_deleted() {
            FileStatus::Deleted
        } else if status.is_wt_renamed() {
            FileStatus::Renamed
        } else {
            FileStatus::Modified
        };
        items.push(GitFileItem {
            path: path.to_string(),
            staged: false,
            status: file_status,
        });
    }

    if items.is_empty() {
        items.push(GitFileItem {
            path: path.to_string(),
            staged: false,
            status: FileStatus::Clean,
        });
    }

    items
}
