//! File-level comparison report: `FileComparisonReport`, `HunkSummaryRow` (RFC-006, RFC-092).

use super::FileChecksum;
use super::changes::{ChangeHunk, change_hunks_from_diff};
use crate::diff::{DiffDocument, DiffOptions, DiffWarning, HunkKind};
use std::fmt::Write as _;
use std::path::Path;

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
    pub path_mode: ReportPathMode,
    /// Include hunk-level detail (line ranges, types).
    pub include_hunks: bool,
    /// Include compare option settings.
    pub include_options: bool,
    /// Include diff warnings (large file, deadline, etc.).
    pub include_warnings: bool,
    /// Include per-file size columns in directory reports.
    pub include_sizes: bool,
    /// Unified-layout context lines kept around each change, for the
    /// HTML report's change hunks (RFC-092 §3 item 5). Matches
    /// `PatchOptions`' own default so the HTML report and the unified-diff
    /// export agree on what "the changes" means.
    pub context_lines: usize,
    /// The named compare profile the diff was run under, if the caller
    /// knows one (RFC-092 §3 item 3: "How: the compare profile's name").
    /// `None` falls back to "Default" when every option is at its default,
    /// or "Custom" otherwise — `from_diff` decides which once it has
    /// compared the actual options.
    pub profile_name: Option<String>,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            path_mode: ReportPathMode::NameOnly,
            include_hunks: true,
            include_options: true,
            include_warnings: true,
            include_sizes: true,
            context_lines: 3,
            profile_name: None,
        }
    }
}

// ── File comparison report ────────────────────────────────────────────────────

/// A rendered report for a single file comparison.
#[derive(Debug, Clone)]
pub struct FileComparisonReport {
    /// Display name for the left/old side.
    pub left_display: String,
    /// Display name for the right/new side.
    pub right_display: String,
    pub is_identical: bool,
    pub hunks_total: usize,
    pub hunks_changed: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
    pub warnings: Vec<String>,
    pub hunk_rows: Vec<HunkSummaryRow>,
    pub options_summary: Vec<(String, String)>,
    pub options: ReportOptions,
    /// Content-carrying, windowed change hunks for the HTML report (RFC-092
    /// §3 item 5) — `hunk_rows` stays range-only for `to_markdown`/`to_json`,
    /// which are unaffected by RFC-092 (§6: "stay as they are").
    pub change_hunks: Vec<ChangeHunk>,
    /// Compare options that differ from `DiffOptions::default()` — RFC-092
    /// §3 item 3: listed "only where they differ from the defaults",
    /// separate from `options_summary` (which `to_markdown`/`to_json` keep
    /// showing unconditionally, unchanged).
    pub non_default_options: Vec<(String, String)>,
    /// The compare profile's name, resolved from `ReportOptions::profile_name`
    /// (see its doc comment for the "Default"/"Custom" fallback).
    pub profile_name: String,
    pub left_checksum: Option<FileChecksum>,
    pub right_checksum: Option<FileChecksum>,
}

/// One row in the hunk summary table.
#[derive(Debug, Clone)]
pub struct HunkSummaryRow {
    pub hunk_num: usize,
    pub left_start: u32,
    pub left_end: u32,
    pub right_start: u32,
    pub right_end: u32,
    pub kind: String,
}

