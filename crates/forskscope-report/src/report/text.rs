//! Plain-text rendering (review 182 §2) — built from the same report
//! model `html.rs` renders, following every rule the HTML already
//! follows: the app's own status words (never "Equal"/"Modified"), a
//! non-zero-only counts line, no option/profile section, and change
//! content shown as old/new line numbers with a `−`/`+` mark, never as
//! unexplained jargon ("hunk", a bare range).
//!
//! **English only.** `forskscope-cli`, the only caller, has no language
//! setting — see the user guide. This replaces the old `to_markdown`
//! (and `to_json`, superseded by `report::json`'s versioned contract),
//! removed in the same change that added this module: neither followed
//! RFC-092's own rules (zero-count noise, jargon, options shown even at
//! their defaults, and `to_markdown` never showed what changed at all).

use std::fmt::Write as _;

use super::changes::ChangeHunk;
use super::dir::DirComparisonReport;
use super::file::FileComparisonReport;
use super::html::{counts_line, mark_for};
use forskscope_compare::dir::{RecStatus, status_key};

/// `counts_line`'s translator parameter, with nothing to translate.
fn identity(s: &str) -> String {
    s.to_string()
}

fn render_hunk(out: &mut String, hunk: &ChangeHunk) {
    let _ = writeln!(
        out,
        "  @@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_len, hunk.new_start, hunk.new_len
    );
    for line in &hunk.lines {
        let left = line.left_line.map(|n| n.to_string()).unwrap_or_default();
        let right = line.right_line.map(|n| n.to_string()).unwrap_or_default();
        let _ = writeln!(
            out,
            "  {left:>5} {right:>5} {} {}",
            mark_for(line.kind),
            line.content
        );
    }
}

/// Plain-text rendering of a single-file comparison.
pub fn render_file_report_text(report: &FileComparisonReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "What was compared");
    let _ = writeln!(out, "  Left:  {}", report.left_display);
    let _ = writeln!(out, "  Right: {}", report.right_display);
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", report.one_sentence_summary());

    if !report.change_hunks.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "The changes");
        for hunk in &report.change_hunks {
            render_hunk(&mut out, hunk);
        }
    }

    if !report.warnings.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Notes");
        for w in &report.warnings {
            let _ = writeln!(out, "  - {w}");
        }
    }

    out
}

