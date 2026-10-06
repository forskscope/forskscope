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
//!
//! ## Ignore rules (F111, F149, RFC-056)
//!
//! `recursive_diff_with_rules` / `list_recursive_for_display_with_rules` take an
//! [`IgnoreRules`] and apply it **during the walk**, on both sides by the same
//! rules: an ignored directory is not descended into and does not appear, and an
//! ignored file is not reported, so an ignored entry present on one side only
//! never becomes a one-sided difference. The decision is made from the entry's
//! name and file type *before* its metadata is read on the common platforms, so
//! an ignored entry that could not be read is neither failed on nor flagged
//! `Unreadable`. The `*_with_cancel` functions are the same walks with empty
//! rules. `IgnoreRules::hide_dotfiles` (F149) is the same mechanism: a hidden
//! entry is excluded from the walk exactly as a named pattern is — see
//! `ignore.rs`'s module doc for why this flag reaches the walk but not the
//! Explorer tree's own scan.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::digest::{DigestOutcome, file_digest_equal_with_cancel};
use crate::cancel::CancellationToken;
use crate::error::{CoreError, IoOperation, Result};
use crate::ignore::IgnoreRules;

// ── Public types ──────────────────────────────────────────────────────────────

/// Status of one entry in the recursive comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecStatus {
    Equal,
    Changed,
    LeftOnly,
    RightOnly,
    /// A directory that exists on the left side only, and is empty - the
    /// only shape `LeftOnly`/`RightOnly` cannot report, since they are
    /// emitted per *file*, never for a directory itself (F135). A
    /// populated one-sided directory needs no variant of its own: every
    /// file beneath it is already reported one-sided at its own path, and
    /// that is what makes the directory's existence visible. Emitted only
    /// where it carries information that would otherwise be lost - a
    /// directory present on *both* sides, however different their
    /// contents, is never this, even when one side's copy happens to be
    /// empty. No payload, for the same reason `Unreadable` has none: see
    /// its own doc comment below.
    LeftOnlyDir,
    /// The same, for the right side.
    RightOnlyDir,
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
    recursive_diff_with_rules(left_root, right_root, token, &IgnoreRules::default())
}

