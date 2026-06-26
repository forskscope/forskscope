//! Comparison report types for file and directory diffs (RFC-006, RFC-008).
//! File-level report lives in `file`; directory-level in `dir`.

pub mod dir;
pub mod file;

pub use file::{
    FileComparisonReport, HunkSummaryRow, HistoryEntry, ReportOptions, ReportPathMode,
};
pub use dir::{
    BatchSummary, DirComparisonReport, DirFileRow,
};

use std::path::Path;
use crate::diff::HunkKind;

/// Format a path according to `mode`, optionally stripping a `root` prefix.
/// Shared by both file and directory reports.
pub(crate) fn display_path(path: Option<&Path>, mode: &file::ReportPathMode, root: Option<&Path>) -> String {
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
        HunkKind::Equal   => "equal".into(),
        HunkKind::Insert  => "insert".into(),
        HunkKind::Delete  => "delete".into(),
        HunkKind::Replace => "replace".into(),
    }
}
