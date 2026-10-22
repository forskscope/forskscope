//! Plain-text rendering (review 182 §2) — built from the same report
//! model `html.rs` renders, following every rule the HTML already
//! follows: the app's own status words (never "Equal"/"Modified"), a
//! non-zero-only counts line, a "How" section naming non-default compare
//! options only when any apply (review 183 §3.1), and change content
//! shown as old/new line numbers with a `−`/`+` mark, never as
//! unexplained jargon ("hunk", a bare range).
//!
//! **English only.** `forskscope-commands`'s `compare` subcommand, the
//! only caller, has no language setting — see the user guide. This
//! replaces the old `to_markdown`
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

/// Review 183 §3.1: a non-default compare option (`--ignore-case`,
/// `--algorithm patience`, …) can change the result without the text
/// report ever saying so — shown only when at least one option differs
/// from its default, same condition the HTML "How" section uses.
fn render_how_section(out: &mut String, non_default_options: &[(String, String)]) {
    if non_default_options.is_empty() {
        return;
    }
    let _ = writeln!(out, "How");
    for (k, v) in non_default_options {
        let _ = writeln!(out, "  {k}: {v}");
    }
    let _ = writeln!(out);
}

/// Plain-text rendering of a single-file comparison.
pub fn render_file_report_text(report: &FileComparisonReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "What was compared");
    let _ = writeln!(out, "  Left:  {}", report.left_display);
    let _ = writeln!(out, "  Right: {}", report.right_display);
    let _ = writeln!(out);
    render_how_section(&mut out, &report.non_default_options);
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
    let extra_counts = [
        (status_key(RecStatus::Symlink, false), report.symlinks),
        (status_key(RecStatus::Unreadable, false), report.unreadable),
    ];
    let main_line = counts_line(&identity, &main_counts);
    let extra_line = counts_line(&identity, &extra_counts);
    // Review 184 §3.2: "No files." must mean every count is zero,
    // symlinks and unreadable entries included - not just the four main
    // ones. A folder whose only entry is unreadable used to print "No
    // files." directly above "Unreadable: 1", contradicting itself.
    match (&main_line, &extra_line) {
        (None, None) => {
            let _ = writeln!(out, "No files.");
        }
        _ => {
            if let Some(line) = &main_line {
                let _ = writeln!(out, "{line}");
            }
            if let Some(line) = &extra_line {
                let _ = writeln!(out, "{line}");
            }
        }
    }

    if !report.non_default_options.is_empty() {
        let _ = writeln!(out);
        render_how_section(&mut out, &report.non_default_options);
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
            "  {}",
            cap_omission_sentence(
                report.change_hunks_omitted.len(),
                report.options.max_files_with_content,
                report.options.max_change_bytes,
            )
        );
        for name in &report.change_hunks_omitted {
            let _ = writeln!(out, "    - {name}");
        }
    }

    out
}

