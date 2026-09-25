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

    let replace = doc
        .hunks
        .iter()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap();
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
    let hunk = doc
        .hunks
        .iter_mut()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap();
    inline_diff_rows(hunk);
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
    let opts = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    let doc = compute_diff("Hello World\n", "hello world\n", opts);
    assert!(
        doc.is_identical(),
        "case-only change should be invisible when ignore_case is set"
    );
    assert_eq!(doc.stats.hunks_changed, 0);
}

#[test]
fn ignore_case_does_not_hide_content_change() {
    let opts = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    let doc = compute_diff("Hello World\n", "hello Rust\n", opts);
    assert!(
        !doc.is_identical(),
        "'World' vs 'Rust' differs even case-insensitively"
    );
}

#[test]
fn histogram_algorithm_finds_same_change_as_myers() {
    use crate::diff::DiffAlgorithm;
    let left = "a\nb\nc\nd\n";
    let right = "a\nB\nc\nd\n";
    let myers_doc = compute_diff(
        left,
        right,
        DiffOptions {
            algorithm: DiffAlgorithm::Myers,
            ..DiffOptions::default()
        },
    );
    let hist_doc = compute_diff(
        left,
        right,
        DiffOptions {
            algorithm: DiffAlgorithm::Histogram,
            ..DiffOptions::default()
        },
    );
    assert_eq!(
        myers_doc.stats.hunks_changed, hist_doc.stats.hunks_changed,
        "both algorithms should detect the same number of changed hunks"
    );
    assert_eq!(myers_doc.stats.lines_deleted, hist_doc.stats.lines_deleted);
    assert_eq!(
        myers_doc.stats.lines_inserted,
        hist_doc.stats.lines_inserted
    );
}

#[test]
fn patience_algorithm_finds_same_change_as_myers() {
    use crate::diff::DiffAlgorithm;
    let left = "fn foo() {\n    42\n}\n";
    let right = "fn foo() {\n    99\n}\n";
    let myers = compute_diff(
        left,
        right,
        DiffOptions {
            algorithm: DiffAlgorithm::Myers,
            ..DiffOptions::default()
        },
    );
    let patience = compute_diff(
        left,
        right,
        DiffOptions {
            algorithm: DiffAlgorithm::Patience,
            ..DiffOptions::default()
        },
    );
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
    assert_eq!(
        doc.stats.lines_deleted, 0,
        "nothing deleted from empty left"
    );
    assert_eq!(doc.stats.lines_inserted, 2);
}

#[test]
fn right_empty_left_non_empty_is_pure_delete() {
    let doc = compute_diff("line1\nline2\n", "", DiffOptions::default());
    assert!(!doc.is_identical());
    assert_eq!(
        doc.stats.lines_inserted, 0,
        "nothing inserted into empty right"
    );
    assert_eq!(doc.stats.lines_deleted, 2);
}

#[test]
fn diff_stats_count_changed_lines_correctly() {
    // 3 lines: keep, replace, keep → 1 deleted + 1 inserted.
    let doc = compute_diff(
        "keep\nold\nkeep\n",
        "keep\nnew\nkeep\n",
        DiffOptions::default(),
    );
    assert_eq!(doc.stats.hunks_changed, 1);
    assert_eq!(doc.stats.lines_deleted, 1);
    assert_eq!(doc.stats.lines_inserted, 1);
}

#[test]
fn multi_block_changes_are_each_counted() {
    let left = "a\nX\nb\nY\nc\n";
    let right = "a\nX2\nb\nY2\nc\n";
    let doc = compute_diff(left, right, DiffOptions::default());
    assert_eq!(doc.stats.hunks_changed, 2, "two separate changed blocks");
}

