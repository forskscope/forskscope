//! Save and file safety (RFC-007, RFC-023).
//!
//! Saving is conservative and explicit. Before writing, the loaded
//! fingerprint is compared against the current on-disk fingerprint; a
//! mismatch is reported as [`CoreError::Conflict`] so the UI can offer
//! reload / overwrite / save-as rather than silently clobbering external
//! edits. Writes are atomic (temp file in the same directory, then rename)
//! and an optional backup is taken before the rename.

use std::fs;
use std::path::{Path, PathBuf};

use crate::document::{ExternalFileState, FileFingerprint, check_external_state};
use crate::encoding::encode_text;
use crate::error::{CoreError, IoOperation, Result};

/// Backup behavior for a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackupPolicy {
    /// Never create a backup.
    None,
    /// Copy the existing target to `<name>.bak` before overwriting.
    #[default]
    SiblingBak,
}

/// A save request describing exactly what will be written.
#[derive(Debug, Clone)]
pub struct SaveRequest {
    pub target: PathBuf,
    pub content: String,
    /// Encoding label to encode with; unknown labels fall back to UTF-8.
    pub encoding_label: String,
    /// Fingerprint captured when the file was loaded. `None` for a new file
    /// (Save As to a non-existent path); `Some` enables conflict detection.
    pub expected_fingerprint: Option<FileFingerprint>,
    pub backup: BackupPolicy,
}

/// The result of a successful save.
#[derive(Debug, Clone)]
pub struct SaveOutcome {
    pub written_bytes: u64,
    pub new_fingerprint: FileFingerprint,
    pub backup_path: Option<PathBuf>,
    /// `true` when the requested encoding label was unknown and UTF-8 was
    /// substituted; the UI should warn rather than treat this as success.
    pub encoding_fallback_to_utf8: bool,
}

/// Save text to a file with conflict detection, optional backup, and an
/// atomic temp-then-rename write.
pub fn save_text(request: &SaveRequest) -> Result<SaveOutcome> {
    let target = request.target.as_path();

    if let Some(expected) = request.expected_fingerprint
        && target.exists()
    {
        let current = FileFingerprint::capture(target, None)?;
        if current.len != expected.len
            || current.modified_unix_nanos != expected.modified_unix_nanos
        {
            return Err(CoreError::Conflict {
                message: "target changed on disk after it was loaded".into(),
            });
        }
    }

    let (bytes, fallback) = encode_text(&request.content, &request.encoding_label);

    let backup_path = if request.backup == BackupPolicy::SiblingBak && target.exists() {
        let bak = backup_path_for(target);
        fs::copy(target, &bak).map_err(|e| CoreError::io(target, IoOperation::CreateBackup, &e))?;
        Some(bak)
    } else {
        None
    };

    // Create parent directories for Save As to new nested paths.
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, IoOperation::Write, &e))?;
    }

    atomic_replace(target, &bytes)?;

    let new_fingerprint = FileFingerprint::capture(target, Some(&bytes))?;
    Ok(SaveOutcome {
        written_bytes: bytes.len() as u64,
        new_fingerprint,
        backup_path,
        encoding_fallback_to_utf8: fallback,
    })
}

/// Writes `bytes` to a sibling temp file, then renames it onto `target`.
/// Atomic on POSIX (`rename` within the same volume); on failure the temp
/// file is removed and the original at `target` is left untouched.
///
/// `pub(crate)` so `persist::schema`'s repositories (RFC-076) can reuse this
/// primitive for settings/session writes instead of hand-rolling their own
/// temp-then-rename logic. This function carries no document-save-specific
/// behavior (no fingerprint check, no `.bak` backup) — those stay in
/// [`save_text`]; callers needing them apply their own policy.
pub(crate) fn atomic_replace(target: &Path, bytes: &[u8]) -> Result<()> {
    let temp = temp_path_for(target);
    fs::write(&temp, bytes).map_err(|e| CoreError::io(&temp, IoOperation::Write, &e))?;
    if let Err(e) = fs::rename(&temp, target) {
        let _ = fs::remove_file(&temp);
        return Err(CoreError::io(target, IoOperation::Rename, &e));
    }
    Ok(())
}

// ── RFC-077: explicit target precondition and no-clobber commit ───────────────
//
// Additive to the `SaveRequest`/`save_text` path above — nothing here is
// wired into it yet. `TargetPrecondition` is the save-time counterpart of
// `compare_prep::TargetExpectation`; routing `save_text` through it is a
// later patch in the same milestone (RFC-077 §"Core commit semantics").

