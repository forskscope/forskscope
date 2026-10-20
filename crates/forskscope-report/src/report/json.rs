//! The versioned `forskscope.comparison/1` JSON contract (RFC-094 §3).
//!
//! Amends RFC-027's JSON report: it was never shipped, and this is its
//! replacement. **Its own types, built from the engine's results — the
//! engine's types are never serialized directly**, so the engine can
//! change shape without the contract changing (the same rule the persisted
//! settings' `Persisted*` types already follow — see
//! `forskscope-desktop::persist::schema`'s module doc).
//!
//! `schema: "forskscope.comparison/1"` on every document. Adding a field
//! keeps `/1`; removing a field, or changing its meaning, moves to `/2`. A
//! JSON Schema file (`docs/schemas/forskscope.comparison.v1.json`) and a
//! gate that validates emitted documents against it live outside this
//! crate — `xtask` reads this module's output, not the other way around.

use serde::Serialize;

use forskscope_compare::dir::RecStatus;

use super::changes::{ChangeHunk, ChangeLine, ChangeLineKind};
use super::dir::{DirComparisonReport, DirFileRow};
use super::file::FileComparisonReport;

/// The schema identifier every document in this module's output carries.
pub const COMPARISON_SCHEMA: &str = "forskscope.comparison/1";

// ── Shared pieces ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SideInfo {
    pub name: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub encoding: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonChangeLineKind {
    Context,
    Removed,
    Added,
}

