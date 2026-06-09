use crate::diff::{
    DiffOptions, DiffWarning, HunkKind, InlineKind, InlineMode, NewlineMarker, compute_diff,
    inline_diff_rows,
};

#[test]
fn equal_files_produce_no_changed_hunks() {
    let doc = compute_diff("a\nb\nc\n", "a\nb\nc\n", DiffOptions::default());
    assert!(doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 0);
}

#[test]
fn insert_delete_replace_ranges_are_correct() {
    let left = "keep\nold\nkeep2\n";
    let right = "keep\nnew\nkeep2\nadded\n";
    let doc = compute_diff(left, right, DiffOptions::default());
    let kinds: Vec<HunkKind> = doc.hunks.iter().map(|h| h.kind).collect();
    assert!(kinds.contains(&HunkKind::Replace));
    assert!(kinds.contains(&HunkKind::Insert));

    let replace = doc.hunks.iter().find(|h| h.kind == HunkKind::Replace).unwrap();
    // "old" is line 2 on the left, "new" is line 2 on the right.
    assert_eq!(replace.left_range.start, 2);
    assert_eq!(replace.right_range.start, 2);
}

#[test]
fn hunk_ids_are_unique_and_stable_within_a_document() {
    let doc = compute_diff("a\nx\nb\n", "a\ny\nb\n", DiffOptions::default());
    let mut ids: Vec<u64> = doc.hunks.iter().map(|h| h.hunk_id).collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "hunk ids must be unique");

    // Looking a hunk up by id returns the same hunk.
    let first = &doc.hunks[0];
    assert_eq!(doc.hunk(first.hunk_id).unwrap().kind, first.kind);
}

#[test]
fn newline_markers_are_preserved_per_line() {
    let doc = compute_diff("a\r\nb\n", "a\r\nb\n", DiffOptions::default());
    let markers: Vec<NewlineMarker> = doc
        .hunks
        .iter()
        .flat_map(|h| h.rows.iter())
        .filter_map(|r| r.left.as_ref().map(|l| l.newline))
        .collect();
    assert!(markers.contains(&NewlineMarker::CrLf));
    assert!(markers.contains(&NewlineMarker::Lf));
}

#[test]
fn crlf_vs_lf_only_change_is_detected() {
    let doc = compute_diff("a\n", "a\r\n", DiffOptions::default());
    assert!(!doc.is_identical(), "newline-style change must be visible");
}

#[test]
fn ignore_whitespace_option_collapses_whitespace_only_change() {
    let opts = DiffOptions {
        ignore_whitespace: true,
        ..DiffOptions::default()
    };
    let doc = compute_diff("a  b\n", "a b\n", opts);
    assert!(doc.is_identical());
}

#[test]
fn inline_spans_are_unicode_safe_for_multibyte_text() {
    let opts = DiffOptions {
        inline_mode: InlineMode::EagerForSmallHunks,
        ..DiffOptions::default()
    };
    let mut doc = compute_diff("あいう\n", "あXう\n", opts);
    let hunk = doc.hunks.iter_mut().find(|h| h.kind == HunkKind::Replace).unwrap();
    inline_diff_rows(hunk, 4096);
    let inline = hunk.rows[0].inline.as_ref().unwrap();
    // The shared prefix/suffix must remain intact, proving char-boundary safety.
    let left_equal: String = inline
        .left_spans
        .iter()
        .filter(|s| s.kind == InlineKind::Equal)
        .map(|s| s.text.clone())
        .collect();
    assert!(left_equal.contains('あ'));
    assert!(left_equal.contains('う'));
}

#[test]
fn large_file_policy_disables_inline_and_warns() {
    let big = "x\n".repeat(10);
    let opts = DiffOptions {
        max_file_bytes_for_full_diff: 4,
        inline_mode: InlineMode::EagerForSmallHunks,
        ..DiffOptions::default()
    };
    let doc = compute_diff(&big, &"y\n".repeat(10), opts);
    assert!(doc.warnings.contains(&DiffWarning::LargeFilePolicyApplied));
}

// ── New tests for v0.32.0 ─────────────────────────────────────────────────────

#[test]
fn ignore_case_collapses_case_only_change() {
    let opts = DiffOptions { ignore_case: true, ..DiffOptions::default() };
    let doc = compute_diff("Hello World\n", "hello world\n", opts);
    assert!(doc.is_identical(),
        "case-only change should be invisible when ignore_case is set");
    assert_eq!(doc.stats.hunks_changed, 0);
}

