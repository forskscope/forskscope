//! Report export tests (RFC-027 §"Acceptance criteria", §"Test strategy").
//!
//! Tests validate: Markdown and JSON output for file comparisons, directory
//! comparisons, privacy (path redaction), hunk summary accuracy, empty diff
//! reports, and batch summary inclusion.

use std::path::PathBuf;

use crate::report::{DirComparisonReport, FileComparisonReport, ReportOptions, ReportPathMode};
use forskscope_compare::diff::{DiffOptions, compute_diff};
use forskscope_compare::dir::{RecEntry, RecStatus};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn opts_default() -> ReportOptions {
    ReportOptions::default()
}

fn opts_name_only() -> ReportOptions {
    ReportOptions {
        path_mode: ReportPathMode::NameOnly,
        ..Default::default()
    }
}

fn opts_absolute() -> ReportOptions {
    ReportOptions {
        path_mode: ReportPathMode::Absolute,
        ..Default::default()
    }
}

fn simple_report() -> FileComparisonReport {
    let diff = compute_diff("hello\nworld\n", "hello\nRust\n", DiffOptions::default());
    FileComparisonReport::from_diff(
        &diff,
        Some(&PathBuf::from("/left/old.txt")),
        Some(&PathBuf::from("/right/new.txt")),
        opts_default(),
    )
}

fn identical_report() -> FileComparisonReport {
    let diff = compute_diff("same\n", "same\n", DiffOptions::default());
    FileComparisonReport::from_diff(&diff, None, None, opts_default())
}

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
        RecEntry {
            rel_path: PathBuf::from("d.rs"),
            status: RecStatus::RightOnly,
            left_size: None,
            right_size: Some(75),
        },
    ]
}

// ── File report — Markdown ────────────────────────────────────────────────────

#[test]
fn file_markdown_contains_title() {
    let md = simple_report().to_markdown();
    assert!(
        md.contains("# ForskScope File Comparison Report"),
        "missing title"
    );
}

#[test]
fn file_markdown_shows_status_different() {
    let md = simple_report().to_markdown();
    assert!(
        md.contains("different"),
        "must show 'different' for changed files"
    );
}

#[test]
fn file_markdown_shows_status_identical() {
    let md = identical_report().to_markdown();
    assert!(
        md.contains("identical"),
        "must show 'identical' for equal files"
    );
}

#[test]
fn file_markdown_contains_hunk_table_for_changed_file() {
    let md = simple_report().to_markdown();
    assert!(md.contains("## Changed Hunks"), "must have hunk section");
    assert!(md.contains("replace"), "hunk kind must appear");
}

#[test]
fn file_markdown_no_hunk_table_for_identical_file() {
    let md = identical_report().to_markdown();
    assert!(
        !md.contains("## Changed Hunks"),
        "no hunk section for identical files"
    );
}

#[test]
fn file_markdown_contains_options_section() {
    let md = simple_report().to_markdown();
    assert!(
        md.contains("## Compare Options"),
        "must have options section"
    );
    assert!(md.contains("Whitespace"), "must show whitespace option");
}

// ── File report — JSON ───────────────────────────────────────────────────────

#[test]
fn file_json_is_valid_object() {
    let json = simple_report().to_json();
    assert!(json.trim().starts_with('{'), "must start with {{");
    assert!(json.trim().ends_with('}'), "must end with }}");
}

#[test]
fn file_json_contains_schema_version() {
    let json = simple_report().to_json();
    assert!(
        json.contains("\"schema_version\": 1"),
        "must have schema_version"
    );
}

#[test]
fn file_json_contains_kind_file_comparison() {
    let json = simple_report().to_json();
    assert!(
        json.contains("\"kind\": \"file_comparison\""),
        "must have kind field"
    );
}

#[test]
fn file_json_summary_has_correct_identical_flag() {
    let json_diff = simple_report().to_json();
    let json_same = identical_report().to_json();
    assert!(
        json_diff.contains("\"identical\": false"),
        "changed file: identical=false"
    );
    assert!(
        json_same.contains("\"identical\": true"),
        "equal file: identical=true"
    );
}

#[test]
fn file_json_hunks_array_present() {
    let json = simple_report().to_json();
    assert!(json.contains("\"hunks\": ["), "must have hunks array");
}

// ── Path privacy ─────────────────────────────────────────────────────────────

#[test]
fn name_only_mode_strips_absolute_path() {
    let diff = compute_diff("a\n", "b\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(
        &diff,
        Some(&PathBuf::from("/very/sensitive/project/old.txt")),
        Some(&PathBuf::from("/very/sensitive/project/new.txt")),
        opts_name_only(),
    );
    let md = report.to_markdown();
    assert!(
        !md.contains("/very/sensitive/project/"),
        "absolute path must not appear"
    );
    assert!(md.contains("old.txt"), "filename must appear");
    assert!(md.contains("new.txt"), "filename must appear");
}