impl From<ChangeLineKind> for JsonChangeLineKind {
    fn from(kind: ChangeLineKind) -> Self {
        match kind {
            ChangeLineKind::Context => Self::Context,
            ChangeLineKind::Delete => Self::Removed,
            ChangeLineKind::Insert => Self::Added,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonChangeLine {
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub kind: JsonChangeLineKind,
    pub text: String,
}

impl From<&ChangeLine> for JsonChangeLine {
    fn from(line: &ChangeLine) -> Self {
        Self {
            old_line: line.left_line,
            new_line: line.right_line,
            kind: line.kind.into(),
            text: line.content.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonChangeHunk {
    pub old_start: u32,
    pub old_len: u32,
    pub new_start: u32,
    pub new_len: u32,
    pub lines: Vec<JsonChangeLine>,
}

impl From<&ChangeHunk> for JsonChangeHunk {
    fn from(hunk: &ChangeHunk) -> Self {
        Self {
            old_start: hunk.old_start,
            old_len: hunk.old_len,
            new_start: hunk.new_start,
            new_len: hunk.new_len,
            lines: hunk.lines.iter().map(JsonChangeLine::from).collect(),
        }
    }
}

fn hunks_to_json(hunks: &[ChangeHunk]) -> Vec<JsonChangeHunk> {
    hunks.iter().map(JsonChangeHunk::from).collect()
}

/// The stable, machine-readable status identifier for one `RecStatus`
/// (RFC-094 §3) — one-to-one with the word `status_key`
/// (`forskscope_compare::dir::status_key`) shows a person. Deliberately no
/// wildcard arm: adding a `RecStatus` variant must fail this match at
/// compile time until this function names its identifier too, the same
/// discipline `status_key` itself already follows for its own English
/// words.
pub fn status_identifier(status: RecStatus) -> &'static str {
    match status {
        RecStatus::Equal => "identical",
        RecStatus::Changed => "different",
        RecStatus::LeftOnly => "left_only",
        RecStatus::RightOnly => "right_only",
        RecStatus::LeftOnlyDir => "left_only_empty_folder",
        RecStatus::RightOnlyDir => "right_only_empty_folder",
        RecStatus::Computing => "not_compared",
        RecStatus::Symlink => "symlink_not_followed",
        RecStatus::Unreadable => "unreadable",
    }
}

// ── File comparison document ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FileCounts {
    pub changes: usize,
    pub lines_removed: usize,
    pub lines_added: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileComparisonDocument {
    pub schema: &'static str,
    pub kind: &'static str,
    pub left: SideInfo,
    pub right: SideInfo,
    pub identical: bool,
    pub counts: FileCounts,
    pub changes: Vec<JsonChangeHunk>,
    pub notes: Vec<String>,
}

/// Build the `"file"` document from an already-assembled
/// [`FileComparisonReport`] (built by [`super::build_file_report`], with its
/// checksums attached) — `left_encoding`/`right_encoding` are the only
/// piece this needs that the report itself does not carry (RFC-094 §3's
/// "an encoding that is not UTF-8" note lives in `notes`, but the
/// encoding *label* itself is reported unconditionally, like a file's
/// size).
pub fn file_comparison_document(
    report: &FileComparisonReport,
    left_encoding: &str,
    right_encoding: &str,
) -> FileComparisonDocument {
    let side = |display: &str, checksum: &Option<super::FileChecksum>, encoding: &str| SideInfo {
        name: display.to_string(),
        size: checksum.as_ref().map(|c| c.size).unwrap_or(0),
        sha256: checksum.as_ref().map(|c| c.sha256_hex.clone()),
        encoding: encoding.to_string(),
    };
    FileComparisonDocument {
        schema: COMPARISON_SCHEMA,
        kind: "file",
        left: side(&report.left_display, &report.left_checksum, left_encoding),
        right: side(
            &report.right_display,
            &report.right_checksum,
            right_encoding,
        ),
        identical: report.is_identical,
        counts: FileCounts {
            changes: report.hunks_changed,
            lines_removed: report.lines_deleted,
            lines_added: report.lines_added,
        },
        changes: hunks_to_json(&report.change_hunks),
        notes: report.warnings.clone(),
    }
}

// ── Directory comparison document ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DirEntryJson {
    pub path: String,
    pub status: String,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
    pub left_sha256: Option<String>,
    pub right_sha256: Option<String>,
    pub changes: Option<Vec<JsonChangeHunk>>,
}

impl From<&DirFileRow> for DirEntryJson {
    fn from(row: &DirFileRow) -> Self {
        Self {
            path: row.path.clone(),
            status: status_identifier(row.rec_status).to_string(),
            left_size: row.left_size,
            right_size: row.right_size,
            left_sha256: row.left_checksum.as_ref().map(|c| c.sha256_hex.clone()),
            right_sha256: row.right_checksum.as_ref().map(|c| c.sha256_hex.clone()),
            changes: row.change_hunks.as_ref().map(|hunks| hunks_to_json(hunks)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OmittedEntry {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirComparisonDocument {
    pub schema: &'static str,
    pub kind: &'static str,
    pub left: String,
    pub right: String,
    /// Entry count by [`status_identifier`] — a status this scan never
    /// produced is simply absent, not listed at zero (RFC-094 §3: "counts
    /// by status").
    pub counts: std::collections::BTreeMap<String, usize>,
    pub entries: Vec<DirEntryJson>,
    /// Never silent (RFC-094 §3): every changed text file the
    /// `--max-files`/`--max-change-bytes` caps left out, by path.
    pub omitted: Vec<OmittedEntry>,
}

/// Build the `"directory"` document from an already-assembled
/// [`DirComparisonReport`] (built by [`super::build_dir_report`]).
pub fn dir_comparison_document(report: &DirComparisonReport) -> DirComparisonDocument {
    let mut counts = std::collections::BTreeMap::new();
    for row in &report.file_rows {
        *counts
            .entry(status_identifier(row.rec_status).to_string())
            .or_insert(0) += 1;
    }
    DirComparisonDocument {
        schema: COMPARISON_SCHEMA,
        kind: "directory",
        left: report.left_display.clone(),
        right: report.right_display.clone(),
        counts,
        entries: report.file_rows.iter().map(DirEntryJson::from).collect(),
        omitted: report
            .change_hunks_omitted
            .iter()
            .map(|path| OmittedEntry {
                path: path.clone(),
                reason: "change content omitted: --max-files or --max-change-bytes limit reached",
            })
            .collect(),
    }
}

impl FileComparisonDocument {
    /// Pretty-printed JSON, `forskscope-cli compare --format json`'s exact
    /// output for a file comparison - kept here so the CLI needs no direct
    /// `serde_json` dependency of its own (RFC-094 §1: "nothing from this
    /// workspace" beyond `forskscope-compare`/`forskscope-report"; this
    /// keeps its *external* dependency list minimal too, in the same
    /// spirit).
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

impl DirComparisonDocument {
    /// Pretty-printed JSON, the directory-comparison counterpart of
    /// [`FileComparisonDocument::to_json_string`].
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recstatus_has_a_distinct_identifier_matching_the_rfc() {
        let expected: &[(RecStatus, &str)] = &[
            (RecStatus::Equal, "identical"),
            (RecStatus::Changed, "different"),
            (RecStatus::LeftOnly, "left_only"),
            (RecStatus::RightOnly, "right_only"),
            (RecStatus::LeftOnlyDir, "left_only_empty_folder"),
            (RecStatus::RightOnlyDir, "right_only_empty_folder"),
            (RecStatus::Computing, "not_compared"),
            (RecStatus::Symlink, "symlink_not_followed"),
            (RecStatus::Unreadable, "unreadable"),
        ];
        for (status, id) in expected {
            assert_eq!(status_identifier(*status), *id);
        }
        // Every identifier is unique - two statuses must never collapse to
        // the same machine-readable word.
        let mut ids: Vec<&str> = expected.iter().map(|(_, id)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), expected.len());
    }

    #[test]
    fn status_identifier_also_matches_forskscope_compares_own_status_key_one_to_one() {
        // RFC-094 §3: "Each maps one-to-one to the words the app shows."
        // `status_key`'s English word and this identifier must exist for
        // exactly the same set of statuses - this is the compile-time half
        // (an exhaustive match in both functions); this test is the
        // runtime half, that neither ever panics/fails on the other's
        // input.
        use forskscope_compare::dir::status_key;
        for status in [
            RecStatus::Equal,
            RecStatus::Changed,
            RecStatus::LeftOnly,
            RecStatus::RightOnly,
            RecStatus::LeftOnlyDir,
            RecStatus::RightOnlyDir,
            RecStatus::Computing,
            RecStatus::Symlink,
            RecStatus::Unreadable,
        ] {
            assert!(!status_identifier(status).is_empty());
            assert!(!status_key(status, false).is_empty());
        }
    }
}
