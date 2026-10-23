use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use prowly::app::{App, AppTab, FocusedPane};
use prowly::config::Config;
use prowly::i18n::Language;
use prowly::ui::ThemeId;

#[test]
fn test_app_initialization_defaults() {
    let cfg = Config::default();
    let app = App::new(cfg);

    assert_eq!(app.active_tab, AppTab::Changes);
    assert_eq!(app.focused_pane, FocusedPane::MainContent);
    assert!(!app.should_quit);
    assert!(!app.ai_modal.is_open);
}

#[test]
fn test_tab_navigation_keys() {
    let mut app = App::new(Config::default());

    // Press '2' -> History
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::History);

    // Press '3' -> Issues
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Issues);

    // Press '4' -> PRs
    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Prs);

    // Press '5' -> Actions
    app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Actions);

    // Press '6' -> Alerts
    app.handle_key(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Alerts);

    // Press '7' -> Worktrees
    app.handle_key(KeyEvent::new(KeyCode::Char('7'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Worktrees);

    // Press 'i' -> Issues
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Issues);

    // Press '1' -> Changes
    app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Changes);
}

#[test]
fn test_pane_focus_toggle() {
    let mut app = App::new(Config::default());
    assert_eq!(app.focused_pane, FocusedPane::MainContent);

    // Press Tab
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, FocusedPane::DirectoryMap);

    // Press Tab again
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.focused_pane, FocusedPane::MainContent);
}

#[test]
fn test_theme_cycling() {
    let mut app = App::new(Config::default());
    let initial_theme = app.theme.id;

    // Cycle through all 21 themes
    for _ in 0..ThemeId::ALL.len() {
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    }
    // Must return to initial theme after full cycle
    assert_eq!(app.theme.id, initial_theme);
}

#[test]
fn test_language_toggle() {
    let mut app = App::new(Config::default());
    assert_eq!(app.i18n.language(), Language::En);

    // Press 'l' -> Japanese
    app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(app.i18n.language(), Language::Ja);

    // Press 'l' -> English
    app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(app.i18n.language(), Language::En);
}

#[test]
fn test_ai_modal_interaction() {
    let mut app = App::new(Config::default());
    assert!(!app.ai_modal.is_open);

    // Press '?' to open
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    assert!(app.ai_modal.is_open);
    assert!(!app.ai_modal.context_summary.is_empty());

    // Type query
    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(app.ai_modal.input_query, "Hi");

    // Press Enter to submit
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(!app.ai_modal.response_text.is_empty());

    // Press Esc to close
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.ai_modal.is_open);
}

#[test]
fn test_stage_checkbox_toggle_space() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.local").unwrap();

    let file = repo_path.join("file.txt");
    std::fs::write(&file, "test content").unwrap();

    let orig_dir = std::env::current_dir().ok();
    let cfg = Config {
        auto_save_enabled: false,
        ..Default::default()
    };
    let mut app = App::new(cfg);
    app.switch_repository(repo_path);

    assert_eq!(app.active_tab, AppTab::Changes);
    assert_eq!(app.focused_pane, FocusedPane::MainContent);

    assert_eq!(app.repo_status.files.len(), 1);
    let initial_staged = app.repo_status.files[0].staged;
    assert!(!initial_staged);

    // Press Space to toggle staging
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(app.repo_status.files[0].staged);

    // Press Space again to toggle back
    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(!app.repo_status.files[0].staged);

    if let Some(orig) = orig_dir {
        let _ = std::env::set_current_dir(orig);
    }
}

#[tokio::test]
async fn test_app_initialization_inside_tokio_runtime() {
    let app = App::new(Config::default());
    assert!(!app.should_quit);
}

