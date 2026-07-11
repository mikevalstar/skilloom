//! On-disk config: `~/.config/skilloom/config.toml`.
//!
//! Deliberately tiny for now — just the loom-skills repo location, which is all
//! first-run setup captures. Tracked projects, agent targets, and the
//! which-goes-where curation land here as the engine grows (see overview.md).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::paths;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the loom-skills repo. `None` until first-run setup completes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
}

impl Config {
    /// Load the config, or a default (unconfigured) one if the file is absent.
    pub fn load() -> Result<Config> {
        let Some(path) = paths::config_file() else {
            return Ok(Config::default());
        };
        if !path.exists() {
            return Ok(Config::default());
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Write the config, creating `~/.config/skilloom/` if needed.
    pub fn save(&self) -> Result<()> {
        let path = paths::config_file().context("could not determine config path (HOME unset?)")?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }

    /// Whether a usable repo path is set.
    pub fn is_configured(&self) -> bool {
        self.repo_path
            .as_deref()
            .is_some_and(|p| !p.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_roundtrip_preserves_repo_path() {
        let cfg = Config {
            repo_path: Some("/Users/x/projects/loom-skills".to_string()),
        };
        let text = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(
            back.repo_path.as_deref(),
            Some("/Users/x/projects/loom-skills")
        );
    }

    #[test]
    fn empty_config_is_not_configured() {
        assert!(!Config::default().is_configured());
        let blank = Config {
            repo_path: Some("   ".to_string()),
        };
        assert!(!blank.is_configured());
    }

    #[test]
    fn set_repo_is_configured() {
        let cfg = Config {
            repo_path: Some("/x".to_string()),
        };
        assert!(cfg.is_configured());
    }
}