#[test]
fn absolute_mode_shows_full_path() {
    let diff = compute_diff("a\n", "b\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(
        &diff,
        Some(&PathBuf::from("/root/old.txt")),
        Some(&PathBuf::from("/root/new.txt")),
        opts_absolute(),
    );
    let md = report.to_markdown();
    assert!(
        md.contains("/root/old.txt"),
        "absolute path must appear in absolute mode"
    );
}

// ── Directory report — Markdown ───────────────────────────────────────────────

#[test]
fn dir_markdown_contains_title() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert!(
        report
            .to_markdown()
            .contains("# ForskScope Directory Comparison Report")
    );
}

#[test]
fn dir_markdown_summary_counts_are_correct() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert_eq!(report.equal, 1);
    assert_eq!(report.changed, 1);
    assert_eq!(report.left_only, 1);
    assert_eq!(report.right_only, 1);
    assert_eq!(report.total, 4);
    let md = report.to_markdown();
    assert!(md.contains("| Equal         | 1 |"), "equal count wrong");
    assert!(md.contains("| Modified      | 1 |"), "modified count wrong");
}

/// F135: an empty one-sided directory counts into the ordinary
/// `left_only`/`right_only` totals (it genuinely is one-sided) but keeps
/// its own distinguishing status text, so the row never claims a file was
/// found where there was only an empty directory.
#[test]
fn dir_report_counts_an_empty_one_sided_directory_as_one_sided_with_its_own_text() {
    let mut entries = dir_entries();
    entries.push(RecEntry {
        rel_path: PathBuf::from("empty_l"),
        status: RecStatus::LeftOnlyDir,
        left_size: None,
        right_size: None,
    });
    entries.push(RecEntry {
        rel_path: PathBuf::from("empty_r"),
        status: RecStatus::RightOnlyDir,
        left_size: None,
        right_size: None,
    });
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert_eq!(report.left_only, 2, "c.rs and empty_l");
    assert_eq!(report.right_only, 2, "d.rs and empty_r");
    let row = |path: &str| {
        report
            .file_rows
            .iter()
            .find(|r| r.path == path)
            .unwrap()
            .status
            .clone()
    };
    assert_eq!(row("empty_l"), "left only (empty directory)");
    assert_eq!(row("empty_r"), "right only (empty directory)");
    assert_ne!(
        row("empty_l"),
        row("c.rs"),
        "the empty-directory row must not read the same as an ordinary file row"
    );
}

#[test]
fn dir_markdown_omits_equal_from_changed_table() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let md = report.to_markdown();
    // a.rs is equal — should NOT appear in the changed files table
    assert!(
        !md.contains("`a.rs` | equal"),
        "equal files must not appear in changed table"
    );
    // b.rs is changed — MUST appear
    assert!(md.contains("b.rs"), "changed file must appear");
}

#[test]
fn dir_markdown_includes_sizes_by_default() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let md = report.to_markdown();
    // b.rs has left=200, right=210 bytes
    assert!(
        md.contains("200 B") || md.contains("210 B"),
        "sizes must appear in default mode"
    );
}

// ── Directory report — JSON ───────────────────────────────────────────────────

#[test]
fn dir_json_is_valid_object() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let json = report.to_json();
    assert!(json.trim().starts_with('{'));
    assert!(json.trim().ends_with('}'));
}

#[test]
fn dir_json_kind_is_directory_comparison() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert!(
        report
            .to_json()
            .contains("\"kind\": \"directory_comparison\"")
    );
}

#[test]
fn dir_json_files_array_omits_equal_entries() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let json = report.to_json();
    // a.rs is equal — must not appear in the files array as a file row.
    // The summary section may still contain the word "equal" as a count key.
    assert!(
        !json.contains("\"a.rs\""),
        "equal file path must not appear in files array"
    );
    // b.rs is modified — must appear.
    assert!(json.contains("\"b.rs\""), "modified file must appear");
    assert!(json.contains("\"modified\""), "modified status must appear");
}

// ── RFC-092 (handoff 084): HTML-report model extensions ───────────────────────

#[test]
fn identical_files_have_no_differences_summary_sentence() {
    let diff = compute_diff("same\n", "same\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    assert_eq!(
        report.one_sentence_summary(),
        "No differences — the two files are identical."
    );
}

#[test]
fn a_replace_hunk_summary_sentence_names_lines_not_hunks() {
    let diff = compute_diff(
        "one\ntwo\nthree\n",
        "one\nTWO\nthree\n",
        DiffOptions::default(),
    );
    let report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    let sentence = report.one_sentence_summary();
    // Review 174 §4: no "hunk"/"replaced" jargon - lines only.
    assert_eq!(sentence, "1 change: 1 line removed, 1 line added.");
    assert!(!sentence.contains("hunk"));
    assert!(!sentence.contains("replace"));
}

