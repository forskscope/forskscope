//! # forskscope-edit
//!
//! ForskScope's write path (RFC-093 §1): merge sessions including three-way
//! merge, edit operations, atomic save with fingerprint preconditions,
//! folder batch-copy/plan/execute, and the save-target half of prepared
//! comparison. This is the only crate besides the GUI itself that writes a
//! user's files.

pub mod compare_prep;
pub mod dir;
pub mod edit_op;
pub mod merge;
pub mod save;

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
