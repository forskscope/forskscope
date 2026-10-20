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

/// Two lines of length `n` with **no common prefix or suffix at all** — `x`
/// and `y` differ at every position, so the differing middle is exactly `n`
/// characters on each side, not shrunk by any shared run. The fixture F127's
/// "dissimilar middle" tests need: a pair built from a single repeated
/// character on each side (`line_of(n, 'a')` vs `line_of(n, 'a')` shifted by
/// one, as this module's tests used before F127) looks long but is actually
/// almost entirely a common prefix, which is exactly the cheap case F127
/// exists to stop rejecting — using it to test the "still skipped" path
/// would test nothing.
fn dissimilar_pair_of(n: usize) -> (String, String) {
    (line_of(n, 'x'), line_of(n, 'y'))
}

/// Like [`dissimilar_pair_of`], but the dissimilar run of length `n` sits
/// between a short shared prefix and suffix — so trimming has real work to
/// do on both ends, not just one, and the resulting middle is still exactly
/// `n` on each side. Used to pin the limit's exact boundary.
fn bracketed_dissimilar_middle(n: usize) -> (String, String) {
    (
        format!("PRE{}POST", line_of(n, 'x')),
        format!("PRE{}POST", line_of(n, 'y')),
    )
}

/// F127 (handoff 076 §3): without the fix, a pair over the limit is not
/// merely slow, it **produces spans it must not have produced** (handoff 043
/// §Verification 1) — so this still asserts `refine_pair` on a long,
/// genuinely dissimilar pair (`x`≠`y` at every position, no common run to
/// trim away). Deliberately only 3x the limit (well under a millisecond even
/// un-trimmed) — the 400,000-character case belongs to the release-only
/// timing test below, not this one.
#[test]
fn a_pair_over_the_limit_is_not_refined_and_produces_no_spans() {
    let (left, right) = dissimilar_pair_of(3 * MAX_INLINE_CHARS_PER_SIDE);
    assert!(
        refine_pair(&left, &right).is_none(),
        "a pair three times the limit, with nothing to trim, must not be refined"
    );
}

/// F127: a long line differing in only one character trims down to a tiny
/// middle and must be refined, not skipped — this is the case the whole-line
/// check used to reject wrongly. `3 * MAX` is a comfortable margin past the
/// limit for the *line itself*; only the one-character middle is checked
/// against it.
#[test]
fn a_long_line_differing_in_one_character_is_refined_not_skipped() {
    let base = line_of(3 * MAX_INLINE_CHARS_PER_SIDE, 'a');
    let changed = format!("{}X", &base[1..]); // shifted by one, one char different
    assert!(
        !pair_over_inline_limit(&base, &changed),
        "the differing middle is one character, far under the limit"
    );
    assert!(
        refine_pair(&base, &changed).is_some(),
        "a long line that differs in one place must still be refined"
    );
}

/// F127, multi-byte: Japanese text differing in a single character. The
/// prefix/suffix trim walks `char`s, not bytes, so this must neither panic
/// nor miscount — each "character" here is 3 UTF-8 bytes.
#[test]
fn a_long_japanese_line_differing_in_one_character_is_refined_not_panicking() {
    let base: String = "あ".repeat(3 * MAX_INLINE_CHARS_PER_SIDE);
    let mut chars: Vec<char> = base.chars().collect();
    let mid = chars.len() / 2;
    chars[mid] = 'い';
    let changed: String = chars.into_iter().collect();

    assert!(!pair_over_inline_limit(&base, &changed));
    let diff = refine_pair(&base, &changed).expect("must be refined, not skipped");
    assert!(
        diff.right_spans
            .iter()
            .any(|s| s.kind == InlineKind::Insert || s.kind == InlineKind::Delete)
            || diff.left_spans.iter().any(|s| s.kind == InlineKind::Delete),
        "the one-character change must show up as a span, not be lost"
    );
}

/// F127 (handoff 076 §3, test plan item 1): the 400,000-character case —
/// the exact length that aborted the process before F120's guard existed —
/// differing in one character in the middle, must be highlighted and must
/// be fast. `#[ignore]`d: timing assertions do not belong in the default
/// suite (flaky under CI load, and a debug build's constant-factor noise
/// could trip a tight bound for reasons unrelated to the fix). Run with
/// `cargo test --release -p forskscope-core -- --ignored a_400_000`.
#[test]
#[ignore = "timing assertion; run explicitly in release mode"]
fn a_400_000_character_line_differing_in_one_character_is_fast() {
    let base = line_of(400_000, 'a');
    let changed = format!("{}X{}", &base[..200_000], &base[200_001..]);

    let start = std::time::Instant::now();
    let diff = refine_pair(&base, &changed).expect("must be refined, not skipped");
    let elapsed = start.elapsed();

    assert!(
        diff.right_spans
            .iter()
            .any(|s| s.kind == InlineKind::Insert)
            || diff.left_spans.iter().any(|s| s.kind == InlineKind::Delete),
        "the one-character change must show up as a span"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "expected well under 50ms for a one-character middle, took {elapsed:?}"
    );
}

/// The bound is on the assertion, not on wall-clock, so it cannot be flaky:
/// the limit is exact and asserted at the boundary, in characters (not
/// bytes) — against the differing middle (F127), using
/// [`bracketed_dissimilar_middle`] so the boundary is real and not shrunk by
/// an accidental common run.
#[test]
fn the_limit_is_exact_on_the_differing_middle_and_counts_characters_not_bytes() {
    let (left_at, right_at) = bracketed_dissimilar_middle(MAX_INLINE_CHARS_PER_SIDE);
    let (left_over, right_over) = bracketed_dissimilar_middle(MAX_INLINE_CHARS_PER_SIDE + 1);
    assert!(
        refine_pair(&left_at, &right_at).is_some(),
        "a middle exactly at the limit is refined"
    );
    assert!(
        refine_pair(&left_over, &right_over).is_none(),
        "a middle one over the limit is not refined"
    );
    assert!(pair_over_inline_limit(&left_over, &right_over));
    assert!(!pair_over_inline_limit(&left_at, &right_at));

    // 2,000 three-byte characters are 6,000 bytes but 2,000 characters.
    let wide_left = format!("PRE{}POST", "あ".repeat(MAX_INLINE_CHARS_PER_SIDE));
    let wide_right = format!("PRE{}POST", "い".repeat(MAX_INLINE_CHARS_PER_SIDE));
    assert!(wide_left.len() > MAX_INLINE_CHARS_PER_SIDE);
    assert!(refine_pair(&wide_left, &wide_right).is_some());
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
    let (long_left, long_right) = dissimilar_pair_of(MAX_INLINE_CHARS_PER_SIDE + 1);
    let left = format!("short one\n{long_left}\n");
    let right = format!("short two\n{long_right}\n");
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
    let (long_left, long_right) = dissimilar_pair_of(MAX_INLINE_CHARS_PER_SIDE + 1);
    let left = format!("short one\n{long_left}\nkeep\n");
    let right = format!("short two\n{long_right}\nkeep\n");
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
