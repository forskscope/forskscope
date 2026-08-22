//! Build a directory-scope [`PatchDocument`] from a recursive comparison
//! (RFC-039, RFC-037).
//!
//! Given two directory roots, this assembles one patch covering every
//! differing file: `Modify` for files that exist on both sides, `Add` /
//! `Delete` for one-sided files, and optional `BinaryNotice` entries for
//! files that cannot be decoded as text. The walk reuses `recursive_diff`
//! so the file set and ordering match the Directory Report.

use std::path::Path;

use crate::diff::{DiffOptions, compute_diff};
use crate::dir::{RecStatus, recursive_diff};
use crate::document::{LoadOptions, load_path};
use crate::error::{CoreError, IoOperation, Result};
use crate::file_kind::FileKind;

use super::build::{PatchOptions, Side, hunks_from_diff, whole_side_lines};
use super::model::{PatchDocument, PatchFileChange, PatchFormat, PatchSummary};

/// Build a patch transforming the left tree into the right tree.
///
/// `diff_options` controls the text diff for modified files; `patch_options`
/// controls context size and whether creation/deletion and binary entries
/// are included.
///
/// F79: a patch document claims to be a complete, trustworthy record of the
/// difference between two trees, and (unlike Deep Compare) has no reader
/// surface to carry a per-entry caveat through to whatever consumes the
/// patch. Rather than silently omitting content a reader cannot know is
/// missing - the defect this handoff exists to close - an unreadable root
/// or entry fails patch generation outright, through the `Result` this
/// function already returns for every other failure mode here.
pub fn patch_from_directories(
    left_root: &Path,
    right_root: &Path,
    diff_options: DiffOptions,
    patch_options: PatchOptions,
) -> Result<PatchDocument> {
    let scan = recursive_diff(left_root, right_root);
    if scan.left_root_unreadable {
        return Err(CoreError::Io {
            path: Some(left_root.to_path_buf()),
            operation: IoOperation::ListDir,
            message: "left root could not be read; the patch would be incomplete".to_string(),
        });
    }
    if scan.right_root_unreadable {
        return Err(CoreError::Io {
            path: Some(right_root.to_path_buf()),
            operation: IoOperation::ListDir,
            message: "right root could not be read; the patch would be incomplete".to_string(),
        });
    }

    let mut files = Vec::new();

    for entry in scan.entries {
        let rel = entry.rel_path;
        match entry.status {
            RecStatus::Equal | RecStatus::Computing => {}
            RecStatus::Unreadable => {
                return Err(CoreError::Io {
                    path: Some(rel),
                    operation: IoOperation::Read,
                    message:
                        "entry could not be read during the directory scan; the patch would be incomplete"
                            .to_string(),
                });
            }
            RecStatus::Symlink => {
                // Symlinks are not text-diffable; emit a notice if requested.
                if patch_options.include_binary_notices {
                    files.push(PatchFileChange::BinaryNotice {
                        path: rel.to_path_buf(),
                    });
                }
            }
            RecStatus::Changed => {
                let left = left_root.join(&rel);
                let right = right_root.join(&rel);
                if let Some(change) =
                    modify_entry(&rel, &left, &right, diff_options, patch_options)?
                {
                    files.push(change);
                }
            }
            RecStatus::RightOnly => {
                if patch_options.include_creation_deletion {
                    let right = right_root.join(&rel);
                    if let Some(change) = add_entry(&rel, &right, patch_options)? {
                        files.push(change);
                    }
                }
            }
            RecStatus::LeftOnly => {
                if patch_options.include_creation_deletion {
                    let left = left_root.join(&rel);
                    if let Some(change) = delete_entry(&rel, &left, patch_options)? {
                        files.push(change);
                    }
                }
            }
        }
    }

    let mut doc = PatchDocument {
        format: PatchFormat::Unified,
        files,
        summary: PatchSummary::default(),
    };
    doc.recompute_summary();
    Ok(doc)
}

fn modify_entry(
    rel: &Path,
    left: &Path,
    right: &Path,
    diff_options: DiffOptions,
    patch_options: PatchOptions,
) -> Result<Option<PatchFileChange>> {
    let left_doc = load_path(left, LoadOptions::default())?;
    let right_doc = load_path(right, LoadOptions::default())?;
    if !is_textual(&left_doc.kind) || !is_textual(&right_doc.kind) {
        return Ok(patch_options
            .include_binary_notices
            .then(|| PatchFileChange::BinaryNotice {
                path: rel.to_path_buf(),
            }));
    }
    let diff = compute_diff(left_doc.diff_text(), right_doc.diff_text(), diff_options);
    let hunks = hunks_from_diff(&diff, patch_options);
    if hunks.is_empty() {
        return Ok(None);
    }
    Ok(Some(PatchFileChange::Modify {
        path: rel.to_path_buf(),
        hunks,
    }))
}

fn add_entry(
    rel: &Path,
    right: &Path,
    patch_options: PatchOptions,
) -> Result<Option<PatchFileChange>> {
    let right_doc = load_path(right, LoadOptions::default())?;
    if !is_textual(&right_doc.kind) {
        return Ok(patch_options
            .include_binary_notices
            .then(|| PatchFileChange::BinaryNotice {
                path: rel.to_path_buf(),
            }));
    }
    // Diff against an empty left side to obtain a clean insert-only stream.
    let diff = compute_diff("", right_doc.diff_text(), DiffOptions::default());
    let lines = whole_side_lines(&diff, Side::Right);
    if lines.is_empty() {
        return Ok(None);
    }
    Ok(Some(PatchFileChange::Add {
        path: rel.to_path_buf(),
        lines,
    }))
}

fn delete_entry(
    rel: &Path,
    left: &Path,
    patch_options: PatchOptions,
) -> Result<Option<PatchFileChange>> {
    let left_doc = load_path(left, LoadOptions::default())?;
    if !is_textual(&left_doc.kind) {
        return Ok(patch_options
            .include_binary_notices
            .then(|| PatchFileChange::BinaryNotice {
                path: rel.to_path_buf(),
            }));
    }
    let diff = compute_diff(left_doc.diff_text(), "", DiffOptions::default());
    let lines = whole_side_lines(&diff, Side::Left);
    if lines.is_empty() {
        return Ok(None);
    }
    Ok(Some(PatchFileChange::Delete {
        path: rel.to_path_buf(),
        lines,
    }))
}

fn is_textual(kind: &FileKind) -> bool {
    matches!(kind, FileKind::Text)
}
