use prowly::git::rebase::{list_recent_commits, quick_squash, reword_head_commit, RebaseAction};
use std::fs;
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
        .env_remove("GIT_PREFIX")
        .env_remove("GIT_AUTHOR_NAME")
        .env_remove("GIT_AUTHOR_EMAIL")
        .env_remove("GIT_AUTHOR_DATE")
        .env_remove("GIT_COMMITTER_NAME")
        .env_remove("GIT_COMMITTER_EMAIL")
        .env_remove("GIT_COMMITTER_DATE")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null");
    cmd
}

fn setup_test_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let p = dir.path();

    git_cmd(p).args(["init", "-b", "main"]).output().unwrap();

    git_cmd(p)
        .args(["config", "user.name", "TestUser"])
        .output()
        .unwrap();

    git_cmd(p)
        .args(["config", "user.email", "test@example.com"])
        .output()
        .unwrap();

    // Commit 1
    fs::write(p.join("file1.txt"), "hello").unwrap();
    git_cmd(p).args(["add", "file1.txt"]).output().unwrap();
    git_cmd(p)
        .args(["commit", "-m", "feat: initial commit"])
        .output()
        .unwrap();

    // Commit 2
    fs::write(p.join("file2.txt"), "world").unwrap();
    git_cmd(p).args(["add", "file2.txt"]).output().unwrap();
    git_cmd(p)
        .args(["commit", "-m", "fix: second commit"])
        .output()
        .unwrap();

    // Commit 3
    fs::write(p.join("file3.txt"), "foo").unwrap();
    git_cmd(p).args(["add", "file3.txt"]).output().unwrap();
    git_cmd(p)
        .args(["commit", "-m", "docs: third commit"])
        .output()
        .unwrap();

    dir
}

#[test]
fn test_list_recent_commits() {
    let repo = setup_test_repo();
    let commits = list_recent_commits(repo.path(), 5).unwrap();

    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].summary, "docs: third commit");
    assert_eq!(commits[1].summary, "fix: second commit");
    assert_eq!(commits[2].summary, "feat: initial commit");
    assert_eq!(commits[0].action, RebaseAction::Pick);
    assert_eq!(commits[0].action.badge(), "pick");
}

#[test]
fn test_reword_head_commit() {
    let repo = setup_test_repo();
    reword_head_commit(repo.path(), "docs: updated third commit message").unwrap();

    let commits = list_recent_commits(repo.path(), 2).unwrap();
    assert_eq!(commits[0].summary, "docs: updated third commit message");
}

#[test]
fn test_quick_squash() {
    let repo = setup_test_repo();
    // Squash top 2 commits
    quick_squash(repo.path(), 2, "feat: squashed commit 2 and 3").unwrap();

    let commits = list_recent_commits(repo.path(), 5).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].summary, "feat: squashed commit 2 and 3");
    assert_eq!(commits[1].summary, "feat: initial commit");
}
