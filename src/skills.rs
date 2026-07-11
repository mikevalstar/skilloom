//! Discovering installed skills on disk — folder name plus a bit of metadata.
//!
//! Two sources: the global agent dirs under `$HOME` (Claude Code, the
//! tool-agnostic `~/.agents` store, Codex, Cursor) and the loom-skills repo
//! (`personal/` + `vendor/`). Global skills are kept **grouped by location** so
//! the UI can show a left nav; [`nav_rows`] flattens those groups into
//! selectable rows. Each entry carries its symlink target (if any) and the
//! `description` parsed from its `SKILL.md` frontmatter, so the UI can render a
//! two-line card. In the nav a skill occupies two visual lines; the height math
//! ([`nav_row_height`], [`skill_index_at_line`]) is shared with click hit-testing.

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

/// One installed skill: its folder name, symlink target (if any, home-abbreviated),
/// and the `description` from its `SKILL.md` frontmatter.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    pub link_target: Option<String>,
    pub description: Option<String>,
}

#[cfg(test)]
impl SkillEntry {
    pub fn new(name: impl Into<String>) -> Self {
        SkillEntry {
            name: name.into(),
            link_target: None,
            description: None,
        }
    }
}

/// Skills found in one location (a single agent dir).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SkillGroup {
    /// Display label, e.g. `"~/.claude/skills"`.
    pub label: String,
    /// Skill entries in this location, sorted by name.
    pub skills: Vec<SkillEntry>,
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
/// selectable skill carrying everything the card needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavRow {
    Header(String),
    Empty,
    Skill {
        index: usize,
        name: String,
        location: String,
        link_target: Option<String>,
        description: Option<String>,
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

/// Skill entries (dirs, incl. symlinked dirs) in `dir`, sorted, with symlink
/// targets resolved and `SKILL.md` descriptions parsed. Hidden entries skipped.
pub fn list_skill_entries(dir: &Path) -> Vec<SkillEntry> {
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            // `is_dir` follows symlinks, so a symlink-to-dir still counts.
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let is_symlink = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
            let link_target = if is_symlink {
                std::fs::canonicalize(&path)
                    .ok()
                    .or_else(|| std::fs::read_link(&path).ok())
                    .map(|real| abbreviate_home(&real))
            } else {
                None
            };
            entries.push(SkillEntry {
                name,
                link_target,
                description: read_description(&path),
            });
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Replace a leading `$HOME` with `~` for display.
fn abbreviate_home(path: &Path) -> String {
    if let Some(home) = paths::home_dir()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

/// The `description` from a skill's `SKILL.md` YAML frontmatter, if present.
fn read_description(skill_dir: &Path) -> Option<String> {
    let md = std::fs::read_to_string(skill_dir.join("SKILL.md")).ok()?;
    frontmatter_value(&md, "description")
}

/// Pull a single-line `key: value` out of the leading `---` frontmatter block.
fn frontmatter_value(md: &str, key: &str) -> Option<String> {
    let mut lines = md.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.trim() == key
        {
            let value = unquote(v.trim());
            // Skip empties and YAML block-scalar markers we don't parse.
            if value.is_empty() || value == ">" || value == "|" {
                return None;
            }
            return Some(value);
        }
    }
    None
}

/// Strip a single pair of surrounding single or double quotes.
fn unquote(s: &str) -> String {
    let chars: Vec<char> = s.trim().chars().collect();
    if chars.len() >= 2 {
        let (first, last) = (chars[0], chars[chars.len() - 1]);
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return chars[1..chars.len() - 1].iter().collect();
        }
    }
    chars.into_iter().collect()
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
                skills: list_skill_entries(&dir),
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
            for entry in &group.skills {
                rows.push(NavRow::Skill {
                    index,
                    name: entry.name.clone(),
                    location: group.label.clone(),
                    link_target: entry.link_target.clone(),
                    description: entry.description.clone(),
                });
                index += 1;
            }
        }
    }
    rows
}

/// Visual height of a nav row: skills are two-line cards, everything else one.
pub fn nav_row_height(row: &NavRow) -> usize {
    match row {
        NavRow::Skill { .. } => 2,
        _ => 1,
    }
}

