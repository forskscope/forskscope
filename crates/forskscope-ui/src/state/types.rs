//! Shared data types for modal state and directory operations.

use std::path::PathBuf;

/// Summary of a completed batch copy operation, shown in the result modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchResultSpec {
    pub succeeded: usize,
    pub failed:    usize,
    pub skipped:   usize,
    /// Path to the manifest JSON file, if written successfully.
    pub manifest_path: Option<PathBuf>,
    /// Human-readable description of failure entries (first few).
    pub failure_details: Vec<String>,
}

impl BatchResultSpec {
    pub fn all_succeeded(&self) -> bool { self.failed == 0 }
}

/// A pending directory file operation awaiting user confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirOp {
    pub src:   PathBuf,
    pub dst:   PathBuf,
    /// Human-readable description for the confirmation modal.
    pub label: String,
}
