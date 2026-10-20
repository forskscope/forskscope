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

pub mod checksum;
pub mod command;
pub mod compare_prep;
pub mod conflict_nav;
pub mod diff_decoration;
pub mod dir;
pub mod edit_op;
pub mod env_passthrough;
pub mod external_tool;
pub mod line_map;
pub mod merge;
pub mod patch;
pub mod persist;
pub mod platform;
pub mod report;
pub mod save;
pub mod settings;
pub mod vcs;
pub mod watcher;

pub use compare_prep::{
    // RFC-077
    PreparedCompare,
    SaveTargetBlockReason,
    SaveTargetSnapshot,
    SaveTargetState,
    TargetExpectation,
    inspect_save_target,
    save_target_from_loaded,
};
pub use merge::{
    ConflictId, ConflictStatus, HunkState, MergeConflict, MergeHunk, MergeSession, SessionRevision,
    ThreeWayMergeSession, ThreeWayStats, TransactionEntry, TransactionKind, TransactionLog,
    UnixTimestamp,
};
pub use patch::{
    LineOrigin, PatchDocument, PatchFileChange, PatchFormat, PatchHunk, PatchLine, PatchOptions,
    PatchSummary, patch_from_directories, patch_from_file_diff, to_unified,
};
pub use save::{
    BackupPolicy,
    SaveOutcome,
    SaveRequest,
    // RFC-077
    TargetPrecondition,
    check_precondition,
    persist_noclobber,
    save_text,
};

#[cfg(test)]
mod tests;

pub use dir::batch_copy;
pub use dir::copy_file;
