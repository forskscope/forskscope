//! Directory-level comparison report: `DirComparisonReport` (RFC-008, RFC-092).

use super::FileChecksum;
use super::change_hunks_from_diff;
use super::changes::ChangeHunk;
use super::file::{ReportOptions, ReportPathMode, non_default_options_summary};
use forskscope_compare::CancellationToken;
use forskscope_compare::diff::DiffOptions;
use forskscope_compare::dir::{RecEntry, RecStatus};
use std::fmt::Write as _;
use std::path::Path;

// ── Directory comparison report ───────────────────────────────────────────────

/// A rendered report for a directory comparison.
#[derive(Debug, Clone)]
pub struct DirComparisonReport {
    pub left_display: String,
    pub right_display: String,
    pub total: usize,
    pub equal: usize,
    pub changed: usize,
    pub left_only: usize,
    pub right_only: usize,
    pub symlinks: usize,
    /// Entries that could not be read (F79) - not a verdict, counted
    /// separately from `changed`/`equal`/etc.
    pub unreadable: usize,
    pub file_rows: Vec<DirFileRow>,
    pub options: ReportOptions,
    /// Compare options that differ from `DiffOptions::default()` (RFC-092
    /// §3 item 3), from the `diff_options` passed to `from_entries` — empty
    /// when the caller passed `None` (no single comparison settings apply,
    /// e.g. a listing with no digest pass yet).
    pub non_default_options: Vec<(String, String)>,
    /// The compare profile's name — see `ReportOptions::profile_name`'s doc
    /// comment for the "Default"/"Custom" fallback. `None` when
    /// `diff_options` was `None`.
    pub profile_name: Option<String>,
    /// Changed text files whose change hunks were left out by the HTML
    /// report's cap (RFC-092 §3a: "50 files or 5 MiB of change text,
    /// whichever comes first"), by path, in the order they were left out.
    /// Populated by the caller (the export job, which decides how many
    /// files it has budget for) — empty means the cap did not apply.
    /// Never omits silently: a non-empty list is itself the report's only
    /// record of what got left out.
    pub change_hunks_omitted: Vec<String>,
}

/// One file row in the directory report.
#[derive(Debug, Clone)]
pub struct DirFileRow {
    pub path: String,
    /// The machine word `to_markdown`/`to_json` render unchanged (RFC-092
    /// §6: out of UI scope, kept as-is — review 174 §3). The HTML renderer
    /// reads `rec_status` instead, through the same `status_key` Deep
    /// Compare's own live view reads, so the two can't drift apart.
    pub status: String,
    /// The raw status this row's `status`/HTML-report word were both
    /// derived from (review 174 §3).
    pub rec_status: RecStatus,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
    /// SHA-256 of each side's bytes as read at export time (RFC-092 §5) —
    /// `None` until the caller attaches them (checksumming is file I/O,
    /// which `from_entries` itself never performs). `None` also for a side
    /// where the file does not exist (one-sided entries).
    pub left_checksum: Option<FileChecksum>,
    pub right_checksum: Option<FileChecksum>,
    /// Content-carrying change hunks for a changed text file, attached by
    /// the caller once it has read and diffed this file — `None` for every
    /// status other than "modified", and for a modified file the cap left
    /// out (see `DirComparisonReport::change_hunks_omitted`).
    pub change_hunks: Option<Vec<ChangeHunk>>,
}

