//! Tests for `build_dir_report`/`build_file_report`/`checksum_if_fingerprinted`
//! (RFC-092 §5), moved from `forskscope-ui` by RFC-093 §1 step 6.

use std::path::PathBuf;

use crate::report::{
    BuildOutcome, DirComparisonReport, ReportOptions, build_dir_report, build_file_report,
};
use forskscope_compare::CancellationToken;
use forskscope_compare::diff::{DiffOptions, compute_diff};
use forskscope_compare::dir::{RecEntry, RecStatus};

// ── Review 174 §6: a checksum appears only where it's reproducible ────────────

#[test]
fn no_fingerprint_means_no_checksum_even_with_real_bytes_present() {
    // Exactly LoadedDocument::empty()'s shape: real (empty) bytes, no
    // fingerprint - the case this exists to not show a misleading
    // checksum-of-nothing for.
    assert!(crate::report::checksum_if_fingerprinted(Some(&[]), false).is_none());
    // Also true for a non-empty byte slice - `has_fingerprint` is the
    // only thing that matters, not whether bytes happen to be present.
    assert!(crate::report::checksum_if_fingerprinted(Some(b"anything"), false).is_none());
}

#[test]
fn a_real_fingerprint_produces_the_real_checksum() {
    let checksum = crate::report::checksum_if_fingerprinted(Some(b"hello"), true).unwrap();
    assert_eq!(checksum.size, 5);
    assert_eq!(checksum.sha256_hex, crate::checksum::sha256_hex(b"hello"));
}

// ── build_file_report: a thin from_diff + with_checksums combination ─────────

#[test]
fn build_file_report_attaches_exactly_the_checksums_it_was_given() {
    let diff = compute_diff("old\n", "new\n", DiffOptions::default());
    let left = crate::report::checksum_if_fingerprinted(Some(b"old\n"), true);
    let report = build_file_report(&diff, None, None, ReportOptions::default(), left, None);
    assert_eq!(
        report.left_checksum.as_ref().unwrap().sha256_hex,
        crate::checksum::sha256_hex(b"old\n")
    );
    assert!(report.right_checksum.is_none());
}

// ── build_dir_report ───────────────────────────────────────────────────────────

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "fsk-report-build-dir-report-{label}-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(root.join("left")).unwrap();
        std::fs::create_dir_all(root.join("right")).unwrap();
        Self { root }
    }
    fn left(&self) -> PathBuf {
        self.root.join("left")
    }
    fn right(&self) -> PathBuf {
        self.root.join("right")
    }
    fn write(&self, side: &str, name: &str, content: &[u8]) {
        std::fs::write(self.root.join(side).join(name), content).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn expect_report(outcome: BuildOutcome) -> DirComparisonReport {
    match outcome {
        BuildOutcome::Report(r) => *r,
        BuildOutcome::Cancelled => panic!("expected a report, got Cancelled"),
        BuildOutcome::Stale(path) => panic!("expected a report, got Stale({path})"),
    }
}

fn entry(rel: &str, status: RecStatus) -> RecEntry {
    RecEntry {
        rel_path: PathBuf::from(rel),
        status,
        left_size: Some(1),
        right_size: Some(1),
    }
}

#[test]
fn hashes_every_readable_file_and_diffs_a_changed_text_file() {
    let s = Scratch::new("basic");
    s.write("left", "equal.txt", b"same\n");
    s.write("right", "equal.txt", b"same\n");
    s.write("left", "changed.txt", b"old\n");
    s.write("right", "changed.txt", b"new\n");
    s.write("left", "left_only.txt", b"only here\n");

    let entries = vec![
        entry("equal.txt", RecStatus::Equal),
        entry("changed.txt", RecStatus::Changed),
        entry("left_only.txt", RecStatus::LeftOnly),
    ];

    let report = expect_report(build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    ));

    let equal_row = report
        .file_rows
        .iter()
        .find(|r| r.path == "equal.txt")
        .unwrap();
    assert!(equal_row.left_checksum.is_some());
    assert!(equal_row.right_checksum.is_some());
    assert!(
        equal_row.change_hunks.is_none(),
        "equal files are not diffed"
    );

    let changed_row = report
        .file_rows
        .iter()
        .find(|r| r.path == "changed.txt")
        .unwrap();
    assert!(changed_row.left_checksum.is_some());
    assert!(changed_row.right_checksum.is_some());
    assert!(
        changed_row
            .change_hunks
            .as_ref()
            .is_some_and(|h| !h.is_empty())
    );

    let left_only_row = report
        .file_rows
        .iter()
        .find(|r| r.path == "left_only.txt")
        .unwrap();
    assert!(left_only_row.left_checksum.is_some());
    assert!(left_only_row.right_checksum.is_none());

    assert!(report.change_hunks_omitted.is_empty());
}

