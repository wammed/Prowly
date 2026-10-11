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

/// Overall repository status summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRepoStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<GitFileItem>,
}