/// Plain-text rendering of a directory comparison.
pub fn render_dir_report_text(report: &DirComparisonReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "What was compared");
    let _ = writeln!(out, "  Left:  {}", report.left_display);
    let _ = writeln!(out, "  Right: {}", report.right_display);
    let _ = writeln!(out);

    let main_counts = [
        (status_key(RecStatus::Equal, false), report.equal),
        (status_key(RecStatus::Changed, false), report.changed),
        (status_key(RecStatus::LeftOnly, false), report.left_only),
        (status_key(RecStatus::RightOnly, false), report.right_only),
    ];
    let line = counts_line(&identity, &main_counts).unwrap_or_else(|| "No files.".into());
    let _ = writeln!(out, "{line}");

    let extra_counts = [
        (status_key(RecStatus::Symlink, false), report.symlinks),
        (status_key(RecStatus::Unreadable, false), report.unreadable),
    ];
    if let Some(line) = counts_line(&identity, &extra_counts) {
        let _ = writeln!(out, "{line}");
    }

    let interesting: Vec<_> = report
        .file_rows
        .iter()
        .filter(|r| r.rec_status != RecStatus::Equal)
        .collect();
    if !interesting.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Files");
        for row in &interesting {
            let _ = writeln!(
                out,
                "  {} — {}",
                row.path,
                status_key(row.rec_status, false)
            );
        }
    }

    let changed_with_hunks: Vec<_> = report
        .file_rows
        .iter()
        .filter(|r| r.change_hunks.as_ref().is_some_and(|h| !h.is_empty()))
        .collect();
    if !changed_with_hunks.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "The changes");
        for row in changed_with_hunks {
            let _ = writeln!(out, "{}:", row.path);
            for hunk in row.change_hunks.as_ref().unwrap() {
                render_hunk(&mut out, hunk);
            }
        }
    }

    if !report.change_hunks_omitted.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "Notes");
        let _ = writeln!(
            out,
            "  {} changed files' content is not shown in this report (the cap of 50 files or 5 MiB of change text was reached):",
            report.change_hunks_omitted.len()
        );
        for name in &report.change_hunks_omitted {
            let _ = writeln!(out, "    - {name}");
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_compare::diff::{DiffOptions, compute_diff};
    use forskscope_compare::dir::RecEntry;
    use std::path::PathBuf;

    fn opts() -> super::super::file::ReportOptions {
        super::super::file::ReportOptions::default()
    }

    // ── File report ──────────────────────────────────────────────────────────

    #[test]
    fn a_changed_file_shows_the_change_with_marks_and_line_numbers_never_hunk_jargon() {
        let diff = compute_diff(
            "one\ntwo\nthree\n",
            "one\nTWO\nthree\n",
            DiffOptions::default(),
        );
        let report = FileComparisonReport::from_diff(&diff, None, None, opts());
        let text = render_file_report_text(&report);

        assert!(!text.contains("hunk"), "no jargon:\n{text}");
        assert!(!text.contains("replace"), "no jargon:\n{text}");
        assert!(text.contains("− two"), "the removed line:\n{text}");
        assert!(text.contains("+ TWO"), "the added line:\n{text}");
        assert!(text.contains("1 change: 1 line removed, 1 line added."));
    }

    #[test]
    fn an_identical_file_shows_no_changes_section() {
        let diff = compute_diff("same\n", "same\n", DiffOptions::default());
        let report = FileComparisonReport::from_diff(&diff, None, None, opts());
        let text = render_file_report_text(&report);

        assert!(!text.contains("The changes"));
        assert!(text.contains("No differences — the two files are identical."));
    }

    // ── Directory report ─────────────────────────────────────────────────────

    fn dir_entries() -> Vec<RecEntry> {
        vec![
            RecEntry {
                rel_path: PathBuf::from("a.rs"),
                status: RecStatus::Equal,
                left_size: Some(100),
                right_size: Some(100),
            },
            RecEntry {
                rel_path: PathBuf::from("b.rs"),
                status: RecStatus::Changed,
                left_size: Some(200),
                right_size: Some(210),
            },
            RecEntry {
                rel_path: PathBuf::from("c.rs"),
                status: RecStatus::LeftOnly,
                left_size: Some(50),
                right_size: None,
            },
        ]
    }

    #[test]
    fn a_folder_comparison_uses_the_apps_own_words_never_zero_counts_never_old_jargon() {
        let entries = dir_entries();
        let report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        let text = render_dir_report_text(&report);

        // Absences (review 182 §2): the old to_markdown's words and its
        // zero-count noise must never appear.
        assert!(!text.contains("Equal"), "wrong word:\n{text}");
        assert!(!text.contains("Modified"), "wrong word:\n{text}");
        assert!(!text.contains("hunk"), "jargon:\n{text}");
        assert!(!text.contains(": 0"), "a zero count was printed:\n{text}");

        // Presences: the app's own words, and the entry that isn't identical.
        assert!(text.contains("Identical: 1"));
        assert!(text.contains("Different: 1"));
        assert!(text.contains("Only on the left: 1"));
        assert!(text.contains("c.rs"));
        assert!(!text.contains("a.rs"), "the equal entry must not be listed");
    }

    #[test]
    fn a_folder_changed_files_content_shows_marks_and_line_numbers() {
        let entries = dir_entries();
        let mut report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        let diff = compute_diff("x\n", "y\n", DiffOptions::default());
        let row = report
            .file_rows
            .iter_mut()
            .find(|r| r.path == "b.rs")
            .unwrap();
        row.change_hunks = Some(super::super::changes::change_hunks_from_diff(&diff, 3));

        let text = render_dir_report_text(&report);
        assert!(text.contains("− x"), "{text}");
        assert!(text.contains("+ y"), "{text}");
    }

    #[test]
    fn the_caps_omission_list_is_named_never_silent() {
        let entries = dir_entries();
        let mut report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        report.change_hunks_omitted.push("huge.rs".into());
        let text = render_dir_report_text(&report);
        assert!(text.contains("huge.rs"));
        assert!(text.contains("1 changed files"));
    }

    #[test]
    fn an_entirely_equal_folder_says_so_without_a_zero_count() {
        let entries = vec![RecEntry {
            rel_path: PathBuf::from("a.rs"),
            status: RecStatus::Equal,
            left_size: Some(1),
            right_size: Some(1),
        }];
        let report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        let text = render_dir_report_text(&report);
        assert!(!text.contains(": 0"));
        assert!(!text.contains("Files"), "no entries to list:\n{text}");
    }
}
