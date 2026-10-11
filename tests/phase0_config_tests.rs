use prowly::config::Config;
use prowly::i18n::Language;
use prowly::ui::ThemeId;
use tempfile::NamedTempFile;

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert_eq!(cfg.theme, ThemeId::CatppuccinMocha);
    assert_eq!(cfg.language, Language::En);
    assert!(cfg.use_nerd_font);
    assert!(cfg.git.soft_discard_backup);
    assert_eq!(cfg.ai.provider, prowly::ai::AiProvider::OpenAi);
}

#[test]
fn test_config_save_and_load_roundtrip() {
    let mut original = Config::default();
    original.theme = ThemeId::TokyoNight;
    original.language = Language::Ja;
    original.use_nerd_font = false;
    original.git.soft_discard_backup = false;
    original.git.auto_fetch_interval_secs = 120;
    original.editor.command = Some("nvim".to_string());
    original.ai.model = "gpt-4o".to_string();

    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    original.save_to_path(temp_path).unwrap();

    let loaded = Config::load_from_path(temp_path).unwrap();

    assert_eq!(loaded.theme, ThemeId::TokyoNight);
    assert_eq!(loaded.language, Language::Ja);
    assert!(!loaded.use_nerd_font);
    assert!(!loaded.git.soft_discard_backup);
    assert_eq!(loaded.git.auto_fetch_interval_secs, 120);
    assert_eq!(loaded.editor.command.as_deref(), Some("nvim"));
    assert_eq!(loaded.ai.model, "gpt-4o");
}

#[test]
fn test_toml_serialization_format() {
    let cfg = Config::default();
    let toml_str = toml::to_string_pretty(&cfg).unwrap();
    assert!(toml_str.contains("catppuccin_mocha"));
    assert!(toml_str.contains("soft_discard_backup = true"));
}
