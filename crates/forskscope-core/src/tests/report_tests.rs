//! Report export tests (RFC-027 §"Acceptance criteria", §"Test strategy").
//!
//! Tests validate: Markdown and JSON output for file comparisons, directory
//! comparisons, privacy (path redaction), hunk summary accuracy, empty diff
//! reports, and batch summary inclusion.

use std::path::PathBuf;

use crate::diff::{DiffOptions, compute_diff};
use crate::dir::{RecEntry, RecStatus};
use crate::report::{
    DirComparisonReport, FileComparisonReport, ReportOptions, ReportPathMode,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn opts_default() -> ReportOptions { ReportOptions::default() }

fn opts_name_only() -> ReportOptions {
    ReportOptions { path_mode: ReportPathMode::NameOnly, ..Default::default() }
}

fn opts_absolute() -> ReportOptions {
    ReportOptions { path_mode: ReportPathMode::Absolute, ..Default::default() }
}

fn simple_report() -> FileComparisonReport {
    let diff = compute_diff("hello\nworld\n", "hello\nRust\n", DiffOptions::default());
    FileComparisonReport::from_diff(
        &diff,
        Some(&PathBuf::from("/left/old.txt")),
        Some(&PathBuf::from("/right/new.txt")),
        None,
        opts_default(),
    )
}

fn identical_report() -> FileComparisonReport {
    let diff = compute_diff("same\n", "same\n", DiffOptions::default());
    FileComparisonReport::from_diff(&diff, None, None, None, opts_default())
}

fn dir_entries() -> Vec<RecEntry> {
    vec![
        RecEntry { rel_path: PathBuf::from("a.rs"), status: RecStatus::Equal,
                   left_size: Some(100), right_size: Some(100) },
        RecEntry { rel_path: PathBuf::from("b.rs"), status: RecStatus::Changed,
                   left_size: Some(200), right_size: Some(210) },
        RecEntry { rel_path: PathBuf::from("c.rs"), status: RecStatus::LeftOnly,
                   left_size: Some(50),  right_size: None },
        RecEntry { rel_path: PathBuf::from("d.rs"), status: RecStatus::RightOnly,
                   left_size: None,      right_size: Some(75) },
    ]
}

// ── File report — Markdown ────────────────────────────────────────────────────

#[test]
fn file_markdown_contains_title() {
    let md = simple_report().to_markdown();
    assert!(md.contains("# ForskScope File Comparison Report"), "missing title");
}

#[test]
fn file_markdown_shows_status_different() {
    let md = simple_report().to_markdown();
    assert!(md.contains("different"), "must show 'different' for changed files");
}

#[test]
fn file_markdown_shows_status_identical() {
    let md = identical_report().to_markdown();
    assert!(md.contains("identical"), "must show 'identical' for equal files");
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
    assert!(!md.contains("## Changed Hunks"), "no hunk section for identical files");
}

#[test]
fn file_markdown_contains_options_section() {
    let md = simple_report().to_markdown();
    assert!(md.contains("## Compare Options"), "must have options section");
    assert!(md.contains("Whitespace"), "must show whitespace option");
}

// ── File report — JSON ───────────────────────────────────────────────────────

#[test]
fn file_json_is_valid_object() {
    let json = simple_report().to_json();
    assert!(json.trim().starts_with('{'), "must start with {{");
    assert!(json.trim().ends_with('}'),   "must end with }}");
}

#[test]
fn file_json_contains_schema_version() {
    let json = simple_report().to_json();
    assert!(json.contains("\"schema_version\": 1"), "must have schema_version");
}

#[test]
fn file_json_contains_kind_file_comparison() {
    let json = simple_report().to_json();
    assert!(json.contains("\"kind\": \"file_comparison\""), "must have kind field");
}

#[test]
fn file_json_summary_has_correct_identical_flag() {
    let json_diff  = simple_report().to_json();
    let json_same  = identical_report().to_json();
    assert!(json_diff.contains("\"identical\": false"), "changed file: identical=false");
    assert!(json_same.contains("\"identical\": true"),  "equal file: identical=true");
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
        None,
        opts_name_only(),
    );
    let md = report.to_markdown();
    assert!(!md.contains("/very/sensitive/project/"), "absolute path must not appear");
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
        None,
        opts_absolute(),
    );
    let md = report.to_markdown();
    assert!(md.contains("/root/old.txt"), "absolute path must appear in absolute mode");
}

// ── Directory report — Markdown ───────────────────────────────────────────────

#[test]
fn dir_markdown_contains_title() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert!(report.to_markdown().contains("# ForskScope Directory Comparison Report"));
}

#[test]
fn dir_markdown_summary_counts_are_correct() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert_eq!(report.equal,       1);
    assert_eq!(report.changed,     1);
    assert_eq!(report.left_only,   1);
    assert_eq!(report.right_only,  1);
    assert_eq!(report.total,       4);
    let md = report.to_markdown();
    assert!(md.contains("| Equal         | 1 |"), "equal count wrong");
    assert!(md.contains("| Modified      | 1 |"), "modified count wrong");
}

#[test]
fn dir_markdown_omits_equal_from_changed_table() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let md = report.to_markdown();
    // a.rs is equal — should NOT appear in the changed files table
    assert!(!md.contains("`a.rs` | equal"), "equal files must not appear in changed table");
    // b.rs is changed — MUST appear
    assert!(md.contains("b.rs"), "changed file must appear");
}

#[test]
fn dir_markdown_includes_sizes_by_default() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let md = report.to_markdown();
    // b.rs has left=200, right=210 bytes
    assert!(md.contains("200 B") || md.contains("210 B"), "sizes must appear in default mode");
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
    assert!(report.to_json().contains("\"kind\": \"directory_comparison\""));
}

#[test]
fn dir_json_files_array_omits_equal_entries() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    let json = report.to_json();
    // a.rs is equal — must not appear in the files array as a file row.
    // The summary section may still contain the word "equal" as a count key.
    assert!(!json.contains("\"a.rs\""), "equal file path must not appear in files array");
    // b.rs is modified — must appear.
    assert!(json.contains("\"b.rs\""), "modified file must appear");
    assert!(json.contains("\"modified\""), "modified status must appear");
}