/// The selectable skill index at visual line offset `line` in the nav, if any.
pub fn skill_index_at_line(rows: &[NavRow], line: usize) -> Option<usize> {
    let mut y = 0;
    for row in rows {
        let h = nav_row_height(row);
        if line >= y && line < y + h {
            return match row {
                NavRow::Skill { index, .. } => Some(*index),
                _ => None,
            };
        }
        y += h;
    }
    None
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

    #[cfg(unix)]
    #[test]
    fn list_skill_entries_flags_symlinks_with_target() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real-skill");
        fs::create_dir(&real).unwrap();
        let store = tmp.path().join("store");
        fs::create_dir(&store).unwrap();
        symlink(&real, store.join("linked")).unwrap();
        fs::create_dir(store.join("plain")).unwrap();

        let entries = list_skill_entries(&store);
        let linked = entries.iter().find(|e| e.name == "linked").unwrap();
        let plain = entries.iter().find(|e| e.name == "plain").unwrap();
        assert!(plain.link_target.is_none());
        assert!(
            linked
                .link_target
                .as_ref()
                .is_some_and(|t| t.contains("real-skill"))
        );
    }

    #[test]
    fn list_skill_entries_reads_description() {
        let tmp = tempfile::tempdir().unwrap();
        let skill = tmp.path().join("mine");
        fs::create_dir(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: mine\ndescription: A tiny skill\n---\n# Body\n",
        )
        .unwrap();
        let entries = list_skill_entries(tmp.path());
        let mine = entries.iter().find(|e| e.name == "mine").unwrap();
        assert_eq!(mine.description.as_deref(), Some("A tiny skill"));
    }

    #[test]
    fn frontmatter_reads_quoted_and_plain_values() {
        let md = "---\nname: herdr\ndescription: \"Control herdr from inside it.\"\n---\n# Body\n";
        assert_eq!(
            frontmatter_value(md, "description").as_deref(),
            Some("Control herdr from inside it.")
        );
        assert_eq!(frontmatter_value(md, "name").as_deref(), Some("herdr"));
        assert_eq!(frontmatter_value("no frontmatter here", "name"), None);
    }

    #[test]
    fn nav_rows_number_skills_and_mark_empty_groups() {
        let scan = GlobalScan {
            groups: vec![
                SkillGroup {
                    label: "A".to_string(),
                    skills: vec![SkillEntry::new("x"), SkillEntry::new("y")],
                },
                SkillGroup {
                    label: "B".to_string(),
                    skills: vec![],
                },
                SkillGroup {
                    label: "C".to_string(),
                    skills: vec![SkillEntry::new("z")],
                },
            ],
        };
        let rows = nav_rows(&scan);
        assert_eq!(skill_count(&rows), 3);
        assert!(matches!(&rows[0], NavRow::Header(l) if l == "A"));
        assert!(matches!(&rows[1], NavRow::Skill { index: 0, .. }));
        assert!(matches!(&rows[4], NavRow::Empty));
        let last = skill_at(&rows, 2).unwrap();
        assert!(
            matches!(last, NavRow::Skill { name, location, .. } if name == "z" && location == "C")
        );
    }

    #[test]
    fn skill_index_at_line_accounts_for_two_line_cards() {
        let scan = GlobalScan {
            groups: vec![SkillGroup {
                label: "A".to_string(),
                skills: vec![SkillEntry::new("x"), SkillEntry::new("y")],
            }],
        };
        let rows = nav_rows(&scan);
        // Header @0; skill x @1,2; skill y @3,4.
        assert_eq!(skill_index_at_line(&rows, 0), None);
        assert_eq!(skill_index_at_line(&rows, 1), Some(0));
        assert_eq!(skill_index_at_line(&rows, 2), Some(0));
        assert_eq!(skill_index_at_line(&rows, 3), Some(1));
        assert_eq!(skill_index_at_line(&rows, 4), Some(1));
        assert_eq!(skill_index_at_line(&rows, 5), None);
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
