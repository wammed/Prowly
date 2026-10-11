use prowly::git::rebase::{list_recent_commits, quick_squash, reword_head_commit, RebaseAction};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn setup_test_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let p = dir.path();

    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(p)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "TestUser"])
        .current_dir(p)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(p)
        .output()
        .unwrap();

    // Commit 1
    fs::write(p.join("file1.txt"), "hello").unwrap();
    Command::new("git")
        .args(["add", "file1.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feat: initial commit"])
        .current_dir(p)
        .output()
        .unwrap();

    // Commit 2
    fs::write(p.join("file2.txt"), "world").unwrap();
    Command::new("git")
        .args(["add", "file2.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "fix: second commit"])
        .current_dir(p)
        .output()
        .unwrap();

    // Commit 3
    fs::write(p.join("file3.txt"), "foo").unwrap();
    Command::new("git")
        .args(["add", "file3.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "docs: third commit"])
        .current_dir(p)
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
