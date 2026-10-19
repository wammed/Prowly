pub mod conflict;
pub mod history;
pub mod rebase;
pub mod service;
pub mod types;
pub mod worktree;

pub use conflict::{apply_resolutions, parse_conflicts, ConflictChoice, ConflictHunk};
pub use history::{
    checkout_commit, cherry_pick_commit, create_branch_at_commit, get_commit_details,
    get_commit_history, revert_commit, time_machine_reset, CommitDetail, HistoryCommitItem,
    HistoryRow,
};
pub use rebase::{
    list_recent_commits, quick_squash, reword_head_commit, RebaseAction, RebaseCommitItem,
};
pub use service::{classify_git_status_items, GitService};
pub use types::{
    DiffHunk, DiffLine, DiffLineType, FileDiff, FileStatus, GitError, GitFileItem, GitRepoStatus,
    RepoOperationState,
};
pub use worktree::{create_worktree, list_worktrees, remove_worktree, WorktreeItem};

use std::path::Path;
use std::process::Command;

/// Spawns a Git Command with parent/hook environment variables stripped,
/// preventing leakage of GIT_DIR, GIT_INDEX_FILE, GIT_WORK_TREE, etc.
pub fn clean_git_command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_PREFIX");
    cmd
}
