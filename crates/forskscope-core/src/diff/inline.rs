//! Inline (character-level) refinement (RFC-002 §5.2, §9).
//!
//! Inline spans are decorations derived per row pair inside a replace hunk:
//! row `i` on the left is refined against row `i` on the right. Rows
//! without a counterpart get a single whole-line span. Char-level diffing
//! through `similar` operates on `char` boundaries, so spans are
//! Unicode-safe by construction.

use similar::{Algorithm, ChangeTag, TextDiff};

use super::model::{DiffHunk, HunkKind, InlineDiff, InlineKind, InlineSpan};

/// Compute inline spans for every row of a replace hunk, in place.
///
/// Returns `false` (leaving the hunk untouched) when the hunk's combined
/// text exceeds `max_chars`, implementing the bounded-inline policy.
pub fn inline_diff_rows(hunk: &mut DiffHunk, max_chars: usize) -> bool {
    if hunk.kind != HunkKind::Replace {
        return true;
    }
    let total: usize = hunk
        .rows
        .iter()
        .map(|r| {
            r.left.as_ref().map(|l| l.content.chars().count()).unwrap_or(0)
                + r.right.as_ref().map(|l| l.content.chars().count()).unwrap_or(0)
        })
        .sum();
    if total > max_chars {
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
pub fn refine_pair(left: &str, right: &str) -> InlineDiff {
    pairwise_inline(left, right)
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
