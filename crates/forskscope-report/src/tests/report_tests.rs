//! Report model tests (RFC-027 §"Acceptance criteria", §"Test strategy").
//!
//! Tests validate the report *model* `FileComparisonReport::from_diff`/
//! `DirComparisonReport::from_entries` build - counts, privacy (path
//! redaction), the one-sentence summary, options/profile resolution,
//! checksums and change hunks. Rendering tests (what `html`/`text`/`json`
//! print from that model) live in those modules instead - review 182 §2
//! removed the old `to_markdown`/`to_json` methods this file used to
//! drive its own assertions through, so every test here now reads the
//! model's fields directly.

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
    assert_eq!(report.left_display, "old.txt");
    assert_eq!(report.right_display, "new.txt");
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
    assert_eq!(report.left_display, "/root/old.txt");
}

// ── Directory report — counts ─────────────────────────────────────────────────

#[test]
fn dir_report_summary_counts_are_correct() {
    let entries = dir_entries();
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    assert_eq!(report.equal, 1);
    assert_eq!(report.changed, 1);
    assert_eq!(report.left_only, 1);
    assert_eq!(report.right_only, 1);
    assert_eq!(report.total, 4);
}

/// F135: an empty one-sided directory counts into the ordinary
/// `left_only`/`right_only` totals (it genuinely is one-sided) but keeps
/// its own `rec_status` (`LeftOnlyDir`/`RightOnlyDir`), distinct from an
/// ordinary one-sided file's (`LeftOnly`/`RightOnly`), so a renderer never
/// claims a file was found where there was only an empty directory.
#[test]
fn dir_report_counts_an_empty_one_sided_directory_as_one_sided_with_its_own_rec_status() {
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
    let rec_status = |path: &str| {
        report
            .file_rows
            .iter()
            .find(|r| r.path == path)
            .unwrap()
            .rec_status
    };
    assert_eq!(rec_status("empty_l"), RecStatus::LeftOnlyDir);
    assert_eq!(rec_status("empty_r"), RecStatus::RightOnlyDir);
    assert_ne!(
        rec_status("empty_l"),
        rec_status("c.rs"),
        "the empty-directory row must not read the same as an ordinary one-sided file"
    );
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
fn a_cancelled_scans_unfinished_entry_stays_computing_never_equal_or_changed() {
    let entries = vec![RecEntry {
        rel_path: PathBuf::from("still-going.rs"),
        status: RecStatus::Computing,
        left_size: Some(10),
        right_size: Some(10),
    }];
    let report = DirComparisonReport::from_entries(&entries, None, None, None, opts_default());
    // A finished report has nothing "currently comparing" - this entry's
    // own status survived the scan unchanged, into `rec_status`; it is a
    // renderer's job (html/text's own `status_key(.., false)` call) to
    // turn that into the word "Not compared", never "Comparing…".
    assert_eq!(report.file_rows[0].rec_status, RecStatus::Computing);
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