/// Compare options that differ from `DiffOptions::default()`, in display
/// order (RFC-092 §3 item 3). Shared with `DirComparisonReport`, since a
/// folder comparison's "How" section describes the same `DiffOptions`.
pub(crate) fn non_default_options_summary(o: &DiffOptions) -> Vec<(String, String)> {
    let d = DiffOptions::default();
    let mut v = Vec::new();
    if o.ignore_whitespace != d.ignore_whitespace {
        v.push(("Whitespace".into(), "ignored".into()));
    }
    if o.ignore_case != d.ignore_case {
        v.push(("Case".into(), "ignored".into()));
    }
    if o.ignore_newlines != d.ignore_newlines {
        v.push(("Newlines".into(), "ignored".into()));
    }
    if o.algorithm != d.algorithm {
        v.push(("Algorithm".into(), format!("{:?}", o.algorithm)));
    }
    v
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

impl FileComparisonReport {
    /// Build a report from a `DiffDocument`, optional file paths for
    /// display, and rendering options.
    pub fn from_diff(
        diff: &DiffDocument,
        left_path: Option<&Path>,
        right_path: Option<&Path>,
        opts: ReportOptions,
    ) -> Self {
        let left_display = super::display_path(left_path, &opts.path_mode, None);
        let right_display = super::display_path(right_path, &opts.path_mode, None);

        let warnings = diff
            .warnings
            .iter()
            .map(|w| match w {
                DiffWarning::LargeFilePolicyApplied => "Large file — inline diff disabled.".into(),
                DiffWarning::DeadlineExpired => {
                    "Diff timed out — result may be approximate.".into()
                }
                DiffWarning::InlineSkippedHunkTooLarge => {
                    "Some hunks were too large for inline diff.".into()
                }
            })
            .collect();

        let hunk_rows: Vec<HunkSummaryRow> = if opts.include_hunks {
            diff.hunks
                .iter()
                .enumerate()
                .filter(|(_, h)| h.kind != HunkKind::Equal)
                .map(|(i, h)| HunkSummaryRow {
                    hunk_num: i + 1,
                    left_start: h.left_range.start,
                    left_end: h.left_range.start + h.left_range.len,
                    right_start: h.right_range.start,
                    right_end: h.right_range.start + h.right_range.len,
                    kind: super::hunk_kind_label(h.kind),
                })
                .collect()
        } else {
            vec![]
        };

        let options_summary = if opts.include_options {
            let o = &diff.options;
            vec![
                (
                    "Whitespace".into(),
                    if o.ignore_whitespace {
                        "ignored".into()
                    } else {
                        "significant".into()
                    },
                ),
                (
                    "Case".into(),
                    if o.ignore_case {
                        "ignored".into()
                    } else {
                        "significant".into()
                    },
                ),
                ("Algorithm".into(), format!("{:?}", o.algorithm)),
            ]
        } else {
            vec![]
        };

        let non_default_options = non_default_options_summary(&diff.options);
        let profile_name = opts.profile_name.clone().unwrap_or_else(|| {
            if non_default_options.is_empty() {
                "Default".into()
            } else {
                "Custom".into()
            }
        });
        let change_hunks = change_hunks_from_diff(diff, opts.context_lines);

        Self {
            left_display,
            right_display,
            is_identical: diff.is_identical(),
            hunks_total: diff.stats.hunks_total,
            hunks_changed: diff.stats.hunks_changed,
            lines_added: diff.stats.lines_inserted,
            lines_deleted: diff.stats.lines_deleted,
            warnings,
            hunk_rows,
            options_summary,
            change_hunks,
            non_default_options,
            profile_name,
            left_checksum: None,
            right_checksum: None,
            options: opts,
        }
    }

    /// Attach checksums computed from the exact bytes read at export time
    /// (RFC-092 §5). Not part of `from_diff` itself, since checksumming
    /// requires file I/O the diff computation never performs.
    pub fn with_checksums(
        mut self,
        left: Option<FileChecksum>,
        right: Option<FileChecksum>,
    ) -> Self {
        self.left_checksum = left;
        self.right_checksum = right;
        self
    }

    /// Record a note the caller knows but the diff itself does not carry
    /// (RFC-092 §3: "an encoding that is not UTF-8", or anything else the
    /// comparison could not do) — rendered alongside `warnings` in the
    /// HTML report's Notes section.
    pub fn push_note(&mut self, note: impl Into<String>) {
        self.warnings.push(note.into());
    }

    /// One sentence summarizing the change, for example "3 changes: 2 lines
    /// removed, 5 lines added, 1 hunk replaced." (RFC-092 §3 item 4).
    pub fn one_sentence_summary(&self) -> String {
        if self.is_identical {
            return "No differences — the two files are identical.".into();
        }
        // Review 174 §4: lines only, never hunks — a count of *hunks*
        // alongside counts of *lines* reads as more change stacked on top
        // of the lines already counted, and the change list below already
        // shows which hunks are replacements.
        let mut clauses = Vec::new();
        if self.lines_deleted > 0 {
            clauses.push(format!(
                "{} line{} removed",
                self.lines_deleted,
                plural(self.lines_deleted)
            ));
        }
        if self.lines_added > 0 {
            clauses.push(format!(
                "{} line{} added",
                self.lines_added,
                plural(self.lines_added)
            ));
        }
        let lead = format!(
            "{} change{}",
            self.hunks_changed,
            plural(self.hunks_changed)
        );
        if clauses.is_empty() {
            format!("{lead}.")
        } else {
            format!("{lead}: {}.", clauses.join(", "))
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
        let status = if self.is_identical {
            "identical"
        } else {
            "different"
        };
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
            for w in &self.warnings {
                let _ = writeln!(s, "- {w}");
            }
            let _ = writeln!(s);
        }

        if !self.hunk_rows.is_empty() {
            let _ = writeln!(s, "## Changed Hunks\n");
            let _ = writeln!(s, "| # | Left lines | Right lines | Type |");
            let _ = writeln!(s, "|---:|---|---|---|");
            for h in &self.hunk_rows {
                let _ = writeln!(
                    s,
                    "| {} | {}-{} | {}-{} | {} |",
                    h.hunk_num, h.left_start, h.left_end, h.right_start, h.right_end, h.kind
                );
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
        let _ = writeln!(s, "    \"left\": {:?},", self.left_display);
        let _ = writeln!(s, "    \"right\": {:?},", self.right_display);
        let _ = writeln!(s, "    \"identical\": {},", self.is_identical);
        let _ = writeln!(s, "    \"hunks_changed\": {},", self.hunks_changed);
        let _ = writeln!(s, "    \"lines_added\": {},", self.lines_added);
        let _ = writeln!(s, "    \"lines_deleted\": {}", self.lines_deleted);
        let _ = writeln!(s, "  }},");

        // options
        let _ = write!(s, "  \"options\": {{");
        for (i, (k, v)) in self.options_summary.iter().enumerate() {
            let comma = if i + 1 < self.options_summary.len() {
                ","
            } else {
                ""
            };
            let _ = write!(s, " {:?}: {:?}{}", k, v, comma);
        }
        let _ = writeln!(s, " }},");

        // warnings
        let _ = write!(s, "  \"warnings\": [");
        for (i, w) in self.warnings.iter().enumerate() {
            let comma = if i + 1 < self.warnings.len() {
                ", "
            } else {
                ""
            };
            let _ = write!(s, "{:?}{}", w, comma);
        }
        let _ = writeln!(s, "],");

        // hunks
        let _ = writeln!(s, "  \"hunks\": [");
        for (i, h) in self.hunk_rows.iter().enumerate() {
            let comma = if i + 1 < self.hunk_rows.len() {
                ","
            } else {
                ""
            };
            let _ = writeln!(
                s,
                "    {{ \"num\": {}, \"left\": [{},{}], \"right\": [{},{}], \"kind\": {:?} }}{}",
                h.hunk_num, h.left_start, h.left_end, h.right_start, h.right_end, h.kind, comma
            );
        }
        let _ = writeln!(s, "  ]");
        let _ = write!(s, "}}");
        s
    }
}
