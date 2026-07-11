//! Path resolution helpers: home, the skilloom config location, and `~` expansion.
//!
//! skilloom's config lives at `~/.config/skilloom/config.toml` (honoring
//! `XDG_CONFIG_HOME`) on every platform — an explicit choice over the macOS-native
//! `~/Library/Application Support`, matching the docs (AGENTS.md, overview.md).

use std::path::PathBuf;

/// The user's home directory, from `$HOME` (empty treated as unset).
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// `~/.config/skilloom` (or `$XDG_CONFIG_HOME/skilloom`).
pub fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("skilloom"));
    }
    home_dir().map(|h| h.join(".config").join("skilloom"))
}

/// `~/.config/skilloom/config.toml`.
pub fn config_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// `~/.config/skilloom/sync.toml` — the sync ledger (what's synced where).
/// Kept beside `config.toml` in the config folder, separate from pure intent.
pub fn ledger_file() -> Option<PathBuf> {
    config_dir().map(|d| d.join("sync.toml"))
}

/// Expand a leading `~` or `~/…` to the home directory. Anything else is
/// returned as-is.
pub fn expand_tilde(input: &str) -> PathBuf {
    if input == "~"
        && let Some(home) = home_dir()
    {
        return home;
    }
    if let Some(rest) = input.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(input)
}