#[test]
fn ignore_whitespace_plus_ignore_case_both_apply() {
    let opts = DiffOptions {
        ignore_whitespace: true,
        ignore_case: true,
        ..DiffOptions::default()
    };
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

// ── v0.34.0 additions ─────────────────────────────────────────────────────────

#[test]
fn many_insertions_are_counted_correctly() {
    // Insert 5 lines after a single equal line.
    let right = "keep\na\nb\nc\nd\ne\n";
    let doc = compute_diff("keep\n", right, DiffOptions::default());
    assert_eq!(doc.stats.lines_inserted, 5);
    assert_eq!(doc.stats.lines_deleted, 0);
}

#[test]
fn replace_counts_both_a_deletion_and_an_insertion() {
    // One old line replaced by two new lines → 1 deleted, 2 inserted.
    let doc = compute_diff("one\n", "two\nthree\n", DiffOptions::default());
    assert_eq!(doc.stats.lines_deleted, 1);
    assert_eq!(doc.stats.lines_inserted, 2);
}

#[test]
fn completely_different_files_have_full_counts() {
    let left = "a\nb\nc\n";
    let right = "x\ny\nz\n";
    let doc = compute_diff(left, right, DiffOptions::default());
    // All three lines replaced → 3 deleted, 3 inserted.
    assert_eq!(doc.stats.lines_deleted, 3);
    assert_eq!(doc.stats.lines_inserted, 3);
}

#[test]
fn ignore_whitespace_does_not_hide_non_whitespace_change() {
    let opts = DiffOptions {
        ignore_whitespace: true,
        ..DiffOptions::default()
    };
    let doc = compute_diff("  hello\n", "  world\n", opts);
    assert!(
        !doc.is_identical(),
        "non-whitespace content change must still be visible"
    );
}

#[test]
fn context_lines_warning_absent_for_small_file() {
    use crate::diff::DiffWarning;
    let left = "line1\nline2\nline3\n";
    let right = "line1\nLINE2\nline3\n";
    let doc = compute_diff(left, right, DiffOptions::default());
    assert!(
        !doc.warnings.contains(&DiffWarning::LargeFilePolicyApplied),
        "small file must not trigger the large-file policy warning"
    );
}

// RFC-086 §6 falsification 1: identical recompute must preserve hunk ids —
// falsify by reintroducing a process-global counter into `hunk_id_for`'s
// hash, and this must fail.
#[test]
fn identical_recompute_produces_identical_hunk_ids() {
    let left = "a\nb\n";
    let right = "a\nc\n";
    let d1 = compute_diff(left, right, DiffOptions::default());
    let d2 = compute_diff(left, right, DiffOptions::default());
    let ids1: Vec<_> = d1.hunks.iter().map(|h| h.hunk_id).collect();
    let ids2: Vec<_> = d2.hunks.iter().map(|h| h.hunk_id).collect();
    assert_eq!(
        ids1, ids2,
        "recomputing the same document with unchanged options must yield \
         identical hunk ids, or MergeSession's undo/redo log can never \
         survive a recompute (F47, RFC-086)"
    );
}

// ── F120: the one character-level limit ──────────────────────────────────────

use crate::diff::{
    MAX_INLINE_CHARS_PER_SIDE, pair_over_inline_limit, refine_pair, skipped_inline_pairs,
};

fn line_of(n: usize, ch: char) -> String {
    std::iter::repeat_n(ch, n).collect()
}

/// The case that matters (handoff 043 §Verification 1): without the guard a
/// pair over the limit is not merely slow, it **produces spans it must not
/// have produced**. Deliberately only 3x the limit (about 30 ms in a release
/// build) — the 400,000-character case that aborted the process is not run
/// here, or anywhere in CI. Falsified for real: making `refine_pair` skip the
/// `pair_over_inline_limit` check returns `Some` with spans and this fails.
#[test]
fn a_pair_over_the_limit_is_not_refined_and_produces_no_spans() {
    let left = line_of(3 * MAX_INLINE_CHARS_PER_SIDE, 'a');
    let right = format!("{}X", &left[1..]);
    assert!(
        refine_pair(&left, &right).is_none(),
        "a pair three times the limit must not be refined"
    );
}

/// The bound is on the assertion, not on wall-clock, so it cannot be flaky:
/// the limit is exact and asserted at the boundary, in characters (not bytes).
#[test]
fn the_limit_is_exact_per_side_and_counts_characters_not_bytes() {
    let at = line_of(MAX_INLINE_CHARS_PER_SIDE, 'a');
    let over = line_of(MAX_INLINE_CHARS_PER_SIDE + 1, 'a');
    assert!(
        refine_pair(&at, &at).is_some(),
        "exactly at the limit is refined"
    );
    assert!(refine_pair(&over, &at).is_none(), "left one over");
    assert!(refine_pair(&at, &over).is_none(), "right one over");
    assert!(pair_over_inline_limit(&at, &over));
    assert!(!pair_over_inline_limit(&at, &at));

    // 2,000 three-byte characters are 6,000 bytes but 2,000 characters.
    let wide = line_of(MAX_INLINE_CHARS_PER_SIDE, 'あ');
    assert!(wide.len() > MAX_INLINE_CHARS_PER_SIDE);
    assert!(refine_pair(&wide, &wide).is_some());
}

#[test]
fn an_ordinary_source_line_still_gets_character_level_highlighting() {
    let d = refine_pair(
        "let total = price * quantity;",
        "let total = price * quantity + tax;",
    )
    .expect("a normal line is refined");
    assert!(d.right_spans.iter().any(|s| s.kind == InlineKind::Insert));
    assert!(d.left_spans.iter().any(|s| s.kind == InlineKind::Equal));
}

#[test]
fn a_hunk_with_one_over_limit_row_is_left_untouched_by_inline_diff_rows() {
    let long = line_of(MAX_INLINE_CHARS_PER_SIDE + 1, 'a');
    let left = format!("short one\n{long}\n");
    let right = format!("short two\n{long}b\n");
    let mut doc = compute_diff(&left, &right, DiffOptions::default());
    let hunk = doc
        .hunks
        .iter_mut()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap();
    assert!(!inline_diff_rows(hunk), "must report that it skipped");
    assert!(
        hunk.rows.iter().all(|r| r.inline.is_none()),
        "and must not have refined any row of the hunk"
    );
}

#[test]
fn skipped_inline_pairs_counts_only_over_limit_two_sided_pairs() {
    let long = line_of(MAX_INLINE_CHARS_PER_SIDE + 1, 'a');
    let left = format!("short one\n{long}\nkeep\n");
    let right = format!("short two\n{long}b\nkeep\n");
    let doc = compute_diff(&left, &right, DiffOptions::default());
    let over = skipped_inline_pairs(&doc);
    assert_eq!(over, 1, "the long pair is counted, the short one is not");

    let short = compute_diff("a\nb\n", "a\nc\n", DiffOptions::default());
    assert_eq!(skipped_inline_pairs(&short), 0);
}

/// Three numbers existed and none was in force (F120). There is one.
#[test]
fn the_persisted_inline_limit_defaults_to_the_one_the_renderer_obeys() {
    assert_eq!(
        crate::job::PerformanceLimits::default().max_inline_diff_chars_per_hunk,
        MAX_INLINE_CHARS_PER_SIDE
    );
}

/// Two texts of about a thousand lines each, built so that `similar` 3.2.0's
/// `Myers` and `RawMyers` disagree (F124). Deterministic; no fixture file.
///
/// The disagreement needs an expensive search: none turned up in 200,000
/// random pairs under a hundred and sixty lines, and the first appeared near a
/// thousand. On 856 real file pairs from this repository's history the new
/// `Myers` changed 7, `RawMyers` none.
fn myers_separating_pair() -> (String, String) {
    let mut state: u64 = 435 * 7919 + 1;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state >> 33
    };
    let len = 200 + (next() % 1800) as usize;
    let alphabet = 3 + next() % 20;
    let a: Vec<u64> = (0..len).map(|_| next() % alphabet).collect();
    let mut b = a.clone();
    let edits = 5 + next() % (len as u64 / 2);
    for _ in 0..edits {
        let i = (next() as usize) % b.len();
        match next() % 3 {
            0 => {
                b.remove(i);
            }
            1 => b.insert(i, next() % alphabet),
            _ => b[i] = next() % alphabet,
        }
        if b.is_empty() {
            b.push(0);
        }
    }
    let text = |v: &[u64]| v.iter().map(|n| format!("v{n}\n")).collect::<String>();
    (text(&a), text(&b))
}

