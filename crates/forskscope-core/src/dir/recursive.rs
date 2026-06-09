//! Recursive directory comparison (RFC-037).
//!
//! `recursive_diff` walks both trees and produces a flat, sorted list of
//! every file that differs (or is unique to one side). Equal files are
//! included with `RecStatus::Equal` so the caller can filter them.
//!
//! This is a blocking scan. The UI wraps it in `spawn_blocking`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::digest::file_digest_equal;
use crate::error::{CoreError, IoOperation, Result};

/// Status of one file in the recursive comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStatus {
    Equal,
    Changed,
    LeftOnly,
    RightOnly,
    /// Exists on both sides; digest comparison not yet complete.
    /// Used by the incremental UI; never returned by `recursive_diff`.
    Computing,
}

/// One entry in the recursive comparison report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecEntry {
    /// Path relative to both roots.
    pub rel_path:   PathBuf,
    pub status:     RecStatus,
    pub left_size:  Option<u64>,
    pub right_size: Option<u64>,
}

/// Recursively compare two directory trees.
///
/// Returns all files found in either tree, sorted by relative path.
/// I/O errors on individual files are skipped (the entry receives
/// `Changed` status) so a permission error on one file does not abort
/// the whole scan.
pub fn recursive_diff(left_root: &Path, right_root: &Path) -> Vec<RecEntry> {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    // Collect from the left tree.
    let _ = walk(left_root, left_root, &mut map, |rel, meta| RecEntry {
        rel_path:  rel.clone(),
        status:    RecStatus::LeftOnly,
        left_size: Some(meta.len()),
        right_size: None,
    });
    // Reconcile with the right tree.
    let _ = walk_and_merge(right_root, right_root, &mut map, left_root);
    map.into_values().collect()
}

fn walk(
    root: &Path, dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    make: impl Fn(&PathBuf, &fs::Metadata) -> RecEntry + Copy,
) -> Result<()> {
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let rel = entry.path().strip_prefix(root).unwrap_or(&entry.path()).to_path_buf();
        if meta.is_dir() {
            let _ = walk(root, &entry.path(), map, make);
        } else if meta.is_file() {
            map.insert(rel.clone(), make(&rel, &meta));
        }
    }
    Ok(())
}

fn walk_and_merge(
    right_root: &Path, dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    left_root: &Path,
) -> Result<()> {
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let rel = entry.path().strip_prefix(right_root).unwrap_or(&entry.path()).to_path_buf();
        if meta.is_dir() {
            let _ = walk_and_merge(right_root, &entry.path(), map, left_root);
        } else if meta.is_file() {
            let right_path = entry.path();
            let right_size = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                // File existed on the left side too.
                let left_path = left_root.join(&rel);
                let equal = file_digest_equal(&left_path, &right_path).unwrap_or(false);
                existing.status     = if equal { RecStatus::Equal } else { RecStatus::Changed };
                existing.right_size = Some(right_size);
            } else {
                // Only on the right side.
                map.insert(rel.clone(), RecEntry {
                    rel_path: rel, status: RecStatus::RightOnly,
                    left_size: None, right_size: Some(right_size),
                });
            }
        }
    }
    Ok(())
}

/// Fast first-pass listing without digest comparisons.
///
/// Common files receive `RecStatus::Computing`; the caller should then
/// run per-file digests to upgrade each entry to `Equal` or `Changed`.
/// This enables the UI to show partial results immediately rather than
/// waiting for a full blocking scan.
pub fn list_recursive_for_display(left_root: &Path, right_root: &Path) -> Vec<RecEntry> {
    let mut map: std::collections::BTreeMap<PathBuf, RecEntry> = Default::default();
    // Seed from the left tree.
    let _ = walk(left_root, left_root, &mut map, |rel, meta| RecEntry {
        rel_path: rel.clone(), status: RecStatus::LeftOnly,
        left_size: Some(meta.len()), right_size: None,
    });
    // Merge the right tree — common files become Computing.
    let _ = walk_and_merge_fast(right_root, right_root, &mut map);
    map.into_values().collect()
}

fn walk_and_merge_fast(
    right_root: &Path, dir: &Path,
    map: &mut std::collections::BTreeMap<PathBuf, RecEntry>,
) -> super::super::error::Result<()> {
    use std::fs;
    let rd = fs::read_dir(dir)
        .map_err(|e| super::super::error::CoreError::io(dir, super::super::error::IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let rel = entry.path().strip_prefix(right_root).unwrap_or(&entry.path()).to_path_buf();
        if meta.is_dir() {
            let _ = walk_and_merge_fast(right_root, &entry.path(), map);
        } else if meta.is_file() {
            let rs = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                existing.status     = RecStatus::Computing;
                existing.right_size = Some(rs);
            } else {
                map.insert(rel.clone(), RecEntry {
                    rel_path: rel, status: RecStatus::RightOnly,
                    left_size: None, right_size: Some(rs),
                });
            }
        }
    }
    Ok(())
}
