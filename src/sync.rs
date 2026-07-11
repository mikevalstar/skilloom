//! The sync engine — the first *write* path.
//!
//! Copies a skill from the loom-skills repo into the global agent dirs, and
//! removes an installed global skill. Deliberately TUI-free (no ratatui): the
//! interactive TUI drives it, and the future `--json` surface will reuse it.
//!
//! **Global sync model** (per the design decision): the real content lands once
//! in a canonical store — `~/.agents/skills/<name>/` — and is **symlinked** into
//! the other detected agent dirs (`~/.claude/skills`, `~/.codex/skills`,
//! `~/.cursor/skills`), mirroring how skills.sh fans a store out. Which dirs get
//! the symlink is chosen per-sync (all detected ones, on by default).

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::paths;

/// The canonical store the real content is copied into, relative to `$HOME`.
pub const CANONICAL_DIR: &str = ".agents/skills";

/// Agent dirs (relative to `$HOME`) that a global sync can symlink the canonical
/// copy into — i.e. everything in [`crate::skills::GLOBAL_SKILL_DIRS`] except the
/// canonical store itself.
pub fn link_target_rels() -> Vec<&'static str> {
    crate::skills::GLOBAL_SKILL_DIRS
        .iter()
        .copied()
        .filter(|rel| *rel != CANONICAL_DIR)
        .collect()
}

/// A candidate symlink target for the sync modal: its `$HOME`-relative dir, a
/// display label, and whether that dir currently exists ("detected").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTarget {
    pub rel: String,
    pub label: String,
    pub exists: bool,
}

/// The detected symlink targets under `$HOME` (only dirs that exist on disk).
pub fn detected_link_targets() -> Vec<LinkTarget> {
    let home = paths::home_dir();
    link_target_rels()
        .into_iter()
        .filter_map(|rel| {
            let exists = home.as_ref().map(|h| h.join(rel).is_dir()).unwrap_or(false);
            exists.then(|| LinkTarget {
                rel: rel.to_string(),
                label: format!("~/{rel}"),
                exists,
            })
        })
        .collect()
}

/// Sync a repo skill (`<origin>/<name>`) to the global store, symlinking it into
/// each of `link_rels`. Resolves `$HOME` and the repo path, then delegates to the
/// path-injectable core.
pub fn sync_to_global(
    repo_path: &str,
    origin: &str,
    name: &str,
    link_rels: &[String],
) -> Result<()> {
    let home = paths::home_dir().context("HOME is not set")?;
    let repo_base = paths::expand_tilde(repo_path);
    sync_to_global_in(&home, &repo_base, origin, name, link_rels)
}

/// Path-injectable core of [`sync_to_global`] (so it's testable without touching
/// the real `$HOME`).
fn sync_to_global_in(
    home: &Path,
    repo_base: &Path,
    origin: &str,
    name: &str,
    link_rels: &[String],
) -> Result<()> {
    let src = repo_base.join(origin).join(name);
    if !src.is_dir() {
        bail!("skill not found in repo: {}", src.display());
    }
    let canonical = home.join(CANONICAL_DIR).join(name);
    copy_skill(&src, &canonical)?;
    for rel in link_rels {
        let link = home.join(rel).join(name);
        link_skill(&canonical, &link)?;
    }
    Ok(())
}

/// Copy a skill folder `src` → `dst`, replacing `dst` if it already exists.
pub fn copy_skill(src: &Path, dst: &Path) -> Result<()> {
    remove_path(dst)?;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    copy_dir_recursive(src, dst)
}

/// Point a symlink at `canonical` from `link`, replacing whatever is at `link`.
/// On non-unix platforms, falls back to a copy.
pub fn link_skill(canonical: &Path, link: &Path) -> Result<()> {
    remove_path(link)?;
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    symlink_dir(canonical, link)
}

/// Remove an installed global skill entry at `path`. When `path` is the canonical
/// store copy (`~/.agents/skills/<name>`), also remove any symlinks in the other
/// agent dirs that point at it — otherwise they'd be left dangling. Returns the
/// `$HOME`-relative dirs whose symlinks were cleaned up.
pub fn remove_installed(name: &str, path: &Path) -> Result<Vec<String>> {
    remove_installed_in(paths::home_dir().as_deref(), name, path)
}