#[test]
fn a_binary_changed_file_is_hashed_but_never_listed_as_cap_omitted() {
    let s = Scratch::new("binary");
    s.write("left", "data.bin", &[0u8, 1, 2, 3]);
    s.write("right", "data.bin", &[0u8, 1, 2, 4]);
    let entries = vec![entry("data.bin", RecStatus::Changed)];

    let report = expect_report(build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    ));

    let row = &report.file_rows[0];
    assert!(row.left_checksum.is_some());
    assert!(row.right_checksum.is_some());
    assert!(row.change_hunks.is_none());
    assert!(
        report.change_hunks_omitted.is_empty(),
        "a binary file's missing diff is not a cap omission"
    );
}

#[test]
fn a_cancelled_token_returns_cancelled_and_writes_nothing() {
    let s = Scratch::new("cancel");
    s.write("left", "a.txt", b"a\n");
    s.write("right", "a.txt", b"b\n");
    let entries = vec![entry("a.txt", RecStatus::Changed)];

    let token = CancellationToken::new();
    token.cancel();

    let outcome = build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &token,
        &mut |_, _| {},
    );
    assert!(matches!(outcome, BuildOutcome::Cancelled));
}

#[test]
fn an_entry_listed_equal_whose_hashes_now_differ_refuses_the_whole_export() {
    let s = Scratch::new("stale-equal-now-differs");
    // The scan found these equal; the right file changed since then.
    s.write("left", "a.txt", b"same\n");
    s.write("right", "a.txt", b"not the same anymore\n");
    let entries = vec![entry("a.txt", RecStatus::Equal)];

    let outcome = build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    );
    match outcome {
        BuildOutcome::Stale(path) => assert_eq!(path, "a.txt"),
        _ => panic!("expected Stale(\"a.txt\"), got a different outcome"),
    }
}

#[test]
fn an_entry_listed_changed_whose_hashes_now_match_refuses_the_whole_export() {
    let s = Scratch::new("stale-changed-now-matches");
    // The scan found these different; one side was edited back since.
    s.write("left", "a.txt", b"now the same\n");
    s.write("right", "a.txt", b"now the same\n");
    let entries = vec![entry("a.txt", RecStatus::Changed)];

    let outcome = build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    );
    match outcome {
        BuildOutcome::Stale(path) => assert_eq!(path, "a.txt"),
        _ => panic!("expected Stale(\"a.txt\"), got a different outcome"),
    }
}

#[test]
fn an_unaffected_equal_entry_does_not_refuse_the_export() {
    let s = Scratch::new("stale-unaffected");
    s.write("left", "still-equal.txt", b"same\n");
    s.write("right", "still-equal.txt", b"same\n");
    let entries = vec![entry("still-equal.txt", RecStatus::Equal)];

    let outcome = build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    );
    assert!(matches!(outcome, BuildOutcome::Report(_)));
}

#[test]
fn the_cap_names_every_file_it_leaves_out_never_silently() {
    // Handoff 084 §7 criterion 2's own fixture size: 60 changed text
    // files against the 50-file cap, so exactly 10 are left out.
    let s = Scratch::new("cap");
    let mut entries = Vec::new();
    for i in 0..60 {
        let name = format!("f{i:03}.txt");
        s.write("left", &name, b"old\n");
        s.write("right", &name, format!("new {i}\n").as_bytes());
        entries.push(entry(&name, RecStatus::Changed));
    }

    let report = expect_report(build_dir_report(
        &s.left(),
        &s.right(),
        entries,
        DiffOptions::default(),
        ReportOptions::default(),
        &CancellationToken::new(),
        &mut |_, _| {},
    ));

    let with_content = report
        .file_rows
        .iter()
        .filter(|r| r.change_hunks.is_some())
        .count();
    assert_eq!(with_content, 50);
    assert_eq!(report.change_hunks_omitted.len(), 10);
    // Every omitted file still has its checksum - the cap limits
    // *content shown*, never what gets hashed.
    for name in &report.change_hunks_omitted {
        let row = report.file_rows.iter().find(|r| &r.path == name).unwrap();
        assert!(row.left_checksum.is_some());
        assert!(row.right_checksum.is_some());
    }
}
