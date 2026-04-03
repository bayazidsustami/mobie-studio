use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::llm::LlmConfig;

// ---------------------------------------------------------------------------
// AppConfig – top-level persisted configuration
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    pub llm: LlmConfig,
}

fn default_version() -> u32 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            llm: LlmConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mobie");
    dir.join("config.toml")
}

/// Load config from `~/.config/mobie/config.toml`.
/// Returns `AppConfig::default()` if the file is missing or unreadable.
pub fn load_config() -> AppConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
            Ok(cfg) => {
                info!("Loaded config from {:?}", path);
                cfg
            }
            Err(e) => {
                warn!(
                    "Failed to parse config file {:?}: {}. Using defaults.",
                    path, e
                );
                AppConfig::default()
            }
        },
        Err(_) => {
            info!("No config file found at {:?}, using defaults.", path);
            AppConfig::default()
        }
    }
}

/// Persist config to `~/.config/mobie/config.toml`.
pub fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir {:?}", parent))?;
    }

    let contents = toml::to_string_pretty(cfg).context("Failed to serialize config to TOML")?;

    std::fs::write(&path, contents)
        .with_context(|| format!("Failed to write config to {:?}", path))?;

    info!("Saved config to {:?}", path);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_round_trips() {
        let cfg = AppConfig::default();
        let serialized = toml::to_string_pretty(&cfg).expect("serialize");
        let deserialized: AppConfig = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(cfg.llm.model, deserialized.llm.model);
        assert_eq!(cfg.llm.provider, deserialized.llm.provider);
    }

    #[test]
    fn test_config_with_api_key() {
        let cfg = AppConfig {
            version: 1,
            llm: LlmConfig {
                api_key: "sk-test-key".to_string(),
                model: "gpt-4o-mini".to_string(),
                ..LlmConfig::default()
            },
        };
        let s = toml::to_string_pretty(&cfg).expect("serialize");
        let loaded: AppConfig = toml::from_str(&s).expect("deserialize");
        assert_eq!(loaded.llm.api_key, "sk-test-key");
        assert_eq!(loaded.llm.model, "gpt-4o-mini");
    }
}
