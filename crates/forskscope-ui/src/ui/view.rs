//! Main user-facing workspace views.

pub mod deep_compare;
pub mod diff;
pub mod diff_actions;
pub mod digest_epoch;
pub mod dir_pane;
pub mod explorer;
pub mod export_dir_report;
pub mod hunk;
/// F235: every other place a file/dir *name* is shown reuses RFC-100's
/// own `BidiText` (previously private to `hunk.rs`'s diff rows) instead
/// of a second implementation.
pub(crate) use hunk::BidiText;
pub mod search;
pub mod settings;
