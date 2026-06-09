//! Single-file copy for directory merge operations (RFC-007, RFC-031).
//!
//! The copy respects the same save-safety model as text merge:
//! an optional sibling `.bak` is created before overwriting an existing
//! destination file, so the operation is reversible.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, IoOperation, Result};
use crate::save::BackupPolicy;

/// Result of a successful copy.
#[derive(Debug, Clone)]
pub struct CopyOutcome {
    pub src: PathBuf,
    pub dst: PathBuf,
    /// Path of the backup that was created, if any.
    pub backup_path: Option<PathBuf>,
    pub bytes_copied: u64,
}

/// Copy `src` to `dst`, optionally creating a `.bak` sibling of the
/// destination before overwriting it.
///
/// Creates the destination's parent directory if it does not exist.
pub fn copy_file(src: &Path, dst: &Path, backup: BackupPolicy) -> Result<CopyOutcome> {
    if !src.exists() {
        return Err(CoreError::InvalidPath {
            path: src.display().to_string(),
            reason: "source does not exist".into(),
        });
    }
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| CoreError::io(parent, IoOperation::Write, &e))?;
        }
    }
    let backup_path = if backup == BackupPolicy::SiblingBak && dst.exists() {
        let bak = bak_path(dst);
        fs::copy(dst, &bak).map_err(|e| CoreError::io(dst, IoOperation::CreateBackup, &e))?;
        Some(bak)
    } else {
        None
    };
    let bytes_copied =
        fs::copy(src, dst).map_err(|e| CoreError::io(dst, IoOperation::Write, &e))?;
    Ok(CopyOutcome {
        src: src.to_path_buf(),
        dst: dst.to_path_buf(),
        backup_path,
        bytes_copied,
    })
}

fn bak_path(dst: &Path) -> PathBuf {
    let name = dst
        .file_name()
        .map(|n| format!("{}.bak", n.to_string_lossy()))
        .unwrap_or_else(|| "backup.bak".into());
    dst.parent().map(|p| p.join(&name)).unwrap_or_else(|| PathBuf::from(&name))
}