#[test]
fn ignore_case_does_not_hide_content_change() {
    let opts = DiffOptions { ignore_case: true, ..DiffOptions::default() };
    let doc = compute_diff("Hello World\n", "hello Rust\n", opts);
    assert!(!doc.is_identical(), "'World' vs 'Rust' differs even case-insensitively");
}

#[test]
fn histogram_algorithm_finds_same_change_as_myers() {
    use crate::diff::DiffAlgorithm;
    let left  = "a\nb\nc\nd\n";
    let right = "a\nB\nc\nd\n";
    let myers_doc  = compute_diff(left, right, DiffOptions { algorithm: DiffAlgorithm::Myers,     ..DiffOptions::default() });
    let hist_doc   = compute_diff(left, right, DiffOptions { algorithm: DiffAlgorithm::Histogram, ..DiffOptions::default() });
    assert_eq!(myers_doc.stats.hunks_changed, hist_doc.stats.hunks_changed,
        "both algorithms should detect the same number of changed hunks");
    assert_eq!(myers_doc.stats.lines_deleted, hist_doc.stats.lines_deleted);
    assert_eq!(myers_doc.stats.lines_inserted, hist_doc.stats.lines_inserted);
}

#[test]
fn patience_algorithm_finds_same_change_as_myers() {
    use crate::diff::DiffAlgorithm;
    let left  = "fn foo() {\n    42\n}\n";
    let right = "fn foo() {\n    99\n}\n";
    let myers   = compute_diff(left, right, DiffOptions { algorithm: DiffAlgorithm::Myers,   ..DiffOptions::default() });
    let patience = compute_diff(left, right, DiffOptions { algorithm: DiffAlgorithm::Patience, ..DiffOptions::default() });
    assert_eq!(myers.stats.hunks_changed, patience.stats.hunks_changed);
}

#[test]
fn both_empty_files_are_identical() {
    let doc = compute_diff("", "", DiffOptions::default());
    assert!(doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 0);
    assert_eq!(doc.stats.lines_inserted, 0);
    assert_eq!(doc.stats.lines_deleted, 0);
}

#[test]
fn left_empty_right_non_empty_is_pure_insert() {
    let doc = compute_diff("", "line1\nline2\n", DiffOptions::default());
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.lines_deleted, 0, "nothing deleted from empty left");
    assert_eq!(doc.stats.lines_inserted, 2);
}

#[test]
fn right_empty_left_non_empty_is_pure_delete() {
    let doc = compute_diff("line1\nline2\n", "", DiffOptions::default());
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.lines_inserted, 0, "nothing inserted into empty right");
    assert_eq!(doc.stats.lines_deleted, 2);
}

#[test]
fn diff_stats_count_changed_lines_correctly() {
    // 3 lines: keep, replace, keep → 1 deleted + 1 inserted.
    let doc = compute_diff("keep\nold\nkeep\n", "keep\nnew\nkeep\n", DiffOptions::default());
    assert_eq!(doc.stats.hunks_changed, 1);
    assert_eq!(doc.stats.lines_deleted,  1);
    assert_eq!(doc.stats.lines_inserted, 1);
}

#[test]
fn multi_block_changes_are_each_counted() {
    let left  = "a\nX\nb\nY\nc\n";
    let right = "a\nX2\nb\nY2\nc\n";
    let doc = compute_diff(left, right, DiffOptions::default());
    assert_eq!(doc.stats.hunks_changed, 2, "two separate changed blocks");
}



#[test]
fn ignore_whitespace_plus_ignore_case_both_apply() {
    let opts = DiffOptions { ignore_whitespace: true, ignore_case: true, ..DiffOptions::default() };
    // Only difference is case + trailing space — should be invisible.
    let doc = compute_diff("Hello  \n", "hello\n", opts);
    assert!(doc.is_identical());
}

#[test]
fn no_trailing_newline_handled_gracefully() {
    // Files without trailing newline should still diff correctly.
    let doc = compute_diff("line1\nline2", "line1\nline2", DiffOptions::default());
    assert!(doc.is_identical());
}

#[test]
fn single_line_no_newline_change() {
    let doc = compute_diff("hello", "world", DiffOptions::default());
    assert!(!doc.is_identical());
    assert_eq!(doc.stats.hunks_changed, 1);
}
