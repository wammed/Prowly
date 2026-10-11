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

    // Press '2' -> PRs
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Prs);

    // Press '3' -> Actions
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Actions);

    // Press '4' -> Alerts
    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Alerts);

    // Press '5' -> Worktrees
    app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
    assert_eq!(app.active_tab, AppTab::Worktrees);

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
    let mut app = App::new(Config::default());
    assert_eq!(app.active_tab, AppTab::Changes);
    assert_eq!(app.focused_pane, FocusedPane::MainContent);

    if !app.repo_status.files.is_empty() {
        let initial_staged = app.repo_status.files[0].staged;
        // Press Space
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.repo_status.files[0].staged, !initial_staged);
    }
}

#[tokio::test]
async fn test_app_initialization_inside_tokio_runtime() {
    let app = App::new(Config::default());
    assert!(!app.should_quit);
}