fn remove_installed_in(home: Option<&Path>, name: &str, path: &Path) -> Result<Vec<String>> {
    remove_path(path)?;
    let mut cleaned = Vec::new();
    if let Some(home) = home
        && *path == home.join(CANONICAL_DIR).join(name)
    {
        // The link targets still resolve to the (now-removed) canonical path, so
        // this finds exactly the symlinks we just orphaned.
        for rel in dependent_link_rels_in(home, name) {
            remove_path(&home.join(&rel).join(name))?;
            cleaned.push(rel);
        }
    }
    Ok(cleaned)
}

/// Agent dirs (`$HOME`-relative) holding a symlink `<name>` that points at the
/// canonical copy — the links that removing the canonical would orphan.
pub fn dependent_link_rels(name: &str) -> Vec<String> {
    match paths::home_dir() {
        Some(home) => dependent_link_rels_in(&home, name),
        None => Vec::new(),
    }
}

fn dependent_link_rels_in(home: &Path, name: &str) -> Vec<String> {
    let canonical = home.join(CANONICAL_DIR).join(name);
    link_target_rels()
        .into_iter()
        .filter(|rel| symlink_points_to(&home.join(rel).join(name), &canonical))
        .map(|rel| rel.to_string())
        .collect()
}

/// Whether `link` is a symlink that resolves to `target`. Handles **relative**
/// link targets (skills.sh writes e.g. `../../.agents/skills/<name>`) by joining
/// them onto the link's parent, and compares **lexically** (no fs canonicalize) —
/// so it still matches after the target has been removed, and doesn't trip over
/// `/var`↔`/private/var`-style symlinked path prefixes.
fn symlink_points_to(link: &Path, target: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(link) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(dest) = std::fs::read_link(link) else {
        return false;
    };
    let abs = if dest.is_absolute() {
        dest
    } else {
        match link.parent() {
            Some(parent) => parent.join(dest),
            None => dest,
        }
    };
    lexical_normalize(&abs) == lexical_normalize(target)
}