#[test]
fn options_at_their_defaults_produce_an_empty_non_default_list_and_a_default_profile_name() {
    let diff = compute_diff("a\nb\n", "a\nB\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    assert!(report.non_default_options.is_empty());
    assert_eq!(report.profile_name, "Default");
}

#[test]
fn a_non_default_option_is_listed_and_the_profile_falls_back_to_custom() {
    let options = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    let diff = compute_diff("Hello\n", "hello\n", options);
    let report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    assert_eq!(
        report.non_default_options,
        vec![("Case".to_string(), "ignored".to_string())]
    );
    assert_eq!(report.profile_name, "Custom");
}

#[test]
fn an_explicit_profile_name_is_kept_even_with_non_default_options() {
    let options = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    let diff = compute_diff("Hello\n", "hello\n", options);
    let opts = ReportOptions {
        profile_name: Some("Ignore case".into()),
        ..opts_default()
    };
    let report = FileComparisonReport::from_diff(&diff, None, None, opts);
    assert_eq!(report.profile_name, "Ignore case");
}

#[test]
fn change_hunks_are_populated_for_a_real_change_and_empty_when_identical() {
    let changed = compute_diff("one\ntwo\n", "one\nTWO\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(&changed, None, None, opts_default());
    assert!(!report.change_hunks.is_empty());

    let identical = compute_diff("same\n", "same\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(&identical, None, None, opts_default());
    assert!(report.change_hunks.is_empty());
}

#[test]
fn checksums_are_absent_until_attached_and_present_after() {
    use crate::report::FileChecksum;

    let diff = compute_diff("a\n", "b\n", DiffOptions::default());
    let report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    assert!(report.left_checksum.is_none());
    assert!(report.right_checksum.is_none());

    let report = report.with_checksums(
        Some(FileChecksum::of_bytes(b"a\n")),
        Some(FileChecksum::of_bytes(b"b\n")),
    );
    let left = report.left_checksum.as_ref().unwrap();
    let right = report.right_checksum.as_ref().unwrap();
    assert_eq!(left.size, 2);
    assert_eq!(right.size, 2);
    assert_ne!(left.sha256_hex, right.sha256_hex);
}

#[test]
fn dir_report_with_no_diff_options_has_no_profile_name() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert!(report.profile_name.is_none());
    assert!(report.non_default_options.is_empty());
}

#[test]
fn dir_report_with_diff_options_resolves_a_profile_name() {
    let entries = dir_entries();
    let default_options = DiffOptions::default();
    let report = DirComparisonReport::from_entries(
        &entries,
        None,
        None,
        Some(&default_options),
        opts_default(),
    );
    assert_eq!(report.profile_name, Some("Default".to_string()));

    let ignoring_case = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    let report = DirComparisonReport::from_entries(
        &entries,
        None,
        None,
        Some(&ignoring_case),
        opts_default(),
    );
    assert_eq!(report.profile_name, Some("Custom".to_string()));
    assert_eq!(
        report.non_default_options,
        vec![("Case".to_string(), "ignored".to_string())]
    );
}

#[test]
fn a_cancelled_scans_unfinished_entry_reads_not_compared_not_computing() {
    let entries = vec![RecEntry {
        rel_path: PathBuf::from("still-going.rs"),
        status: RecStatus::Computing,
        left_size: Some(10),
        right_size: Some(10),
    }];
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert_eq!(report.file_rows[0].status, "not compared");
}

#[test]
fn dir_file_row_checksums_and_change_hunks_start_absent_and_are_attachable() {
    use crate::report::FileChecksum;

    let entries = dir_entries();
    let mut report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert!(report.file_rows.iter().all(|r| r.left_checksum.is_none()
        && r.right_checksum.is_none()
        && r.change_hunks.is_none()));

    // b.rs is the "modified" row - attach as the export job would.
    let row = report
        .file_rows
        .iter_mut()
        .find(|r| r.path == "b.rs")
        .unwrap();
    row.left_checksum = Some(FileChecksum::of_bytes(b"left"));
    row.right_checksum = Some(FileChecksum::of_bytes(b"right"));
    let diff = compute_diff("left\n", "right\n", DiffOptions::default());
    row.change_hunks = Some(crate::report::change_hunks_from_diff(&diff, 3));

    assert!(row.left_checksum.is_some());
    assert!(row.change_hunks.as_ref().is_some_and(|h| !h.is_empty()));

    // The cap's omission list is empty until the export job sets it.
    assert!(report.change_hunks_omitted.is_empty());
    report.change_hunks_omitted.push("huge-file.rs".into());
    assert_eq!(
        report.change_hunks_omitted,
        vec!["huge-file.rs".to_string()]
    );
}

#[test]
fn push_note_is_visible_alongside_diff_warnings() {
    let diff = compute_diff("a\n", "b\n", DiffOptions::default());
    let mut report = FileComparisonReport::from_diff(&diff, None, None, opts_default());
    let before = report.warnings.len();
    report.push_note("Left file encoding: Shift-JIS (not UTF-8).");
    assert_eq!(report.warnings.len(), before + 1);
    assert!(report.warnings.last().unwrap().contains("Shift-JIS"));
}
