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
        .env_remove("GIT_PREFIX")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never");
    cmd
}

/// Default timeout applied to Git CLI operations (30 seconds).
pub const DEFAULT_GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Execute a Command with a timeout. If the command exceeds the timeout,
/// the process is killed and GitError::Timeout is returned.
pub fn run_command_with_timeout(
    cmd: &mut Command,
    timeout: std::time::Duration,
) -> Result<std::process::Output, GitError> {
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let stdout_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = stdout_pipe.take() {
            let _ = std::io::Read::read_to_end(&mut r, &mut buf);
        }
        buf
    });

    let stderr_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = stderr_pipe.take() {
            let _ = std::io::Read::read_to_end(&mut r, &mut buf);
        }
        buf
    });

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_handle.join().unwrap_or_default();
                let stderr = stderr_handle.join().unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();
                    return Err(GitError::Timeout(timeout));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(GitError::Io(e));
            }
        }
    }
}

/// Execute a git command in the given directory with standard environment stripping and timeout guard.
pub fn clean_git_run(dir: &Path, args: &[&str]) -> Result<std::process::Output, GitError> {
    let mut cmd = clean_git_command(dir);
    cmd.args(args);
    run_command_with_timeout(&mut cmd, DEFAULT_GIT_TIMEOUT)
}
