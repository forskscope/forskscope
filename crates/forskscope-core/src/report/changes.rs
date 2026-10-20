//! Windowed, content-carrying change hunks for the HTML/Markdown-ready
//! report (RFC-092 §3 item 5) — unlike `HunkSummaryRow` (range-only), each
//! line here carries its real content, its original line number on each
//! side, and, for a changed line where the diff computed it, character-level
//! highlight spans.
//!
//! This is a **sibling** to `patch::build`'s unified-diff hunk builder, not
//! a reuse of it: `patch/` is explicitly unchanged by RFC-092, and its
//! `PatchLine` has no slot for inline spans or original line numbers. The
//! windowing rule — keep at most `context_lines` of unchanged context on
//! either side of a change run, merge runs whose context regions touch — is
//! the same one `patch::build::coalesce` implements; this module
//! reimplements it independently, against the same `DiffDocument`, so a
//! change to one never silently changes the other.

use forskscope_compare::diff::{DiffDocument, DiffRow, HunkKind, InlineSpan};

/// The role of one line within a [`ChangeHunk`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeLineKind {
    /// Unchanged, present on both sides.
    Context,
    /// Present on the left/old side only.
    Delete,
    /// Present on the right/new side only.
    Insert,
}

/// One rendered line: its real content, its line number on whichever
/// side(s) it belongs to, and character-level highlight spans when the
/// diff computed them (never for a `Context` line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeLine {
    pub kind: ChangeLineKind,
    pub left_line: Option<u32>,
    pub right_line: Option<u32>,
    pub content: String,
    pub spans: Option<Vec<InlineSpan>>,
}

/// One contiguous unified-layout hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeHunk {
    pub old_start: u32,
    pub old_len: u32,
    pub new_start: u32,
    pub new_len: u32,
    pub lines: Vec<ChangeLine>,
}

struct FlatLine {
    kind: ChangeLineKind,
    left_line: Option<u32>,
    right_line: Option<u32>,
    content: String,
    spans: Option<Vec<InlineSpan>>,
}

/// Build windowed change hunks for one file's diff — empty when the files
/// are identical.
pub fn change_hunks_from_diff(diff: &DiffDocument, context_lines: usize) -> Vec<ChangeHunk> {
    let flat = flatten(diff);
    coalesce(&flat, context_lines)
}

fn flatten(diff: &DiffDocument) -> Vec<FlatLine> {
    let mut out = Vec::new();
    for hunk in &diff.hunks {
        match hunk.kind {
            HunkKind::Equal => {
                for row in &hunk.rows {
                    push_context(&mut out, row);
                }
            }
            HunkKind::Insert | HunkKind::Delete | HunkKind::Replace => {
                // Deletions first, then insertions — same convention
                // `patch::build::flatten` uses, so a Replace renders as a
                // `-`-block followed by a `+`-block.
                for row in &hunk.rows {
                    if let Some(left) = &row.left {
                        out.push(FlatLine {
                            kind: ChangeLineKind::Delete,
                            left_line: left.original_line_number,
                            right_line: None,
                            content: left.content.clone(),
                            spans: row.inline.as_ref().map(|i| i.left_spans.clone()),
                        });
                    }
                }
                for row in &hunk.rows {
                    if let Some(right) = &row.right {
                        out.push(FlatLine {
                            kind: ChangeLineKind::Insert,
                            left_line: None,
                            right_line: right.original_line_number,
                            content: right.content.clone(),
                            spans: row.inline.as_ref().map(|i| i.right_spans.clone()),
                        });
                    }
                }
            }
        }
    }
    out
}

fn push_context(out: &mut Vec<FlatLine>, row: &DiffRow) {
    let left_line = row.left.as_ref().and_then(|l| l.original_line_number);
    let right_line = row.right.as_ref().and_then(|r| r.original_line_number);
    let content = row
        .left
        .as_ref()
        .map(|l| l.content.clone())
        .or_else(|| row.right.as_ref().map(|r| r.content.clone()))
        .unwrap_or_default();
    out.push(FlatLine {
        kind: ChangeLineKind::Context,
        left_line,
        right_line,
        content,
        spans: None,
    });
}

/// Group the flat line stream into unified-layout hunks, keeping at most
/// `context` context lines on either side of each change run and merging
/// runs separated by `<= 2 * context` context lines — the same windowing
/// `patch::build::coalesce` performs. `old_start`/`new_start` are tracked
/// with running counters (not read off individual lines) so a hunk made
/// entirely of insertions or deletions at a file boundary — where no line
/// in the window carries a number on the other side — still gets a correct
/// header, exactly as `patch::build` does via its own `advance`/
/// `normalize_range`.
fn coalesce(flat: &[FlatLine], context: usize) -> Vec<ChangeHunk> {
    let change_idx: Vec<usize> = flat
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind != ChangeLineKind::Context)
        .map(|(i, _)| i)
        .collect();
    if change_idx.is_empty() {
        return Vec::new();
    }

    let mut windows: Vec<(usize, usize)> = Vec::new();
    for &ci in &change_idx {
        let lo = ci.saturating_sub(context);
        let hi = (ci + context).min(flat.len().saturating_sub(1));
        match windows.last_mut() {
            Some((_, prev_hi)) if lo <= *prev_hi + 1 => {
                if hi > *prev_hi {
                    *prev_hi = hi;
                }
            }
            _ => windows.push((lo, hi)),
        }
    }

    let mut old_line: u32 = 1;
    let mut new_line: u32 = 1;
    let mut cursor = 0usize;

    let mut hunks = Vec::with_capacity(windows.len());
    for (lo, hi) in windows {
        while cursor < lo {
            advance(&flat[cursor], &mut old_line, &mut new_line);
            cursor += 1;
        }
        let old_start = old_line;
        let new_start = new_line;
        let mut lines = Vec::with_capacity(hi - lo + 1);
        let mut old_len = 0u32;
        let mut new_len = 0u32;
        for fl in &flat[lo..=hi] {
            match fl.kind {
                ChangeLineKind::Context => {
                    old_len += 1;
                    new_len += 1;
                }
                ChangeLineKind::Delete => old_len += 1,
                ChangeLineKind::Insert => new_len += 1,
            }
            lines.push(ChangeLine {
                kind: fl.kind,
                left_line: fl.left_line,
                right_line: fl.right_line,
                content: fl.content.clone(),
                spans: fl.spans.clone(),
            });
            advance(fl, &mut old_line, &mut new_line);
        }
        cursor = hi + 1;
        let (old_start, old_len) = normalize_range(old_start, old_len);
        let (new_start, new_len) = normalize_range(new_start, new_len);
        hunks.push(ChangeHunk {
            old_start,
            old_len,
            new_start,
            new_len,
            lines,
        });
    }
    hunks
}