#[test]
fn test_middle_pane_mouse_click_select_file() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let mut app = App::new(Config::default());
    app.active_tab = AppTab::Changes;
    app.layout_areas = prowly::ui::LayoutAreas::compute(Rect::new(0, 0, 120, 40));

    // Simulate dummy files
    app.repo_status.files = vec![
        prowly::git::GitFileItem {
            path: "file1.rs".to_string(),
            status: prowly::git::FileStatus::Modified,
            staged: false,
        },
        prowly::git::GitFileItem {
            path: "file2.rs".to_string(),
            status: prowly::git::FileStatus::Untracked,
            staged: false,
        },
    ];
    app.changes_view.file_scroll.set_total_items(2);

    // Setup changes view areas
    app.changes_view.file_list_area = Rect {
        x: app.layout_areas.right_content.x,
        y: app.layout_areas.right_content.y,
        width: 30,
        height: 20,
    };

    // Click on row 1 (the 2nd item, "file2.rs")
    // Border is at y, so row 0 is y+1, row 1 is y+2.
    let click_x = app.changes_view.file_list_area.x + 10;
    let click_y = app.changes_view.file_list_area.y + 2;

    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: click_x,
        row: click_y,
        modifiers: KeyModifiers::NONE,
    };

    app.handle_mouse(mouse);

    assert_eq!(app.changes_view.file_scroll.selected_index, 1);
}

#[test]
fn test_app_commit_staged_requires_explicit_staging_and_prevents_empty() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.local").unwrap();

    // Initial commit
    let file = repo_path.join("file.txt");
    std::fs::write(&file, "initial").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    let tree_oid = index.write_tree().unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .unwrap();

    let orig_dir = std::env::current_dir().ok();
    let cfg = Config {
        auto_save_enabled: false,
        ..Default::default()
    };
    let mut app = App::new(cfg);
    app.switch_repository(repo_path);

    // 1. Without any changes, commit_staged should report "No changes to commit"
    app.changes_view.commit_summary = "Trying empty commit".to_string();
    app.commit_staged();
    assert_eq!(app.last_status.as_deref(), Some("No changes to commit"));

    // 2. Modify file without staging it
    std::fs::write(&file, "modified content").unwrap();
    app.refresh_git_status();

    // Verify file is unstaged
    assert_eq!(app.repo_status.files.len(), 1);
    assert!(!app.repo_status.files[0].staged);

    // 3. Call commit_staged() with commit summary - MUST NOT automatically stage unstaged changes!
    app.changes_view.commit_summary = "feat: auto staged change".to_string();
    app.commit_staged();

    let status_str = app.last_status.clone().unwrap_or_default();
    assert!(
        status_str.contains("No staged changes"),
        "Expected warning about no staged changes, got: {}",
        status_str
    );

    // File must remain unstaged and uncommitted
    assert_eq!(app.repo_status.files.len(), 1);
    assert!(!app.repo_status.files[0].staged);

    // 4. Now explicitly stage the files
    app.stage_all_files();
    assert!(app.repo_status.files[0].staged);

    // 5. Now committing staged changes succeeds
    app.commit_staged();
    let status_str2 = app.last_status.unwrap_or_default();
    assert!(
        status_str2.starts_with("Committed:"),
        "Expected Committed: prefix, got: {}",
        status_str2
    );

    // After commit, working directory and files should be clean
    assert!(app.repo_status.files.is_empty());
    if let Some(orig) = orig_dir {
        let _ = std::env::set_current_dir(orig);
    }
}

