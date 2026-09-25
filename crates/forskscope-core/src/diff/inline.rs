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
/// **Basis, measured on a release build with the `similar` this workspace
/// locks (3.2.0)**, two lines of the stated length with several edits spread
/// through them (an independent, dissimilar pair costs the same to within 10%):
///
/// | characters per side | time per pair | transient table |
/// |---|---|---|
/// | 500 | 0.3–0.4 ms | 1 MB |
/// | 1,000 | 1.1–1.4 ms | 4 MB |
/// | 2,000 | 4.3 ms | 16 MB |
/// | 4,000 | 16–19 ms | 64 MB |
///
/// A pair with one edit is far cheaper (55 µs at 2,000): cost follows the length
/// of the differing middle, not of the line. **Both columns are quadratic**, and
/// they are why the limit is load-bearing on either version of the library — on
/// 3.1.1, which this workspace locked until F124, the *time* was the problem
/// (225 ms per pair at 2,000, 250 pairs of 1,900 characters took 49 s from the
/// toggle to the first paint); on 3.2.0 the time is 50× smaller but the LCS table
/// is a flat `n × m × 4` bytes, so the *memory* is quadratic (400,000 characters
/// would ask for hundreds of gigabytes, which is the abort F120 found).
///
/// The diff view refines every changed pair it renders (nothing in it is
/// virtualised), so what matters is the sum over a document: on 3.2.0, 250
/// pairs of 1,900 characters take 1.3 s from the toggle to the first paint and
/// 1,000 pairs take 4.9 s. **2,000 is where a pair crosses about 5 ms.** An
/// ordinary source line (under 200 characters) is 0.1 ms. A pair over the limit
/// is **not refined**: [`refine_pair`] returns `None` and callers must show that
/// the pair was skipped, never render it as "no character-level differences".
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
