//! Configuration management for Prowly.
//! Handles persistent user settings, theme selection, i18n, AI settings, and Git preferences.

use crate::ai::AiConfig;
use crate::i18n::Language;
pub mod theme_id;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
pub use theme_id::ThemeId;
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
    /// Whether auto-save to disk is enabled (disabled during corrupted load or in tests)
    #[serde(skip)]
    pub auto_save_enabled: bool,
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
            auto_save_enabled: true,
        }
    }
}

/// Partial configuration for local workspace repository (`./prowly.toml`).
#[derive(Debug, Deserialize)]
struct WorkspaceConfig {
    theme: Option<ThemeId>,
    language: Option<Language>,
    use_nerd_font: Option<bool>,
    git: Option<WorkspaceGitConfig>,
    ai: Option<WorkspaceAiConfig>,
    editor: Option<WorkspaceEditorConfig>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceGitConfig {
    soft_discard_backup: Option<bool>,
    auto_fetch_interval_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceAiConfig {
    api_key: Option<String>,
    api_base: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceEditorConfig {
    command: Option<String>,
}

impl Config {
    /// Return the standard template content for config.toml directly synced from config.toml.example.
    pub fn template_content() -> &'static str {
        include_str!("../../config.toml.example")
    }

    /// Standard user config path (`~/.config/prowly/config.toml` or OS equivalent, or overridden by `PROWLY_CONFIG_DIR`).
    pub fn standard_config_path() -> Result<PathBuf, ConfigError> {
        if let Ok(dir) = std::env::var("PROWLY_CONFIG_DIR") {
            if !dir.trim().is_empty() {
                return Ok(PathBuf::from(dir).join("config.toml"));
            }
        }
        let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(base.join("prowly").join("config.toml"))
    }

    /// Ensure ~/.config/prowly/config.toml template exists, creating it if absent with restricted permissions (0700 dir, 0600 file).
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
            #[cfg(unix)]
            {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&std_path)?;
                file.write_all(Self::template_content().as_bytes())?;
                file.sync_all()?;
            }
            #[cfg(not(unix))]
            {
                fs::write(&std_path, Self::template_content())?;
            }
        }
        Ok(std_path)
    }

    /// Load configuration following trust boundaries:
    /// 1. Authoritative user config in `~/.config/prowly/config.toml` (or OS equivalent)
    /// 2. Environment variables for API keys
    /// 3. Workspace preferences in `./prowly.toml` (safe UI options only, NEVER api_key, api_base, or editor.command)
    pub fn load() -> Self {
        // Step 1: Load trusted user config
        let mut config = if let Ok(std_path) = Self::ensure_template_exists() {
            if std_path.is_file() {
                match Self::load_from_path(&std_path) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!(
                            "⚠️ Warning: Failed to parse user configuration at {:?}: {}",
                            std_path, e
                        );
                        let bak_path = std_path.with_extension("toml.bak");
                        if let Err(bak_err) = fs::copy(&std_path, &bak_path) {
                            eprintln!(
                                "⚠️ Failed to create config backup {:?}: {}",
                                bak_path, bak_err
                            );
                        } else {
                            eprintln!("⚠️ Saved backup of existing config to {:?}", bak_path);
                        }
                        // CRITICAL: Protect user's broken file from being silently overwritten
                        Self {
                            auto_save_enabled: false,
                            ..Default::default()
                        }
                    }
                }
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        // Step 2: Populate environment API keys into ephemeral runtime config
        config.populate_api_keys_from_env();

        // Step 3: Check local repository `./prowly.toml` for safe workspace overrides
        let local_path = Path::new("prowly.toml");
        if local_path.is_file() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(local_cfg) = toml::from_str::<WorkspaceConfig>(&content) {
                    if let Some(ref ai) = local_cfg.ai {
                        if ai.api_base.is_some() || ai.api_key.is_some() {
                            eprintln!(
                                "⚠️ Security Guard: Local prowly.toml cannot override AI API keys or endpoints. Ignored."
                            );
                        }
                    }
                    if let Some(ref ed) = local_cfg.editor {
                        if ed.command.is_some() {
                            eprintln!(
                                "⚠️ Security Guard: Local prowly.toml cannot override editor command. Ignored."
                            );
                        }
                    }
                    if let Some(theme) = local_cfg.theme {
                        config.theme = theme;
                    }
                    if let Some(language) = local_cfg.language {
                        config.language = language;
                    }
                    if let Some(use_nerd_font) = local_cfg.use_nerd_font {
                        config.use_nerd_font = use_nerd_font;
                    }
                    if let Some(git) = local_cfg.git {
                        if let Some(soft) = git.soft_discard_backup {
                            config.git.soft_discard_backup = soft;
                        }
                        if let Some(interval) = git.auto_fetch_interval_secs {
                            config.git.auto_fetch_interval_secs = interval;
                        }
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

    /// Save configuration to an explicit file path atomically, creating parent directories if needed and setting 0600 on Unix.
    pub fn save_to_path(&self, path: &Path) -> Result<(), ConfigError> {
        if !self.auto_save_enabled {
            return Ok(());
        }
        let parent = match path.parent() {
            Some(p) => p,
            None => return Err(ConfigError::NoConfigDir),
        };
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
        let content = toml::to_string_pretty(self)?;

        // Atomic write: write to a temporary file with restricted mode 0600, then rename to target path.
        let tmp_path = parent.join(format!(
            ".prowly_cfg_{}_{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&tmp_path, content)?;
        }

        fs::rename(&tmp_path, path)?;
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

    /// Automatically read standard environment variables into ephemeral env_api_key (never saved to file).
    pub fn populate_api_keys_from_env(&mut self) {
        if self.ai.api_key.is_none() {
            self.ai.env_api_key = match self.ai.provider {
                crate::ai::AiProvider::OpenAi => std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|k| !k.trim().is_empty()),
                crate::ai::AiProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY")
                    .ok()
                    .filter(|k| !k.trim().is_empty()),
                crate::ai::AiProvider::Gemini => std::env::var("GEMINI_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                    .filter(|k| !k.trim().is_empty()),
                _ => None,
            };
        } else {
            self.ai.env_api_key = None;
        }
    }
}
