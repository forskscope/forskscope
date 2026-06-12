//! Corpus-driven diff correctness tests (RFC-002, RFC-007 §"Acceptance corpus").
//!
//! These tests load fixture files from `tests/fixtures/` and verify that
//! `compute_diff` produces correct results for the documented corpus cases.
//! The fixture files serve as both test inputs and readable documentation
//! of the expected diff behaviour.

use std::fs;
use std::path::Path;

use forskscope_core::{DiffOptions, compute_diff};
use forskscope_core::diff::HunkKind;

fn load(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"))
}

fn opts_default() -> DiffOptions { DiffOptions::default() }
fn opts_ignore_ws() -> DiffOptions {
    DiffOptions { ignore_whitespace: true, ..Default::default() }
}
fn opts_ignore_case() -> DiffOptions {
    DiffOptions { ignore_case: true, ..Default::default() }
}

// ── Text: identical files ─────────────────────────────────────────────────────

#[test]
fn identical_fixture_produces_no_changes() {
    let left  = load("text/left_identical.txt");
    let right = load("text/right_identical.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(doc.is_identical(), "identical fixture must produce no changed hunks");
    assert_eq!(doc.stats.hunks_changed, 0);
}

// ── Text: single changed line ─────────────────────────────────────────────────

#[test]
fn one_changed_line_produces_one_replace_hunk() {
    let left  = load("text/left_one_changed.txt");
    let right = load("text/right_one_changed.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 1);
    let hunk = doc.hunks.iter().find(|h| h.kind == HunkKind::Replace)
        .expect("one-changed fixture must contain a Replace hunk");
    let left_text: Vec<&str> = hunk.rows.iter()
        .filter_map(|r| r.left.as_ref().map(|l| l.content.trim()))
        .collect();
    let right_text: Vec<&str> = hunk.rows.iter()
        .filter_map(|r| r.right.as_ref().map(|r| r.content.trim()))
        .collect();
    assert!(left_text.iter().any(|s| *s == "charlie"),
        "left side must contain original 'charlie'");
    assert!(right_text.iter().any(|s| *s == "CHARLIE"),
        "right side must contain changed 'CHARLIE'");
}

// ── Text: insertions ──────────────────────────────────────────────────────────

#[test]
fn insertions_fixture_produces_insert_hunks() {
    let left  = load("text/left_insertions.txt");
    let right = load("text/right_insertions.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
    let has_insert = doc.hunks.iter().any(|h| h.kind == HunkKind::Insert);
    assert!(has_insert, "insertions fixture must contain at least one Insert hunk");
    assert!(doc.stats.lines_inserted > 0);
}

// ── Text: deletions ───────────────────────────────────────────────────────────

#[test]
fn deletions_fixture_produces_delete_hunks() {
    let left  = load("text/left_deletions.txt");
    let right = load("text/right_deletions.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
    let has_delete = doc.hunks.iter().any(|h| h.kind == HunkKind::Delete);
    assert!(has_delete, "deletions fixture must contain at least one Delete hunk");
    assert!(doc.stats.lines_deleted > 0);
}

// ── Text: empty file comparisons ──────────────────────────────────────────────

#[test]
fn both_empty_fixtures_are_identical() {
    let empty = load("text/empty.txt");
    let doc = compute_diff(&empty, &empty, opts_default());
    assert!(doc.is_identical());
}

#[test]
fn empty_vs_nonempty_is_pure_insert() {
    let empty    = load("text/empty.txt");
    let nonempty = load("text/nonempty.txt");
    let doc = compute_diff(&empty, &nonempty, opts_default());
    assert!(!doc.is_identical());
    let all_inserts = doc.hunks.iter()
        .filter(|h| h.kind.is_change())
        .all(|h| h.kind == HunkKind::Insert);
    assert!(all_inserts, "empty vs nonempty must produce only Insert hunks");
}

// ── Newlines: LF vs CRLF ──────────────────────────────────────────────────────

#[test]
fn lf_and_crlf_fixtures_differ_by_newline() {
    let lf   = load("newlines/lf.txt");
    let crlf = load("newlines/crlf.txt");
    let doc = compute_diff(&lf, &crlf, opts_default());
    // LF vs CRLF is a real difference (newline style changed)
    assert!(!doc.is_identical(),
        "LF vs CRLF must differ when newline comparison is significant");
}

#[test]
fn identical_newline_content_is_identical() {
    let lf = load("newlines/lf.txt");
    let doc = compute_diff(&lf, &lf, opts_default());
    assert!(doc.is_identical());
}

// ── Newlines: no-final-newline ────────────────────────────────────────────────

#[test]
fn file_with_and_without_final_newline_differ() {
    let lf              = load("newlines/lf.txt");
    let no_final_nl     = load("newlines/no_final_newline.txt");
    let doc = compute_diff(&no_final_nl, &lf, opts_default());
    assert!(!doc.is_identical(),
        "file without final newline must differ from file with final newline");
}

// ── Whitespace: extra space ───────────────────────────────────────────────────

#[test]
fn extra_space_detected_by_default() {
    let left  = load("whitespace/left_spaces.txt");
    let right = load("whitespace/right_extra_space.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical(),
        "extra space must be detected with default options");
}

#[test]
fn extra_space_hidden_with_ignore_whitespace() {
    let left  = load("whitespace/left_spaces.txt");
    let right = load("whitespace/right_extra_space.txt");
    let doc = compute_diff(&left, &right, opts_ignore_ws());
    assert!(doc.is_identical(),
        "extra space must be ignored with ignore_whitespace option");
}

// ── Whitespace: trailing spaces ───────────────────────────────────────────────

#[test]
fn trailing_space_detected_by_default() {
    let left  = load("whitespace/left_trailing.txt");
    let right = load("whitespace/right_no_trailing.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical(),
        "trailing space must be detected with default options");
}

// ── Case sensitivity ──────────────────────────────────────────────────────────

#[test]
fn case_change_detected_by_default() {
    let left  = load("text/left_one_changed.txt");
    let right = load("text/right_one_changed.txt");
    // The fixture has charlie vs CHARLIE — a case change.
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical(),
        "case change must be detected with default (case-sensitive) options");
}

#[test]
fn case_change_hidden_with_ignore_case() {
    let left  = load("text/left_one_changed.txt");
    let right = load("text/right_one_changed.txt");
    let doc = compute_diff(&left, &right, opts_ignore_case());
    assert!(doc.is_identical(),
        "charlie vs CHARLIE must be identical with ignore_case option");
}

// ── Indentation: tab vs spaces ────────────────────────────────────────────────

#[test]
fn tab_vs_space_indent_differs_by_default() {
    let tabs   = load("whitespace/tab_indent.txt");
    let spaces = load("whitespace/space_indent.txt");
    let doc = compute_diff(&tabs, &spaces, opts_default());
    assert!(!doc.is_identical(),
        "tab vs space indentation must differ with default options");
}

// ── Function edit: realistic code change ─────────────────────────────────────

#[test]
fn function_return_value_change_produces_replace_hunk() {
    let left  = load("text/left_function.txt");
    let right = load("text/right_function.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
    // Only one line changes (return a → return a + 1)
    assert_eq!(doc.stats.hunks_changed, 1,
        "single-line function edit must produce exactly one changed hunk");
}

// ── Unicode content ───────────────────────────────────────────────────────────

#[test]
fn unicode_content_diffed_correctly() {
    let left  = load("text/left_unicode.txt");
    let right = load("text/right_unicode.txt");
    let doc = compute_diff(&left, &right, opts_default());
    // "world" vs "WORLD" — should detect a change
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 1);
}

#[test]
fn unicode_content_equal_with_ignore_case() {
    let left  = load("text/left_unicode.txt");
    let right = load("text/right_unicode.txt");
    let doc = compute_diff(&left, &right, opts_ignore_case());
    // こんにちは is identical both sides; world/WORLD differs only by case
    assert!(doc.is_identical(), "world vs WORLD must be equal with ignore_case");
}

// ── UTF-8 BOM handling ────────────────────────────────────────────────────────

#[test]
fn utf8_bom_differs_from_no_bom() {
    let bom    = load("text/utf8_bom.txt");
    let no_bom = load("text/utf8_no_bom.txt");
    // The BOM is U+FEFF at the start — byte-level difference
    assert!(!doc_identical(&bom, &no_bom),
        "UTF-8 BOM file must differ from no-BOM file at byte level");
}

// ── Mixed trailing whitespace ──────────────────────────────────────────────

#[test]
fn mixed_trailing_whitespace_detected_by_default() {
    let left  = load("whitespace/left_mixed_trailing.txt");
    let right = load("whitespace/right_clean.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
}

#[test]
fn mixed_trailing_whitespace_hidden_with_ignore_ws() {
    let left  = load("whitespace/left_mixed_trailing.txt");
    let right = load("whitespace/right_clean.txt");
    let doc = compute_diff(&left, &right, opts_ignore_ws());
    assert!(doc.is_identical(),
        "trailing spaces and tabs must be ignored with ignore_whitespace");
}

// ── Large files: context collapsing ──────────────────────────────────────────

#[test]
fn large_equal_files_are_identical() {
    let left  = load("text/large_equal_left.txt");
    let right = load("text/large_equal_right.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(doc.is_identical(), "200-line identical files must produce no changes");
}

#[test]
fn large_file_with_one_change_produces_one_hunk() {
    let left  = load("text/large_one_change_left.txt");
    let right = load("text/large_one_change_right.txt");
    let doc = compute_diff(&left, &right, opts_default());
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 1,
        "200-line file with one changed line must produce exactly one changed hunk");
    // Equal context should surround the change but not be reported as changed
    assert_eq!(doc.stats.lines_inserted + doc.stats.lines_deleted, 2,
        "one replace = one deleted + one inserted");
}

// ── FileKind classification of corpus fixtures ────────────────────────────────

#[test]
fn binary_fixture_classifies_as_binary() {
    use std::path::Path;
    use forskscope_core::file_kind::{FileKind, classify};
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/text/binary_nul.bin");
    let kind = classify(&path).expect("classify must not error");
    assert_eq!(kind, FileKind::Binary,
        "file with NUL byte must classify as Binary");
}

#[test]
fn text_fixtures_classify_as_text() {
    use std::path::Path;
    use forskscope_core::file_kind::{FileKind, classify};
    for name in &["left_identical.txt", "left_unicode.txt", "utf8_bom.txt"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/text")
            .join(name);
        let kind = classify(&path).expect("classify must succeed");
        assert_eq!(kind, FileKind::Text,
            "fixture {name} must classify as Text");
    }
}

// Helper: compare without requiring ownership
fn doc_identical(left: &str, right: &str) -> bool {
    compute_diff(left, right, DiffOptions::default()).is_identical()
}
