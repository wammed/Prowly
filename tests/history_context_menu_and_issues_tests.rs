use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use prowly::app::{App, AppTab};
use prowly::config::Config;
use prowly::git::{HistoryCommitItem, HistoryRow};
use prowly::github::{GitHubRepoTarget, GitHubService, IssueItem};
use prowly::ui::modals::context_menu::{ContextMenu, ContextMenuAction};
use prowly::ui::modals::history_actions::{
    HistoryActionsModal, HistoryMenuAction, HistoryModalClickResult,
};
use ratatui::layout::Rect;

#[test]
fn test_history_context_menu_commit_actions() {
    let mut menu = ContextMenu::new();
    let commit = HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "a1b2c3d".to_string(),
        full_hash: "a1b2c3d4e5f67890123456789abcdef012345678".to_string(),
        refs: "HEAD -> main".to_string(),
        author: "Dev <dev@example.com>".to_string(),
        date: "2 hours ago".to_string(),
        summary: "feat: implement awesome feature".to_string(),
    };

    menu.open_for_commit(commit, 20, 10);
    assert!(menu.is_open);
    assert!(menu.target_commit.is_some());
    assert_eq!(menu.target_commit.as_ref().unwrap().hash, "a1b2c3d");
    assert_eq!(menu.selected_action(), ContextMenuAction::CheckoutCommit);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::CreateBranch);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::CherryPick);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::RevertCommit);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::TimeMachine);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::CopyCommitSha);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::CopyCommitMessage);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::AskAiCommit);

    menu.previous();
    assert_eq!(menu.selected_action(), ContextMenuAction::CopyCommitMessage);

    menu.close();
    assert!(!menu.is_open);
}

#[test]
fn test_history_actions_modal_mouse_interaction() {
    let mut modal = HistoryActionsModal::new();
    let commit = HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "deadbeef".to_string(),
        full_hash: "deadbeef1234567890123456789abcdef0123456".to_string(),
        refs: "".to_string(),
        author: "Tester".to_string(),
        date: "1 day ago".to_string(),
        summary: "test commit".to_string(),
    };

    modal.open(commit);
    assert!(modal.is_open);

    let screen = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 40,
    };
    let rect = modal.modal_rect(screen);

    // Clicking outside modal should close it
    let res = modal.handle_click(rect.x.saturating_sub(5), rect.y.saturating_sub(5), screen);
    assert_eq!(res, Some(HistoryModalClickResult::Close));

    // Clicking on second action item (Create Branch) in menu
    let inner_y = rect.y + 1;
    let res = modal.handle_click(rect.x + 10, inner_y + 3, screen);
    assert_eq!(
        res,
        Some(HistoryModalClickResult::ExecuteAction(
            HistoryMenuAction::CreateBranch
        ))
    );
}

#[test]
fn test_history_tab_right_click_opens_commit_context_menu() {
    let mut app = App::new(Config::default());
    app.active_tab = AppTab::History;

    // Populate fake commit rows
    let commit = HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "1234567".to_string(),
        full_hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
        refs: "HEAD -> main".to_string(),
        author: "Dev".to_string(),
        date: "now".to_string(),
        summary: "initial test commit".to_string(),
    };
    app.history_view
        .set_rows(vec![HistoryRow::Commit(commit.clone())]);

    // Right-click inside right content area
    let right_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: 30,
        row: 10,
        modifiers: KeyModifiers::NONE,
    };
    app.handle_mouse(right_click);

    assert!(app.context_menu.is_open);
    assert!(app.context_menu.target_commit.is_some());
    assert_eq!(
        app.context_menu.target_commit.as_ref().unwrap().hash,
        "1234567"
    );
    assert_eq!(
        app.context_menu.selected_action(),
        ContextMenuAction::CheckoutCommit
    );

    // Context menu hit-test copy SHA
    let action = app.context_menu.selected_action();
    assert_eq!(action, ContextMenuAction::CheckoutCommit);
    app.context_menu.close();
    assert!(!app.context_menu.is_open);
}

#[tokio::test]
async fn test_github_service_list_issues_mock() {
    let service = GitHubService::new(None);
    let target = GitHubRepoTarget {
        owner: "wammed".to_string(),
        repo: "Prowly".to_string(),
    };

    let mock_issue = IssueItem {
        number: 42,
        title: "Test Issue Title".to_string(),
        author: "issue-author".to_string(),
        state: "open".to_string(),
        body: Some("Detailed issue description".to_string()),
        comments_count: 5,
        labels: vec!["bug".to_string(), "tui".to_string()],
        html_url: "https://github.com/wammed/Prowly/issues/42".to_string(),
        updated_at: "2026-10-16T12:00:00Z".to_string(),
    };

    service.set_mock_issues(vec![mock_issue.clone()]).await;

    let issues = service.list_issues(&target, false).await.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].number, 42);
    assert_eq!(issues[0].title, "Test Issue Title");
    assert_eq!(issues[0].state, "open");
    assert_eq!(issues[0].comments_count, 5);
}

#[test]
fn test_issues_view_navigation_and_tab_switch() {
    let mut app = App::new(Config::default());
    assert_eq!(app.active_tab, AppTab::Changes);

    // Switch to Issues via '3'
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Issues);

    let mock_issue1 = IssueItem {
        number: 101,
        title: "Issue One".to_string(),
        author: "alice".to_string(),
        state: "open".to_string(),
        body: Some("Body 1".to_string()),
        comments_count: 2,
        labels: vec!["enhancement".to_string()],
        html_url: "https://github.com/wammed/Prowly/issues/101".to_string(),
        updated_at: "2026-10-16T10:00:00Z".to_string(),
    };
    let mock_issue2 = IssueItem {
        number: 102,
        title: "Issue Two".to_string(),
        author: "bob".to_string(),
        state: "closed".to_string(),
        body: Some("Body 2".to_string()),
        comments_count: 0,
        labels: vec!["wontfix".to_string()],
        html_url: "https://github.com/wammed/Prowly/issues/102".to_string(),
        updated_at: "2026-10-16T11:00:00Z".to_string(),
    };

    app.issues_view.set_issues(vec![mock_issue1, mock_issue2]);
    assert_eq!(app.issues_view.selected_issue().unwrap().number, 101);

    // Navigate down via 'j'
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(app.issues_view.selected_issue().unwrap().number, 102);

    // Navigate up via 'k'
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(app.issues_view.selected_issue().unwrap().number, 101);

    // Toggle focus via Tab
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.issues_view.focus, prowly::ui::views::IssueFocus::Detail);
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.issues_view.focus, prowly::ui::views::IssueFocus::List);

    // Yank issue URL via 'y'
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert!(app.last_status.is_some());
    assert!(app
        .last_status
        .as_ref()
        .unwrap()
        .contains("https://github.com/wammed/Prowly/issues/101"));

    // Switch to Changes via '1', then back via 'i'
    app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Changes);
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Issues);
}

#[test]
fn test_issues_switch_button_click() {
    let mut app = App::new(Config::default());
    // Simulate issues switch rect at (60, 3, w: 16, h: 1)
    app.issues_switch_rect = Rect {
        x: 60,
        y: 3,
        width: 16,
        height: 1,
    };

    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 65,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };

    app.handle_mouse(click);
    assert_eq!(
        app.last_status,
        Some("Reloading GitHub issues...".to_string())
    );
}
