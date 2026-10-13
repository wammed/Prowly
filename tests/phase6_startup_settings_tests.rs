use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use prowly::config::Config;
use prowly::git::GitService;
use prowly::i18n::Language;
use prowly::ui::modals::open_repo::{OpenRepoAction, OpenRepoModal};
use prowly::ui::modals::settings::{SettingsAction, SettingsModal};
use prowly::ui::theme::ThemeId;
use ratatui::layout::Rect;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_config_template_auto_creation_and_defaults() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");

    let cfg = Config::default();
    assert_eq!(cfg.theme, ThemeId::CatppuccinMocha);
    assert_eq!(cfg.language, Language::En);
    assert!(cfg.use_nerd_font);

    // Save and check content
    cfg.save_to_path(&config_path).unwrap();
    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("theme = \"catppuccin_mocha\""));
    assert!(content.contains("[ai]"));
    assert!(content.contains("provider = \"open_ai\""));
    assert!(Config::template_content().contains("provider = \"openai\""));

    let loaded = Config::load_from_path(&config_path).unwrap();
    assert_eq!(loaded.theme, ThemeId::CatppuccinMocha);
}

#[test]
fn test_settings_modal_interaction_and_save() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let mut config = Config::default();
    config.save_to_path(&config_path).unwrap();

    let mut modal = SettingsModal::new();
    modal.open_with_config(&config);
    assert!(modal.is_open);

    let viewport = Rect::new(0, 0, 100, 30);
    let modal_rect = modal.modal_rect(viewport);

    // Click TokyoNight theme in Row 2
    // Mocha is at cur_x = 10, width = 5 + 4 = 9, so next is at cur_x = 20. TokyoNight width = 10 + 4 = 14 (inner_x: 20..34)
    let _ = modal.handle_click(modal_rect.x + 1 + 22, modal_rect.y + 1 + 2, viewport);
    assert_eq!(modal.theme, ThemeId::TokyoNight);

    // Click Japanese language in Row 3 (inner_x: 23..34)
    let _ = modal.handle_click(modal_rect.x + 1 + 25, modal_rect.y + 1 + 3, viewport);
    assert_eq!(modal.language, Language::Ja);

    // Click Anthropic AI provider in Row 5 (inner_x: OpenAI is 12..22, Anthropic is 23..36)
    let _ = modal.handle_click(modal_rect.x + 1 + 25, modal_rect.y + 1 + 5, viewport);
    assert_eq!(modal.ai_provider, prowly::ai::AiProvider::Anthropic);

    // Click Save button in header row (inner_y == 0, inner_x between width-24 and width-12)
    let save_click = modal.handle_click(
        modal_rect.x + 1 + modal_rect.width - 18,
        modal_rect.y + 1,
        viewport,
    );
    assert_eq!(save_click, Some(SettingsAction::SaveAndApply));

    // Apply & Save to config
    modal.apply_to_config(&mut config);
    config.save_to_path(&config_path).unwrap();

    assert_eq!(config.theme, ThemeId::TokyoNight);
    assert_eq!(config.language, Language::Ja);
    assert_eq!(config.ai.provider, prowly::ai::AiProvider::Anthropic);

    // Verify it was persisted to disk
    let reloaded = Config::load_from_path(&config_path).unwrap();
    assert_eq!(reloaded.theme, ThemeId::TokyoNight);
    assert_eq!(reloaded.language, Language::Ja);
    assert_eq!(reloaded.ai.provider, prowly::ai::AiProvider::Anthropic);
}

#[test]
fn test_settings_modal_responsive_sizing_and_scrolling() {
    let mut modal = SettingsModal::new();
    let config = Config::default();
    modal.open_with_config(&config);

    // 1. Large viewport (140x45) should expand beyond the old 76x28 hard limits
    let large_viewport = Rect::new(0, 0, 140, 45);
    let large_rect = modal.modal_rect(large_viewport);
    assert!(
        large_rect.width >= 90,
        "Modal width should expand to at least 90 on large screens (got {})",
        large_rect.width
    );
    assert!(
        large_rect.height >= 32,
        "Modal height should expand to at least 32 on large screens (got {})",
        large_rect.height
    );

    // 2. Compact viewport (80x24) should support scrolling
    let small_viewport = Rect::new(0, 0, 80, 24);
    assert_eq!(modal.scroll_offset, 0);

    modal.scroll_down(small_viewport);
    assert_eq!(modal.scroll_offset, 1);

    modal.scroll_up();
    assert_eq!(modal.scroll_offset, 0);

    // Keyboard scrolling (j/k)
    let key_down = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    modal.handle_key(key_down);
    assert_eq!(modal.scroll_offset, 1);

    let key_up = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    modal.handle_key(key_up);
    assert_eq!(modal.scroll_offset, 0);
}