impl DirComparisonReport {
    /// Build a directory comparison report.
    ///
    /// `left_root` / `right_root` are used for path display when
    /// `path_mode` is `Relative`. Pass `None` to fall back to `NameOnly`.
    pub fn from_entries(
        entries: &[RecEntry],
        left_root: Option<&Path>,
        right_root: Option<&Path>,
        diff_options: Option<&DiffOptions>,
        opts: ReportOptions,
    ) -> Self {
        let left_display = super::display_path(left_root, &opts.path_mode, None);
        let right_display = super::display_path(right_root, &opts.path_mode, None);

        let mut equal = 0usize;
        let mut changed = 0usize;
        let mut left_only = 0usize;
        let mut right_only = 0usize;
        let mut symlinks = 0usize;
        let mut unreadable = 0usize;

        let file_rows: Vec<DirFileRow> = entries
            .iter()
            .map(|e| {
                let status = match e.status {
                    RecStatus::Equal => {
                        equal += 1;
                        "equal".into()
                    }
                    RecStatus::Changed => {
                        changed += 1;
                        "modified".into()
                    }
                    RecStatus::LeftOnly => {
                        left_only += 1;
                        "left only".into()
                    }
                    RecStatus::RightOnly => {
                        right_only += 1;
                        "right only".into()
                    }
                    // F135: an empty directory present on one side only -
                    // counted alongside the ordinary one-sided files
                    // (it genuinely is one-sided), with its own status
                    // text so the row does not claim a file was found.
                    RecStatus::LeftOnlyDir => {
                        left_only += 1;
                        "left only (empty directory)".into()
                    }
                    RecStatus::RightOnlyDir => {
                        right_only += 1;
                        "right only (empty directory)".into()
                    }
                    RecStatus::Symlink => {
                        symlinks += 1;
                        "symlink".into()
                    }
                    RecStatus::Unreadable => {
                        unreadable += 1;
                        "unreadable".into()
                    }
                    // F77/RFC-092 §0: this status survives only from a scan
                    // that was cancelled mid-flight — a *finished* report
                    // can never legitimately show "computing" (something
                    // still in progress), so this is the report-only label
                    // for "its comparison never completed", distinct from
                    // deep_compare.rs's own live "Comparing…" label for the
                    // same `RecStatus::Computing` while a scan is running.
                    RecStatus::Computing => "not compared".into(),
                };
                let path = match opts.path_mode {
                    ReportPathMode::NameOnly => e
                        .rel_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| e.rel_path.display().to_string()),
                    _ => e.rel_path.display().to_string(),
                };
                DirFileRow {
                    path,
                    status,
                    rec_status: e.status,
                    left_size: e.left_size,
                    right_size: e.right_size,
                    left_checksum: None,
                    right_checksum: None,
                    change_hunks: None,
                }
            })
            .collect();

        let non_default_options = diff_options
            .map(non_default_options_summary)
            .unwrap_or_default();
        let profile_name = diff_options.map(|_| {
            opts.profile_name.clone().unwrap_or_else(|| {
                if non_default_options.is_empty() {
                    "Default".into()
                } else {
                    "Custom".into()
                }
            })
        });

        let total = entries.len();
        Self {
            left_display,
            right_display,
            total,
            equal,
            changed,
            left_only,
            right_only,
            symlinks,
            unreadable,
            file_rows,
            non_default_options,
            profile_name,
            change_hunks_omitted: Vec::new(),
            options: opts,
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
        if self.unreadable > 0 {
            let _ = writeln!(s, "| Unreadable    | {} |", self.unreadable);
        }
        let _ = writeln!(s);

        // Only include non-equal rows in the report by default.
        let interesting: Vec<&DirFileRow> = self
            .file_rows
            .iter()
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
        let _ = writeln!(s, "    \"left\": {:?},", self.left_display);
        let _ = writeln!(s, "    \"right\": {:?},", self.right_display);
        let _ = writeln!(s, "    \"total\": {},", self.total);
        let _ = writeln!(s, "    \"equal\": {},", self.equal);
        let _ = writeln!(s, "    \"changed\": {},", self.changed);
        let _ = writeln!(s, "    \"left_only\": {},", self.left_only);
        let _ = writeln!(s, "    \"right_only\": {},", self.right_only);
        let _ = writeln!(s, "    \"symlinks\": {},", self.symlinks);
        let _ = writeln!(s, "    \"unreadable\": {}", self.unreadable);
        let _ = writeln!(s, "  }},");

        // changed files
        let _ = writeln!(s, "  \"files\": [");
        let interesting: Vec<&DirFileRow> = self
            .file_rows
            .iter()
            .filter(|r| r.status != "equal")
            .collect();
        for (i, r) in interesting.iter().enumerate() {
            let comma = if i + 1 < interesting.len() { "," } else { "" };
            let ls = r
                .left_size
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".into());
            let rs = r
                .right_size
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".into());
            let _ = writeln!(
                s,
                "    {{ \"path\": {:?}, \"status\": {:?}, \"left_bytes\": {}, \"right_bytes\": {} }}{}",
                r.path, r.status, ls, rs, comma
            );
        }
        let _ = writeln!(s, "  ]");

        let _ = write!(s, "}}");
        s
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Shared with `report::html` (review 175 §3) so the folder report's
/// per-row sizes carry the same unit the file report's checksum row does.
pub(super) fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1_048_576 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    }
}

// ── Building a report from disk (RFC-092 §5, moved from forskscope-ui by
// RFC-093 §1 step 6) ────────────────────────────────────────────────────────

/// RFC-092 §3a: "50 files or 5 MiB of change text, whichever comes first."
const MAX_CHANGED_FILES_WITH_CONTENT: usize = 50;
const MAX_CHANGE_TEXT_BYTES: usize = 5 * 1024 * 1024;

