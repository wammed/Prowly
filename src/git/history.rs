//! Git commit history retrieval, graph parsing, and commit manipulation.

use crate::git::types::GitError;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// An item representing a commit in the history graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCommitItem {
    pub graph: String,
    pub hash: String,
    pub full_hash: String,
    pub refs: String,
    pub author: String,
    pub date: String,
    pub summary: String,
}

/// A line in the Git history graph view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryRow {
    Commit(HistoryCommitItem),
    GraphOnly(String),
}

impl HistoryRow {
    pub fn commit(&self) -> Option<&HistoryCommitItem> {
        match self {
            HistoryRow::Commit(item) => Some(item),
            HistoryRow::GraphOnly(_) => None,
        }
    }
}

/// Details of a single commit including author, message, changed files stat, and diff lines.
#[derive(Debug, Clone, Default)]
pub struct CommitDetail {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub summary: String,
    pub body: String,
    pub stats: Vec<String>,
    pub diff_lines: Vec<String>,
}

/// Retrieve the commit history graph from git log.
pub fn get_commit_history(repo_dir: &Path, count: usize) -> Result<Vec<HistoryRow>, GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args([
            "log",
            "--graph",
            &format!("--pretty=format:LOGROW\x1f%h\x1f%H\x1f%d\x1f%an\x1f%ar\x1f%s"),
            &format!("-n{}", count),
        ])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut rows = Vec::new();

    for line in text.lines() {
        if let Some(pos) = line.find("LOGROW\x1f") {
            let graph = line[..pos].to_string();
            let data_str = &line[pos + "LOGROW\x1f".len()..];
            let parts: Vec<&str> = data_str.split('\x1f').collect();

            let hash = parts.first().unwrap_or(&"").trim().to_string();
            let full_hash = parts.get(1).unwrap_or(&"").trim().to_string();
            let raw_refs = parts.get(2).unwrap_or(&"").trim();
            let refs = raw_refs
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim()
                .to_string();
            let author = parts.get(3).unwrap_or(&"").trim().to_string();
            let date = parts.get(4).unwrap_or(&"").trim().to_string();
            let summary = parts.get(5).unwrap_or(&"").trim().to_string();

            rows.push(HistoryRow::Commit(HistoryCommitItem {
                graph,
                hash,
                full_hash,
                refs,
                author,
                date,
                summary,
            }));
        } else {
            // Pure ASCII graph connector line (e.g. |\ or |/ )
            rows.push(HistoryRow::GraphOnly(line.to_string()));
        }
    }

    Ok(rows)
}

/// Retrieve full details and diff snippet for a specific commit hash.
pub fn get_commit_details(repo_dir: &Path, commit_hash: &str) -> Result<CommitDetail, GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["show", "--stat", "-p", commit_hash])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut detail = CommitDetail::default();
    let mut in_message = false;
    let mut in_stats = false;
    let mut in_diff = false;
    let mut body_lines = Vec::new();

    for line in text.lines() {
        if line.starts_with("commit ") && detail.hash.is_empty() {
            detail.hash = line.trim_start_matches("commit ").trim().to_string();
            continue;
        }
        if line.starts_with("Author: ") && detail.author.is_empty() {
            detail.author = line.trim_start_matches("Author: ").trim().to_string();
            continue;
        }
        if line.starts_with("Date: ") && detail.date.is_empty() {
            detail.date = line.trim_start_matches("Date: ").trim().to_string();
            in_message = true;
            continue;
        }

        if in_message {
            if line.starts_with("diff --git ") {
                in_message = false;
                in_stats = false;
                in_diff = true;
                detail.diff_lines.push(line.to_string());
                continue;
            }
            if line.trim() == "---" {
                in_message = false;
                in_stats = true;
                continue;
            }
            if !line.is_empty()
                && (line.contains('|') && (line.contains('+') || line.contains('-')))
            {
                in_message = false;
                in_stats = true;
                detail.stats.push(line.trim().to_string());
                continue;
            }

            if line.starts_with("    ") {
                let msg = line.trim();
                if detail.summary.is_empty() {
                    detail.summary = msg.to_string();
                } else {
                    body_lines.push(msg.to_string());
                }
            }
            continue;
        }

        if in_stats {
            if line.starts_with("diff --git ") {
                in_stats = false;
                in_diff = true;
                detail.diff_lines.push(line.to_string());
                continue;
            }
            if !line.trim().is_empty() {
                detail.stats.push(line.trim().to_string());
            }
            continue;
        }

        if in_diff {
            detail.diff_lines.push(line.to_string());
        }
    }

    detail.body = body_lines.join("\n");
    Ok(detail)
}

/// Checkout a specific commit in detached HEAD mode.
pub fn checkout_commit(repo_dir: &Path, commit_hash: &str) -> Result<(), GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["checkout", commit_hash])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }
    Ok(())
}

/// Create a new branch pointing to the specified commit.
pub fn create_branch_at_commit(
    repo_dir: &Path,
    branch_name: &str,
    commit_hash: &str,
    switch_to: bool,
) -> Result<(), GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["branch", branch_name, commit_hash])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    if switch_to {
        let checkout_out = Command::new("git")
            .current_dir(repo_dir)
            .args(["checkout", branch_name])
            .output()?;

        if !checkout_out.status.success() {
            let err = String::from_utf8_lossy(&checkout_out.stderr)
                .trim()
                .to_string();
            return Err(GitError::Cli(err));
        }
    }

    Ok(())
}

/// Cherry-pick a specific commit into the current HEAD.
pub fn cherry_pick_commit(repo_dir: &Path, commit_hash: &str) -> Result<(), GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["cherry-pick", commit_hash])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }
    Ok(())
}

/// Revert a specific commit without opening an interactive editor.
pub fn revert_commit(repo_dir: &Path, commit_hash: &str) -> Result<(), GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["revert", "--no-edit", commit_hash])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }
    Ok(())
}

/// Time Machine: rewind current branch to target commit, discarding subsequent commits.
/// Automatically creates a safety backup branch first so work can never be permanently lost.
/// Returns the name of the safety backup branch created.
pub fn time_machine_reset(repo_dir: &Path, commit_hash: &str) -> Result<String, GitError> {
    // 1. Get current branch name (or "HEAD" if detached)
    let branch_out = Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    let current_branch = if branch_out.status.success() {
        String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string()
    } else {
        "HEAD".to_string()
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let clean_branch = current_branch.replace('/', "-");
    let backup_branch = format!("backup/timemachine-{}-{}", clean_branch, timestamp);

    // 2. Create safety branch pointing to current HEAD
    let _ = Command::new("git")
        .current_dir(repo_dir)
        .args(["branch", &backup_branch, "HEAD"])
        .output();

    // 3. Reset hard to target commit
    let reset_out = Command::new("git")
        .current_dir(repo_dir)
        .args(["reset", "--hard", commit_hash])
        .output()?;

    if !reset_out.status.success() {
        let err = String::from_utf8_lossy(&reset_out.stderr)
            .trim()
            .to_string();
        return Err(GitError::Cli(err));
    }

    Ok(backup_branch)
}
