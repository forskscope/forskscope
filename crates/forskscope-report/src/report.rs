//! Comparison report types for file and directory diffs (RFC-006, RFC-008).
//! File-level report lives in `file`; directory-level in `dir`.
//!
//! ## Schema versioning (F31, decided 2026-08-13)
//!
//! `FileComparisonReport::to_json`/`DirComparisonReport::to_json` hand-roll
//! JSON with no schema envelope, deliberately, for the same reason
//! `dir::batch::BatchManifest` does (see its module doc): both are built
//! only from live in-memory data (`from_diff`/`from_entries`), and nothing
//! in ForskScope ever parses an exported report JSON file back in. They are
//! a write-only export for the user — something to save, share, or feed to
//! another tool — not something this app reloads across versions, so there
//! is no read path a schema version would protect.

pub mod changes;
pub mod dir;
pub mod file;
pub mod html;
pub mod json;

pub use changes::{ChangeHunk, ChangeLine, ChangeLineKind, change_hunks_from_diff};
pub use dir::{BuildOutcome, DirComparisonReport, DirFileRow, build_dir_report};
pub use file::{
    DEFAULT_MAX_CHANGE_TEXT_BYTES, DEFAULT_MAX_CHANGED_FILES_WITH_CONTENT, FileComparisonReport,
    HunkSummaryRow, ReportOptions, ReportPathMode, build_file_report, checksum_if_fingerprinted,
};
pub use html::{render_dir_report, render_file_report};
pub use json::{
    COMPARISON_SCHEMA, DirComparisonDocument, DirEntryJson, FileComparisonDocument, FileCounts,
    JsonChangeHunk, JsonChangeLine, JsonChangeLineKind, OmittedEntry, SideInfo,
    dir_comparison_document, file_comparison_document, status_identifier,
};

use forskscope_compare::diff::HunkKind;
use std::path::Path;

/// Size and SHA-256 of one compared file's bytes, as read at export time
/// (RFC-092 §5). Computed only for a report export, never kept or written
/// anywhere else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChecksum {
    pub size: u64,
    pub sha256_hex: String,
}

impl FileChecksum {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self {
            size: bytes.len() as u64,
            sha256_hex: crate::checksum::sha256_hex(bytes),
        }
    }
}

/// Format a path according to `mode`, optionally stripping a `root` prefix.
/// Shared by both file and directory reports.
pub(crate) fn display_path(
    path: Option<&Path>,
    mode: &file::ReportPathMode,
    root: Option<&Path>,
) -> String {
    match path {
        None => "(unknown)".into(),
        Some(p) => match mode {
            file::ReportPathMode::NameOnly => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            file::ReportPathMode::Relative => {
                if let Some(r) = root {
                    p.strip_prefix(r)
                        .map(|rel| rel.display().to_string())
                        .unwrap_or_else(|_| p.display().to_string())
                } else {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string())
                }
            }
            file::ReportPathMode::Absolute => p.display().to_string(),
        },
    }
}

/// Human-readable hunk kind label. Shared by both report types.
pub(crate) fn hunk_kind_label(kind: HunkKind) -> String {
    match kind {
        HunkKind::Equal => "equal".into(),
        HunkKind::Insert => "insert".into(),
        HunkKind::Delete => "delete".into(),
        HunkKind::Replace => "replace".into(),
    }
}
