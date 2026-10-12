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
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            soft_discard_backup: true,
            auto_fetch_interval_secs: 60,
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
    pub theme: ThemeId,
    /// Active UI and prompt language
    pub language: Language,
    /// Whether to render high-definition Nerd Font glyphs
    pub use_nerd_font: bool,
    /// AI service settings
    pub ai: AiConfig,
    /// Git engine preferences
    pub git: GitConfig,
    /// External editor integration
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

# UI Theme: CatppuccinMocha, TokyoNight, Dracula, GruvboxDark, OneDark, Nord, GitHubDark, CatppuccinLatte
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

    /// Ensure ~/.config/prowly/config.toml template exists, creating it if absent.
    pub fn ensure_template_exists() -> Result<PathBuf, ConfigError> {
        let std_path = Self::standard_config_path()?;
        if !std_path.is_file() {
            if let Some(parent) = std_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&std_path, Self::template_content())?;
        }
        Ok(std_path)
    }

    /// Load configuration following precedence:
    /// 1. Local `./prowly.toml` in current directory if present
    /// 2. User config in `~/.config/prowly/config.toml`
    /// 3. Built-in defaults
    /// Automatically writes template config to ~/.config/prowly/config.toml if not present.
    pub fn load() -> Self {
        let local_path = Path::new("prowly.toml");
        if local_path.is_file() {
            if let Ok(cfg) = Self::load_from_path(local_path) {
                return cfg;
            }
        }

        // Try standard config path and auto-generate template if missing
        if let Ok(std_path) = Self::ensure_template_exists() {
            if std_path.is_file() {
                if let Ok(cfg) = Self::load_from_path(&std_path) {
                    return cfg;
                }
            }
        }

        let mut default_cfg = Self::default();
        default_cfg.populate_api_keys_from_env();
        default_cfg
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

    /// Save configuration to an explicit file path, creating parent directories if needed.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Add a path to recent repositories (up to 10 most recent).
    pub fn add_recent_repository(&mut self, path: String) {
        self.recent_repositories.retain(|p| p != &path);
        self.recent_repositories.insert(0, path.clone());
        if self.recent_repositories.len() > 10 {
            self.recent_repositories.truncate(10);
        }
        self.last_repository = Some(path);
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