/// Collapse `.` and `..` components without touching the filesystem.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Remove a file, directory, or symlink at `path`. A no-op if it doesn't exist.
/// For a symlink this removes the *link*, never its target.
pub fn remove_path(path: &Path) -> Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("stat {}", path.display())),
    };
    let file_type = meta.file_type();
    if file_type.is_dir() {
        std::fs::remove_dir_all(path).with_context(|| format!("removing dir {}", path.display()))
    } else {
        // Files and symlinks (incl. symlinks to dirs) unlink with remove_file.
        std::fs::remove_file(path).with_context(|| format!("removing {}", path.display()))
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .with_context(|| format!("copying {} → {}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("symlinking {} → {}", link.display(), target.display()))
}

#[cfg(not(unix))]
fn symlink_dir(target: &Path, link: &Path) -> Result<()> {
    // No portable dir-symlink without extra privileges on Windows; copy instead.
    copy_dir_recursive(target, link)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn copy_skill_copies_tree_and_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        write(&src.join("SKILL.md"), "---\nname: x\n---\n");
        write(&src.join("sub/helper.sh"), "echo hi");
        let dst = tmp.path().join("dst");
        // Pre-existing dst content should be replaced, not merged.
        write(&dst.join("stale.txt"), "old");

        copy_skill(&src, &dst).unwrap();
        assert!(dst.join("SKILL.md").is_file());
        assert!(dst.join("sub/helper.sh").is_file());
        assert!(!dst.join("stale.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn link_skill_makes_a_symlink_to_the_canonical_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let canonical = tmp.path().join("agents/skills/x");
        write(&canonical.join("SKILL.md"), "x");
        let link = tmp.path().join("claude/skills/x");

        link_skill(&canonical, &link).unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_link(&link).unwrap(), canonical);
    }

    #[cfg(unix)]
    #[test]
    fn remove_path_unlinks_a_symlink_without_touching_its_target() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        write(&real.join("SKILL.md"), "x");
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        remove_path(&link).unwrap();
        assert!(!link.exists());
        assert!(real.join("SKILL.md").is_file()); // target survives
    }

    #[test]
    fn remove_path_on_missing_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        remove_path(&tmp.path().join("nope")).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn sync_to_global_copies_canonical_and_symlinks_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let repo = tmp.path().join("repo");
        write(
            &repo.join("personal/sample/SKILL.md"),
            "---\nname: sample\n---\n",
        );
        // A "detected" link target dir already exists.
        fs::create_dir_all(home.join(".claude/skills")).unwrap();

        sync_to_global_in(
            &home,
            &repo,
            "personal",
            "sample",
            &[".claude/skills".to_string()],
        )
        .unwrap();

        let canonical = home.join(".agents/skills/sample");
        assert!(canonical.join("SKILL.md").is_file()); // real content
        let link = home.join(".claude/skills/sample");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_link(&link).unwrap(), canonical);
    }

    #[test]
    fn sync_to_global_errors_when_skill_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let err = sync_to_global_in(
            &tmp.path().join("home"),
            &tmp.path().join("repo"),
            "personal",
            "ghost",
            &[],
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn link_target_rels_excludes_the_canonical_store() {
        assert!(!link_target_rels().contains(&CANONICAL_DIR));
        assert!(link_target_rels().contains(&".claude/skills"));
    }

    #[cfg(unix)]
    #[test]
    fn removing_the_canonical_cleans_up_its_symlinks() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let canonical = home.join(".agents/skills/x");
        write(&canonical.join("SKILL.md"), "x");
        // An absolute symlink to the canonical (skilloom's own style)…
        fs::create_dir_all(home.join(".claude/skills")).unwrap();
        symlink(&canonical, home.join(".claude/skills/x")).unwrap();
        // …a relative symlink to it (skills.sh style)…
        fs::create_dir_all(home.join(".cursor/skills")).unwrap();
        symlink("../../.agents/skills/x", home.join(".cursor/skills/x")).unwrap();
        // …and one pointing elsewhere (must be left alone).
        let other = home.join("other");
        write(&other.join("SKILL.md"), "y");
        fs::create_dir_all(home.join(".codex/skills")).unwrap();
        symlink(&other, home.join(".codex/skills/x")).unwrap();

        let cleaned = remove_installed_in(Some(&home), "x", &canonical).unwrap();
        assert_eq!(
            cleaned,
            vec![".claude/skills".to_string(), ".cursor/skills".to_string()]
        );
        assert!(!canonical.exists());
        assert!(!home.join(".claude/skills/x").exists()); // absolute link cleaned
        assert!(!home.join(".cursor/skills/x").exists()); // relative link cleaned
        // The unrelated symlink survives.
        assert!(
            fs::symlink_metadata(home.join(".codex/skills/x"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_symlink_entry_does_not_touch_the_canonical() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let canonical = home.join(".agents/skills/x");
        write(&canonical.join("SKILL.md"), "x");
        fs::create_dir_all(home.join(".claude/skills")).unwrap();
        let link = home.join(".claude/skills/x");
        symlink(&canonical, &link).unwrap();

        let cleaned = remove_installed_in(Some(&home), "x", &link).unwrap();
        assert!(cleaned.is_empty());
        assert!(!link.exists());
        assert!(canonical.join("SKILL.md").is_file()); // canonical untouched
    }

    #[cfg(unix)]
    #[test]
    fn dependent_link_rels_matches_relative_and_absolute_symlinks() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let canonical = home.join(".agents/skills/x");
        write(&canonical.join("SKILL.md"), "x");
        fs::create_dir_all(home.join(".claude/skills")).unwrap();
        symlink("../../.agents/skills/x", home.join(".claude/skills/x")).unwrap(); // relative

        assert_eq!(
            dependent_link_rels_in(&home, "x"),
            vec![".claude/skills".to_string()]
        );
        assert!(dependent_link_rels_in(&home, "absent").is_empty());
    }
}
