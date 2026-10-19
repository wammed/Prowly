use prowly::git::worktree::{create_worktree, list_worktrees, remove_worktree};
use std::process::Command;
use tempfile::tempdir;

fn git_cmd(path: &std::path::Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_PREFIX");
    cmd
}

fn setup_temp_repo() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create tempdir");
    let path = dir.path();

    // Initialize git repo
    let status = git_cmd(path)
        .args(["init", "-b", "main"])
        .status()
        .expect("git init failed");
    assert!(status.success());

    // Configure user name and email for commits
    let _ = git_cmd(path)
        .args(["config", "user.name", "Test User"])
        .status();
    let _ = git_cmd(path)
        .args(["config", "user.email", "test@example.com"])
        .status();

    // Create an initial commit so HEAD exists
    std::fs::write(path.join("init.txt"), "hello").unwrap();
    let _ = git_cmd(path).args(["add", "init.txt"]).status();
    let _ = git_cmd(path)
        .args(["commit", "-m", "initial commit"])
        .status();

    dir
}

#[test]
fn test_worktree_lifecycle() {
    let repo = setup_temp_repo();
    let repo_path = repo.path();

    // 1. List worktrees initially
    let list = list_worktrees(repo_path).expect("list_worktrees failed");
    assert_eq!(list.len(), 1);
    assert!(list[0].is_current);

    // 2. Create a new worktree in a separate directory
    let wt_dir = tempdir().expect("failed to create wt dir");
    let wt_path = wt_dir.path().join("feature_wt");

    let created =
        create_worktree(repo_path, &wt_path, Some("feature-1")).expect("create_worktree failed");
    assert_eq!(created.name, "feature_wt");
    assert_eq!(created.head_branch, Some("feature-1".to_string()));

    // 3. List should now show 2 worktrees
    let updated_list = list_worktrees(repo_path).expect("list_worktrees after add failed");
    assert_eq!(updated_list.len(), 2);

    // 4. Safely remove worktree
    let remove_res = remove_worktree(repo_path, wt_path.to_str().unwrap(), false);
    assert!(remove_res.is_ok());

    let final_list = list_worktrees(repo_path).expect("list_worktrees after remove failed");
    assert_eq!(final_list.len(), 1);
}
