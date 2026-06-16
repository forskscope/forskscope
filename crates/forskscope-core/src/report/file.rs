//! File-level comparison report: `FileComparisonReport`, `HunkSummaryRow` (RFC-006).

use std::fmt::Write as _;
use std::path::Path;
use crate::diff::{DiffDocument, DiffWarning, HunkKind};
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
        let left_display  = super::display_path(left_path,  &opts.path_mode, None);
        let right_display = super::display_path(right_path, &opts.path_mode, None);

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
                    kind:        super::hunk_kind_label(h.kind),
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

