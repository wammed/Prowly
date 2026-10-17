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

    switcher.open(
        "main".to_string(),
        branches,
        Some("feat/user-auth".to_string()),
    );
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

#[test]
fn test_branch_switcher_modal_click_and_mouse_interaction() {
    use prowly::ui::modals::BranchSwitcherClickResult;
    use ratatui::layout::Rect;

    let mut switcher = BranchSwitcherModal::new();
    let viewport = Rect::new(0, 0, 100, 30);

    // Closed switcher returns None
    assert_eq!(switcher.handle_click(10, 10, viewport), None);

    let branches = vec![
        "main".to_string(),
        "feat/chat".to_string(),
        "fix/bug".to_string(),
    ];
    switcher.open("main".to_string(), branches, None);
    assert!(switcher.is_open);

    let rect = switcher.modal_rect(viewport);
    assert!(rect.width > 0 && rect.height > 0);

    // Click outside -> Close
    assert_eq!(
        switcher.handle_click(0, 0, viewport),
        Some(BranchSwitcherClickResult::Close)
    );

    // Click on filter input line inside modal -> Handled
    let inner_x = rect.x + 1;
    let inner_y = rect.y + 1;
    assert_eq!(
        switcher.handle_click(inner_x + 5, inner_y, viewport),
        Some(BranchSwitcherClickResult::Handled)
    );

    // Click on first branch item (inner_y + 2) -> Select("main")
    assert_eq!(
        switcher.handle_click(inner_x + 5, inner_y + 2, viewport),
        Some(BranchSwitcherClickResult::Select("main".to_string()))
    );
    assert_eq!(switcher.selected_index, 0);

    // Click on second branch item (inner_y + 3) -> Select("feat/chat")
    assert_eq!(
        switcher.handle_click(inner_x + 5, inner_y + 3, viewport),
        Some(BranchSwitcherClickResult::Select("feat/chat".to_string()))
    );
    assert_eq!(switcher.selected_index, 1);
}

#[test]
fn test_remote_diff_preview_click_interaction() {
    use prowly::ui::modals::RemoteDiffClickResult;
    use ratatui::layout::Rect;

    let mut modal = RemoteDiffPreviewModal::new();
    let viewport = Rect::new(0, 0, 100, 30);

    assert_eq!(modal.handle_click(10, 10, viewport), None);

    modal.open(vec!["unpushed".to_string()], vec![], None);
    assert!(modal.is_open);

    let rect = modal.modal_rect(viewport);

    // Click outside -> Close
    assert_eq!(
        modal.handle_click(0, 0, viewport),
        Some(RemoteDiffClickResult::Close)
    );

    // Click inside body -> Handled
    assert_eq!(
        modal.handle_click(rect.x + 5, rect.y + 5, viewport),
        Some(RemoteDiffClickResult::Handled)
    );

    // Click on bottom hint -> Close
    let bottom_y = rect.y + rect.height.saturating_sub(2);
    assert_eq!(
        modal.handle_click(rect.x + 5, bottom_y, viewport),
        Some(RemoteDiffClickResult::Close)
    );
}

#[test]
fn test_rebase_modal_click_interaction() {
    use prowly::ui::modals::RebaseClickResult;
    use ratatui::layout::Rect;

    let mut modal = RebaseModal::new();
    let viewport = Rect::new(0, 0, 100, 30);

    assert_eq!(modal.handle_click(10, 10, viewport), None);

    let commits = vec![
        RebaseCommitItem {
            action: RebaseAction::Pick,
            hash: "abc1234".to_string(),
            author: "Author 1".to_string(),
            summary: "first".to_string(),
            is_verified: true,
        },
        RebaseCommitItem {
            action: RebaseAction::Pick,
            hash: "def5678".to_string(),
            author: "Author 2".to_string(),
            summary: "second".to_string(),
            is_verified: false,
        },
    ];

    modal.open(commits);
    assert!(modal.is_open);

    let rect = modal.modal_rect(viewport);

    // Click outside -> Close
    assert_eq!(
        modal.handle_click(0, 0, viewport),
        Some(RebaseClickResult::Close)
    );

    let inner_x = rect.x + 1;
    let inner_y = rect.y + 1;

    // Click on first commit (inner_y + 2) -> SelectCommit(0)
    assert_eq!(
        modal.handle_click(inner_x + 5, inner_y + 2, viewport),
        Some(RebaseClickResult::SelectCommit(0))
    );
    assert_eq!(modal.selected_index, 0);

    // Click on second commit (inner_y + 3) -> SelectCommit(1)
    assert_eq!(
        modal.handle_click(inner_x + 5, inner_y + 3, viewport),
        Some(RebaseClickResult::SelectCommit(1))
    );
    assert_eq!(modal.selected_index, 1);

    // Click on bottom action buttons: Squash top 2
    let action_y = inner_y + rect.height.saturating_sub(2).saturating_sub(1);
    assert_eq!(
        modal.handle_click(inner_x + 5, action_y, viewport),
        Some(RebaseClickResult::Squash)
    );
}
