use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use prowly::app::{App, AppTab};
use prowly::config::Config;
use ratatui::layout::Rect;

#[test]
fn test_all_modals_click_penetration_prevention_in_app() {
    let mut app = App::new(Config::default());
    app.layout_areas.root = Rect::new(0, 0, 100, 30);
    // Tab bar is around y = 1, e.g. x = 20 is History tab, x = 30 is Issues tab
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.is_any_modal_open());

    // -------------------------------------------------------------
    // 1. Branch Switcher Modal
    // -------------------------------------------------------------
    app.branch_switcher.open(
        "main".to_string(),
        vec!["main".to_string(), "feat/user-auth".to_string()],
        None,
    );
    assert!(app.branch_switcher.is_open);
    assert!(app.is_any_modal_open());

    // Click at (20, 1) which would normally switch tab to History if clicked through!
    let click_tab = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 20,
        row: 1,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    app.handle_mouse(click_tab);

    // Tab must NOT have changed!
    assert_eq!(app.active_tab, AppTab::Changes);
    // Modal was outside (20, 1), so it closed cleanly
    assert!(!app.branch_switcher.is_open);
    assert!(!app.is_any_modal_open());

    // Reopen branch switcher and test wheel scroll
    app.branch_switcher.open(
        "main".to_string(),
        vec!["main".to_string(), "feat/user-auth".to_string()],
        None,
    );
    let wheel_down = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 50,
        row: 15,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    app.handle_mouse(wheel_down);
    assert_eq!(app.branch_switcher.selected_index, 1);
    app.branch_switcher.close();

    // -------------------------------------------------------------
    // 2. Conflict Resolver Modal
    // -------------------------------------------------------------
    let conflicted = "fn main() {\n<<<<<<< HEAD\n1\n=======\n2\n>>>>>>> inc\n}";
    app.conflict_resolver
        .open("conflict.rs".to_string(), conflicted.to_string());
    assert!(app.conflict_resolver.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    // Must NOT switch tab!
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.conflict_resolver.is_open);

    // -------------------------------------------------------------
    // 3. AI Assistant Modal
    // -------------------------------------------------------------
    let ctx = prowly::ai::AiContext::new("Changes");
    app.ai_modal.open_with_context(&ctx);
    assert!(app.ai_modal.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.ai_modal.is_open);

    // -------------------------------------------------------------
    // 4. Remote Diff Preview Modal
    // -------------------------------------------------------------
    app.remote_diff_preview.open(vec![], vec![], None);
    assert!(app.remote_diff_preview.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.remote_diff_preview.is_open);

    // -------------------------------------------------------------
    // 5. Quick Interactive Rebase Modal
    // -------------------------------------------------------------
    app.rebase_modal.open(vec![]);
    assert!(app.rebase_modal.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.rebase_modal.is_open);

    // -------------------------------------------------------------
    // 6. Markdown Preview Modal
    // -------------------------------------------------------------
    let theme = prowly::ui::Theme::get(prowly::ui::ThemeId::TokyoNight);
    app.markdown_preview_modal
        .open("README.md".to_string(), "# Hello".to_string(), &theme);
    assert!(app.markdown_preview_modal.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.markdown_preview_modal.is_open);

    // -------------------------------------------------------------
    // 7. Command Palette Modal
    // -------------------------------------------------------------
    app.command_palette.open();
    assert!(app.command_palette.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.command_palette.is_open);

    // -------------------------------------------------------------
    // 8. Settings Modal
    // -------------------------------------------------------------
    app.settings_modal.open_with_config(&app.config);
    assert!(app.settings_modal.is_open);
    assert!(app.is_any_modal_open());

    // Click at (20, 1) is inside settings modal: must NOT switch tab
    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    // Click outside at (0, 0) closes settings modal
    let click_outside = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    app.handle_mouse(click_outside);
    assert!(!app.settings_modal.is_open);

    // -------------------------------------------------------------
    // 9. Open Repo Modal
    // -------------------------------------------------------------
    app.open_open_repo();
    assert!(app.open_repo_modal.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.open_repo_modal.is_open);

    // -------------------------------------------------------------
    // 10. History Actions Modal
    // -------------------------------------------------------------
    let commit = prowly::git::history::HistoryCommitItem {
        graph: "* ".to_string(),
        hash: "abc1234".to_string(),
        full_hash: "abc1234567890123456789abcdef012345678".to_string(),
        refs: "HEAD -> main".to_string(),
        author: "Dev <dev@example.com>".to_string(),
        date: "1m ago".to_string(),
        summary: "test".to_string(),
    };
    app.history_modal.open(commit);
    assert!(app.history_modal.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.history_modal.is_open);

    // -------------------------------------------------------------
    // 11. Context Menu
    // -------------------------------------------------------------
    app.context_menu
        .open_at(Some("test.rs".to_string()), 50, 15);
    assert!(app.context_menu.is_open);
    assert!(app.is_any_modal_open());

    app.handle_mouse(click_tab);
    assert_eq!(app.active_tab, AppTab::Changes);
    assert!(!app.context_menu.is_open);

    assert!(!app.is_any_modal_open());
}
