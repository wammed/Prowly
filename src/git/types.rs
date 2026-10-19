//! Data structures and error definitions for Git hybrid engine.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error types occurring during Git operations.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("libgit2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git CLI error: {0}")]
    Cli(String),

    #[error("Repository not found: {0}")]
    NotFound(String),
}

/// Status of a file in the working directory or staging index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Clean,
}

impl FileStatus {
    pub const fn badge(&self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "+",
            Self::Deleted => "-",
            Self::Renamed => "R",
            Self::Untracked => "?",
            Self::Conflicted => "!",
            Self::Clean => " ",
        }
    }
}

/// A tracked or untracked file entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileItem {
    pub path: String,
    pub staged: bool,
    pub status: FileStatus,
}

/// Origin type for a line inside a Diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Deleted,
    HunkHeader,
}

/// A single line in a diff hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub line_type: DiffLineType,
}

/// A diff hunk grouping contiguous changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Structured diff for a single file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileDiff {
    pub file_path: String,
    pub is_binary: bool,
    pub hunks: Vec<DiffHunk>,
    pub stats_added: usize,
    pub stats_deleted: usize,
}

impl FileDiff {
    /// Create a preview diff representation from raw file content.
    pub fn from_file_content(path: &str, content: &str) -> Self {
        let lines: Vec<DiffLine> = content
            .lines()
            .take(500)
            .enumerate()
            .map(|(idx, line)| DiffLine {
                origin: ' ',
                content: line.to_string(),
                old_lineno: Some((idx + 1) as u32),
                new_lineno: Some((idx + 1) as u32),
                line_type: DiffLineType::Context,
            })
            .collect();
        let total_lines = content.lines().count();
        Self {
            file_path: path.to_string(),
            is_binary: false,
            hunks: vec![DiffHunk {
                header: format!("@@ Content: {} ({} lines) @@", path, total_lines),
                lines,
            }],
            stats_added: 0,
            stats_deleted: 0,
        }
    }

    /// Format hunks and lines into standard unified diff text.
    pub fn to_diff_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "--- a/{}\n+++ b/{}\n",
            self.file_path, self.file_path
        ));
        for hunk in &self.hunks {
            out.push_str(&hunk.header);
            out.push('\n');
            for line in &hunk.lines {
                out.push(line.origin);
                out.push_str(&line.content);
                out.push('\n');
            }
        }
        out
    }
}

/// In-progress interactive Git operation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RepoOperationState {
    #[default]
    None,
    RebaseInProgress,
    MergeInProgress,
    CherryPickInProgress,
    RevertInProgress,
}

impl RepoOperationState {
    pub const fn badge(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::None => None,
            Self::RebaseInProgress => Some(("REBASE", "Interactive Rebase in progress")),
            Self::MergeInProgress => Some(("MERGE", "Merge in progress (conflicts exist)")),
            Self::CherryPickInProgress => Some(("CHERRY-PICK", "Cherry-pick in progress")),
            Self::RevertInProgress => Some(("REVERT", "Revert in progress")),
        }
    }
}

/// Overall repository status summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRepoStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<GitFileItem>,
    pub operation_state: RepoOperationState,
}
