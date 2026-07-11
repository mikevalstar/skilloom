//! Discovering installed skills on disk — by folder name, for now.
//!
//! Two sources: the global agent dirs under `$HOME` (Claude Code, the
//! tool-agnostic `~/.agents` store, Codex, Cursor) and the loom-skills repo
//! (`personal/` + `vendor/`). Global skills are kept **grouped by location** so
//! the UI can show a left nav; [`nav_rows`] flattens those groups into
//! renderable/selectable rows. A "skill" is just a subdirectory here; reading
//! `SKILL.md` and richer metadata comes later.

use std::path::Path;

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

/// Skills found in one location (a single agent dir).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SkillGroup {
    /// Display label, e.g. `"~/.claude/skills"`.
    pub label: String,
    /// Skill folder names in this location, sorted.
    pub skills: Vec<String>,
}

/// Skills across the global agent dirs, grouped by location (existing dirs only).
#[derive(Debug, Default, Clone)]
pub struct GlobalScan {
    pub groups: Vec<SkillGroup>,
}

/// Skills present in the loom-skills repo.
#[derive(Debug, Default, Clone)]
pub struct RepoScan {
    pub personal: Vec<String>,
    pub vendor: Vec<String>,
}

/// A row in the grouped left-nav: a location header, an empty-group marker, or a
/// selectable skill carrying its flat selection index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavRow {
    Header(String),
    Empty,
    Skill {
        index: usize,
        name: String,
        location: String,
    },
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

/// Scan the known global agent skill dirs under `$HOME`, grouped by location.
pub fn scan_global() -> GlobalScan {
    let Some(home) = paths::home_dir() else {
        return GlobalScan::default();
    };
    let mut groups = Vec::new();
    for &rel in GLOBAL_SKILL_DIRS {
        let dir = home.join(rel);
        if dir.is_dir() {
            groups.push(SkillGroup {
                label: format!("~/{rel}"),
                skills: subdirs(&dir),
            });
        }
    }
    GlobalScan { groups }
}

/// Scan `<repo>/personal` and `<repo>/vendor` for skill folders.
pub fn scan_repo(repo_path: &str) -> RepoScan {
    let base = paths::expand_tilde(repo_path);
    RepoScan {
        personal: subdirs(&base.join("personal")),
        vendor: subdirs(&base.join("vendor")),
    }
}

/// Flatten groups into left-nav rows, numbering selectable skills in order.
pub fn nav_rows(scan: &GlobalScan) -> Vec<NavRow> {
    let mut rows = Vec::new();
    let mut index = 0;
    for group in &scan.groups {
        rows.push(NavRow::Header(group.label.clone()));
        if group.skills.is_empty() {
            rows.push(NavRow::Empty);
        } else {
            for name in &group.skills {
                rows.push(NavRow::Skill {
                    index,
                    name: name.clone(),
                    location: group.label.clone(),
                });
                index += 1;
            }
        }
    }
    rows
}

/// Number of selectable skills across the nav rows.
pub fn skill_count(rows: &[NavRow]) -> usize {
    rows.iter()
        .filter(|r| matches!(r, NavRow::Skill { .. }))
        .count()
}

/// The nav row for the skill at flat selection `index`, if any.
pub fn skill_at(rows: &[NavRow], index: usize) -> Option<&NavRow> {
    rows.iter()
        .find(|r| matches!(r, NavRow::Skill { index: i, .. } if *i == index))
}

/// Whether a skill (by folder name) is tracked in the loom-skills repo.
pub fn is_in_repo(repo: &RepoScan, name: &str) -> bool {
    repo.personal.iter().any(|n| n == name) || repo.vendor.iter().any(|n| n == name)
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
    fn nav_rows_number_skills_and_mark_empty_groups() {
        let scan = GlobalScan {
            groups: vec![
                SkillGroup {
                    label: "A".to_string(),
                    skills: vec!["x".to_string(), "y".to_string()],
                },
                SkillGroup {
                    label: "B".to_string(),
                    skills: vec![],
                },
                SkillGroup {
                    label: "C".to_string(),
                    skills: vec!["z".to_string()],
                },
            ],
        };
        let rows = nav_rows(&scan);
        assert_eq!(skill_count(&rows), 3);
        // Header(A), Skill0(x), Skill1(y), Header(B), Empty, Header(C), Skill2(z)
        assert!(matches!(&rows[0], NavRow::Header(l) if l == "A"));
        assert!(matches!(&rows[1], NavRow::Skill { index: 0, .. }));
        assert!(matches!(&rows[4], NavRow::Empty));
        let last = skill_at(&rows, 2).unwrap();
        assert!(
            matches!(last, NavRow::Skill { name, location, .. } if name == "z" && location == "C")
        );
    }

    #[test]
    fn is_in_repo_checks_personal_and_vendor() {
        let repo = RepoScan {
            personal: vec!["mine".to_string()],
            vendor: vec!["pdf".to_string()],
        };
        assert!(is_in_repo(&repo, "mine"));
        assert!(is_in_repo(&repo, "pdf"));
        assert!(!is_in_repo(&repo, "nope"));
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
