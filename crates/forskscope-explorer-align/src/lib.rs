//! Aligned-row computation for the two-pane Explorer (RFC-059 §M5).
//!
//! This module contains the pure data logic that merges two flat
//! visible-row lists into an aligned sequence where same-name entries share
//! the same horizontal row. It has no Dioxus dependency and is fully
//! unit-testable without a GUI runtime.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Flat row as produced by `DirectoryTree::visible_rows`.
///
/// `(abs_path, is_dir, is_expanded, is_selected, depth)`
pub type FlatRow = (PathBuf, bool, bool, bool, u32);

/// Per-row data extracted from a `FlatRow` and enriched with the relative path.
#[derive(Clone, Debug, PartialEq)]
pub struct RowData {
    pub abs_path:    PathBuf,
    pub rel_path:    PathBuf,
    pub is_dir:      bool,
    pub is_expanded: bool,
    pub is_selected: bool,
    pub depth:       u32,
}

/// A paired left/right row. `None` on one side means the entry exists only
/// on the other side (renders as a spacer in that half).
pub type AlignedRow = (Option<RowData>, Option<RowData>);

// ── Public entry point ────────────────────────────────────────────────────────

/// Merge two flat visible-row lists into an aligned sequence.
///
/// Same-name entries share the same [`AlignedRow`]; entries only on one
/// side produce a row where the other half is `None`.
///
/// Ordering: directories first, then files, each group alphabetical.
/// Subdirectories are recursed into when *either* side has them expanded.
pub fn compute_aligned_rows(
    left_rows:  &[FlatRow],
    right_rows: &[FlatRow],
    left_root:  &Path,
    right_root: &Path,
) -> Vec<AlignedRow> {
    let mut l_by_parent: HashMap<PathBuf, Vec<RowData>> = HashMap::new();
    let mut r_by_parent: HashMap<PathBuf, Vec<RowData>> = HashMap::new();

    for (rows, by_parent, root) in [
        (left_rows,  &mut l_by_parent, left_root),
        (right_rows, &mut r_by_parent, right_root),
    ] {
        for (abs, is_dir, expanded, selected, depth) in rows.iter() {
            if let Ok(rel) = abs.strip_prefix(root) {
                let parent = rel.parent().unwrap_or(Path::new("")).to_path_buf();
                by_parent.entry(parent).or_default().push(RowData {
                    abs_path:    abs.clone(),
                    rel_path:    rel.to_path_buf(),
                    is_dir:      *is_dir,
                    is_expanded: *expanded,
                    is_selected: *selected,
                    depth:       *depth,
                });
            }
        }
    }

    merge_level(Path::new(""), &l_by_parent, &r_by_parent)
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn merge_level(
    parent:      &Path,
    l_by_parent: &HashMap<PathBuf, Vec<RowData>>,
    r_by_parent: &HashMap<PathBuf, Vec<RowData>>,
) -> Vec<AlignedRow> {
    let empty = vec![];
    let l_kids = l_by_parent.get(parent).unwrap_or(&empty);
    let r_kids = r_by_parent.get(parent).unwrap_or(&empty);

    // Collect all unique names present on either side.
    let mut name_is_dir: HashMap<OsString, bool> = HashMap::new();
    for row in l_kids.iter().chain(r_kids.iter()) {
        if let Some(name) = row.abs_path.file_name() {
            name_is_dir.insert(name.to_os_string(), row.is_dir);
        }
    }

    // Sort: directories first, then alphabetical within each group.
    let mut sorted: Vec<(OsString, bool)> = name_is_dir.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut result = Vec::new();
    for (name, _) in &sorted {
        let find = |kids: &[RowData]| -> Option<RowData> {
            kids.iter()
                .find(|r| r.abs_path.file_name().map(|n| n.to_os_string()) == Some(name.clone()))
                .cloned()
        };
        let l = find(l_kids);
        let r = find(r_kids);

        result.push((l.clone(), r.clone()));

        // Recurse into any expanded subdirectory on either side.
        let l_exp = l.as_ref().map(|d| d.is_dir && d.is_expanded).unwrap_or(false);
        let r_exp = r.as_ref().map(|d| d.is_dir && d.is_expanded).unwrap_or(false);
        if l_exp || r_exp {
            let child_parent = parent.join(name);
            result.extend(merge_level(&child_parent, l_by_parent, r_by_parent));
        }
    }
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(root: &str, entries: &[(&str, bool, bool)]) -> (Vec<FlatRow>, PathBuf) {
        let root = PathBuf::from(root);
        let rows = entries
            .iter()
            .map(|(rel, is_dir, expanded)| {
                (root.join(rel), *is_dir, *expanded, false, rel.matches('/').count() as u32)
            })
            .collect();
        (rows, root)
    }

    fn names(rows: &[AlignedRow]) -> Vec<(Option<String>, Option<String>)> {
        rows.iter()
            .map(|(l, r)| {
                let name = |d: &Option<RowData>| {
                    d.as_ref().and_then(|d| d.abs_path.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                };
                (name(l), name(r))
            })
            .collect()
    }

    #[test]
    fn identical_trees_produce_all_paired_rows() {
        let (lr, lroot) = flat("/l", &[("a.txt", false, false), ("b.txt", false, false)]);
        let (rr, rroot) = flat("/r", &[("a.txt", false, false), ("b.txt", false, false)]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert_eq!(rows.len(), 2);
        for (l, r) in &rows {
            assert!(l.is_some() && r.is_some(), "all rows should be paired");
        }
    }

    #[test]
    fn left_only_entry_has_none_on_right() {
        let (lr, lroot) = flat("/l", &[("only_left.txt", false, false)]);
        let (rr, rroot) = flat("/r", &[]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert_eq!(rows.len(), 1);
        let (l, r) = &rows[0];
        assert!(l.is_some(), "left-only entry should have left data");
        assert!(r.is_none(), "left-only entry should have None on right");
    }

    #[test]
    fn right_only_entry_has_none_on_left() {
        let (lr, lroot) = flat("/l", &[]);
        let (rr, rroot) = flat("/r", &[("only_right.txt", false, false)]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert_eq!(rows.len(), 1);
        let (l, r) = &rows[0];
        assert!(l.is_none());
        assert!(r.is_some());
    }

    #[test]
    fn directories_sort_before_files() {
        let (lr, lroot) = flat("/l", &[
            ("aaa.txt", false, false),
            ("src",     true,  false),
        ]);
        let (rr, rroot) = flat("/r", &[
            ("aaa.txt", false, false),
            ("src",     true,  false),
        ]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert_eq!(rows.len(), 2);
        let ns = names(&rows);
        assert_eq!(ns[0].0.as_deref(), Some("src"),    "dir should come first");
        assert_eq!(ns[1].0.as_deref(), Some("aaa.txt"), "file should come second");
    }

    #[test]
    fn entries_within_same_type_are_alphabetical() {
        let (lr, lroot) = flat("/l", &[
            ("zebra.txt", false, false),
            ("alpha.txt", false, false),
            ("mango.txt", false, false),
        ]);
        let (rr, rroot) = flat("/r", &[
            ("zebra.txt", false, false),
            ("alpha.txt", false, false),
            ("mango.txt", false, false),
        ]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        let ns = names(&rows);
        let left_names: Vec<_> = ns.iter().map(|(l, _)| l.clone().unwrap()).collect();
        assert_eq!(left_names, ["alpha.txt", "mango.txt", "zebra.txt"]);
    }

    #[test]
    fn expanded_directory_recurses_into_children() {
        let (lr, lroot) = flat("/l", &[
            ("src",         true,  true),
            ("src/main.rs", false, false),
        ]);
        let (rr, rroot) = flat("/r", &[
            ("src",         true,  false),
            ("src/lib.rs",  false, false),
        ]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        // Row 0: src/ (paired on both sides).
        // Rows 1+: children — main.rs (left-only) and lib.rs (right-only).
        assert_eq!(rows.len(), 3, "expected src + 2 children");
        let ns = names(&rows);
        assert_eq!(ns[0], (Some("src".into()), Some("src".into())));
        // Collect child names regardless of position.
        let child_names: Vec<_> = ns[1..].iter()
            .map(|(l, r)| l.as_deref().or(r.as_deref()).unwrap().to_string())
            .collect();
        assert!(child_names.contains(&"lib.rs".to_string()),  "lib.rs missing");
        assert!(child_names.contains(&"main.rs".to_string()), "main.rs missing");
    }

    #[test]
    fn mixed_same_name_and_one_sided_entries() {
        let (lr, lroot) = flat("/l", &[
            ("common.rs",    false, false),
            ("left_only.rs", false, false),
        ]);
        let (rr, rroot) = flat("/r", &[
            ("common.rs",     false, false),
            ("right_only.rs", false, false),
        ]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert_eq!(rows.len(), 3);
        // Every name accounted for.
        let ns = names(&rows);
        let all_names: Vec<_> = ns.iter()
            .map(|(l, r)| l.as_deref().or(r.as_deref()).unwrap())
            .collect();
        assert!(all_names.contains(&"common.rs"));
        assert!(all_names.contains(&"left_only.rs"));
        assert!(all_names.contains(&"right_only.rs"));
        // common.rs is paired.
        let common = ns.iter().find(|(l, _)| l.as_deref() == Some("common.rs")).unwrap();
        assert!(common.0.is_some() && common.1.is_some());
    }

    #[test]
    fn rel_path_is_relative_to_root() {
        let (lr, lroot) = flat("/project/l", &[("README.md", false, false)]);
        let (rr, rroot) = flat("/project/r", &[("README.md", false, false)]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        let (l, r) = &rows[0];
        assert_eq!(l.as_ref().unwrap().rel_path, PathBuf::from("README.md"));
        assert_eq!(r.as_ref().unwrap().rel_path, PathBuf::from("README.md"));
    }

    #[test]
    fn empty_both_sides_produces_empty_output() {
        let (lr, lroot) = flat("/l", &[]);
        let (rr, rroot) = flat("/r", &[]);
        let rows = compute_aligned_rows(&lr, &rr, &lroot, &rroot);
        assert!(rows.is_empty());
    }
}
