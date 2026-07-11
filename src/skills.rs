//! Discovering installed skills on disk — by folder name, for now.
//!
//! Two sources: the global agent dirs under `$HOME` (where Claude Code, the
//! tool-agnostic `~/.agents` store, Codex, and Cursor look), and the loom-skills
//! repo (`personal/` + `vendor/`). A "skill" is just a subdirectory here; reading
//! `SKILL.md` and richer metadata comes later.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::paths;

/// Global agent skill directories to scan, relative to `$HOME`.
/// Claude Code (`~/.claude/skills`), the tool-agnostic `~/.agents/skills` store,
/// Codex (`~/.codex/skills`), and Cursor (`~/.cursor/skills`). Easily extended.
pub const GLOBAL_SKILL_DIRS: &[&str] = &[
    ".claude/skills",
    ".agents/skills",
    ".codex/skills",
    ".cursor/skills",
];

/// A skill installed globally, deduped by folder name across agent dirs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSkill {
    pub name: String,
    /// Short labels of the agent dirs it appears in, e.g. `["claude", "agents"]`.
    pub locations: Vec<String>,
}

/// Result of scanning the global agent dirs.
#[derive(Debug, Default, Clone)]
pub struct GlobalScan {
    /// Agent skill dirs that exist and were scanned (absolute).
    pub scanned_dirs: Vec<PathBuf>,
    /// Skills found, sorted by name.
    pub skills: Vec<GlobalSkill>,
}

/// Skills present in the loom-skills repo.
#[derive(Debug, Default, Clone)]
pub struct RepoScan {
    pub personal: Vec<String>,
    pub vendor: Vec<String>,
}

/// Immediate subdirectory names of `dir`, sorted, hidden dirs skipped.
pub fn subdirs(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with('.') {
                    names.push(name);
                }
            }
        }
    }
    names.sort();
    names
}

/// `".claude/skills"` → `"claude"`.
fn short_label(rel: &str) -> String {
    rel.split('/')
        .next()
        .unwrap_or(rel)
        .trim_start_matches('.')
        .to_string()
}

/// Scan the known global agent skill dirs under `$HOME`.
pub fn scan_global() -> GlobalScan {
    let Some(home) = paths::home_dir() else {
        return GlobalScan::default();
    };
    let resolved: Vec<(String, PathBuf)> = GLOBAL_SKILL_DIRS
        .iter()
        .map(|rel| (short_label(rel), home.join(rel)))
        .collect();
    aggregate_global(&resolved)
}

/// Core aggregation, split out so it's testable with arbitrary dirs.
fn aggregate_global(dirs: &[(String, PathBuf)]) -> GlobalScan {
    let mut scanned_dirs = Vec::new();
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (label, dir) in dirs {
        if !dir.is_dir() {
            continue;
        }
        scanned_dirs.push(dir.clone());
        for name in subdirs(dir) {
            by_name.entry(name).or_default().push(label.clone());
        }
    }
    let skills = by_name
        .into_iter()
        .map(|(name, locations)| GlobalSkill { name, locations })
        .collect();
    GlobalScan {
        scanned_dirs,
        skills,
    }
}

/// Scan `<repo>/personal` and `<repo>/vendor` for skill folders.
pub fn scan_repo(repo_path: &str) -> RepoScan {
    let base = paths::expand_tilde(repo_path);
    RepoScan {
        personal: subdirs(&base.join("personal")),
        vendor: subdirs(&base.join("vendor")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn subdirs_lists_sorted_dirs_only() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("b-skill")).unwrap();
        fs::create_dir(tmp.path().join("a-skill")).unwrap();
        fs::write(tmp.path().join("SKILL.md"), b"x").unwrap();
        fs::create_dir(tmp.path().join(".hidden")).unwrap();
        assert_eq!(subdirs(tmp.path()), vec!["a-skill", "b-skill"]);
    }

    #[test]
    fn aggregate_dedupes_by_name_across_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let claude = tmp.path().join("claude");
        let agents = tmp.path().join("agents");
        fs::create_dir_all(claude.join("herdr")).unwrap();
        fs::create_dir_all(claude.join("okq")).unwrap();
        fs::create_dir_all(agents.join("herdr")).unwrap();
        let dirs = vec![
            ("claude".to_string(), claude),
            ("agents".to_string(), agents),
            ("missing".to_string(), tmp.path().join("nope")),
        ];
        let scan = aggregate_global(&dirs);

        assert_eq!(scan.scanned_dirs.len(), 2);
        let herdr = scan.skills.iter().find(|s| s.name == "herdr").unwrap();
        assert_eq!(herdr.locations, vec!["claude", "agents"]);
        let okq = scan.skills.iter().find(|s| s.name == "okq").unwrap();
        assert_eq!(okq.locations, vec!["claude"]);
    }

    #[test]
    fn scan_repo_reads_personal_and_vendor() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("personal/my-skill")).unwrap();
        fs::create_dir_all(tmp.path().join("vendor/pdf-filling")).unwrap();
        let scan = scan_repo(&tmp.path().to_string_lossy());
        assert_eq!(scan.personal, vec!["my-skill"]);
        assert_eq!(scan.vendor, vec!["pdf-filling"]);
    }

    #[test]
    fn scan_repo_missing_dirs_are_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let scan = scan_repo(&tmp.path().to_string_lossy());
        assert!(scan.personal.is_empty());
        assert!(scan.vendor.is_empty());
    }
}
