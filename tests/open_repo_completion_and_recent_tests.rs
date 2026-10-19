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
    let inner_w = modal_rect.width.saturating_sub(2);
    let split_x = inner_w / 2;

    // Right pane click for first recent repo item (inner_y == 3, right_inner_x == 5)
    let click_x = modal_rect.x + 1 + split_x + 5;
    let click_y = modal_rect.y + 1 + 3;

    // First click should SELECT it in right pane
    let action = modal.handle_click(click_x, click_y, viewport);
    assert_eq!(action, None);
    assert_eq!(modal.active_section, OpenRepoSection::Recent);
    assert_eq!(modal.recent_scroll.selected_index, 0);
    assert_eq!(modal.path_input, repo_a.display().to_string());

    // Clicking it a second time (or double click) should OPEN it
    let action_open = modal.handle_click(click_x, click_y, viewport);
    assert_eq!(action_open, Some(OpenRepoAction::Open(repo_a.clone())));

    // Clicking the [ Open ] button on row 2 (inner_y == 4)
    let open_btn_x = modal_rect.x + modal_rect.width - 12;
    let click_y_b = modal_rect.y + 1 + 4;
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
    let inner_w = modal_rect.width.saturating_sub(2);
    let split_x = inner_w / 2;

    // 1. Click on Left Pane (Discovery) at row 3
    let left_x = modal_rect.x + 1 + 5;
    let click_y = modal_rect.y + 1 + 3;
    let action_left = modal.handle_click(left_x, click_y, viewport);
    assert_eq!(action_left, None);
    assert_eq!(modal.active_section, OpenRepoSection::Discovery);

    // 2. Click on Right Pane (Recent) at row 3
    let right_x = modal_rect.x + 1 + split_x + 5;
    let action_right = modal.handle_click(right_x, click_y, viewport);
    assert_eq!(action_right, None);
    assert_eq!(modal.active_section, OpenRepoSection::Recent);

    // Verify layout constants exist for backwards compatibility
    let layout = OpenRepoLayout::default_layout();
    assert_eq!(layout.subdirs_start, 5);
    assert_eq!(layout.subdirs_count, 6);
    assert_eq!(layout.recent_start, 12);
    assert_eq!(layout.recent_count, 4);
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
    assert_eq!(modal.active_section, OpenRepoSection::Input);

    // Down from Input moves to Discovery (left pane)
    let act = modal.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(act, None);
    assert_eq!(modal.active_section, OpenRepoSection::Discovery);

    // Tab in Discovery moves to Recent (right pane top)
    let act = modal.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(act, None);
    assert_eq!(modal.active_section, OpenRepoSection::Recent);

    // Enter in Recent opens the selected repo
    let act = modal.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(act, Some(OpenRepoAction::Open(repo.clone())));
}

#[test]
fn test_clone_url_prompt_flow() {
    let tmp = tempdir().unwrap();
    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(tmp.path()), &[]);

    // Move to Discovery section
    modal.active_section = OpenRepoSection::Discovery;

    // Press 'c' to enter clone mode
    let act = modal.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    assert_eq!(act, None);
    assert!(modal.is_clone_mode);

    // Type remote url
    for c in "https://github.com/foo/bar.git".chars() {
        modal.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    assert_eq!(modal.clone_url_input, "https://github.com/foo/bar.git");

    // Press Enter to confirm clone
    let act = modal.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        act,
        Some(OpenRepoAction::CloneRepo {
            url: "https://github.com/foo/bar.git".to_string(),
            target_dir: tmp.path().to_path_buf(),
        })
    );
    assert!(!modal.is_clone_mode);
}

#[test]
fn test_init_repo_action() {
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("new_empty_folder");
    fs::create_dir_all(&target).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(&target), &[]);
    modal.active_section = OpenRepoSection::Discovery;

    // Press 'i' to initialize repository in current dir
    let act = modal.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    assert_eq!(act, Some(OpenRepoAction::InitRepo(target.clone())));
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

#[test]
fn test_all_click_targets_exact_alignment() {
    use prowly::ui::modals::open_repo::OpenRepoLayoutMap;

    let tmp = tempdir().unwrap();
    let repo_a = tmp.path().join("alpha");
    let repo_b = tmp.path().join("beta");
    fs::create_dir_all(&repo_a).unwrap();
    fs::create_dir_all(&repo_b).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.open_at(
        Some(tmp.path()),
        &[repo_a.display().to_string(), repo_b.display().to_string()],
    );

    let viewport = Rect::new(0, 0, 100, 30);
    let modal_rect = modal.modal_rect(viewport);
    let inner_rect = Rect {
        x: modal_rect.x + 1,
        y: modal_rect.y + 1,
        width: modal_rect.width.saturating_sub(2),
        height: modal_rect.height.saturating_sub(2),
    };
    let layout = OpenRepoLayoutMap::compute(inner_rect);

    // 1. Click Top Header [ Open ] button
    let open_x = inner_rect.x + layout.inner_w - 20;
    let header_y = inner_rect.y + layout.header_y;
    let act = modal.handle_click(open_x, header_y, viewport);
    assert_eq!(act, Some(OpenRepoAction::Open(tmp.path().to_path_buf())));

    // 2. Click Top Header [ Cancel ] button
    let cancel_x = inner_rect.x + layout.inner_w - 5;
    let act = modal.handle_click(cancel_x, header_y, viewport);
    assert_eq!(act, Some(OpenRepoAction::Close));

    // 3. Click Left Pane Toggle [a: All / g: Git]
    let toggle_x = inner_rect.x + layout.split_x - 5;
    let disc_header_y = inner_rect.y + layout.discovery_header_y;
    assert!(!modal.show_all_dirs);
    modal.handle_click(toggle_x, disc_header_y, viewport);
    assert!(modal.show_all_dirs);

    // 4. Click Right Pane Path Input Box
    let path_input_x = inner_rect.x + layout.split_x + 1 + 10;
    let path_input_y = inner_rect.y + layout.path_input_y;
    modal.active_section = OpenRepoSection::Discovery;
    modal.handle_click(path_input_x, path_input_y, viewport);
    assert_eq!(modal.active_section, OpenRepoSection::Input);

    // 5. Click Right Pane [ i ] Init Repo Here button
    let init_x = inner_rect.x + layout.split_x + 1 + 5;
    let actions_y = inner_rect.y + layout.actions_y;
    let act = modal.handle_click(init_x, actions_y, viewport);
    assert_eq!(
        act,
        Some(OpenRepoAction::InitRepo(tmp.path().to_path_buf()))
    );

    // 6. Click Right Pane [ c ] Clone Remote Here button
    let clone_x = inner_rect.x + layout.split_x + 1 + 30;
    let act = modal.handle_click(clone_x, actions_y, viewport);
    assert_eq!(act, None);
    assert!(modal.is_clone_mode);
    assert_eq!(modal.active_section, OpenRepoSection::CloneInput);

    // 7. Click Clone mode URL input
    let clone_input_y = inner_rect.y + layout.clone_url_input_y;
    modal.active_section = OpenRepoSection::Discovery;
    modal.handle_click(path_input_x, clone_input_y, viewport);
    assert_eq!(modal.active_section, OpenRepoSection::CloneInput);

    // 8. Click Clone mode Cancel button
    let clone_cancel_x = inner_rect.x + layout.split_x + 1 + 28;
    let clone_actions_y = inner_rect.y + layout.clone_actions_y;
    modal.handle_click(clone_cancel_x, clone_actions_y, viewport);
    assert!(!modal.is_clone_mode);
}
