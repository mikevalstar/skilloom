//! The sync ledger — `~/.config/skilloom/sync.toml`.
//!
//! Records **what has been synced where**: one entry per (skill × destination).
//! Kept in the config folder but separate from `config.toml` so config stays pure
//! intent (repo path, projects) while this grows into the which-goes-where ledger
//! the engine needs. TUI-free, so the future `--json` surface can read it too.
//!
//! Today it holds just `skill`/`origin`/`destination`; richer per-sync metadata
//! (timestamps, resolved commits, content hashes) is derived state that will live
//! in the XDG state dir when change-detection lands — see overview.md.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::paths;

/// Destination key for a synced skill. `global` is the agent dirs under `$HOME`;
/// project destinations (`project:<path>`) arrive with the Projects tab.
pub const GLOBAL: &str = "global";

/// One synced skill at one destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRecord {
    /// Skill folder name.
    pub skill: String,
    /// Where it came from in the repo: `personal` or `vendor`.
    pub origin: String,
    /// Where it was synced to: `global` (later `project:<path>`).
    pub destination: String,
}

/// The whole ledger. Serializes as a `[[synced]]` array of tables in `sync.toml`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default, rename = "synced")]
    pub records: Vec<SyncRecord>,
}

impl Ledger {
    /// Load from `sync.toml`, or an empty ledger if it's missing/unreadable.
    /// Best-effort (a ledger is recoverable state, not a hard dependency).
    pub fn load() -> Ledger {
        paths::ledger_file()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    /// Load from an explicit path (empty on missing/parse error).
    pub fn load_from(path: &Path) -> Ledger {
        let Ok(text) = fs::read_to_string(path) else {
            return Ledger::default();
        };
        toml::from_str(&text).unwrap_or_default()
    }

    /// Write to `sync.toml`, creating the config dir if needed.
    pub fn save(&self) -> Result<()> {
        let path = paths::ledger_file().context("could not determine ledger path (HOME unset?)")?;
        self.save_to(&path)
    }

    /// Write to an explicit path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing ledger")?;
        fs::write(path, text).with_context(|| format!("writing {}", path.display()))
    }

    /// Record a sync, replacing any existing entry for the same skill × destination.
    pub fn record(&mut self, rec: SyncRecord) {
        self.records
            .retain(|r| !(r.skill == rec.skill && r.destination == rec.destination));
        self.records.push(rec);
    }

    /// Drop the entry (if any) for a skill × destination.
    pub fn forget(&mut self, skill: &str, destination: &str) {
        self.records
            .retain(|r| !(r.skill == skill && r.destination == destination));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(skill: &str, dest: &str) -> SyncRecord {
        SyncRecord {
            skill: skill.to_string(),
            origin: "personal".to_string(),
            destination: dest.to_string(),
        }
    }

    fn has(l: &Ledger, skill: &str, dest: &str) -> bool {
        l.records
            .iter()
            .any(|r| r.skill == skill && r.destination == dest)
    }

    #[test]
    fn record_is_deduped_per_skill_and_destination() {
        let mut l = Ledger::default();
        l.record(rec("a", GLOBAL));
        l.record(rec("a", GLOBAL)); // same key → replace, not duplicate
        l.record(rec("a", "project:/x")); // different destination → separate
        assert_eq!(l.records.len(), 2);
        assert!(has(&l, "a", GLOBAL));
        assert!(has(&l, "a", "project:/x"));
    }

    #[test]
    fn forget_removes_only_the_matching_entry() {
        let mut l = Ledger::default();
        l.record(rec("a", GLOBAL));
        l.record(rec("b", GLOBAL));
        l.forget("a", GLOBAL);
        assert!(!has(&l, "a", GLOBAL));
        assert!(has(&l, "b", GLOBAL));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("skilloom").join("sync.toml");
        let mut l = Ledger::default();
        l.record(rec("sample-skill", GLOBAL));
        l.save_to(&path).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("[[synced]]"));
        assert!(text.contains("sample-skill"));

        let back = Ledger::load_from(&path);
        assert_eq!(back.records, l.records);
    }

    #[test]
    fn load_missing_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let l = Ledger::load_from(&tmp.path().join("nope.toml"));
        assert!(l.records.is_empty());
    }
}
