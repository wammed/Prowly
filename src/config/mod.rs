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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Preferred terminal or GUI editor command (e.g. "nvim", "code", or None to use $EDITOR)
    pub command: Option<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { command: None }
    }
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
    /// Editor integration
    pub editor: EditorConfig,
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
        }
    }
}

impl Config {
    /// Standard user config path (`~/.config/prowly/config.toml` or OS equivalent).
    pub fn standard_config_path() -> Result<PathBuf, ConfigError> {
        let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(base.join("prowly").join("config.toml"))
    }

    /// Load configuration following precedence:
    /// 1. Local `./prowly.toml` in current directory if present
    /// 2. User config in `~/.config/prowly/config.toml`
    /// 3. Built-in defaults
    pub fn load() -> Self {
        let local_path = Path::new("prowly.toml");
        if local_path.is_file() {
            if let Ok(cfg) = Self::load_from_path(local_path) {
                return cfg;
            }
        }

        if let Ok(std_path) = Self::standard_config_path() {
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
                    }
                }
                _ => {}
            }
        }
    }
}