fn advance(line: &FlatLine, old_line: &mut u32, new_line: &mut u32) {
    match line.kind {
        ChangeLineKind::Context => {
            *old_line += 1;
            *new_line += 1;
        }
        ChangeLineKind::Delete => *old_line += 1,
        ChangeLineKind::Insert => *new_line += 1,
    }
}

fn normalize_range(start: u32, len: u32) -> (u32, u32) {
    if len == 0 {
        (start.saturating_sub(1), 0)
    } else {
        (start, len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_compare::diff::{DiffOptions, InlineMode, compute_diff};

    fn diff_of(left: &str, right: &str) -> DiffDocument {
        compute_diff(left, right, DiffOptions::default())
    }

    #[test]
    fn identical_files_produce_no_change_hunks() {
        let diff = diff_of("a\nb\nc\n", "a\nb\nc\n");
        assert!(change_hunks_from_diff(&diff, 3).is_empty());
    }

    #[test]
    fn a_single_line_replace_carries_real_line_numbers_on_both_sides() {
        let diff = diff_of("one\ntwo\nthree\n", "one\nTWO\nthree\n");
        let hunks = change_hunks_from_diff(&diff, 1);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.new_start, 1);
        let delete = hunk
            .lines
            .iter()
            .find(|l| l.kind == ChangeLineKind::Delete)
            .expect("a delete line");
        assert_eq!(delete.left_line, Some(2));
        assert_eq!(delete.content, "two");
        let insert = hunk
            .lines
            .iter()
            .find(|l| l.kind == ChangeLineKind::Insert)
            .expect("an insert line");
        assert_eq!(insert.right_line, Some(2));
        assert_eq!(insert.content, "TWO");
    }

    #[test]
    fn context_lines_bound_how_much_unchanged_text_surrounds_a_change() {
        let diff = diff_of(
            "l1\nl2\nl3\nl4\nl5\nCHANGED\nl7\nl8\nl9\nl10\nl11\n",
            "l1\nl2\nl3\nl4\nl5\nchanged\nl7\nl8\nl9\nl10\nl11\n",
        );
        let hunk = &change_hunks_from_diff(&diff, 2)[0];
        let context_before = hunk
            .lines
            .iter()
            .take_while(|l| l.kind == ChangeLineKind::Context)
            .count();
        assert_eq!(context_before, 2, "exactly 2 lines of context requested");
    }

    #[test]
    fn a_pure_insertion_at_the_very_start_of_the_file_still_gets_a_correct_header() {
        // No context exists before the insertion (it's the first line), and
        // with context_lines = 0 none exists after it either — the
        // insert-only-window edge case the running counters exist for.
        let diff = diff_of("a\nb\n", "NEW\na\nb\n");
        let hunks = change_hunks_from_diff(&diff, 0);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!(hunk.old_len, 0, "nothing deleted");
        assert_eq!(
            hunk.old_start, 0,
            "unified-diff convention for an empty side"
        );
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_len, 1);
    }

    #[test]
    fn inline_spans_are_attached_to_changed_lines_when_the_diff_computed_them() {
        let options = DiffOptions {
            inline_mode: InlineMode::EagerForSmallHunks,
            ..DiffOptions::default()
        };
        let diff = compute_diff("hello world\n", "hello WORLD\n", options);
        let hunk = &change_hunks_from_diff(&diff, 1)[0];
        let delete = hunk
            .lines
            .iter()
            .find(|l| l.kind == ChangeLineKind::Delete)
            .unwrap();
        let insert = hunk
            .lines
            .iter()
            .find(|l| l.kind == ChangeLineKind::Insert)
            .unwrap();
        assert!(
            delete.spans.is_some(),
            "eager inline mode should have computed spans for this small replace hunk"
        );
        assert!(insert.spans.is_some());
    }

    #[test]
    fn a_lazily_computed_diff_carries_no_inline_spans_yet() {
        // `InlineMode::Lazy` (the default) defers inline computation to an
        // explicit UI request - a fresh `compute_diff` call has not done it,
        // so spans are absent. Guards the previous test's positive claim by
        // showing the negative case is real, not an always-true accident.
        let diff = diff_of("hello world\n", "hello WORLD\n");
        let hunk = &change_hunks_from_diff(&diff, 1)[0];
        assert!(hunk.lines.iter().all(|l| l.spans.is_none()));
    }
}
