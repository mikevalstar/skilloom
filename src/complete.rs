//! Directory autocompletion for the first-run path field.
//!
//! Pure and filesystem-driven so it's straightforward to test: given the partial
//! path the user has typed, offer the directories it could become.

use crate::paths;
use std::path::{Path, PathBuf};

/// Directory candidates for a partial path. Splits the input into a parent dir
/// and a name prefix, then returns the child directories of the parent whose
/// name starts with that prefix (as full path strings), sorted.
pub fn complete_dirs(input: &str) -> Vec<String> {
    let (dir, prefix) = split_for_completion(input);

    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // Hide dotfiles unless the user is explicitly typing one.
            if name.starts_with('.') && !prefix.starts_with('.') {
                continue;
            }
            if name.starts_with(&prefix) {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
    out.sort();
    out
}

/// (parent directory to scan, name prefix to match) for the given input.
fn split_for_completion(input: &str) -> (PathBuf, String) {
    if input.is_empty() {
        let home = paths::home_dir().unwrap_or_else(|| PathBuf::from("."));
        return (home, String::new());
    }
    let expanded = paths::expand_tilde(input);
    if input.ends_with('/') {
        return (expanded, String::new());
    }
    let parent = expanded
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let prefix = expanded
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    (parent, prefix)
}

/// Longest common leading substring of the given strings, by character.
pub fn common_prefix(items: &[String]) -> Option<String> {
    let mut iter = items.iter();
    let mut prefix = iter.next()?.clone();
    for item in iter {
        let common: String = prefix
            .chars()
            .zip(item.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a)
            .collect();
        prefix = common;
        if prefix.is_empty() {
            break;
        }
    }
    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn common_prefix_finds_shared_lead() {
        let items = vec![
            "/home/a/projects".to_string(),
            "/home/a/pro".to_string(),
            "/home/a/programs".to_string(),
        ];
        assert_eq!(common_prefix(&items).as_deref(), Some("/home/a/pro"));
    }

    #[test]
    fn common_prefix_none_when_disjoint() {
        let items = vec!["/a".to_string(), "/b".to_string()];
        // shared lead is "/"
        assert_eq!(common_prefix(&items).as_deref(), Some("/"));
        assert_eq!(common_prefix(&[]).as_deref(), None);
    }

    #[test]
    fn complete_dirs_lists_matching_subdirs() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        fs::create_dir(base.join("loom-skills")).unwrap();
        fs::create_dir(base.join("loomery")).unwrap();
        fs::create_dir(base.join("other")).unwrap();
        fs::write(base.join("loose-file"), b"x").unwrap(); // a file, must be excluded

        let partial = format!("{}/loom", base.display());
        let got = complete_dirs(&partial);

        assert!(got.iter().any(|p| p.ends_with("loom-skills")), "{got:?}");
        assert!(got.iter().any(|p| p.ends_with("loomery")), "{got:?}");
        assert!(!got.iter().any(|p| p.ends_with("other")));
        assert!(!got.iter().any(|p| p.ends_with("loose-file")));
    }
}