/// [`recursive_diff_with_cancel`], skipping what `rules` ignore (see the module
/// doc). Empty rules give exactly the unfiltered result.
pub fn recursive_diff_with_rules(
    left_root: &Path,
    right_root: &Path,
    token: &CancellationToken,
    rules: &IgnoreRules,
) -> RecursiveScan {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let left_root_unreadable = walk(
        left_root,
        left_root,
        &mut map,
        right_root,
        token,
        rules,
        |rel, meta| RecEntry {
            rel_path: rel.clone(),
            status: RecStatus::LeftOnly,
            left_size: Some(meta.len()),
            right_size: None,
        },
    )
    .is_err();
    let right_root_unreadable = if token.is_cancelled() {
        false
    } else {
        walk_and_merge(right_root, right_root, &mut map, left_root, token, rules).is_err()
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
    list_recursive_for_display_with_rules(left_root, right_root, token, &IgnoreRules::default())
}

/// [`list_recursive_for_display_with_cancel`], skipping what `rules` ignore
/// (see the module doc). Empty rules give exactly the unfiltered result.
pub fn list_recursive_for_display_with_rules(
    left_root: &Path,
    right_root: &Path,
    token: &CancellationToken,
    rules: &IgnoreRules,
) -> RecursiveScan {
    let mut map: BTreeMap<PathBuf, RecEntry> = BTreeMap::new();
    let left_root_unreadable = walk(
        left_root,
        left_root,
        &mut map,
        right_root,
        token,
        rules,
        |rel, meta| RecEntry {
            rel_path: rel.clone(),
            status: RecStatus::LeftOnly,
            left_size: Some(meta.len()),
            right_size: None,
        },
    )
    .is_err();
    let right_root_unreadable = if token.is_cancelled() {
        false
    } else {
        walk_and_merge_fast(right_root, right_root, &mut map, left_root, token, rules).is_err()
    };
    RecursiveScan {
        entries: map.into_values().collect(),
        left_root_unreadable,
        right_root_unreadable,
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// A symlink on the right-hand side of `rel`. `RecStatus::Symlink` means "one or
/// both sides of this path is a symlink", so it must **replace** an entry the left
/// walk already recorded there — a regular file called `link` on the left is not
/// "left only" when the right has a symlink called `link` — except an
/// `Unreadable` one, which says less still. (Found by RFC-080 tier 1, whose
/// verdict would otherwise call that pair `Different`.)
fn mark_symlink(map: &mut BTreeMap<PathBuf, RecEntry>, rel: PathBuf) {
    let entry = map.entry(rel.clone()).or_insert(RecEntry {
        rel_path: rel,
        status: RecStatus::Symlink,
        left_size: None,
        right_size: None,
    });
    if entry.status != RecStatus::Unreadable {
        entry.status = RecStatus::Symlink;
    }
}

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

/// F111/F149: whether `rules` exclude this directory entry. Decided from the
/// name and the entry's own file type, which needs no `stat` on the common
/// platforms, so an ignored entry is never read and an unreadable one is never
/// flagged. A symlink is judged by what it points at (a dangling one is not
/// ignored, so it is still reported). `hide_dotfiles` is checked first,
/// independent of file type — a hidden entry is skipped whether it is a file,
/// a directory, or a symlink.
fn is_ignored(rules: &IgnoreRules, entry: &fs::DirEntry) -> bool {
    if rules.is_empty() {
        return false;
    }
    let raw_name = entry.file_name();
    // Checked on raw bytes, before the `to_str()` guard below: the tree crate's
    // own hidden-entry rule (`is_dotfile`) reasons on raw bytes too, so a
    // non-UTF-8 dotfile must be caught here or the walk and the tree disagree
    // on it (review 133 §1). The name-based rules just below stay string-only
    // and conservative - a non-UTF-8 name simply can't match an extension or
    // directory pattern - because they have no byte-level counterpart to stay
    // faithful to.
    if rules.hide_dotfiles && is_hidden(&raw_name, entry) {
        return true;
    }
    let Some(name) = raw_name.to_str() else {
        return false;
    };
    match entry.file_type() {
        Ok(ft) if ft.is_dir() => rules.is_dir_ignored(name),
        Ok(ft) if ft.is_file() => rules.is_file_ignored(name),
        Ok(ft) if ft.is_symlink() => match fs::metadata(entry.path()) {
            Ok(m) if m.is_dir() => rules.is_dir_ignored(name),
            Ok(_) => rules.is_file_ignored(name),
            Err(_) => false,
        },
        _ => false,
    }
}

/// Mirrors `dioxus_swdir_tree_core`'s own hidden-entry rule exactly (F149), so
/// the walk and the Explorer tree agree on what "hidden" means: Unix, the name
/// starts with `.`; Windows, that or the filesystem's hidden attribute bit.
/// Duplicated rather than depended on — that crate does not expose its rule as
/// a reusable function, only as part of its own scan — and kept byte-for-byte
/// equivalent to it on purpose, not reinterpreted. Takes the raw `OsStr`, not a
/// `&str`, because the crate's own rule (`is_dotfile`) tests the first raw
/// byte, not a decoded character — a non-UTF-8 dotfile is still a dotfile to
/// it, and must be to this function too (review 133 §1).
fn is_hidden(name: &std::ffi::OsStr, entry: &fs::DirEntry) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.') || is_hidden_attribute(entry)
}

#[cfg(windows)]
fn is_hidden_attribute(entry: &fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    entry
        .metadata()
        .map(|m| m.file_attributes() & 0x2 != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden_attribute(_entry: &fs::DirEntry) -> bool {
    false
}

/// Walk a directory tree, inserting entries via `make`. Symlinks are
/// inserted with `RecStatus::Symlink`. Returns `Err` only on unrecoverable
/// directory-open failures (the caller reports the directory itself as
/// `Unreadable`, F79); a per-entry `metadata()` failure produces an
/// `Unreadable` entry at that path rather than being skipped. On success,
/// returns whether `rules` excluded anything anywhere in this subtree
/// (F135; see the call site's use of it).
///
/// `other_root` is the *other* side's root (F135): a subdirectory found
/// empty here is reported `LeftOnlyDir` only when `other_root` has no
/// directory at the same relative path *and* nothing inside this one was
/// excluded by `rules` - a directory that looks empty only because its one
/// file was ignored is not a one-sided difference (module doc: "an ignored
/// entry present on one side only never becomes a one-sided difference");
/// it must read exactly as it would if that file did not exist to the walk
/// at all, which for a *file* is already true by construction (ignored,
/// never inserted) and for a *directory* needs this check to also be true.
/// A directory present on both sides is never this, however different
/// their contents, and the right-hand walk (`walk_and_merge`/
/// `walk_and_merge_fast`) makes the symmetric check against this side so a
/// directory empty on *both* sides stays invisible, exactly as today.
fn walk(
    root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    other_root: &Path,
    token: &CancellationToken,
    rules: &IgnoreRules,
    make: impl Fn(&PathBuf, &fs::Metadata) -> RecEntry + Copy,
) -> Result<bool> {
    if token.is_cancelled() {
        return Ok(false);
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    let mut ignored_anything = false;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        if is_ignored(rules, &entry) {
            ignored_anything = true;
            continue;
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
            let before = map.len();
            match walk(root, &path, map, other_root, token, rules, make) {
                Err(_) => mark_unreadable(map, rel),
                Ok(child_ignored_anything) => {
                    ignored_anything |= child_ignored_anything;
                    if map.len() == before
                        && !child_ignored_anything
                        && !other_root.join(&rel).is_dir()
                    {
                        // F135: nothing was added for this subtree, nothing
                        // in it was excluded by `rules` either, and the
                        // other side has no directory here at all - a
                        // genuinely one-sided empty directory, the one
                        // shape `LeftOnly`/`RightOnly` (per-file) cannot
                        // report.
                        map.insert(
                            rel.clone(),
                            RecEntry {
                                rel_path: rel,
                                status: RecStatus::LeftOnlyDir,
                                left_size: None,
                                right_size: None,
                            },
                        );
                    }
                }
            }
        } else if meta.is_file() {
            map.insert(rel.clone(), make(&rel, &meta));
        }
        // Other entry kinds (devices, etc.) silently skipped.
    }
    Ok(ignored_anything)
}

fn walk_and_merge(
    right_root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    left_root: &Path,
    token: &CancellationToken,
    rules: &IgnoreRules,
) -> Result<bool> {
    if token.is_cancelled() {
        return Ok(false);
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    let mut ignored_anything = false;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        if is_ignored(rules, &entry) {
            ignored_anything = true;
            continue;
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
            mark_symlink(map, rel);
        } else if meta.is_dir() {
            let before = map.len();
            match walk_and_merge(right_root, &path, map, left_root, token, rules) {
                Err(_) => mark_unreadable(map, rel),
                Ok(child_ignored_anything) => {
                    ignored_anything |= child_ignored_anything;
                    if map.len() == before
                        && !child_ignored_anything
                        && !left_root.join(&rel).is_dir()
                    {
                        // F135: symmetric to `walk`'s own check - empty
                        // here, nothing in it was excluded by `rules`, and
                        // the left side has no directory at this path.
                        map.insert(
                            rel.clone(),
                            RecEntry {
                                rel_path: rel,
                                status: RecStatus::RightOnlyDir,
                                left_size: None,
                                right_size: None,
                            },
                        );
                    }
                }
            }
        } else if meta.is_file() {
            let right_size = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                if existing.status == RecStatus::Symlink {
                    // A left-side symlink stays `Symlink` (see `mark_symlink`).
                    continue;
                }
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
    Ok(ignored_anything)
}

fn walk_and_merge_fast(
    right_root: &Path,
    dir: &Path,
    map: &mut BTreeMap<PathBuf, RecEntry>,
    left_root: &Path,
    token: &CancellationToken,
    rules: &IgnoreRules,
) -> Result<bool> {
    if token.is_cancelled() {
        return Ok(false);
    }
    let rd = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    let mut ignored_anything = false;
    for entry in rd.flatten() {
        if token.is_cancelled() {
            break;
        }
        if is_ignored(rules, &entry) {
            ignored_anything = true;
            continue;
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
            mark_symlink(map, rel);
        } else if meta.is_dir() {
            let before = map.len();
            match walk_and_merge_fast(right_root, &path, map, left_root, token, rules) {
                Err(_) => mark_unreadable(map, rel),
                Ok(child_ignored_anything) => {
                    ignored_anything |= child_ignored_anything;
                    if map.len() == before
                        && !child_ignored_anything
                        && !left_root.join(&rel).is_dir()
                    {
                        // F135: symmetric to `walk`'s own check.
                        map.insert(
                            rel.clone(),
                            RecEntry {
                                rel_path: rel,
                                status: RecStatus::RightOnlyDir,
                                left_size: None,
                                right_size: None,
                            },
                        );
                    }
                }
            }
        } else if meta.is_file() {
            let rs = meta.len();
            if let Some(existing) = map.get_mut(&rel) {
                if existing.status == RecStatus::Symlink {
                    continue;
                }
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
    Ok(ignored_anything)
}

#[cfg(test)]
mod measurement_tests {
    use super::*;

    /// Measurement, not a committed case (F137, handoff 069 §6): the directory walk on
    /// a one-entry tree against a large one, in both directions. Run with
    /// `FSK_DIR_LEFT=<a> FSK_DIR_RIGHT=<b> cargo test --release -p forskscope-core -- --ignored --nocapture`.
    #[test]
    #[ignore = "measurement over generated trees; set FSK_DIR_LEFT and FSK_DIR_RIGHT"]
    fn walk_measured_on_the_given_trees() {
        let left =
            std::path::PathBuf::from(std::env::var("FSK_DIR_LEFT").expect("set FSK_DIR_LEFT"));
        let right =
            std::path::PathBuf::from(std::env::var("FSK_DIR_RIGHT").expect("set FSK_DIR_RIGHT"));
        let started = std::time::Instant::now();
        let scan = recursive_diff(&left, &right);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "recursive_diff {} vs {}: {ms:.1} ms, entries={}",
            left.display(),
            right.display(),
            scan.entries.len()
        );
    }
}