/// Review 183 §3.2: names the caps actually applied (`--max-files`/
/// `--max-change-bytes` can change them from their defaults) instead of
/// a fixed "50 files or 5 MiB", and pluralizes `n` (files omitted) and
/// `max_files` (the cap itself) independently — "1 changed file's
/// content" vs "2 changed files' content", "the cap of 1 file" vs "the
/// cap of 50 files".
fn cap_omission_sentence(n: usize, max_files: usize, max_change_bytes: usize) -> String {
    let n_word = if n == 1 {
        "changed file's content is"
    } else {
        "changed files' content is"
    };
    let max_files_word = if max_files == 1 { "file" } else { "files" };
    format!(
        "{n} {n_word} not shown in this report (the cap of {max_files} {max_files_word} or {} of change text was reached):",
        super::dir::fmt_size(max_change_bytes as u64)
    )
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

    #[test]
    fn default_options_show_no_how_section() {
        let diff = compute_diff("one\n", "two\n", DiffOptions::default());
        let report = FileComparisonReport::from_diff(&diff, None, None, opts());
        let text = render_file_report_text(&report);
        assert!(!text.contains("How"), "{text}");
    }

    #[test]
    fn a_non_default_option_is_named_even_when_it_is_the_only_reason_nothing_differs() {
        // Review 183 §3.1: two files differing only by case, compared
        // with --ignore-case, must not silently read as "No differences"
        // with no sign that case was ignored.
        let diff_options = DiffOptions {
            ignore_case: true,
            ..Default::default()
        };
        let diff = compute_diff("Same\n", "same\n", diff_options);
        let report = FileComparisonReport::from_diff(&diff, None, None, opts());
        let text = render_file_report_text(&report);
        assert!(text.contains("How"), "{text}");
        assert!(text.contains("Case: ignored"), "{text}");
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
    fn a_folder_comparison_with_default_options_shows_no_how_section() {
        let entries = dir_entries();
        let report = DirComparisonReport::from_entries(
            &entries,
            None,
            None,
            Some(&DiffOptions::default()),
            opts(),
        );
        let text = render_dir_report_text(&report);
        assert!(!text.contains("How"), "{text}");
    }

    #[test]
    fn a_folder_comparison_names_a_non_default_option() {
        let entries = dir_entries();
        let diff_options = DiffOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        let report =
            DirComparisonReport::from_entries(&entries, None, None, Some(&diff_options), opts());
        let text = render_dir_report_text(&report);
        assert!(text.contains("How"), "{text}");
        assert!(text.contains("Whitespace: ignored"), "{text}");
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
        // Review 183 §3.2: exactly one omitted file is "1 changed file's
        // content", never the old fixed sentence's plural "files'".
        assert!(text.contains("1 changed file's content"), "{text}");
        assert!(!text.contains("1 changed files"), "{text}");
        assert!(text.contains("the cap of 50 files or 5.0 MiB"), "{text}");
    }

    #[test]
    fn two_omitted_files_use_plural_grammar() {
        let entries = dir_entries();
        let mut report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        report.change_hunks_omitted.push("huge1.rs".into());
        report.change_hunks_omitted.push("huge2.rs".into());
        let text = render_dir_report_text(&report);
        assert!(text.contains("2 changed files' content"), "{text}");
    }

    #[test]
    fn the_caps_omission_notice_names_a_non_default_cap_actually_applied() {
        // Review 183 §3.2: the old sentence was a fixed "cap of 50 files
        // or 5 MiB" even when `--max-files`/`--max-change-bytes` set
        // different ones - the notice must name the caps that actually
        // applied to this report, singular grammar included.
        let entries = dir_entries();
        let mut options = opts();
        options.max_files_with_content = 1;
        options.max_change_bytes = 1024;
        let mut report = DirComparisonReport::from_entries(&entries, None, None, None, options);
        report.change_hunks_omitted.push("huge.rs".into());
        let text = render_dir_report_text(&report);
        assert!(text.contains("the cap of 1 file or 1.0 KiB"), "{text}");
        assert!(!text.contains("cap of 1 files"), "{text}");
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

    #[test]
    fn a_genuinely_empty_folder_says_no_files() {
        let report = DirComparisonReport::from_entries(&[], None, None, None, opts());
        let text = render_dir_report_text(&report);
        assert!(text.contains("No files."), "{text}");
    }

    #[test]
    fn a_folder_whose_only_entry_is_unreadable_never_says_no_files_beside_its_own_count() {
        // Review 184 §3.2: the four main counts are all zero here, but
        // there is genuinely one entry (unreadable) - "No files." right
        // above "Unreadable: 1" would contradict itself.
        let entries = vec![RecEntry {
            rel_path: PathBuf::from("n.txt"),
            status: RecStatus::Unreadable,
            left_size: None,
            right_size: None,
        }];
        let report = DirComparisonReport::from_entries(&entries, None, None, None, opts());
        let text = render_dir_report_text(&report);
        assert!(text.contains("Unreadable: 1"), "{text}");
        assert!(!text.contains("No files."), "{text}");
    }
}
