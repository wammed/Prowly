//! Configuration management for Prowly.
//! Handles persistent user settings, theme selection, i18n, AI settings, and Git preferences.

use crate::ai::AiConfig;
use crate::i18n::Language;
use crate::ui::ThemeId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error while accessing config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse TOML configuration: {0}")]
    ParseToml(#[from] toml::de::Error),

    #[error("Failed to serialize TOML configuration: {0}")]
    SerializeToml(#[from] toml::ser::Error),

    #[error("Could not determine user configuration directory")]
    NoConfigDir,
}

/// Git specific preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Automatically save a soft backup to hidden stash before discarding changes
    pub soft_discard_backup: bool,
    /// Background auto-fetch interval in seconds (0 = disabled)
    pub auto_fetch_interval_secs: u64,
    /// Restore last opened repository on startup when launched without arguments
    #[serde(default = "default_true")]
    pub restore_last_repository: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            soft_discard_backup: true,
            auto_fetch_interval_secs: 60,
            restore_last_repository: true,
        }
    }
}

/// External editor preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Preferred terminal or GUI editor command (e.g. "nvim", "code", or None to use $EDITOR)
    pub command: Option<String>,
}

/// Root Prowly application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Selected UI theme
    #[serde(default)]
    pub theme: ThemeId,
    /// Active UI and prompt language
    #[serde(default)]
    pub language: Language,
    /// Whether to render high-definition Nerd Font glyphs
    #[serde(default = "default_true")]
    pub use_nerd_font: bool,
    /// AI service settings
    #[serde(default)]
    pub ai: AiConfig,
    /// Git engine preferences
    #[serde(default)]
    pub git: GitConfig,
    /// External editor integration
    #[serde(default)]
    pub editor: EditorConfig,
    /// Last opened repository path
    #[serde(default)]
    pub last_repository: Option<String>,
    /// Recent repositories history
    #[serde(default)]
    pub recent_repositories: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeId::CatppuccinMocha,
            language: Language::En,
            use_nerd_font: true,
            ai: AiConfig::default(),
            git: GitConfig::default(),
            editor: EditorConfig::default(),
            last_repository: None,
            recent_repositories: Vec::new(),
        }
    }
}

