//! Input classification (RFC-001 §6.2).
//!
//! A file is classified once at load time. Binary content is never silently
//! treated as editable text; `.xlsx` goes through the spreadsheet adapter.

use std::fs;
use std::path::Path;

use crate::error::{CoreError, IoOperation, Result};
use crate::path::has_extension;

/// How a file participates in comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    /// Decodable text; full diff/merge support.
    Text,
    /// Binary content; compared via hex preview, never merged as text.
    Binary,
    /// Excel workbook; compared through the spreadsheet adapter (read-only).
    ExcelXlsx,
    /// The path does not exist. One missing side is a valid comparison input.
    Missing,
    /// Exists but cannot be compared (e.g. not a regular file).
    Unsupported { reason: String },
}

impl FileKind {
    /// `true` when this side can take part in a text merge and be saved.
    pub fn is_mergeable_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}

/// Number of leading bytes sampled for binary detection.
const SAMPLE_LEN: usize = 8 * 1024;

/// Classify a path without fully loading it.
///
/// Rules, in order:
/// 1. Missing path -> `Missing`.
/// 2. Not a regular file (directory, fifo, ...) -> `Unsupported`.
///    Symlinks are followed; a symlink to a regular file is that file.
/// 3. `.xlsx` extension -> `ExcelXlsx`.
/// 4. NUL byte in the leading sample -> `Binary`.
/// 5. Otherwise -> `Text` (decoding decides the encoding later).
pub fn classify(path: &Path) -> Result<FileKind> {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(FileKind::Missing),
        Err(e) => return Err(CoreError::io(path, IoOperation::Metadata, &e)),
    };
    if !meta.is_file() {
        return Ok(FileKind::Unsupported {
            reason: "not a regular file".into(),
        });
    }
    if has_extension(path, "xlsx") {
        return Ok(FileKind::ExcelXlsx);
    }
    let bytes = read_sample(path)?;
    if bytes.contains(&0u8) {
        Ok(FileKind::Binary)
    } else {
        Ok(FileKind::Text)
    }
}

fn read_sample(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut f = fs::File::open(path).map_err(|e| CoreError::io(path, IoOperation::Read, &e))?;
    let mut buf = vec![0u8; SAMPLE_LEN];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = f
            .read(&mut buf[filled..])
            .map_err(|e| CoreError::io(path, IoOperation::Read, &e))?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    Ok(buf)
}
