use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use prowly::config::Config;
use prowly::ui::modals::open_repo::{
    OpenRepoAction, OpenRepoLayout, OpenRepoModal, OpenRepoSection,
};
use ratatui::layout::Rect;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_recent_repositories_click_selection_and_open() {
    let tmp = tempdir().unwrap();
    let repo_a = tmp.path().join("repo_alpha");
    let repo_b = tmp.path().join("repo_beta");
    fs::create_dir_all(&repo_a).unwrap();
    fs::create_dir_all(&repo_b).unwrap();

    let recents = vec![repo_a.display().to_string(), repo_b.display().to_string()];

    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(tmp.path()), &recents);

    let viewport = Rect::new(0, 0, 100, 30);
    let modal_rect = modal.modal_rect(viewport);
    let layout = OpenRepoLayout::default_layout();

    // Click on the first recent repository row (inner_y == layout.recent_start)
    let click_x = modal_rect.x + 1 + 10;
    let click_y = modal_rect.y + 1 + layout.recent_start;

    // First click should SELECT it, not open a nonexistent/wrong dir like Pictures
    let action = modal.handle_click(click_x, click_y, viewport);
    assert_eq!(action, None);
    assert_eq!(modal.active_section, OpenRepoSection::Recent);
    assert_eq!(modal.recent_scroll.selected_index, 0);
    assert_eq!(modal.path_input, repo_a.display().to_string());

    // Clicking it a second time (or double click) should OPEN it
    let action_open = modal.handle_click(click_x, click_y, viewport);
    assert_eq!(action_open, Some(OpenRepoAction::Open(repo_a.clone())));

    // Clicking the [ Open ] button specifically on row 2 (inner_y == layout.recent_start + 1)
    let open_btn_x = modal_rect.x + modal_rect.width - 15;
    let click_y_b = modal_rect.y + 1 + layout.recent_start + 1;
    let action_open_b = modal.handle_click(open_btn_x, click_y_b, viewport);
    assert_eq!(action_open_b, Some(OpenRepoAction::Open(repo_b.clone())));

    // Clicking the [ ✕ ] delete button removes it
    let del_btn_x = modal_rect.x + modal_rect.width - 4;
    let action_del = modal.handle_click(del_btn_x, click_y, viewport);
    assert_eq!(
        action_del,
        Some(OpenRepoAction::RemoveRecent(repo_a.display().to_string()))
    );
}

#[test]
fn test_subdirectories_click_does_not_conflict_with_recents() {
    let tmp = tempdir().unwrap();
    let sub1 = tmp.path().join("sub_alpha");
    let sub2 = tmp.path().join("sub_beta");
    fs::create_dir_all(&sub1).unwrap();
    fs::create_dir_all(&sub2).unwrap();

    let recent = tmp.path().join("recent_project");
    fs::create_dir_all(&recent).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(tmp.path()), &[recent.display().to_string()]);

    let viewport = Rect::new(0, 0, 100, 30);
    let modal_rect = modal.modal_rect(viewport);
    let layout = OpenRepoLayout::default_layout();

    // Click on a subdirectory (row 5)
    let click_x = modal_rect.x + 1 + 10;
    let click_y = modal_rect.y + 1 + layout.subdirs_start;
    let action = modal.handle_click(click_x, click_y, viewport);
    assert_eq!(action, None);
    assert_eq!(modal.active_section, OpenRepoSection::Subdirs);

    // Verify layout boundaries: subdirs is strictly 5..11 and recent is 12..16
    assert_eq!(layout.subdirs_start, 5);
    assert_eq!(layout.subdirs_count, 6);
    assert_eq!(layout.recent_start, 12);
    assert_eq!(layout.recent_count, 4);
    assert!(layout.subdirs_start + layout.subdirs_count <= layout.recent_start);
}

#[test]
fn test_tab_path_autocompletion() {
    let tmp = tempdir().unwrap();
    let proj = tmp.path().join("cool_project");
    let other = tmp.path().join("cool_other");
    fs::create_dir_all(&proj).unwrap();
    fs::create_dir_all(&other).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.current_dir = tmp.path().to_path_buf();

    // 1. Unique prefix completion
    modal.path_input = format!("{}/cool_p", tmp.path().display());
    modal.complete_path();
    assert_eq!(
        modal.path_input,
        format!("{}/cool_project/", tmp.path().display())
    );

    // 2. Common prefix completion for multiple matches
    modal.completion_state = None;
    modal.path_input = format!("{}/co", tmp.path().display());
    modal.complete_path();
    assert_eq!(modal.path_input, format!("{}/cool_", tmp.path().display()));

    // Pressing Tab again cycles candidate
    modal.complete_path();
    assert!(modal.path_input.contains("cool_project/") || modal.path_input.contains("cool_other/"));
}

#[test]
fn test_keyboard_navigation_enter_and_shortcuts() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("saved_repo");
    fs::create_dir_all(&repo).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(tmp.path()), &[repo.display().to_string()]);

    // Down from Input moves to Subdirs
    let act = modal.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(act, None);
    assert_eq!(modal.active_section, OpenRepoSection::Subdirs);

    // Tab in Subdirs moves to Recent
    let act = modal.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(act, None);
    assert_eq!(modal.active_section, OpenRepoSection::Recent);

    // Enter in Recent opens the selected repo
    let act = modal.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(act, Some(OpenRepoAction::Open(repo.clone())));
}

#[test]
fn test_config_recent_repo_normalization_and_removal() {
    let mut config = Config::default();

    // Adding paths with and without trailing slash should normalize and avoid duplicate
    config.add_recent_repository("/home/test/repo/".to_string());
    config.add_recent_repository("/home/test/repo".to_string());
    assert_eq!(config.recent_repositories.len(), 1);
    assert_eq!(config.recent_repositories[0], "/home/test/repo");
    assert_eq!(config.last_repository, Some("/home/test/repo".to_string()));

    // Removing path
    config.remove_recent_repository("/home/test/repo/");
    assert!(config.recent_repositories.is_empty());
    assert_eq!(config.last_repository, None);
}