impl Config {
    /// Return the standard template content for config.toml.
    pub fn template_content() -> &'static str {
        r#"# ==============================================================================
# Prowly Configuration File
# Location: ~/.config/prowly/config.toml
# ==============================================================================

# UI Theme: CatppuccinMocha, TokyoNight, GruvboxDark, Nord, Dracula, SolarizedDark, OneDark, MonokaiPro, etc. (21 themes)
theme = "CatppuccinMocha"

# Language: "en" (English), "ja" (日本語)
language = "en"

# Nerd Font glyphs: true (recommended for Nerd Font users), false (ASCII fallback)
use_nerd_font = true

# ------------------------------------------------------------------------------
# AI Integration (BYOK - Bring Your Own Key)
# ------------------------------------------------------------------------------
[ai]
# Provider: "openai", "anthropic", "gemini", "ollama", "custom"
provider = "openai"

# Model name:
# - OpenAI: "gpt-4o", "gpt-4o-mini", "o3-mini"
# - Anthropic: "claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022"
# - Gemini: "gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"
# - Ollama: "llama3.2:latest", "deepseek-r1:latest", "codellama"
model = "gpt-4o-mini"

# API Key (or set OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY env vars)
# Note: For Ollama, no API key is required.
# api_key = "your-api-key-here"

# Custom API Base / Endpoint (optional override)
# Supports http://localhost:11434 and http://localhost:11434/api for Ollama.
# api_base = "http://localhost:11434"

temperature = 0.3
max_tokens = 2048
num_ctx = 8192

# ------------------------------------------------------------------------------
# Git Engine Preferences
# ------------------------------------------------------------------------------
[git]
soft_discard_backup = true
auto_fetch_interval_secs = 60

# ------------------------------------------------------------------------------
# External Editor Preferences
# ------------------------------------------------------------------------------
[editor]
# command = "nvim"
"#
    }

    /// Standard user config path (`~/.config/prowly/config.toml` or OS equivalent).
    pub fn standard_config_path() -> Result<PathBuf, ConfigError> {
        let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(base.join("prowly").join("config.toml"))
    }

    /// Ensure ~/.config/prowly/config.toml template exists, creating it if absent with restricted permissions.
    pub fn ensure_template_exists() -> Result<PathBuf, ConfigError> {
        let std_path = Self::standard_config_path()?;
        if !std_path.is_file() {
            if let Some(parent) = std_path.parent() {
                fs::create_dir_all(parent)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
                }
            }
            fs::write(&std_path, Self::template_content())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&std_path, fs::Permissions::from_mode(0o600));
            }
        }
        Ok(std_path)
    }

    /// Load configuration following trust boundaries:
    /// 1. Authoritative user config in `~/.config/prowly/config.toml` (or OS equivalent)
    /// 2. Environment variables for API keys
    /// 3. Workspace preferences in `./prowly.toml` (safe UI options only, NEVER api_key or api_base)
    pub fn load() -> Self {
        // Step 1: Load trusted user config
        let mut config = if let Ok(std_path) = Self::ensure_template_exists() {
            if std_path.is_file() {
                Self::load_from_path(&std_path).unwrap_or_default()
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        // Step 2: Populate environment API keys into user config
        config.populate_api_keys_from_env();

        // Step 3: Check local repository `./prowly.toml` for safe workspace overrides
        let local_path = Path::new("prowly.toml");
        if local_path.is_file() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(local_cfg) = toml::from_str::<Config>(&content) {
                    if local_cfg.ai.api_base.is_some() || local_cfg.ai.api_key.is_some() {
                        eprintln!(
                            "⚠️ Security Guard: Local prowly.toml cannot override AI API keys or endpoints. Ignored."
                        );
                    }
                    config.theme = local_cfg.theme;
                    config.language = local_cfg.language;
                    config.use_nerd_font = local_cfg.use_nerd_font;
                    config.git.soft_discard_backup = local_cfg.git.soft_discard_backup;
                    config.git.auto_fetch_interval_secs = local_cfg.git.auto_fetch_interval_secs;
                    if local_cfg.editor.command.is_some() {
                        config.editor.command = local_cfg.editor.command;
                    }
                }
            }
        }

        config
    }

    /// Load configuration from an explicit file path.
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let mut cfg: Config = toml::from_str(&content)?;
        cfg.populate_api_keys_from_env();
        Ok(cfg)
    }

    /// Save configuration to the standard user config path.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::standard_config_path()?;
        self.save_to_path(&path)
    }

    /// Save configuration to an explicit file path, creating parent directories if needed and setting 0600 on Unix.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    /// Add a path to recent repositories (up to 10 most recent).
    pub fn add_recent_repository(&mut self, path: String) {
        let normalized = if path.len() > 1 && path.ends_with('/') {
            path.trim_end_matches('/').to_string()
        } else {
            path
        };
        self.recent_repositories
            .retain(|p| p.trim_end_matches('/') != normalized);
        self.recent_repositories.insert(0, normalized.clone());
        if self.recent_repositories.len() > 10 {
            self.recent_repositories.truncate(10);
        }
        self.last_repository = Some(normalized);
    }

    /// Remove a path from recent repositories.
    pub fn remove_recent_repository(&mut self, path: &str) {
        let normalized = if path.len() > 1 && path.ends_with('/') {
            path.trim_end_matches('/')
        } else {
            path
        };
        self.recent_repositories
            .retain(|p| p.trim_end_matches('/') != normalized);
        if self
            .last_repository
            .as_deref()
            .map(|p| p.trim_end_matches('/'))
            == Some(normalized)
        {
            self.last_repository = self.recent_repositories.first().cloned();
        }
    }

    /// Automatically read standard environment variables if API keys are not specified.
    pub fn populate_api_keys_from_env(&mut self) {
        if self.ai.api_key.is_none() {
            match self.ai.provider {
                crate::ai::AiProvider::OpenAi => {
                    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        self.ai.api_key = Some(key);
                    }
                }
                crate::ai::AiProvider::Anthropic => {
                    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                        self.ai.api_key = Some(key);
                    }
                }
                crate::ai::AiProvider::Gemini => {
                    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                        self.ai.api_key = Some(key);
                    } else if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
                        self.ai.api_key = Some(key);
                    }
                }
                _ => {}
            }
        }
    }
}