/// What building the report produced.
pub enum BuildOutcome {
    Report(Box<DirComparisonReport>),
    /// `token` was cancelled before finishing — nothing partial is ever
    /// handed back to be written.
    Cancelled,
    /// RFC-092 §0: a file read at export time no longer agrees with what
    /// the scan found — an entry listed `Equal` whose two hashes now
    /// differ, or listed `Changed` whose two hashes now match. The whole
    /// export refuses, naming the one path that triggered it.
    Stale(String),
}

/// Reads, hashes and (for changed text files, under the cap) diffs every
/// entry. The caller (the GUI) runs this off its own UI thread; this
/// function itself does no threading and writes no file — it only reads
/// the compared files and returns the assembled report for the caller to
/// render and write.
pub fn build_dir_report(
    left_root: &Path,
    right_root: &Path,
    entries: Vec<RecEntry>,
    diff_options: DiffOptions,
    report_options: ReportOptions,
    token: &CancellationToken,
    on_progress: &mut dyn FnMut(usize, usize),
) -> BuildOutcome {
    let total = entries.len();
    let mut report = DirComparisonReport::from_entries(
        &entries,
        Some(left_root),
        Some(right_root),
        Some(&diff_options),
        report_options,
    );

    let context_lines = report.options.context_lines;
    let mut change_text_bytes = 0usize;
    let mut files_with_content = 0usize;

    for (i, entry) in entries.iter().enumerate() {
        if token.is_cancelled() {
            return BuildOutcome::Cancelled;
        }
        on_progress(i, total);

        let Some(row) = report.file_rows.get_mut(i) else {
            continue;
        };

        let left_path = left_root.join(&entry.rel_path);
        let right_path = right_root.join(&entry.rel_path);

        let left_bytes = matches!(
            entry.status,
            RecStatus::Equal | RecStatus::Changed | RecStatus::LeftOnly
        )
        .then(|| std::fs::read(&left_path).ok())
        .flatten();
        let right_bytes = matches!(
            entry.status,
            RecStatus::Equal | RecStatus::Changed | RecStatus::RightOnly
        )
        .then(|| std::fs::read(&right_path).ok())
        .flatten();

        if let (Some(l), Some(r)) = (&left_bytes, &right_bytes) {
            let now_equal = l == r;
            let was_equal = entry.status == RecStatus::Equal;
            if now_equal != was_equal {
                return BuildOutcome::Stale(row.path.clone());
            }
        }

        if let Some(bytes) = &left_bytes {
            row.left_checksum = Some(FileChecksum::of_bytes(bytes));
        }
        if let Some(bytes) = &right_bytes {
            row.right_checksum = Some(FileChecksum::of_bytes(bytes));
        }

        if entry.status == RecStatus::Changed
            && let (Some(left_bytes), Some(right_bytes)) = (&left_bytes, &right_bytes)
        {
            // Binary/spreadsheet entries show status only (RFC-092 §3a) -
            // never added to `change_hunks_omitted`, which names only what
            // the *cap* left out; a binary file was never going to be
            // diffed regardless of the cap, so listing it there would
            // misattribute why its content is absent.
            let is_text = matches!(
                forskscope_compare::file_kind::classify(&left_path),
                Ok(forskscope_compare::FileKind::Text)
            ) && matches!(
                forskscope_compare::file_kind::classify(&right_path),
                Ok(forskscope_compare::FileKind::Text)
            );
            if is_text {
                if files_with_content < MAX_CHANGED_FILES_WITH_CONTENT
                    && change_text_bytes < MAX_CHANGE_TEXT_BYTES
                {
                    if let (Ok(left_text), Ok(right_text)) = (
                        String::from_utf8(left_bytes.clone()),
                        String::from_utf8(right_bytes.clone()),
                    ) {
                        let diff =
                            forskscope_compare::compute_diff(&left_text, &right_text, diff_options);
                        let hunks = change_hunks_from_diff(&diff, context_lines);
                        let hunk_text_len: usize = hunks
                            .iter()
                            .flat_map(|h: &ChangeHunk| h.lines.iter())
                            .map(|l| l.content.len())
                            .sum();
                        change_text_bytes += hunk_text_len;
                        files_with_content += 1;
                        row.change_hunks = Some(hunks);
                    }
                    // `classify` sampled a prefix; a full-file UTF-8 decode
                    // failure here is rare and not a cap omission either -
                    // left with no change_hunks, same as a binary file.
                } else {
                    report.change_hunks_omitted.push(row.path.clone());
                }
            }
        }
    }
    on_progress(total, total);
    BuildOutcome::Report(Box::new(report))
}
