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

use super::digest::{DigestOutcome, file_digest_equal_with_cancel};
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
    /// Used by the incremental UI path, and never returned by
    /// `recursive_diff` (a fresh, uncancellable token). `recursive_diff_with_cancel`
    /// can also return it (F77) - a per-file comparison cancelled mid-flight
    /// leaves that entry here rather than asserting `Equal`/`Changed`
    /// for a comparison that was interrupted before it established either.
    Computing,
    /// One or both sides of this path is a symlink.
    /// ForskScope does not follow cross-root symlinks to avoid cycles;
    /// the entry is reported and left to the caller to act on.
    Symlink,
    /// This path could not be read - a `metadata()` failure on the entry
    /// itself, or a directory that could not be opened (F79). Not a
    /// verdict: nothing was measured, so this must never be treated as
    /// `Changed`, included in a copy manifest, or counted as "different".
    /// No payload (which side failed, and why) is deliberate - `RecStatus`
    /// derives `Copy` and is passed by value at every call site; a
    /// `String` payload would break that for a detail this handoff does
    /// not need. See handoff 006 §7a.
    Unreadable,
}

/// One entry in the recursive comparison report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecEntry {
    /// Path relative to both roots.
    pub rel_path: PathBuf,
    pub status: RecStatus,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
}

/// Result of a recursive comparison scan (F79).
///
/// `left_root_unreadable`/`right_root_unreadable` distinguish "the root
/// itself could not be opened" from both "the tree is empty" and "every
/// entry differs by side" - a root that fails to open must never silently
/// read as every file on the other side being one-sided (e.g. an unopenable
/// right root previously made every left-side file read as a confident
/// `LeftOnly`, indistinguishable from the right tree genuinely being empty).
///
/// There is deliberately no synthetic entry at the empty relative path for
/// this case: `explorer.rs` filters empty rel-paths, and Deep Compare would
/// render one as a nameless row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecursiveScan {
    pub entries: Vec<RecEntry>,
    pub left_root_unreadable: bool,
    pub right_root_unreadable: bool,
}

// ── Stable public API (non-cancellable) ───────────────────────────────────────

/// Recursively compare two directory trees.
///
/// Returns all files found in either tree, sorted by relative path.
/// A per-entry read failure (or a directory that cannot be opened) is
/// reported as `RecStatus::Unreadable` at that path rather than dropped
/// (F79); a root itself that cannot be opened is reported via
/// [`RecursiveScan::left_root_unreadable`] / `right_root_unreadable`.
/// Symlinks are reported as `RecStatus::Symlink`.
pub fn recursive_diff(left_root: &Path, right_root: &Path) -> RecursiveScan {
    recursive_diff_with_cancel(left_root, right_root, &CancellationToken::new())
}

/// Fast first-pass listing without digest comparisons.
///
/// Common files receive `RecStatus::Computing`; the caller should then
/// run per-file digests to upgrade each entry to `Equal` or `Changed`.
/// This enables the UI to show partial results immediately.
pub fn list_recursive_for_display(left_root: &Path, right_root: &Path) -> RecursiveScan {
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
) -> RecursiveScan {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let left_root_unreadable = walk(left_root, left_root, &mut map, token, false, |rel, meta| {
        RecEntry {
            rel_path: rel.clone(),
            status: RecStatus::LeftOnly,
            left_size: Some(meta.len()),
            right_size: None,
        }
    })
    .is_err();
    let right_root_unreadable = if token.is_cancelled() {
        false
    } else {
        walk_and_merge(right_root, right_root, &mut map, left_root, token, false).is_err()
    };
    RecursiveScan {
        entries: map.into_values().collect(),
        left_root_unreadable,
        right_root_unreadable,
    }
}

