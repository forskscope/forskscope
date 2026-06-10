//! Comparison report export (RFC-027).
//!
//! Builds human-readable Markdown and machine-readable JSON reports from
//! file or directory comparison results. Reports are privacy-safe by
//! default: absolute paths are redacted unless the caller opts in via
//! [`ReportPathMode`].
//!
//! ## Entry points
//!
//! - [`FileComparisonReport::from_diff`] — build from a [`DiffDocument`]
//!   with optional merge history from a [`TransactionLog`].
//! - [`DirComparisonReport::from_entries`] — build from a `Vec<RecEntry>`
//!   with optional [`BatchManifest`].
//! - `.to_markdown()` / `.to_json()` — render either report format.

use std::fmt::Write as _;
use std::path::Path;

use crate::diff::{DiffDocument, DiffWarning, HunkKind};
use crate::dir::{BatchManifest, RecEntry, RecStatus};
use crate::merge::{TransactionEntry, TransactionLog};

// ── Report options ────────────────────────────────────────────────────────────

/// Controls how paths are rendered in reports (RFC-027 §"Privacy policy").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportPathMode {
    /// Show only the filename (basename). Default. Safe for sharing.
    #[default]
    NameOnly,
    /// Show the path relative to a supplied root, if available.
    Relative,
    /// Show the full absolute path. Must be explicitly requested.
    Absolute,
}

/// What optional sections to include in a report.
#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub path_mode:        ReportPathMode,
    /// Include hunk-level detail (line ranges, types).
    pub include_hunks:    bool,
    /// Include merge / operation history from the transaction log.
    pub include_history:  bool,
    /// Include compare option settings.
    pub include_options:  bool,
    /// Include diff warnings (large file, deadline, etc.).
    pub include_warnings: bool,
    /// Include per-file size columns in directory reports.
    pub include_sizes:    bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            path_mode:        ReportPathMode::NameOnly,
            include_hunks:    true,
            include_history:  true,
            include_options:  true,
            include_warnings: true,
            include_sizes:    true,
        }
    }
}

// ── File comparison report ────────────────────────────────────────────────────

/// A rendered report for a single file comparison.
#[derive(Debug, Clone)]
pub struct FileComparisonReport {
    /// Display name for the left/old side.
    pub left_display:  String,
    /// Display name for the right/new side.
    pub right_display: String,
    pub is_identical:  bool,
    pub hunks_total:   usize,
    pub hunks_changed: usize,
    pub lines_added:   usize,
    pub lines_deleted: usize,
    pub warnings:      Vec<String>,
    pub hunk_rows:     Vec<HunkSummaryRow>,
    pub history:       Vec<HistoryEntry>,
    pub options_summary: Vec<(String, String)>,
    pub options:       ReportOptions,
}

/// One row in the hunk summary table.
#[derive(Debug, Clone)]
pub struct HunkSummaryRow {
    pub hunk_num:    usize,
    pub left_start:  u32,
    pub left_end:    u32,
    pub right_start: u32,
    pub right_end:   u32,
    pub kind:        String,
}

/// One entry in the merge/operation history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp_sec: u64,
    pub label:         String,
}

impl FileComparisonReport {
    /// Build a report from a `DiffDocument`, optional file paths for display,
    /// an optional `TransactionLog` for history, and rendering options.
    pub fn from_diff(
        diff:      &DiffDocument,
        left_path: Option<&Path>,
        right_path: Option<&Path>,
        log:       Option<&TransactionLog>,
        opts:      ReportOptions,
    ) -> Self {
        let left_display  = display_path(left_path,  &opts.path_mode, None);
        let right_display = display_path(right_path, &opts.path_mode, None);

        let warnings = diff.warnings.iter().map(|w| match w {
            DiffWarning::LargeFilePolicyApplied   => "Large file — inline diff disabled.".into(),
            DiffWarning::DeadlineExpired          => "Diff timed out — result may be approximate.".into(),
            DiffWarning::InlineSkippedHunkTooLarge => "Some hunks were too large for inline diff.".into(),
        }).collect();

        let hunk_rows: Vec<HunkSummaryRow> = if opts.include_hunks {
            diff.hunks.iter().enumerate()
                .filter(|(_, h)| h.kind != HunkKind::Equal)
                .map(|(i, h)| HunkSummaryRow {
                    hunk_num:    i + 1,
                    left_start:  h.left_range.start,
                    left_end:    h.left_range.start + h.left_range.len,
                    right_start: h.right_range.start,
                    right_end:   h.right_range.start + h.right_range.len,
                    kind:        hunk_kind_label(h.kind),
                })
                .collect()
        } else { vec![] };

        let history: Vec<HistoryEntry> = if opts.include_history {
            log.map(|l| l.active_entries().iter().map(|e: &TransactionEntry| HistoryEntry {
                timestamp_sec: e.timestamp.0,
                label:         e.label.clone(),
            }).collect()).unwrap_or_default()
        } else { vec![] };

        let options_summary = if opts.include_options {
            let o = &diff.options;
            vec![
                ("Whitespace".into(), if o.ignore_whitespace { "ignored".into() } else { "significant".into() }),
                ("Case".into(),       if o.ignore_case       { "ignored".into() } else { "significant".into() }),
                ("Algorithm".into(),  format!("{:?}", o.algorithm)),
            ]
        } else { vec![] };

        Self {
            left_display, right_display,
            is_identical:  diff.is_identical(),
            hunks_total:   diff.stats.hunks_total,
            hunks_changed: diff.stats.hunks_changed,
            lines_added:   diff.stats.lines_inserted,
            lines_deleted: diff.stats.lines_deleted,
            warnings, hunk_rows, history, options_summary, options: opts,
        }
    }

