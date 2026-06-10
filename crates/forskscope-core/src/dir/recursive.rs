//! Recursive directory comparison (RFC-037).
//!
//! Two entry points are provided:
//!
//! - `recursive_diff` / `list_recursive_for_display` — the original
//!   blocking API, preserved for backwards compatibility; they internally
//!   call the cancellable variants with a never-cancelled token.
//! - `recursive_diff_with_cancel` / `list_recursive_for_display_with_cancel`
//!   — accept a [`CancellationToken`] and return early (with partial results
//!   marked `RecStatus::Computing`) when cancelled.
//!
//! Symlinks are now explicitly reported as `RecStatus::Symlink` rather than
//! silently skipped. The caller decides how to present them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::digest::file_digest_equal;
use crate::cancel::CancellationToken;
use crate::error::{CoreError, IoOperation, Result};

// ── Public types ──────────────────────────────────────────────────────────────

/// Status of one entry in the recursive comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStatus {
    Equal,
    Changed,
    LeftOnly,
    RightOnly,
    /// Exists on both sides; digest comparison not yet complete.
    /// Used by the incremental UI path; never returned by
    /// `recursive_diff` or `recursive_diff_with_cancel`.
    Computing,
    /// One or both sides of this path is a symlink.
    /// ForskScope does not follow cross-root symlinks to avoid cycles;
    /// the entry is reported and left to the caller to act on.
    Symlink,
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

// ── Stable public API (non-cancellable) ───────────────────────────────────────

/// Recursively compare two directory trees.
///
/// Returns all files found in either tree, sorted by relative path.
/// I/O errors on individual files are skipped (the entry receives
/// `Changed` status) so a permission error on one file does not abort
/// the whole scan. Symlinks are reported as `RecStatus::Symlink`.
pub fn recursive_diff(left_root: &Path, right_root: &Path) -> Vec<RecEntry> {
    recursive_diff_with_cancel(left_root, right_root, &CancellationToken::new())
}

/// Fast first-pass listing without digest comparisons.
///
/// Common files receive `RecStatus::Computing`; the caller should then
/// run per-file digests to upgrade each entry to `Equal` or `Changed`.
/// This enables the UI to show partial results immediately.
pub fn list_recursive_for_display(left_root: &Path, right_root: &Path) -> Vec<RecEntry> {
    list_recursive_for_display_with_cancel(left_root, right_root, &CancellationToken::new())
}

// ── Cancellable variants (RFC-037 §"Cancellation") ───────────────────────────

/// Like [`recursive_diff`] but stops early when `token` is cancelled.
///
/// Entries that were not yet compared when cancellation is observed are
/// left at whatever status they reached (typically `LeftOnly` or
/// `Computing`). The caller can distinguish a cancelled result from a
/// completed one by checking `token.is_cancelled()` afterwards.
pub fn recursive_diff_with_cancel(
    left_root: &Path,
    right_root: &Path,
    token: &CancellationToken,
) -> Vec<RecEntry> {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let _ = walk(left_root, left_root, &mut map, token, false, |rel, meta| RecEntry {
        rel_path:   rel.clone(),
        status:     RecStatus::LeftOnly,
        left_size:  Some(meta.len()),
        right_size: None,
    });
    if !token.is_cancelled() {
        let _ = walk_and_merge(right_root, right_root, &mut map, left_root, token, false);
    }
    map.into_values().collect()
}

/// Like [`list_recursive_for_display`] but stops early when `token` is
/// cancelled.
pub fn list_recursive_for_display_with_cancel(
    left_root: &Path,
    right_root: &Path,
    token: &CancellationToken,
) -> Vec<RecEntry> {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let _ = walk(left_root, left_root, &mut map, token, false, |rel, meta| RecEntry {
        rel_path:   rel.clone(),
        status:     RecStatus::LeftOnly,
        left_size:  Some(meta.len()),
        right_size: None,
    });
    if !token.is_cancelled() {
        let _ = walk_and_merge_fast(right_root, right_root, &mut map, token);
    }
    map.into_values().collect()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Walk a directory tree, inserting entries via `make`. Symlinks are
/// inserted with `RecStatus::Symlink`. Returns `Err` only on unrecoverable
/// directory-open failures; per-entry I/O errors are silently skipped.
fn walk(
    root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    token: &CancellationToken,
    _fast: bool,
    make: impl Fn(&PathBuf, &fs::Metadata) -> RecEntry + Copy,
) -> Result<()> {
    if token.is_cancelled() {
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        let path = entry.path();
        // Use symlink_metadata so we detect symlinks without following them.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();

        if meta.is_symlink() {
            // Explicit: report the symlink rather than silently skip or follow.
            map.insert(rel.clone(), RecEntry {
                rel_path:   rel,
                status:     RecStatus::Symlink,
                left_size:  None,
                right_size: None,
            });
        } else if meta.is_dir() {
            let _ = walk(root, &path, map, token, _fast, make);
        } else if meta.is_file() {
            map.insert(rel.clone(), make(&rel, &meta));
        }
        // Other entry kinds (devices, etc.) silently skipped.
    }
    Ok(())
}

fn walk_and_merge(
    right_root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    left_root: &Path,
    token: &CancellationToken,
    _fast: bool,
) -> Result<()> {
    if token.is_cancelled() {
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        let path = entry.path();
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let rel = path.strip_prefix(right_root).unwrap_or(&path).to_path_buf();

        if meta.is_symlink() {
            map.entry(rel.clone()).or_insert(RecEntry {
                rel_path:   rel,
                status:     RecStatus::Symlink,
                left_size:  None,
                right_size: None,
            });
        } else if meta.is_dir() {
            let _ = walk_and_merge(right_root, &path, map, left_root, token, _fast);
        } else if meta.is_file() {
            let right_size = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                let left_path  = left_root.join(&rel);
                let right_path = path;
                let equal = file_digest_equal(&left_path, &right_path).unwrap_or(false);
                existing.status     = if equal { RecStatus::Equal } else { RecStatus::Changed };
                existing.right_size = Some(right_size);
            } else {
                map.insert(rel.clone(), RecEntry {
                    rel_path: rel, status: RecStatus::RightOnly,
                    left_size: None, right_size: Some(right_size),
                });
            }
        }
    }
    Ok(())
}

fn walk_and_merge_fast(
    right_root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    token: &CancellationToken,
) -> Result<()> {
    if token.is_cancelled() {
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        let path = entry.path();
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let rel = path.strip_prefix(right_root).unwrap_or(&path).to_path_buf();

        if meta.is_symlink() {
            map.entry(rel.clone()).or_insert(RecEntry {
                rel_path:   rel,
                status:     RecStatus::Symlink,
                left_size:  None,
                right_size: None,
            });
        } else if meta.is_dir() {
            let _ = walk_and_merge_fast(right_root, &path, map, token);
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