/// Like [`list_recursive_for_display`] but stops early when `token` is
/// cancelled.
pub fn list_recursive_for_display_with_cancel(
    left_root: &Path,
    right_root: &Path,
    token: &CancellationToken,
) -> RecursiveScan {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let left_root_unreadable = walk(left_root, left_root, &mut map, token, false, |rel, meta| {
        RecEntry {
            rel_path: rel.clone(),
            status: RecStatus::LeftOnly,
            left_size: Some(meta.len()),
            right_size: None,
        }
    })
    .is_err();
    let right_root_unreadable = if token.is_cancelled() {
        false
    } else {
        walk_and_merge_fast(right_root, right_root, &mut map, token).is_err()
    };
    RecursiveScan {
        entries: map.into_values().collect(),
        left_root_unreadable,
        right_root_unreadable,
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// F79: marks `rel` `Unreadable` in `map`, overwriting whatever verdict (if
/// any) was already recorded there. A metadata or directory-open failure
/// means nothing was actually established for this path - any prior entry
/// (e.g. `LeftOnly`, inserted before the corresponding right side was known
/// to be unreadable) asserted more than was measured.
fn mark_unreadable(map: &mut BTreeMap<PathBuf, RecEntry>, rel: PathBuf) {
    let entry = map.entry(rel.clone()).or_insert(RecEntry {
        rel_path: rel,
        status: RecStatus::Unreadable,
        left_size: None,
        right_size: None,
    });
    entry.status = RecStatus::Unreadable;
}

/// Walk a directory tree, inserting entries via `make`. Symlinks are
/// inserted with `RecStatus::Symlink`. Returns `Err` only on unrecoverable
/// directory-open failures (the caller reports the directory itself as
/// `Unreadable`, F79); a per-entry `metadata()` failure produces an
/// `Unreadable` entry at that path rather than being skipped.
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
        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        // Use symlink_metadata so we detect symlinks without following them.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                mark_unreadable(map, rel);
                continue;
            }
        };

        if meta.is_symlink() {
            // Explicit: report the symlink rather than silently skip or follow.
            map.insert(
                rel.clone(),
                RecEntry {
                    rel_path: rel,
                    status: RecStatus::Symlink,
                    left_size: None,
                    right_size: None,
                },
            );
        } else if meta.is_dir() {
            // A subdirectory that cannot be opened takes its subtree out of
            // the result - unavoidable, nothing read it - but must itself
            // be visible rather than silently absent (F79).
            if walk(root, &path, map, token, _fast, make).is_err() {
                mark_unreadable(map, rel);
            }
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
        let rel = path.strip_prefix(right_root).unwrap_or(&path).to_path_buf();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                mark_unreadable(map, rel);
                continue;
            }
        };

        if meta.is_symlink() {
            map.entry(rel.clone()).or_insert(RecEntry {
                rel_path: rel,
                status: RecStatus::Symlink,
                left_size: None,
                right_size: None,
            });
        } else if meta.is_dir() {
            if walk_and_merge(right_root, &path, map, left_root, token, _fast).is_err() {
                mark_unreadable(map, rel);
            }
        } else if meta.is_file() {
            let right_size = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                if existing.status == RecStatus::Unreadable {
                    // F79: already known unreadable (e.g. the left side's
                    // metadata failed) - a digest comparison against it
                    // would itself fail to open the left path and collapse
                    // to `Changed` via the pre-existing I/O-error fallback
                    // below, silently reclassifying an `Unreadable` entry
                    // as a verdict. Leave it as `Unreadable`.
                    continue;
                }
                let left_path = left_root.join(&rel);
                let right_path = path;
                // F77: a per-file comparison is now itself cancellable
                // (defect (b) reaching this, the one caller that runs a
                // full blocking comparison rather than the fast/display
                // listing). `Cancelled` leaves the entry at `Computing`
                // rather than asserting a verdict nothing established -
                // the same reasoning as `DigestOutcome`'s own doc comment.
                // An I/O error still collapses to `Changed`, unchanged
                // from before this handoff (a different, already-tracked
                // finding - F76 - not this one; see the review request).
                match file_digest_equal_with_cancel(&left_path, &right_path, token) {
                    Ok(DigestOutcome::Equal) => existing.status = RecStatus::Equal,
                    Ok(DigestOutcome::Different) => existing.status = RecStatus::Changed,
                    Ok(DigestOutcome::Cancelled) => existing.status = RecStatus::Computing,
                    Err(_) => existing.status = RecStatus::Changed,
                }
                existing.right_size = Some(right_size);
            } else {
                map.insert(
                    rel.clone(),
                    RecEntry {
                        rel_path: rel,
                        status: RecStatus::RightOnly,
                        left_size: None,
                        right_size: Some(right_size),
                    },
                );
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
        let rel = path.strip_prefix(right_root).unwrap_or(&path).to_path_buf();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                mark_unreadable(map, rel);
                continue;
            }
        };

        if meta.is_symlink() {
            map.entry(rel.clone()).or_insert(RecEntry {
                rel_path: rel,
                status: RecStatus::Symlink,
                left_size: None,
                right_size: None,
            });
        } else if meta.is_dir() {
            if walk_and_merge_fast(right_root, &path, map, token).is_err() {
                mark_unreadable(map, rel);
            }
        } else if meta.is_file() {
            let rs = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                if existing.status == RecStatus::Unreadable {
                    // F79: see the matching guard in `walk_and_merge` -
                    // an already-`Unreadable` left entry must not be
                    // silently promoted to `Computing` (implying a digest
                    // is pending) when the right side happens to read fine.
                    continue;
                }
                existing.status = RecStatus::Computing;
                existing.right_size = Some(rs);
            } else {
                map.insert(
                    rel.clone(),
                    RecEntry {
                        rel_path: rel,
                        status: RecStatus::RightOnly,
                        left_size: None,
                        right_size: Some(rs),
                    },
                );
            }
        }
    }
    Ok(())
}