    // ── Markdown ──────────────────────────────────────────────────────────────

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "# ForskScope File Comparison Report\n");
        let _ = writeln!(s, "## Summary\n");
        let _ = writeln!(s, "| Field | Value |");
        let _ = writeln!(s, "|---|---|");
        let _ = writeln!(s, "| Left  | `{}` |", self.left_display);
        let _ = writeln!(s, "| Right | `{}` |", self.right_display);
        let status = if self.is_identical { "identical" } else { "different" };
        let _ = writeln!(s, "| Status | {status} |");
        let _ = writeln!(s, "| Changed hunks | {} |", self.hunks_changed);
        let _ = writeln!(s, "| Added lines   | {} |", self.lines_added);
        let _ = writeln!(s, "| Deleted lines | {} |", self.lines_deleted);
        let _ = writeln!(s);

        if !self.options_summary.is_empty() {
            let _ = writeln!(s, "## Compare Options\n");
            for (k, v) in &self.options_summary {
                let _ = writeln!(s, "- **{k}:** {v}");
            }
            let _ = writeln!(s);
        }

        if !self.warnings.is_empty() {
            let _ = writeln!(s, "## Warnings\n");
            for w in &self.warnings { let _ = writeln!(s, "- {w}"); }
            let _ = writeln!(s);
        }

        if !self.hunk_rows.is_empty() {
            let _ = writeln!(s, "## Changed Hunks\n");
            let _ = writeln!(s, "| # | Left lines | Right lines | Type |");
            let _ = writeln!(s, "|---:|---|---|---|");
            for h in &self.hunk_rows {
                let _ = writeln!(s, "| {} | {}-{} | {}-{} | {} |",
                    h.hunk_num, h.left_start, h.left_end,
                    h.right_start, h.right_end, h.kind);
            }
            let _ = writeln!(s);
        }

        if !self.history.is_empty() {
            let _ = writeln!(s, "## Operation History\n");
            for e in &self.history {
                let _ = writeln!(s, "- `{}` {}", e.timestamp_sec, e.label);
            }
            let _ = writeln!(s);
        }

        s
    }

    // ── JSON ──────────────────────────────────────────────────────────────────

    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"schema_version\": 1,");
        let _ = writeln!(s, "  \"app_version\": {:?},", env!("CARGO_PKG_VERSION"));
        let _ = writeln!(s, "  \"kind\": \"file_comparison\",");
        let _ = writeln!(s, "  \"summary\": {{");
        let _ = writeln!(s, "    \"left\": {:?},",  self.left_display);
        let _ = writeln!(s, "    \"right\": {:?},", self.right_display);
        let _ = writeln!(s, "    \"identical\": {},",    self.is_identical);
        let _ = writeln!(s, "    \"hunks_changed\": {},", self.hunks_changed);
        let _ = writeln!(s, "    \"lines_added\": {},",   self.lines_added);
        let _ = writeln!(s, "    \"lines_deleted\": {}",  self.lines_deleted);
        let _ = writeln!(s, "  }},");

        // options
        let _ = write!(s, "  \"options\": {{");
        for (i, (k, v)) in self.options_summary.iter().enumerate() {
            let comma = if i + 1 < self.options_summary.len() { "," } else { "" };
            let _ = write!(s, " {:?}: {:?}{}", k, v, comma);
        }
        let _ = writeln!(s, " }},");

        // warnings
        let _ = write!(s, "  \"warnings\": [");
        for (i, w) in self.warnings.iter().enumerate() {
            let comma = if i + 1 < self.warnings.len() { ", " } else { "" };
            let _ = write!(s, "{:?}{}", w, comma);
        }
        let _ = writeln!(s, "],");

        // hunks
        let _ = writeln!(s, "  \"hunks\": [");
        for (i, h) in self.hunk_rows.iter().enumerate() {
            let comma = if i + 1 < self.hunk_rows.len() { "," } else { "" };
            let _ = writeln!(s, "    {{ \"num\": {}, \"left\": [{},{}], \"right\": [{},{}], \"kind\": {:?} }}{}",
                h.hunk_num, h.left_start, h.left_end,
                h.right_start, h.right_end, h.kind, comma);
        }
        let _ = writeln!(s, "  ],");

        // history
        let _ = writeln!(s, "  \"history\": [");
        for (i, e) in self.history.iter().enumerate() {
            let comma = if i + 1 < self.history.len() { "," } else { "" };
            let _ = writeln!(s, "    {{ \"ts\": {}, \"label\": {:?} }}{}",
                e.timestamp_sec, e.label, comma);
        }
        let _ = writeln!(s, "  ]");
        let _ = write!(s, "}}");
        s
    }
}