/// What a save must find true about its target immediately before writing.
/// Distinct from [`SaveRequest::expected_fingerprint`]: that field is
/// `Option<FileFingerprint>`, which cannot express "the path must not
/// exist" — `None` there means "skip the check," not "must be absent."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPrecondition {
    /// The target must exist, be a plain text file, and match this
    /// fingerprint exactly.
    MustMatch(FileFingerprint),
    /// The target must not exist at all — any entry (file, directory, or
    /// otherwise) at the path is a conflict.
    MustBeAbsent,
    /// Skip the check. Constructed only after an explicit, one-time user
    /// overwrite confirmation — never stored as a tab's load-time snapshot
    /// (RFC-077).
    Force,
}

/// Checks `precondition` against `target`'s current on-disk state, without
/// writing anything. Reuses [`check_external_state`] (RFC-036) for
/// `MustMatch` rather than re-deriving fingerprint comparison — the same
/// missing/changed/replaced distinctions apply to both.
///
/// Returns [`CoreError::Conflict`] on any precondition failure, never
/// [`CoreError::Io`] for an ordinary "not what was expected" case — review
/// 038 C1 (RFC-076) established that a stale-read conflict and a genuine I/O
/// failure must stay distinguishable; only a metadata read failure inside
/// `MustBeAbsent`'s existence check propagates as `Io`.
pub fn check_precondition(target: &Path, precondition: &TargetPrecondition) -> Result<()> {
    match precondition {
        TargetPrecondition::Force => Ok(()),
        TargetPrecondition::MustBeAbsent => {
            if target.exists() {
                Err(CoreError::Conflict {
                    message: "target already exists".into(),
                })
            } else {
                Ok(())
            }
        }
        TargetPrecondition::MustMatch(expected) => {
            match check_external_state(target, expected, false) {
                ExternalFileState::Clean => Ok(()),
                other => Err(CoreError::Conflict {
                    message: format!("target changed on disk after it was loaded ({other:?})"),
                }),
            }
        }
    }
}

/// Atomically commits `bytes` to `target`, failing rather than replacing if
/// any entry already exists there. Same-directory temp file + platform
/// no-clobber commit (`tempfile::NamedTempFile::persist_noclobber`), per
/// RFC-077: "If any filesystem entry appears after preparation, normal save
/// returns a conflict and preserves that entry." Creates missing parent
/// directories first, mirroring [`save_text`]'s Save-As-to-new-path support.
///
/// If the platform cannot provide this guarantee, the error propagates —
/// callers must not fall back to an overwriting write (RFC-077: "normal save
/// fails rather than falling back to replacement").
pub fn persist_noclobber(target: &Path, bytes: &[u8]) -> Result<()> {
    persist_noclobber_with_hook(target, bytes, || {})
}

/// The same commit, with a hook that runs after the temp file is fully
/// written but before the no-clobber commit — the deterministic race seam
/// RFC-077's test design calls for ("a test-only before-commit seam creates
/// the competing file after preparation/temp write but before
/// `persist_noclobber`"), used instead of sleeps to exercise the race
/// reliably. `pub(crate)` — only [`persist_noclobber`] and this module's own
/// tests need it.
pub(crate) fn persist_noclobber_with_hook(
    target: &Path,
    bytes: &[u8],
    before_commit: impl FnOnce(),
) -> Result<()> {
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, IoOperation::Write, &e))?;
    }
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| CoreError::io(dir, IoOperation::Write, &e))?;
    std::io::Write::write_all(&mut tmp, bytes)
        .map_err(|e| CoreError::io(target, IoOperation::Write, &e))?;

    before_commit();

    tmp.persist_noclobber(target).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            CoreError::Conflict {
                message: "target already exists".into(),
            }
        } else {
            CoreError::io(target, IoOperation::Rename, &e.error)
        }
    })?;
    Ok(())
}

fn file_name_string(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "forskscope".into())
}

fn backup_path_for(target: &Path) -> PathBuf {
    let name = format!("{}.bak", file_name_string(target));
    sibling(target, &name)
}

fn temp_path_for(target: &Path) -> PathBuf {
    let name = format!(".{}.fsk-tmp", file_name_string(target));
    sibling(target, &name)
}

fn sibling(target: &Path, name: &str) -> PathBuf {
    match target.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}
