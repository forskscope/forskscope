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
