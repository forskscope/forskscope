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
pub mod command;
pub mod conflict_nav;
pub mod diff;
pub mod diff_decoration;
pub mod dir;
pub mod document;
pub mod edit_op;
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
pub mod platform;
pub mod report;
pub mod save;
pub mod settings;
pub mod vcs;
pub mod watcher;
pub mod xlsx;

pub use diff::{
    // RFC-028
    CaseSensitivity,
    CompareProfile,
    DiffAlgorithm,
    DiffDocument,
    DiffHunk,
    DiffOptions,
    DiffRow,
    DiffWarning,
    HunkKind,
    InlineKind,
    InlineSpan,
    NewlineCompareMode,
    WhitespaceMode,
    compute_diff,
};
pub use document::{
    // RFC-036
    ExternalFileState,
    FileFingerprint,
    FileId,
    LoadOptions,
    LoadedDocument,
    TextDocument,
    check_external_state,
    load_path,
};
pub use encoding::{
    // RFC-012
    NewlinePolicy,
    NewlineStyle,
    TextEncoding,
};
pub use error::{
    // RFC-017
    AppErrorKind,
    CoreError,
    ErrorSeverity,
    IoOperation,
    RecoveryAction,
    RecoveryHint,
    Result,
    UserMessage,
};
pub use file_kind::{
    // RFC-012
    EditabilityClass,
    FileKind,
};
pub use ignore::IgnoreRules;
pub use merge::{
    ConflictId, ConflictStatus, HunkState, MergeConflict, MergeHunk, MergeSession, SessionRevision,
    ThreeWayMergeSession, ThreeWayStats, TransactionEntry, TransactionKind, TransactionLog,
    UnixTimestamp,
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
pub use dir::batch_copy;
pub use dir::copy_file;
pub use job::{
    DIGEST_CONCURRENCY_LIMIT,
    // RFC-013
    FileSizeClass,
    JobHandle,
    JobId,
    JobKind,
    JobProgress,
    LARGE_DIRECTORY_VIRTUAL_THRESHOLD,
    LARGE_FILE_INLINE_DIFF_BYTES,
    LARGE_HUNK_AUTO_EXPAND_LINES,
    PerformanceLimits,
    VERY_LARGE_FILE_BYTES,
};
