use prowly::ai::AiProvider;
use prowly::config::Config;
use prowly::ui::ThemeId;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_template_and_example_parsing() {
    let template = Config::template_content();
    let parsed: Config =
        toml::from_str(template).expect("template_content must parse as valid Config");
    assert_eq!(parsed.theme, ThemeId::CatppuccinMocha);
    assert_eq!(parsed.ai.provider, AiProvider::OpenAi);
}

#[test]
fn test_theme_id_deserialization_formats() {
    #[derive(serde::Deserialize)]
    struct ThemeWrapper {
        theme: ThemeId,
    }

    // PascalCase
    let t1: ThemeWrapper = toml::from_str(r#"theme = "CatppuccinMocha""#).unwrap();
    assert_eq!(t1.theme, ThemeId::CatppuccinMocha);
    let t2: ThemeWrapper = toml::from_str(r#"theme = "TokyoNight""#).unwrap();
    assert_eq!(t2.theme, ThemeId::TokyoNight);

    // snake_case
    let t3: ThemeWrapper = toml::from_str(r#"theme = "catppuccin_mocha""#).unwrap();
    assert_eq!(t3.theme, ThemeId::CatppuccinMocha);

    // kebab-case
    let t4: ThemeWrapper = toml::from_str(r#"theme = "tokyo-night""#).unwrap();
    assert_eq!(t4.theme, ThemeId::TokyoNight);
}

#[test]
fn test_ai_provider_deserialization_formats() {
    #[derive(serde::Deserialize)]
    struct AiWrapper {
        provider: AiProvider,
    }

    let p1: AiWrapper = toml::from_str(r#"provider = "openai""#).unwrap();
    assert_eq!(p1.provider, AiProvider::OpenAi);
    let p2: AiWrapper = toml::from_str(r#"provider = "open_ai""#).unwrap();
    assert_eq!(p2.provider, AiProvider::OpenAi);
    let p3: AiWrapper = toml::from_str(r#"provider = "anthropic""#).unwrap();
    assert_eq!(p3.provider, AiProvider::Anthropic);
}

#[test]
fn test_env_api_key_never_written_to_disk() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let mut cfg = Config::default();
    assert!(cfg.ai.api_key.is_none());
    assert!(cfg.ai.env_api_key.is_none());

    // Inject env key via ephemeral field
    cfg.ai.env_api_key = Some("sk-secret-env-key-12345".to_string());
    assert_eq!(cfg.ai.effective_api_key(), Some("sk-secret-env-key-12345"));

    // Save configuration
    cfg.save_to_path(&config_path)
        .expect("save_to_path must succeed");

    // Read back content and verify the secret key is NOT in file
    let file_content = fs::read_to_string(&config_path).unwrap();
    assert!(!file_content.contains("sk-secret-env-key-12345"));
    assert!(!file_content.contains("api_key"));

    // Ensure reloaded config doesn't have it either
    let reloaded: Config = toml::from_str(&file_content).unwrap();
    assert!(reloaded.ai.api_key.is_none());
}

#[test]
fn test_atomic_save_file_permissions() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let cfg = Config::default();
        cfg.save_to_path(&config_path).unwrap();

        let metadata = fs::metadata(&config_path).unwrap();
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
}