#[test]
fn test_open_repo_modal_navigation_and_selection() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("my_project");
    fs::create_dir_all(&repo_dir).unwrap();

    let mut modal = OpenRepoModal::new();
    modal.open_at(Some(tmp.path()), &[]);
    assert!(modal.is_open);

    let viewport = Rect::new(0, 0, 100, 30);
    let modal_rect = modal.modal_rect(viewport);

    // Click Open in header: inner_y == 0, inner_x between width-20 and width-10
    let action = modal.handle_click(
        modal_rect.x + 1 + modal_rect.width - 15,
        modal_rect.y + 1,
        viewport,
    );
    match action {
        Some(OpenRepoAction::Open(p)) => {
            assert_eq!(p, tmp.path());
        }
        other => panic!("Expected OpenRepoAction::Open, got {:?}", other),
    }
}

#[test]
fn test_clean_repo_all_files_listing() {
    let tmp = tempdir().unwrap();
    let repo_path = tmp.path();

    // Initialize clean git repo with 2 committed files
    let repo = git2::Repository::init(repo_path).unwrap();
    let sig = git2::Signature::now("Tester", "test@example.com").unwrap();

    fs::write(repo_path.join("file1.txt"), "hello world\n").unwrap();
    fs::write(repo_path.join("file2.rs"), "fn main() {}\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file1.txt")).unwrap();
    index.add_path(std::path::Path::new("file2.rs")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    let service = GitService::open(repo_path).unwrap();
    let clean_status = service.get_status().unwrap();
    assert!(clean_status.files.is_empty(), "Repo should be clean");

    // All files should still be discoverable!
    let all_files = service.list_all_repo_files().unwrap();
    assert_eq!(all_files.len(), 2);
    assert!(all_files.iter().any(|f| f.path == "file1.txt"));
    assert!(all_files.iter().any(|f| f.path == "file2.rs"));
}

#[test]
fn test_cli_help_and_version_flags() {
    // 1. Verify --help prints usage and exits cleanly with 0
    let help_output = std::process::Command::new(env!("CARGO_BIN_EXE_prowly"))
        .arg("--help")
        .output()
        .expect("Failed to run prowly binary with --help");
    assert!(help_output.status.success());
    let help_str = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_str.contains("Prowly v0.1.0"));
    assert!(help_str.contains("-h, --help"));
    assert!(help_str.contains("-v, --version"));

    // 2. Verify -h prints help
    let short_h_output = std::process::Command::new(env!("CARGO_BIN_EXE_prowly"))
        .arg("-h")
        .output()
        .expect("Failed to run prowly binary with -h");
    assert!(short_h_output.status.success());
    let short_h_str = String::from_utf8_lossy(&short_h_output.stdout);
    assert!(short_h_str.contains("Prowly v0.1.0"));

    // 3. Verify --version prints summary and exits cleanly with 0
    let ver_output = std::process::Command::new(env!("CARGO_BIN_EXE_prowly"))
        .arg("--version")
        .output()
        .expect("Failed to run prowly binary with --version");
    assert!(ver_output.status.success());
    let ver_str = String::from_utf8_lossy(&ver_output.stdout);
    assert!(ver_str.contains("Prowly v0.1.0"));
    assert!(ver_str.contains("Theme:"));
    assert!(ver_str.contains("Language:"));

    // 4. Verify --lang ja --help outputs Japanese help
    let ja_help_output = std::process::Command::new(env!("CARGO_BIN_EXE_prowly"))
        .args(["--lang", "ja", "--help"])
        .output()
        .expect("Failed to run prowly binary with --lang ja --help");
    assert!(ja_help_output.status.success());
    let ja_help_str = String::from_utf8_lossy(&ja_help_output.stdout);
    assert!(ja_help_str.contains("使用方法:"));
    assert!(ja_help_str.contains("オプション:"));
}

