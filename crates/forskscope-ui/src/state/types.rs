//! Shared data types for modal state and directory operations.

use std::path::PathBuf;

use forskscope_core::DiffOptions;
use forskscope_ui_logic::CompareRequest;

/// F84: which real action a confirmed [`LargeLoadPrompt`] resumes — the two
/// call sites `guard_for_sizes` is consulted from (handoff 011 §5) need
/// different information to resume: a fresh tab (`Open`) or an existing one
/// by index (`Reload`).
#[derive(Debug, Clone, PartialEq)]
pub enum LargeLoadTarget {
    Open(CompareRequest),
    Reload(usize),
}

/// A file pair large enough to require confirmation before loading
/// (`LoadGuard::ConfirmPrompt`, RFC-013 §"Large file prompt"). `opts` is
/// already inline-suppressed (`LoadGuard::suppress_inline()` is always
/// `true` for `ConfirmPrompt`) — confirming resumes with exactly these
/// options, not a freshly recomputed set, so the suppression decided here
/// is what the diff actually runs with.
#[derive(Debug, Clone, PartialEq)]
pub struct LargeLoadPrompt {
    pub target: LargeLoadTarget,
    pub opts: DiffOptions,
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub too_large: bool,
}

/// Summary of a completed batch copy operation, shown in the result modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchResultSpec {
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    /// Path to the manifest JSON file, if written successfully.
    pub manifest_path: Option<PathBuf>,
    /// Human-readable description of failure entries (first few).
    pub failure_details: Vec<String>,
}

impl BatchResultSpec {
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }
}

/// A pending directory file operation awaiting user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirOp {
    pub src: PathBuf,
    pub dst: PathBuf,
    /// Human-readable description for the confirmation modal.
    pub label: String,
}
