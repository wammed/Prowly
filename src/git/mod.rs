pub mod conflict;
pub mod rebase;
pub mod service;
pub mod types;
pub mod worktree;

pub use conflict::{apply_resolutions, parse_conflicts, ConflictChoice, ConflictHunk};
pub use rebase::{
    list_recent_commits, quick_squash, reword_head_commit, RebaseAction, RebaseCommitItem,
};
pub use service::GitService;
pub use types::{
    DiffHunk, DiffLine, DiffLineType, FileDiff, FileStatus, GitError, GitFileItem, GitRepoStatus,
};
pub use worktree::{create_worktree, list_worktrees, remove_worktree, WorktreeItem};