// ── Directory comparison report ───────────────────────────────────────────────

/// A rendered report for a directory comparison.
#[derive(Debug, Clone)]
pub struct DirComparisonReport {
    pub left_display:  String,
    pub right_display: String,
    pub total:         usize,
    pub equal:         usize,
    pub changed:       usize,
    pub left_only:     usize,
    pub right_only:    usize,
    pub symlinks:      usize,
    pub file_rows:     Vec<DirFileRow>,
    pub batch_summary: Option<BatchSummary>,
    pub options:       ReportOptions,
}

/// One file row in the directory report.
#[derive(Debug, Clone)]
pub struct DirFileRow {
    pub path:        String,
    pub status:      String,
    pub left_size:   Option<u64>,
    pub right_size:  Option<u64>,
}

/// Summary of a batch copy operation (from `BatchManifest`).
#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub operation_id: String,
    pub succeeded:    usize,
    pub failed:       usize,
}

impl DirComparisonReport {
    /// Build a directory comparison report.
    ///
    /// `left_root` / `right_root` are used for path display when
    /// `path_mode` is `Relative`. Pass `None` to fall back to `NameOnly`.
    pub fn from_entries(
        entries:    &[RecEntry],
        left_root:  Option<&Path>,
        right_root: Option<&Path>,
        manifest:   Option<&BatchManifest>,
        opts:       ReportOptions,
    ) -> Self {
        let left_display  = display_path(left_root,  &opts.path_mode, None);
        let right_display = display_path(right_root, &opts.path_mode, None);

        let mut equal = 0usize; let mut changed = 0usize;
        let mut left_only = 0usize; let mut right_only = 0usize;
        let mut symlinks = 0usize;

        let file_rows: Vec<DirFileRow> = entries.iter().map(|e| {
            let status = match e.status {
                RecStatus::Equal     => { equal     += 1; "equal".into() }
                RecStatus::Changed   => { changed   += 1; "modified".into() }
                RecStatus::LeftOnly  => { left_only += 1; "left only".into() }
                RecStatus::RightOnly => { right_only += 1; "right only".into() }
                RecStatus::Symlink   => { symlinks  += 1; "symlink".into() }
                RecStatus::Computing => "computing".into(),
            };
            let path = match opts.path_mode {
                ReportPathMode::NameOnly => e.rel_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| e.rel_path.display().to_string()),
                _ => e.rel_path.display().to_string(),
            };
            DirFileRow { path, status, left_size: e.left_size, right_size: e.right_size }
        }).collect();

        let batch_summary = manifest.map(|m| BatchSummary {
            operation_id: m.operation_id.to_string(),
            succeeded:    m.succeeded(),
            failed:       m.failed(),
        });

