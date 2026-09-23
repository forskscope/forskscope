//! Inline (character-level) refinement (RFC-002 §5.2, §9).
//!
//! Inline spans are decorations derived per row pair inside a replace hunk:
//! row `i` on the left is refined against row `i` on the right. Rows
//! without a counterpart get a single whole-line span. Char-level diffing
//! through `similar` operates on `char` boundaries, so spans are
//! Unicode-safe by construction.

use similar::{Algorithm, ChangeTag, TextDiff};

use super::model::{DiffDocument, DiffHunk, HunkKind, InlineDiff, InlineKind, InlineSpan};

/// The one limit on character-level refinement (F120): the most characters
/// either side of one changed line pair may have.
///
/// **Basis, measured on a release build** (two lines of the stated length, ten
/// edits; unrelated lines cost the same to within 10%):
///
/// | characters per side | time per pair |
/// |---|---|
/// | 200 | 0.09 ms |
/// | 1,000 | 0.96 ms |
/// | 2,000 | 3.3 ms |
/// | 4,000 | 13 ms |
/// | 20,000 | 0.38 s |
/// | 100,000 | 11 s |
/// | 400,000 | aborts the process (a 518 GB allocation request) |
///
/// Cost is quadratic in the line length, and the diff view refines every
/// changed pair it renders (nothing in it is virtualised), so the limit has to
/// be survivable for a screenful, not for one pair. An ordinary source line
/// (under 200 characters) is far below it. A pair over the limit is **not
/// refined**: [`refine_pair`] returns `None` and callers must show that the
/// pair was skipped, never render it as "no character-level differences".
///
/// This is the only such number. `PerformanceLimits::max_inline_diff_chars_per_hunk`
/// takes its default from it, and the engine's own bound (there was a second,
/// 16 KiB per hunk, applied only under `InlineMode::EagerForSmallHunks`) was
/// removed in favour of it.
pub const MAX_INLINE_CHARS_PER_SIDE: usize = 2_000;

/// `true` when `s` has more than [`MAX_INLINE_CHARS_PER_SIDE`] characters.
/// Stops counting at the limit, so checking a 400,000-character line is as
/// cheap as checking a 2,001-character one.
fn over_limit(s: &str) -> bool {
    s.chars().nth(MAX_INLINE_CHARS_PER_SIDE).is_some()
}

/// Whether a changed line pair is too long to refine character by character.
pub fn pair_over_inline_limit(left: &str, right: &str) -> bool {
    over_limit(left) || over_limit(right)
}

/// Compute inline spans for every row of a replace hunk, in place.
///
/// Returns `false` (leaving the hunk untouched) when any row of the hunk has a
/// side over [`MAX_INLINE_CHARS_PER_SIDE`].
pub fn inline_diff_rows(hunk: &mut DiffHunk) -> bool {
    if hunk.kind != HunkKind::Replace {
        return true;
    }
    let too_long = hunk.rows.iter().any(|r| {
        r.left.as_ref().is_some_and(|l| over_limit(&l.content))
            || r.right.as_ref().is_some_and(|l| over_limit(&l.content))
    });
    if too_long {
        return false;
    }
    for row in &mut hunk.rows {
        let left = row.left.as_ref().map(|l| l.content.clone());
        let right = row.right.as_ref().map(|l| l.content.clone());
        row.inline = Some(match (left, right) {
            (Some(l), Some(r)) => pairwise_inline(&l, &r),
            (Some(l), None) => InlineDiff {
                left_spans: whole_line(&l, InlineKind::Delete),
                right_spans: Vec::new(),
            },
            (None, Some(r)) => InlineDiff {
                left_spans: Vec::new(),
                right_spans: whole_line(&r, InlineKind::Insert),
            },
            (None, None) => InlineDiff::default(),
        });
    }
    true
}

fn whole_line(content: &str, kind: InlineKind) -> Vec<InlineSpan> {
    if content.is_empty() {
        Vec::new()
    } else {
        vec![InlineSpan {
            kind,
            text: content.to_string(),
        }]
    }
}

/// Refine a single left/right line pair into inline spans. Public so the UI
/// can compute character highlights lazily for a focused/visible row without
/// reimplementing diff logic (RFC-002: diff logic stays in core).
///
/// Returns `None`, doing no diff work, when either side is over
/// [`MAX_INLINE_CHARS_PER_SIDE`]. The bound is here, at the only entry the UI
/// has, rather than in each caller, so a new caller cannot forget it.
pub fn refine_pair(left: &str, right: &str) -> Option<InlineDiff> {
    if pair_over_inline_limit(left, right) {
        return None;
    }
    Some(pairwise_inline(left, right))
}

/// How many changed line pairs in `doc` are too long for [`refine_pair`], so a
/// view with character mode on can say that some pairs were skipped.
pub fn skipped_inline_pairs(doc: &DiffDocument) -> usize {
    doc.hunks
        .iter()
        .filter(|h| h.kind == HunkKind::Replace)
        .flat_map(|h| h.rows.iter())
        .filter(|r| match (&r.left, &r.right) {
            (Some(l), Some(rt)) => pair_over_inline_limit(&l.content, &rt.content),
            _ => false,
        })
        .count()
}

fn pairwise_inline(left: &str, right: &str) -> InlineDiff {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Lcs)
        .diff_chars(left, right);
    let mut left_spans: Vec<InlineSpan> = Vec::new();
    let mut right_spans: Vec<InlineSpan> = Vec::new();
    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                push_span(&mut left_spans, InlineKind::Equal, &text);
                push_span(&mut right_spans, InlineKind::Equal, &text);
            }
            ChangeTag::Delete => push_span(&mut left_spans, InlineKind::Delete, &text),
            ChangeTag::Insert => push_span(&mut right_spans, InlineKind::Insert, &text),
        }
    }
    InlineDiff {
        left_spans,
        right_spans,
    }
}

/// Append text, merging with the previous span when the kind matches so the
/// UI renders a compact span list.
fn push_span(spans: &mut Vec<InlineSpan>, kind: InlineKind, text: &str) {
    if let Some(last) = spans.last_mut()
        && last.kind == kind
    {
        last.text.push_str(text);
        return;
    }
    spans.push(InlineSpan {
        kind,
        text: text.to_string(),
    });
}
