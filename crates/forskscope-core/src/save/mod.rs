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

use crate::document::FileFingerprint;
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
        if current.len != expected.len || current.modified_unix_nanos != expected.modified_unix_nanos
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

    let temp = temp_path_for(target);
    fs::write(&temp, &bytes).map_err(|e| CoreError::io(&temp, IoOperation::Write, &e))?;
    if let Err(e) = fs::rename(&temp, target) {
        let _ = fs::remove_file(&temp);
        return Err(CoreError::io(target, IoOperation::Rename, &e));
    }

    let new_fingerprint = FileFingerprint::capture(target, Some(&bytes))?;
    Ok(SaveOutcome {
        written_bytes: bytes.len() as u64,
        new_fingerprint,
        backup_path,
        encoding_fallback_to_utf8: fallback,
    })
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
