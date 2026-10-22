//! File-level comparison report: `FileComparisonReport` (RFC-006, RFC-092).

use super::FileChecksum;
use super::changes::{ChangeHunk, change_hunks_from_diff};
use forskscope_compare::diff::{DiffDocument, DiffOptions, DiffWarning};
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

/// RFC-092 §3a: "50 files or 5 MiB of change text, whichever comes first" —
/// a directory report's own default caps, carried as two fields on
/// [`ReportOptions`] (below) rather than as two extra parameters on
/// `build_dir_report` (which would push that function over clippy's
/// `too_many_arguments` limit — the same reasoning
/// `build_file_report`/`checksum_if_fingerprinted`'s split already follows
/// for checksums). RFC-094 §3's `forskscope compare --max-files`/
/// `--max-change-bytes` set these two fields directly; the GUI
/// (`export_dir_report.rs`) never sets them, so it keeps these defaults
/// unchanged.
pub const DEFAULT_MAX_CHANGED_FILES_WITH_CONTENT: usize = 50;
pub const DEFAULT_MAX_CHANGE_TEXT_BYTES: usize = 5 * 1024 * 1024;

/// What optional sections to include in a report.
#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub path_mode: ReportPathMode,
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
    /// Directory reports only: the most changed text files that get full
    /// change-hunk content before `build_dir_report` starts listing the
    /// rest in `change_hunks_omitted` (RFC-092 §3a / RFC-094 §3).
    pub max_files_with_content: usize,
    /// Directory reports only: the most cumulative change-text bytes
    /// across those files before the same cap applies.
    pub max_change_bytes: usize,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            path_mode: ReportPathMode::NameOnly,
            include_warnings: true,
            include_sizes: true,
            context_lines: 3,
            profile_name: None,
            max_files_with_content: DEFAULT_MAX_CHANGED_FILES_WITH_CONTENT,
            max_change_bytes: DEFAULT_MAX_CHANGE_TEXT_BYTES,
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
    pub options: ReportOptions,
    /// Content-carrying, windowed change hunks for the HTML and plain-text
    /// reports (RFC-092 §3 item 5, review 182 §2).
    pub change_hunks: Vec<ChangeHunk>,
    /// Compare options that differ from `DiffOptions::default()` — RFC-092
    /// §3 item 3: listed "only where they differ from the defaults".
    pub non_default_options: Vec<(String, String)>,
    /// The compare profile's name, resolved from `ReportOptions::profile_name`
    /// (see its doc comment for the "Default"/"Custom" fallback).
    pub profile_name: String,
    pub left_checksum: Option<FileChecksum>,
    pub right_checksum: Option<FileChecksum>,
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
}

// ── Building a report from disk (RFC-092 §5, moved from forskscope-ui by
// RFC-093 §1 step 6) ────────────────────────────────────────────────────────

/// Review 174 §6: a checksum appears only where a reader can reproduce it
/// by running `sha256sum` on a real file — `has_fingerprint` is `false`
/// for exactly one case in this codebase today, `LoadedDocument::empty()`
/// ("one side is intentionally absent"), whose `raw_bytes` would otherwise
/// still produce a real-looking checksum (of zero bytes) that proves
/// nothing about any actual file.
pub fn checksum_if_fingerprinted(
    bytes: Option<&[u8]>,
    has_fingerprint: bool,
) -> Option<FileChecksum> {
    if has_fingerprint {
        bytes.map(FileChecksum::of_bytes)
    } else {
        None
    }
}

/// Assembles a file comparison report with its checksums attached - the
/// caller's own fingerprint/staleness check, the actual file I/O that
/// produced the bytes behind `left_checksum`/`right_checksum`
/// (`checksum_if_fingerprinted`, above), and that decision itself, all
/// happen before this is called; this function itself does no file I/O and
/// writes nothing. A thin `from_diff` + `with_checksums` combination, kept
/// as its own function so the GUI's export job has one call to make
/// instead of two.
pub fn build_file_report(
    diff: &DiffDocument,
    left_path: Option<&Path>,
    right_path: Option<&Path>,
    report_options: ReportOptions,
    left_checksum: Option<FileChecksum>,
    right_checksum: Option<FileChecksum>,
) -> FileComparisonReport {
    FileComparisonReport::from_diff(diff, left_path, right_path, report_options)
        .with_checksums(left_checksum, right_checksum)
}
