//! Git hybrid service engine combining in-process libgit2 queries with Git CLI.

use crate::git::types::{
    DiffHunk, DiffLine, DiffLineType, FileDiff, FileStatus, GitError, GitFileItem, GitRepoStatus,
};
use crate::services::{BackupRecord, BackupService};
use git2::{DiffOptions, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

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

            let (staged, file_status) = classify_git_status(status_flags);
            files.push(GitFileItem {
                path,
                staged,
                status: file_status,
            });
        }

        Ok(GitRepoStatus {
            branch: branch_name,
            upstream: upstream_name,
            ahead,
            behind,
            files,
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

    /// Discard modifications for a file safely with an automated soft backup.
    pub fn soft_discard(
        &self,
        path: &str,
        backup_service: &BackupService,
    ) -> Result<BackupRecord, GitError> {
        let full_path = self.workdir.join(path);
        let record = backup_service.backup_file(&full_path)?;
        self.apply_discard_to_worktree_and_index(path)?;
        Ok(record)
    }

    /// Discard modifications for a file directly without creating a backup.
    pub fn hard_discard(&self, path: &str) -> Result<(), GitError> {
        self.apply_discard_to_worktree_and_index(path)
    }

    /// Internal helper that cleanly discards changes based on Git status (untracked, staged, unstaged, deleted).
    /// Internal helper that cleanly discards changes based on Git status (untracked, staged, unstaged, deleted).
    fn apply_discard_to_worktree_and_index(&self, path: &str) -> Result<(), GitError> {
        let full_path = self.workdir.join(path);
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

        if !in_head {
            // File is not in HEAD (it is newly added or untracked)
            if status.intersects(
                git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE,
            ) {
                let _ = self.unstage_file(path);
            }

            if full_path.is_file() {
                let _ = std::fs::remove_file(&full_path);
            } else if full_path.is_dir() {
                let _ = std::fs::remove_dir_all(&full_path);
            }
            return Ok(());
        }

        // File is in HEAD: unstage any index changes and restore HEAD version to worktree
        if status.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            let _ = self.unstage_file(path);
        }

        let mut opts = git2::build::CheckoutBuilder::new();
        opts.force().path(path);
        self.repo.checkout_head(Some(&mut opts))?;

        Ok(())
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

    /// Create a commit with the staged index.
    pub fn commit(&self, message: &str) -> Result<git2::Oid, GitError> {
        let mut index = self.repo.index()?;
        let tree_oid = index.write_tree()?;
        index.write()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let sig = self.repo.signature()?;
        let head = self.repo.head();

        let parents: Vec<git2::Commit> = match head {
            Ok(ref h) => match h.peel_to_commit() {
                Ok(c) => vec![c],
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        };

        // Guard against creating accidental empty commits
        if let Some(parent) = parents.first() {
            if parent.tree_id() == tree_oid {
                return Err(GitError::Cli("No staged changes to commit".to_string()));
            }
        }

        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        let commit_oid =
            self.repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)?;

        Ok(commit_oid)
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
        let output = Command::new("git")
            .current_dir(dir)
            .args(["init"])
            .output()?;
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
            .rsplit(|c| c == '/' || c == ':')
            .next()
            .unwrap_or("repo");

        let dest = target_dir.join(name);

        let output = Command::new("git")
            .current_dir(target_dir)
            .args(["clone", trimmed, name])
            .output()?;

        if output.status.success() {
            Ok(dest)
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::Cli(err))
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
                            return self.run_git_cli(&["push", "-u", "origin", branch]);
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
        self.run_git_cli(&["pull"])
    }

    /// Save working tree changes to stash via Git CLI.
    pub fn cli_stash_save(&self, message: &str) -> Result<String, GitError> {
        self.run_git_cli(&["stash", "push", "-m", message])
    }

    /// Restore and remove the latest stash via Git CLI.
    pub fn cli_stash_pop(&self) -> Result<String, GitError> {
        self.run_git_cli(&["stash", "pop"])
    }

    /// Helper to run git command in the repository working directory.
    fn run_git_cli(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .current_dir(&self.workdir)
            .args(args)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(GitError::Cli(err))
        }
    }
}

fn classify_git_status(status: Status) -> (bool, FileStatus) {
    let is_staged = status.is_index_new()
        || status.is_index_modified()
        || status.is_index_deleted()
        || status.is_index_renamed()
        || status.is_index_typechange();

    let file_status = if status.is_conflicted() {
        FileStatus::Conflicted
    } else if status.is_wt_new() {
        FileStatus::Untracked
    } else if status.is_index_new() {
        FileStatus::Added
    } else if status.is_wt_deleted() || status.is_index_deleted() {
        FileStatus::Deleted
    } else if status.is_index_renamed() || status.is_wt_renamed() {
        FileStatus::Renamed
    } else {
        FileStatus::Modified
    };

    (is_staged, file_status)
}
