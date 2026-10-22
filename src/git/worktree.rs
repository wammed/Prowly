//! Git Worktree management for multi-branch concurrent workflow.

use crate::git::types::GitError;
use std::path::{Path, PathBuf};

/// Representation of an active Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeItem {
    pub name: String,
    pub path: PathBuf,
    pub head_branch: Option<String>,
    pub is_current: bool,
    pub is_locked: bool,
}

use crate::git::clean_git_command;

/// Retrieve all linked worktrees and main worktree.
pub fn list_worktrees(repo_dir: &Path) -> Result<Vec<WorktreeItem>, GitError> {
    let output = clean_git_command(repo_dir)
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

    let repo_dir_canon = repo_dir.canonicalize().ok();
    let is_same_path = |p: &Path| -> bool {
        if p == repo_dir {
            return true;
        }
        if let Some(ref r_canon) = repo_dir_canon {
            if p == r_canon {
                return true;
            }
            if let Ok(p_canon) = p.canonicalize() {
                if &p_canon == r_canon || p_canon == repo_dir {
                    return true;
                }
            }
        } else if let Ok(p_canon) = p.canonicalize() {
            if p_canon == repo_dir {
                return true;
            }
        }
        false
    };

    let mut push_item = |p: PathBuf, branch: Option<String>, locked: bool| {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "worktree".to_string());
        let is_curr = is_same_path(&p);
        items.push(WorktreeItem {
            name,
            path: p,
            head_branch: branch,
            is_current: is_curr,
            is_locked: locked,
        });
    };

    for line in text.lines() {
        if line.starts_with("worktree ") {
            if let Some(p) = current_path.take() {
                push_item(p, current_branch.take(), is_locked);
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
        push_item(p, current_branch, is_locked);
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
    } else if !items.iter().any(|wt| wt.is_current) {
        // If path comparison didn't match any worktree (e.g. symlink edge case),
        // fallback to marking the first (main) worktree as current.
        items[0].is_current = true;
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
        let branch_check = clean_git_command(repo_dir)
            .args(["rev-parse", "--verify", &format!("refs/heads/{}", b)])
            .output()?;
        if !branch_check.status.success() {
            args.push("-b");
        }
        args.push(b);
    }

    let output = clean_git_command(repo_dir).args(&args).output()?;

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

    let output = clean_git_command(repo_dir).args(&args).output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    Ok(())
}
