//! Git Worktree management for multi-branch concurrent workflow.

use crate::git::types::GitError;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Representation of an active Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeItem {
    pub name: String,
    pub path: PathBuf,
    pub head_branch: Option<String>,
    pub is_current: bool,
    pub is_locked: bool,
}

/// Retrieve all linked worktrees and main worktree.
pub fn list_worktrees(repo_dir: &Path) -> Result<Vec<WorktreeItem>, GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_locked = false;

    for line in text.lines() {
        if line.starts_with("worktree ") {
            if let Some(p) = current_path.take() {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "worktree".to_string());
                let is_curr = p == repo_dir;
                items.push(WorktreeItem {
                    name,
                    path: p,
                    head_branch: current_branch.take(),
                    is_current: is_curr,
                    is_locked,
                });
                is_locked = false;
            }
            let raw_path = line.trim_start_matches("worktree ").trim();
            current_path = Some(PathBuf::from(raw_path));
        } else if line.starts_with("branch ") {
            let raw_branch = line.trim_start_matches("branch ").trim();
            let branch_short = raw_branch.trim_start_matches("refs/heads/").to_string();
            current_branch = Some(branch_short);
        } else if line.starts_with("locked") {
            is_locked = true;
        }
    }

    if let Some(p) = current_path {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "worktree".to_string());
        let is_curr = p == repo_dir;
        items.push(WorktreeItem {
            name,
            path: p,
            head_branch: current_branch,
            is_current: is_curr,
            is_locked,
        });
    }

    // Fallback if worktree command returned empty
    if items.is_empty() {
        items.push(WorktreeItem {
            name: "main".to_string(),
            path: repo_dir.to_path_buf(),
            head_branch: Some("main".to_string()),
            is_current: true,
            is_locked: false,
        });
    }

    Ok(items)
}

/// Create a new Git worktree targeting a branch.
pub fn create_worktree(
    repo_dir: &Path,
    target_path: &Path,
    branch: Option<&str>,
) -> Result<WorktreeItem, GitError> {
    let mut args = vec!["worktree", "add"];
    let path_str = target_path.to_string_lossy();
    args.push(&path_str);

    if let Some(b) = branch {
        let branch_check = Command::new("git")
            .current_dir(repo_dir)
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", b)])
            .output()?;
        if !branch_check.status.success() {
            args.push("-b");
        }
        args.push(b);
    }

    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(&args)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    let name = target_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "worktree".to_string());

    Ok(WorktreeItem {
        name,
        path: target_path.to_path_buf(),
        head_branch: branch.map(|s| s.to_string()),
        is_current: false,
        is_locked: false,
    })
}

/// Safely remove a Git worktree.
pub fn remove_worktree(
    repo_dir: &Path,
    worktree_name_or_path: &str,
    force: bool,
) -> Result<(), GitError> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(worktree_name_or_path);

    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(&args)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    Ok(())
}
