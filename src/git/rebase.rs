//! Quick Interactive Rebase operations (Squash, Fixup, Reword).

use crate::git::types::GitError;
use std::path::Path;
use std::process::Command;

/// Interactive rebase action per commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseAction {
    Pick,
    Squash,
    Fixup,
    Reword,
    Drop,
}

impl RebaseAction {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Squash => "squash",
            Self::Fixup => "fixup",
            Self::Reword => "reword",
            Self::Drop => "drop",
        }
    }
}

/// Commit summary item in rebase plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseCommitItem {
    pub hash: String,
    pub author: String,
    pub summary: String,
    pub action: RebaseAction,
}

/// Retrieve the last `count` commits for interactive rebase preview.
pub fn list_recent_commits(
    repo_dir: &Path,
    count: usize,
) -> Result<Vec<RebaseCommitItem>, GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["log", &format!("-n{}", count), "--pretty=format:%h|%an|%s"])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    for line in text.lines() {
        let mut parts = line.split('|');
        if let (Some(hash), Some(author), Some(summary)) =
            (parts.next(), parts.next(), parts.next())
        {
            items.push(RebaseCommitItem {
                hash: hash.to_string(),
                author: author.to_string(),
                summary: summary.to_string(),
                action: RebaseAction::Pick,
            });
        }
    }

    Ok(items)
}

/// Quick squash of the top `count` commits into a single commit with message.
pub fn quick_squash(repo_dir: &Path, count: usize, new_message: &str) -> Result<(), GitError> {
    if count < 2 {
        return Ok(());
    }

    // Soft reset HEAD~count
    let reset_out = Command::new("git")
        .current_dir(repo_dir)
        .args(["reset", "--soft", &format!("HEAD~{}", count)])
        .output()?;

    if !reset_out.status.success() {
        let err = String::from_utf8_lossy(&reset_out.stderr)
            .trim()
            .to_string();
        return Err(GitError::Cli(err));
    }

    // Commit with combined message
    let commit_out = Command::new("git")
        .current_dir(repo_dir)
        .args(["commit", "-m", new_message])
        .output()?;

    if !commit_out.status.success() {
        let err = String::from_utf8_lossy(&commit_out.stderr)
            .trim()
            .to_string();
        return Err(GitError::Cli(err));
    }

    Ok(())
}

/// Reword the most recent commit message.
pub fn reword_head_commit(repo_dir: &Path, new_message: &str) -> Result<(), GitError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["commit", "--amend", "-m", new_message])
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Cli(err));
    }

    Ok(())
}
