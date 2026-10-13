use prowly::git::{
    checkout_commit, cherry_pick_commit, create_branch_at_commit, get_commit_details,
    get_commit_history, revert_commit, time_machine_reset, HistoryCommitItem, HistoryRow,
};
use prowly::ui::modals::{HistoryActionState, HistoryActionsModal, HistoryMenuAction};
use prowly::ui::views::history::HistoryView;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn setup_test_git_repo() -> (tempfile::TempDir, Vec<String>) {
    let dir = tempdir().expect("Failed to create tempdir");
    let p = dir.path();

    // git init
    assert!(Command::new("git")
        .current_dir(p)
        .args(["init", "-b", "main"])
        .status()
        .expect("git init")
        .success());

    assert!(Command::new("git")
        .current_dir(p)
        .args(["config", "user.name", "Tester"])
        .status()
        .expect("git config user.name")
        .success());

    assert!(Command::new("git")
        .current_dir(p)
        .args(["config", "user.email", "tester@example.com"])
        .status()
        .expect("git config user.email")
        .success());

    let mut commit_hashes = Vec::new();

    // Commit 1
    fs::write(p.join("file1.txt"), "Initial content line 1\n").unwrap();
    Command::new("git")
        .current_dir(p)
        .args(["add", "file1.txt"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(p)
        .args(["commit", "-m", "feat: initial commit"])
        .status()
        .unwrap();

    let out1 = Command::new("git")
        .current_dir(p)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    commit_hashes.push(String::from_utf8_lossy(&out1.stdout).trim().to_string());

    // Commit 2
    fs::write(p.join("file1.txt"), "Initial content line 1\nSecond line\n").unwrap();
    Command::new("git")
        .current_dir(p)
        .args(["commit", "-am", "feat: second commit"])
        .status()
        .unwrap();

    let out2 = Command::new("git")
        .current_dir(p)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    commit_hashes.push(String::from_utf8_lossy(&out2.stdout).trim().to_string());

    // Commit 3
    fs::write(p.join("file2.txt"), "Another file\n").unwrap();
    Command::new("git")
        .current_dir(p)
        .args(["add", "file2.txt"])
        .status()
        .unwrap();
    Command::new("git")
        .current_dir(p)
        .args(["commit", "-m", "feat: third commit"])
        .status()
        .unwrap();

    let out3 = Command::new("git")
        .current_dir(p)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    commit_hashes.push(String::from_utf8_lossy(&out3.stdout).trim().to_string());

    (dir, commit_hashes)
}

#[test]
fn test_get_commit_history_and_graph() {
    let (repo, hashes) = setup_test_git_repo();
    let rows = get_commit_history(repo.path(), 10).expect("get_commit_history");

    assert!(!rows.is_empty());
    let commits: Vec<&HistoryCommitItem> = rows.iter().filter_map(|r| r.commit()).collect();
    assert_eq!(commits.len(), 3);

    // Latest commit is commit 3
    assert_eq!(commits[0].summary, "feat: third commit");
    assert_eq!(commits[0].full_hash, hashes[2]);
    assert_eq!(commits[1].summary, "feat: second commit");
    assert_eq!(commits[1].full_hash, hashes[1]);
    assert_eq!(commits[2].summary, "feat: initial commit");
    assert_eq!(commits[2].full_hash, hashes[0]);
}

#[test]
fn test_get_commit_details_and_diff() {
    let (repo, hashes) = setup_test_git_repo();
    let detail = get_commit_details(repo.path(), &hashes[1]).expect("get_commit_details");

    assert_eq!(detail.summary, "feat: second commit");
    assert_eq!(detail.author, "Tester <tester@example.com>");
    assert!(!detail.diff_lines.is_empty());

    // Verify diff patch contains additions
    let has_add = detail.diff_lines.iter().any(|l| l.starts_with("+Second line"));
    assert!(has_add, "Diff should contain added line '+Second line'");
}

#[test]
fn test_checkout_and_branch_creation() {
    let (repo, hashes) = setup_test_git_repo();

    // 1. Checkout commit 1 (Detached HEAD)
    checkout_commit(repo.path(), &hashes[0]).expect("checkout_commit");
    let cur_head = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&cur_head.stdout).trim(), hashes[0]);

    // 2. Create new branch at commit 1 and switch to it
    create_branch_at_commit(repo.path(), "feature-history", &hashes[0], true)
        .expect("create_branch_at_commit");
    let cur_branch = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&cur_branch.stdout).trim(),
        "feature-history"
    );
}

