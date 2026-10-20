//! # forskscope-compare
//!
//! ForskScope's read-only comparison engine (RFC-093 §1). Loads, decodes
//! and classifies documents with fingerprints; computes line diffs under a
//! comparison profile; lists and recursively compares folders, with
//! digests; compares workbooks (behind the `spreadsheet` feature, on by
//! default).
//!
//! **This crate never writes a file and never spawns a process** — enforced
//! by `cargo xtask crate-boundaries` (RFC-093 §2), not just documented
//! here.

pub mod cancel;
pub mod diff;
pub mod dir;
pub mod document;
pub mod encoding;
pub mod error;
pub mod file_kind;
pub mod ignore;
pub mod job;
pub mod path;
#[cfg(feature = "spreadsheet")]
pub mod xlsx;

pub use cancel::CancellationToken;
pub use diff::{
    CaseSensitivity, CompareProfile, DiffAlgorithm, DiffDocument, DiffHunk, DiffOptions, DiffRow,
    DiffWarning, HunkKind, InlineKind, InlineSpan, NewlineCompareMode, WhitespaceMode,
    compute_diff,
};
pub use document::{
    ExternalFileState, FileFingerprint, FileId, LoadOptions, LoadedDocument, TextDocument,
    check_external_state, load_path,
};
pub use encoding::{
    BomPolicy, BomPresence, COMMON_ENCODING_LABELS, NewlinePolicy, NewlineStyle, TextEncoding,
};
pub use error::{
    AppErrorKind, CoreError, ErrorSeverity, IoOperation, RecoveryAction, RecoveryHint, Result,
    UserMessage,
};
pub use file_kind::{EditabilityClass, FileKind};
pub use ignore::IgnoreRules;
pub use job::{
    DIGEST_CONCURRENCY_LIMIT, FileSizeClass, JobHandle, JobId, JobKind, JobProgress,
    LARGE_DIRECTORY_VIRTUAL_THRESHOLD, LARGE_FILE_INLINE_DIFF_BYTES, LARGE_HUNK_AUTO_EXPAND_LINES,
    PerformanceLimits, VERY_LARGE_FILE_BYTES,
};

#[cfg(test)]
mod tests;

/// FNV-1a 64-bit hash used for cheap, deterministic identifiers and
/// non-cryptographic content digests. Not suitable for security purposes.
/// `pub`, not `pub(crate)`: `forskscope-edit`'s `merge` needs it too, and
/// `edit` already depends on `compare` (RFC-093 §1).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
