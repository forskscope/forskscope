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

pub mod cancel;
pub mod diff;
pub mod diff_decoration;
pub mod dir;
pub mod document;
pub mod encoding;
pub mod error;
pub mod external_tool;
pub mod file_kind;
pub mod ignore;
pub mod job;
pub mod line_map;
pub mod merge;
pub mod patch;
pub mod path;
pub mod persist;
pub mod report;
pub mod save;
pub mod session;
pub mod settings;
pub mod vcs;
pub mod xlsx;

pub use diff::{
    DiffAlgorithm, DiffDocument, DiffHunk, DiffOptions, DiffRow, DiffWarning, HunkKind, InlineKind, InlineSpan,
    compute_diff,
    // RFC-028
    CaseSensitivity, CompareProfile, NewlineCompareMode, WhitespaceMode,
};
pub use document::{
    FileFingerprint, FileId, LoadOptions, LoadedDocument, TextDocument, load_path,
    // RFC-036
    ExternalFileState, check_external_state,
};
pub use encoding::{NewlineStyle, TextEncoding,
    // RFC-012
    NewlinePolicy,
};
pub use error::{
    CoreError, ErrorSeverity, IoOperation, RecoveryHint, Result,
    // RFC-017
    AppErrorKind, RecoveryAction, UserMessage,
};
pub use file_kind::{FileKind,
    // RFC-012
    EditabilityClass,
};
pub use ignore::IgnoreRules;
pub use merge::{
    ConflictId, ConflictStatus, HunkState, MergeConflict, MergeHunk, MergeSession,
    SessionRevision, ThreeWayMergeSession, ThreeWayStats, TransactionEntry,
    TransactionKind, TransactionLog, UnixTimestamp,
};
pub use patch::{
    LineOrigin, PatchDocument, PatchFileChange, PatchFormat, PatchHunk, PatchLine, PatchOptions,
    PatchSummary, patch_from_directories, patch_from_file_diff, to_unified,
};
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
pub use cancel::CancellationToken;
pub use job::{
    DIGEST_CONCURRENCY_LIMIT, LARGE_DIRECTORY_VIRTUAL_THRESHOLD,
    LARGE_FILE_INLINE_DIFF_BYTES, LARGE_HUNK_AUTO_EXPAND_LINES,
    VERY_LARGE_FILE_BYTES,
    JobHandle, JobId, JobKind, JobProgress,
    // RFC-013
    FileSizeClass, PerformanceLimits,
};
pub use dir::batch_copy;
pub use dir::copy_file;
