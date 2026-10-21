//! Comparison report types for file and directory diffs (RFC-006, RFC-008).
//! File-level report lives in `file`; directory-level in `dir`. Three
//! renderers read the same model: `html` (the saved report, unchanged),
//! `text` (plain text, RFC-094 §2's `--format text`, review 182 §2), and
//! `json` (the versioned `forskscope.comparison/1` contract, RFC-094
//! §3). The old hand-rolled `to_markdown`/`to_json` methods this module
//! once carried directly on `FileComparisonReport`/`DirComparisonReport`
//! are gone (review 182 §2): `to_markdown` violated RFC-092's own display
//! rules (zero-count noise, raw jargon, options shown at their defaults,
//! and no change content at all), and `to_json` is superseded by the
//! versioned contract `json` now builds.

pub mod changes;
pub mod dir;
pub mod file;
pub mod html;
pub mod json;
pub mod text;

pub use changes::{ChangeHunk, ChangeLine, ChangeLineKind, change_hunks_from_diff};
pub use dir::{BuildOutcome, DirComparisonReport, DirFileRow, build_dir_report};
pub use file::{
    DEFAULT_MAX_CHANGE_TEXT_BYTES, DEFAULT_MAX_CHANGED_FILES_WITH_CONTENT, FileComparisonReport,
    ReportOptions, ReportPathMode, build_file_report, checksum_if_fingerprinted,
};
pub use html::{render_dir_report, render_file_report};
pub use json::{
    COMPARISON_SCHEMA, DirComparisonDocument, DirEntryJson, FileComparisonDocument, FileCounts,
    JsonChangeHunk, JsonChangeLine, JsonChangeLineKind, OmittedEntry, SideInfo,
    dir_comparison_document, file_comparison_document, status_identifier,
};
pub use text::{render_dir_report_text, render_file_report_text};

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