        let total = entries.len();
        Self {
            left_display, right_display, total, equal, changed,
            left_only, right_only, symlinks, file_rows, batch_summary, options: opts,
        }
    }

    // ── Markdown ──────────────────────────────────────────────────────────────

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "# ForskScope Directory Comparison Report\n");
        let _ = writeln!(s, "## Summary\n");
        let _ = writeln!(s, "| Field | Count |");
        let _ = writeln!(s, "|---|---:|");
        let _ = writeln!(s, "| Left  | `{}` |", self.left_display);
        let _ = writeln!(s, "| Right | `{}` |", self.right_display);
        let _ = writeln!(s, "| Total entries | {} |", self.total);
        let _ = writeln!(s, "| Equal         | {} |", self.equal);
        let _ = writeln!(s, "| Modified      | {} |", self.changed);
        let _ = writeln!(s, "| Left only     | {} |", self.left_only);
        let _ = writeln!(s, "| Right only    | {} |", self.right_only);
        if self.symlinks > 0 {
            let _ = writeln!(s, "| Symlinks      | {} |", self.symlinks);
        }
        let _ = writeln!(s);

        // Only include non-equal rows in the report by default.
        let interesting: Vec<&DirFileRow> = self.file_rows.iter()
            .filter(|r| r.status != "equal")
            .collect();

        if !interesting.is_empty() {
            let _ = writeln!(s, "## Changed Files\n");
            if self.options.include_sizes {
                let _ = writeln!(s, "| Path | Status | Left size | Right size |");
                let _ = writeln!(s, "|---|---|---:|---:|");
                for r in &interesting {
                    let ls = r.left_size.map(fmt_size).unwrap_or_else(|| "—".into());
                    let rs = r.right_size.map(fmt_size).unwrap_or_else(|| "—".into());
                    let _ = writeln!(s, "| `{}` | {} | {} | {} |", r.path, r.status, ls, rs);
                }
            } else {
                let _ = writeln!(s, "| Path | Status |");
                let _ = writeln!(s, "|---|---|");
                for r in &interesting {
                    let _ = writeln!(s, "| `{}` | {} |", r.path, r.status);
                }
            }
            let _ = writeln!(s);
        }

        if let Some(bs) = &self.batch_summary {
            let _ = writeln!(s, "## Batch Operation\n");
            let _ = writeln!(s, "- Operation ID: `{}`", bs.operation_id);
            let _ = writeln!(s, "- Succeeded: {}", bs.succeeded);
            let _ = writeln!(s, "- Failed: {}", bs.failed);
            let _ = writeln!(s);
        }

        s
    }

    // ── JSON ──────────────────────────────────────────────────────────────────

    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"schema_version\": 1,");
        let _ = writeln!(s, "  \"app_version\": {:?},", env!("CARGO_PKG_VERSION"));
        let _ = writeln!(s, "  \"kind\": \"directory_comparison\",");
        let _ = writeln!(s, "  \"summary\": {{");
        let _ = writeln!(s, "    \"left\": {:?},",      self.left_display);
        let _ = writeln!(s, "    \"right\": {:?},",     self.right_display);
        let _ = writeln!(s, "    \"total\": {},",        self.total);
        let _ = writeln!(s, "    \"equal\": {},",        self.equal);
        let _ = writeln!(s, "    \"changed\": {},",      self.changed);
        let _ = writeln!(s, "    \"left_only\": {},",    self.left_only);
        let _ = writeln!(s, "    \"right_only\": {},",   self.right_only);
        let _ = writeln!(s, "    \"symlinks\": {}",      self.symlinks);
        let _ = writeln!(s, "  }},");

        // changed files
        let _ = writeln!(s, "  \"files\": [");
        let interesting: Vec<&DirFileRow> = self.file_rows.iter()
            .filter(|r| r.status != "equal")
            .collect();
        for (i, r) in interesting.iter().enumerate() {
            let comma = if i + 1 < interesting.len() { "," } else { "" };
            let ls = r.left_size.map(|n| n.to_string()).unwrap_or_else(|| "null".into());
            let rs = r.right_size.map(|n| n.to_string()).unwrap_or_else(|| "null".into());
            let _ = writeln!(s, "    {{ \"path\": {:?}, \"status\": {:?}, \"left_bytes\": {}, \"right_bytes\": {} }}{}",
                r.path, r.status, ls, rs, comma);
        }
        let _ = writeln!(s, "  ]");

        if let Some(bs) = &self.batch_summary {
            let _ = writeln!(s, "  ,\"batch\": {{");
            let _ = writeln!(s, "    \"operation_id\": {:?},", bs.operation_id);
            let _ = writeln!(s, "    \"succeeded\": {},", bs.succeeded);
            let _ = writeln!(s, "    \"failed\": {}", bs.failed);
            let _ = writeln!(s, "  }}");
        }

        let _ = write!(s, "}}");
        s
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn display_path(path: Option<&Path>, mode: &ReportPathMode, root: Option<&Path>) -> String {
    match path {
        None => "(unknown)".into(),
        Some(p) => match mode {
            ReportPathMode::NameOnly => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            ReportPathMode::Relative => {
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
            ReportPathMode::Absolute => p.display().to_string(),
        },
    }
}

fn hunk_kind_label(kind: HunkKind) -> String {
    match kind {
        HunkKind::Equal   => "equal".into(),
        HunkKind::Insert  => "insert".into(),
        HunkKind::Delete  => "delete".into(),
        HunkKind::Replace => "replace".into(),
    }
}

fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    }
}
