use prowly::git::{time_machine_reset, GitService};
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
        .args(["config", "user.name", "Test User"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(p)
        .output()
        .unwrap();

    fs::write(p.join("file1.txt"), "Initial commit line 1\n").unwrap();
    Command::new("git")
        .args(["add", "file1.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "commit 1"])
        .current_dir(p)
        .output()
        .unwrap();

    dir
}

#[test]
fn test_timemachine_preserves_uncommitted_changes() {
    let dir = setup_test_repo();
    let p = dir.path();

    // Commit 2
    fs::write(p.join("file2.txt"), "Commit 2 content\n").unwrap();
    Command::new("git")
        .args(["add", "file2.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "commit 2"])
        .current_dir(p)
        .output()
        .unwrap();

    // Get commit 1 hash
    let out = Command::new("git")
        .args(["rev-parse", "HEAD~1"])
        .current_dir(p)
        .output()
        .unwrap();
    let commit1_hash = String::from_utf8_lossy(&out.stdout).trim().to_string();

    // Create uncommitted dirty worktree change
    fs::write(p.join("dirty.txt"), "Important uncommitted work!\n").unwrap();

    // Run time_machine_reset back to commit 1
    let res = time_machine_reset(p, &commit1_hash);
    assert!(
        res.is_ok(),
        "time_machine_reset should succeed: {:?}",
        res.err()
    );
    let backup_info = res.unwrap();
    assert!(backup_info.contains("timemachine-autostash"));

    // Verify the uncommitted changes were saved in stash
    let stash_list = Command::new("git")
        .args(["stash", "list"])
        .current_dir(p)
        .output()
        .unwrap();
    let stash_str = String::from_utf8_lossy(&stash_list.stdout);
    assert!(stash_str.contains("timemachine-autostash"));
}

#[test]
fn test_discard_unstaged_preserves_staged_changes() {
    let dir = setup_test_repo();
    let p = dir.path();
    let git = GitService::open(p).unwrap();

    // Stage a new change in file1.txt
    fs::write(
        p.join("file1.txt"),
        "Initial commit line 1\nStaged line 2\n",
    )
    .unwrap();
    git.stage_file("file1.txt").unwrap();

    // Now write an additional unstaged change
    fs::write(
        p.join("file1.txt"),
        "Initial commit line 1\nStaged line 2\nUnstaged line 3\n",
    )
    .unwrap();

    // Discard UNSTAGED modifications only
    git.hard_discard_targeted("file1.txt", false).unwrap();

    // Verify file still contains the staged line 2, but NOT unstaged line 3
    let content = fs::read_to_string(p.join("file1.txt")).unwrap();
    assert!(
        content.contains("Staged line 2"),
        "Staged modifications must be preserved!"
    );
    assert!(
        !content.contains("Unstaged line 3"),
        "Unstaged modification must be discarded!"
    );
}