#[test]
fn test_cherry_pick_and_revert() {
    let (repo, hashes) = setup_test_git_repo();

    // Checkout commit 1 on a new branch
    create_branch_at_commit(repo.path(), "branch-cp", &hashes[0], true).unwrap();

    // Cherry-pick commit 3 (which adds file2.txt)
    cherry_pick_commit(repo.path(), &hashes[2]).expect("cherry_pick_commit");
    assert!(repo.path().join("file2.txt").exists());

    // Revert commit 3
    let head_out = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let cp_head = String::from_utf8_lossy(&head_out.stdout).trim().to_string();

    revert_commit(repo.path(), &cp_head).expect("revert_commit");
    assert!(!repo.path().join("file2.txt").exists());
}

#[test]
fn test_time_machine_reset_with_safety_backup() {
    let (repo, hashes) = setup_test_git_repo();

    // HEAD is currently at commit 3 (hashes[2])
    let initial_head = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&initial_head.stdout).trim(),
        hashes[2]
    );

    // Time Machine: Reset back to commit 1 (hashes[0])
    let backup_branch =
        time_machine_reset(repo.path(), &hashes[0]).expect("time_machine_reset");
    assert!(backup_branch.starts_with("backup/timemachine-"));

    // Current HEAD must now be at commit 1 (hashes[0])
    let new_head = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&new_head.stdout).trim(), hashes[0]);

    // file2.txt from commit 3 should now be gone from working tree
    assert!(!repo.path().join("file2.txt").exists());

    // The safety backup branch must still point to commit 3
    let backup_head = Command::new("git")
        .current_dir(repo.path())
        .args(["rev-parse", &backup_branch])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&backup_head.stdout).trim(),
        hashes[2]
    );
}

#[test]
fn test_history_view_navigation() {
    let mut view = HistoryView::new();

    let row1 = HistoryRow::Commit(HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "aaa1111".to_string(),
        full_hash: "aaa1111222233334444555566667777888899990".to_string(),
        refs: "HEAD -> main".to_string(),
        author: "Tester".to_string(),
        date: "1m ago".to_string(),
        summary: "Commit 1".to_string(),
    });

    let row_graph = HistoryRow::GraphOnly("|/".to_string());

    let row2 = HistoryRow::Commit(HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "bbb2222".to_string(),
        full_hash: "bbb2222222233334444555566667777888899990".to_string(),
        refs: "".to_string(),
        author: "Tester".to_string(),
        date: "2m ago".to_string(),
        summary: "Commit 2".to_string(),
    });

    view.set_rows(vec![row1, row_graph, row2]);
    assert_eq!(view.selected_commit().unwrap().hash, "aaa1111");

    // Move to next commit (should skip the GraphOnly row)
    view.next_commit();
    assert_eq!(view.selected_commit().unwrap().hash, "bbb2222");

    // Move back
    view.previous_commit();
    assert_eq!(view.selected_commit().unwrap().hash, "aaa1111");
}

#[test]
fn test_history_actions_modal_state() {
    let mut modal = HistoryActionsModal::new();
    assert!(!modal.is_open);

    let commit = HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "1234567".to_string(),
        full_hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
        refs: "".to_string(),
        author: "Tester".to_string(),
        date: "now".to_string(),
        summary: "Test commit".to_string(),
    };

    modal.open(commit.clone());
    assert!(modal.is_open);
    assert_eq!(modal.state, HistoryActionState::Menu);
    assert_eq!(modal.selected_action(), HistoryMenuAction::Checkout);

    modal.next();
    assert_eq!(modal.selected_action(), HistoryMenuAction::CreateBranch);

    // Open branch input
    modal.open_branch_input(commit.clone());
    assert_eq!(modal.state, HistoryActionState::BranchInput);
    modal.push_char('f');
    modal.push_char('e');
    modal.push_char('a');
    modal.push_char('t');
    assert_eq!(modal.new_branch_name, "feat");
    modal.pop_char();
    assert_eq!(modal.new_branch_name, "fea");

    // Open Time Machine confirm
    modal.open_time_machine(commit);
    assert_eq!(modal.state, HistoryActionState::TimeMachineConfirm);

    modal.close();
    assert!(!modal.is_open);
}
