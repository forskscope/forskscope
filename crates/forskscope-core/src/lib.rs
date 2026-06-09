//! # forskscope-core
//!
//! GUI-independent core of ForskScope (RFC-001, RFC-002).
//!
//! This crate owns product truth: file identity, text decoding metadata,
//! the normalized diff model built on `similar` v3, the model-backed merge
//! session with its transaction log, save safety policy, and directory
//! comparison. No Dioxus, Tauri, WebView, or JavaScript type appears here.
//!
//! UI layers consume these domain objects and derive their own view models;
//! they must never become an independent source of truth (see RFC-042,
//! "Core First").

pub mod diff;
pub mod dir;
pub mod document;
pub mod encoding;
pub mod error;
pub mod file_kind;
pub mod merge;
pub mod path;
pub mod save;
pub mod xlsx;

pub use diff::{
    DiffDocument, DiffHunk, DiffOptions, DiffRow, DiffWarning, HunkKind, InlineKind, InlineSpan,
    compute_diff,
};
pub use document::{FileFingerprint, FileId, LoadOptions, LoadedDocument, TextDocument, load_path};
pub use encoding::{NewlineStyle, TextEncoding};
pub use error::{CoreError, IoOperation, Result};
pub use file_kind::FileKind;
pub use merge::{HunkState, MergeHunk, MergeSession};
pub use save::{BackupPolicy, SaveOutcome, SaveRequest, save_text};

#[cfg(test)]
mod tests;

/// FNV-1a 64-bit hash used for cheap, deterministic identifiers and
/// non-cryptographic content digests. Not suitable for security purposes.
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
pub use dir::copy_file;
