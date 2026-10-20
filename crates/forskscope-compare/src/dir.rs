//! Directory listing, digest comparison, and recursive diff — read-only
//! (RFC-005, RFC-037). The write half (batch copy, plan/execute) is
//! `forskscope_edit::dir` (RFC-093 §1).

mod digest;
mod evidence;
mod listing;
mod recursive;

pub use digest::{
    DigestOutcome, dir_digest_equal, file_digest_equal, file_digest_equal_with_cancel,
};
pub use evidence::{EntryType, EqualityEvidence};
pub use listing::{DirectoryListing, FileEntry, list_dir};
pub use recursive::{
    RecEntry, RecStatus, RecursiveScan, list_recursive_for_display,
    list_recursive_for_display_with_cancel, list_recursive_for_display_with_rules, recursive_diff,
    recursive_diff_with_cancel, recursive_diff_with_rules, status_key,
};