/// The default line diff is `RawMyers`, so a `similar` upgrade cannot change the
/// hunks users apply and export (F124; F128 is the open question of adopting the
/// new `Myers` on purpose).
#[test]
fn the_myers_line_diff_is_the_shortest_edit_script_not_the_git_style_split() {
    use similar::{Algorithm, DiffOp, capture_diff_slices};

    let (left, right) = myers_separating_pair();
    let a: Vec<&str> = left.lines().collect();
    let b: Vec<&str> = right.lines().collect();
    // The lines a script edits: old-side indices removed, new-side indices added.
    let edited = |algorithm| -> (Vec<usize>, Vec<usize>) {
        let mut lines = (Vec::new(), Vec::new());
        for op in capture_diff_slices(algorithm, &a, &b) {
            match op {
                DiffOp::Equal { .. } => {}
                DiffOp::Delete {
                    old_index, old_len, ..
                } => lines.0.extend(old_index..old_index + old_len),
                DiffOp::Insert {
                    new_index, new_len, ..
                } => lines.1.extend(new_index..new_index + new_len),
                DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => {
                    lines.0.extend(old_index..old_index + old_len);
                    lines.1.extend(new_index..new_index + new_len);
                }
            }
        }
        lines
    };
    let raw = edited(Algorithm::RawMyers);
    // This input is only evidence if it separates the two: the new Myers takes a
    // non-minimal split here, so it edits *different* lines.
    assert_ne!(
        edited(Algorithm::Myers),
        raw,
        "the pair no longer separates Myers from RawMyers; find another"
    );

    let doc = compute_diff(&left, &right, DiffOptions::default());
    let mut shown: (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for hunk in doc.hunks.iter().filter(|h| h.kind != HunkKind::Equal) {
        for row in &hunk.rows {
            if let Some(l) = &row.left {
                shown.0.push(l.original_line_number.unwrap() as usize - 1);
            }
            if let Some(r) = &row.right {
                shown.1.push(r.original_line_number.unwrap() as usize - 1);
            }
        }
    }
    assert_eq!(
        shown, raw,
        "the default line diff must edit exactly the lines the shortest script edits"
    );
}