#[test]
fn test_repo_badge_visibility_rendering() {
    let orig_dir = std::env::current_dir().ok();
    let tmp = tempfile::tempdir().unwrap();
    let repo_path = tmp.path();
    let repo = git2::Repository::init(repo_path).unwrap();
    repo.remote("origin", "https://github.com/example/test-repo.git")
        .unwrap();

    let render_content = |app: &mut App| -> String {
        let backend = ratatui::backend::TestBackend::new(120, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| app.render(f)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    };

    let cfg = Config {
        auto_save_enabled: false,
        ..Default::default()
    };
    let mut app = App::new(cfg);
    app.switch_repository(repo_path);

    // 1. When is_repo_private is Some(false) (Public)
    app.set_repo_visibility(false);
    let content = render_content(&mut app);
    assert!(
        content.contains("Public"),
        "Expected Public badge when is_repo_private=false"
    );
    assert!(
        !content.contains("Private"),
        "Did not expect Private badge when is_repo_private=false"
    );

    // 2. When is_repo_private is Some(true) (Private)
    app.set_repo_visibility(true);
    let content2 = render_content(&mut app);
    assert!(
        content2.contains("Private"),
        "Expected Private badge when is_repo_private=true"
    );

    // 3. In Japanese mode
    app.i18n.set_language(Language::Ja);
    assert_eq!(app.i18n.t("header.public"), "公開");
    assert_eq!(app.i18n.t("header.private"), "非公開");

    app.set_repo_visibility(false);
    let content3 = render_content(&mut app);
    assert!(
        content3.contains("公") && content3.contains("開"),
        "Expected 公開 badge in Japanese mode when is_repo_private=false"
    );

    app.set_repo_visibility(true);
    let content4 = render_content(&mut app);
    assert!(
        content4.contains("非") && content4.contains("開"),
        "Expected 非公開 badge in Japanese mode when is_repo_private=true"
    );
    if let Some(orig) = orig_dir {
        let _ = std::env::set_current_dir(orig);
    }
}

#[test]
fn test_stage_all_action_button_and_batch_staging() {
    use prowly::ui::views::changes::ChangesButtonAction;
    use ratatui::layout::Rect;

    let mut app = App::new(Config::default());
    app.git_service = None;
    app.layout_areas.right_content = Rect {
        x: 0,
        y: 0,
        width: 140,
        height: 30,
    };
    app.changes_view.actions_bar_area = Rect {
        x: 10,
        y: 20,
        width: 120,
        height: 1,
    };

    // 1. Verify hit_test_action_buttons hits StageAll at relative x=0..14
    let hit = app.changes_view.hit_test_action_buttons(12, 20, true);
    assert_eq!(hit, Some(ChangesButtonAction::StageAll));

    // 2. Prepare mock repo status with unstaged files
    app.repo_status.files = vec![
        prowly::git::GitFileItem {
            path: "file1.txt".to_string(),
            status: prowly::git::FileStatus::Modified,
            staged: false,
        },
        prowly::git::GitFileItem {
            path: "file2.txt".to_string(),
            status: prowly::git::FileStatus::Untracked,
            staged: false,
        },
    ];

    // 3. Trigger Stage All via mouse event on the action button
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 12,
        row: 20,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    app.handle_mouse(mouse);

    // 4. Verify all files are now staged
    assert!(app.repo_status.files.iter().all(|f| f.staged));
    assert_eq!(app.last_status.as_deref(), Some("Staged all files"));
}

#[test]
fn test_all_action_buttons_hit_testing() {
    use ratatui::layout::Rect;

    let mut app = App::new(Config::default());
    app.changes_view.actions_bar_area = Rect {
        x: 0,
        y: 10,
        width: 200,
        height: 1,
    };

    let defs = prowly::ui::views::changes::ChangesView::action_button_defs(true);
    let mut cur_x = 0;
    for (icon, text, expected_action) in defs {
        let pill_spans = prowly::ui::capsule::pill(
            icon,
            text,
            ratatui::style::Color::Reset,
            ratatui::style::Color::Reset,
            ratatui::style::Color::Reset,
            true,
        );
        let w = prowly::ui::capsule::spans_width(&pill_spans) as u16;

        // Test start, middle, and end-1 of each pill
        assert_eq!(
            app.changes_view.hit_test_action_buttons(cur_x, 10, true),
            Some(expected_action),
            "Start of button '{}' at col {}",
            text,
            cur_x
        );
        assert_eq!(
            app.changes_view
                .hit_test_action_buttons(cur_x + w / 2, 10, true),
            Some(expected_action),
            "Middle of button '{}' at col {}",
            text,
            cur_x + w / 2
        );
        assert_eq!(
            app.changes_view
                .hit_test_action_buttons(cur_x + w - 1, 10, true),
            Some(expected_action),
            "End of button '{}' at col {}",
            text,
            cur_x + w - 1
        );

        // Gap between buttons should return None
        cur_x += w;
        assert_eq!(
            app.changes_view.hit_test_action_buttons(cur_x, 10, true),
            None,
            "Gap after button '{}' at col {}",
            text,
            cur_x
        );
        cur_x += 1; // plus space separator
    }
}
