use prowly::git::rebase::{RebaseAction, RebaseCommitItem};
use prowly::ui::modals::{BranchSwitcherModal, RebaseModal, RemoteDiffPreviewModal};

#[test]
fn test_branch_switcher_filtering_and_selection() {
    let mut switcher = BranchSwitcherModal::new();
    assert!(!switcher.is_open);

    let branches = vec![
        "main".to_string(),
        "feat/user-auth".to_string(),
        "feat/ai-chat".to_string(),
        "fix/clippy-warning".to_string(),
    ];

    switcher.open("main".to_string(), branches, Some("feat/user-auth".to_string()));
    assert!(switcher.is_open);
    assert_eq!(switcher.branches.len(), 4);

    // Filter by typing 'chat'
    switcher.push_char('c');
    switcher.push_char('h');
    switcher.push_char('a');
    switcher.push_char('t');
    assert_eq!(switcher.query, "chat");

    let filtered = switcher.filtered_branches();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0], "feat/ai-chat");
    assert_eq!(switcher.selected_branch(), Some("feat/ai-chat".to_string()));

    // Backspace query
    switcher.pop_char();
    switcher.pop_char();
    switcher.pop_char();
    switcher.pop_char();
    assert_eq!(switcher.query, "");
    assert_eq!(switcher.filtered_branches().len(), 4);

    // Navigation
    switcher.next();
    assert_eq!(switcher.selected_index, 1);
    switcher.previous();
    assert_eq!(switcher.selected_index, 0);

    switcher.close();
    assert!(!switcher.is_open);
}

#[test]
fn test_remote_diff_preview_modal() {
    let mut modal = RemoteDiffPreviewModal::new();
    assert!(!modal.is_open);

    let unpushed = vec!["c123456 feat: add shiny button".to_string()];
    let unpulled = vec![
        "d789012 fix: remote bugfix".to_string(),
        "e345678 docs: update README".to_string(),
    ];

    modal.open(unpushed, unpulled, Some("origin/main".to_string()));
    assert!(modal.is_open);
    assert_eq!(modal.unpushed_commits.len(), 1);
    assert_eq!(modal.unpulled_commits.len(), 2);

    modal.close();
    assert!(!modal.is_open);
}

#[test]
fn test_rebase_modal_actions() {
    let mut modal = RebaseModal::new();
    assert!(!modal.is_open);

    let commits = vec![
        RebaseCommitItem {
            action: RebaseAction::Pick,
            hash: "abc1234".to_string(),
            author: "Developer".to_string(),
            summary: "feat: initial commit".to_string(),
            is_verified: true,
        },
        RebaseCommitItem {
            action: RebaseAction::Pick,
            hash: "def5678".to_string(),
            author: "Contributor".to_string(),
            summary: "fix: typo".to_string(),
            is_verified: false,
        },
    ];

    modal.open(commits);
    assert!(modal.is_open);
    assert_eq!(modal.commits.len(), 2);
    assert_eq!(modal.selected_commit().unwrap().hash, "abc1234");
    assert!(modal.selected_commit().unwrap().is_verified);

    modal.next();
    assert_eq!(modal.selected_commit().unwrap().hash, "def5678");
    assert!(!modal.selected_commit().unwrap().is_verified);

    // Reword input test
    modal.is_editing_reword = true;
    modal.reword_input = "fix: critical typo".to_string();
    assert_eq!(modal.reword_input, "fix: critical typo");

    modal.close();
    assert!(!modal.is_open);
}
