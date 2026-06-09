//! Build [`PatchDocument`]s from the normalized diff model (RFC-039).
//!
//! The builder consumes a [`DiffDocument`] — the same artifact the merge
//! and UI layers consume — and produces unified-diff hunks with a
//! configurable amount of surrounding context. It performs no I/O and
//! holds no `similar` types, keeping patch export a pure projection of
//! product truth.

use std::path::PathBuf;

use crate::diff::{DiffDocument, DiffRow, HunkKind, NewlineMarker};

use super::model::{
    LineOrigin, PatchDocument, PatchFileChange, PatchFormat, PatchHunk, PatchLine, PatchSummary,
};

/// Options controlling patch generation.
#[derive(Debug, Clone, Copy)]
pub struct PatchOptions {
    /// Unified-diff context lines kept around each change (default 3).
    pub context_lines: usize,
    /// Emit `Add`/`Delete` whole-file entries for one-sided files.
    pub include_creation_deletion: bool,
    /// Emit `BinaryNotice` entries for differing binary files.
    pub include_binary_notices: bool,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            context_lines: 3,
            include_creation_deletion: true,
            include_binary_notices: false,
        }
    }
}

/// A flattened (origin, content, no-newline) view of one document side,
/// produced by walking the diff rows in order.
struct FlatLine {
    origin: LineOrigin,
    content: String,
    no_newline_at_eof: bool,
}

/// Build the unified-diff hunks for a single modified file from its diff.
///
/// Returns an empty vector when the documents are identical, so the caller
/// can decide whether to emit a `Modify` entry at all.
pub fn hunks_from_diff(diff: &DiffDocument, options: PatchOptions) -> Vec<PatchHunk> {
    let flat = flatten(diff);
    coalesce(&flat, options.context_lines)
}

/// Convenience: build a single-file `Modify` patch for the common
/// two-file comparison case. Returns `None` when there is nothing to
/// patch (identical inputs).
pub fn patch_from_file_diff(
    rel_path: impl Into<PathBuf>,
    diff: &DiffDocument,
    options: PatchOptions,
) -> Option<PatchDocument> {
    let hunks = hunks_from_diff(diff, options);
    if hunks.is_empty() {
        return None;
    }
    let mut doc = PatchDocument {
        format: PatchFormat::Unified,
        files: vec![PatchFileChange::Modify {
            path: rel_path.into(),
            hunks,
        }],
        summary: PatchSummary::default(),
    };
    doc.recompute_summary();
    Some(doc)
}

/// Whole-file lines (terminators stripped) for an `Add` or `Delete` entry,
/// reading the right or left content of a diff respectively. Used when a
/// file exists on only one side.
pub(crate) fn whole_side_lines(diff: &DiffDocument, side: Side) -> Vec<PatchLine> {
    let flat = flatten(diff);
    flat.into_iter()
        .filter_map(|l| match (side, l.origin) {
            (Side::Left, LineOrigin::Delete) | (Side::Left, LineOrigin::Context) => Some(PatchLine {
                origin: side.whole_file_origin(),
                content: l.content,
                no_newline_at_eof: l.no_newline_at_eof,
            }),
            (Side::Right, LineOrigin::Insert) | (Side::Right, LineOrigin::Context) => {
                Some(PatchLine {
                    origin: side.whole_file_origin(),
                    content: l.content,
                    no_newline_at_eof: l.no_newline_at_eof,
                })
            }
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Side {
    Left,
    Right,
}

impl Side {
    fn whole_file_origin(self) -> LineOrigin {
        match self {
            Side::Left => LineOrigin::Delete,
            Side::Right => LineOrigin::Insert,
        }
    }
}

/// Flatten the row-structured diff into an ordered line stream where each
/// line is tagged Context/Delete/Insert. A `Replace` row contributes a
/// Delete (left) followed by an Insert (right); `Equal` rows contribute a
/// single Context line.
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
                // Emit all deletions first, then insertions, so a Replace
                // renders as the conventional `-`-block followed by a
                // `+`-block in the unified output.
                for row in &hunk.rows {
                    if let Some(left) = &row.left {
                        out.push(FlatLine {
                            origin: LineOrigin::Delete,
                            content: left.content.clone(),
                            no_newline_at_eof: left.newline == NewlineMarker::None,
                        });
                    }
                }
                for row in &hunk.rows {
                    if let Some(right) = &row.right {
                        out.push(FlatLine {
                            origin: LineOrigin::Insert,
                            content: right.content.clone(),
                            no_newline_at_eof: right.newline == NewlineMarker::None,
                        });
                    }
                }
            }
        }
    }
    out
}

fn push_context(out: &mut Vec<FlatLine>, row: &DiffRow) {
    // Equal rows have matching content on both sides; prefer the left
    // line's terminator for the no-newline marker.
    if let Some(left) = &row.left {
        out.push(FlatLine {
            origin: LineOrigin::Context,
            content: left.content.clone(),
            no_newline_at_eof: left.newline == NewlineMarker::None,
        });
    } else if let Some(right) = &row.right {
        out.push(FlatLine {
            origin: LineOrigin::Context,
            content: right.content.clone(),
            no_newline_at_eof: right.newline == NewlineMarker::None,
        });
    }
}

/// Group the flat line stream into unified-diff hunks, keeping at most
/// `context` context lines on either side of each change run and merging
/// change runs separated by `<= 2 * context` context lines.
fn coalesce(flat: &[FlatLine], context: usize) -> Vec<PatchHunk> {
    // Index the change positions.
    let change_idx: Vec<usize> = flat
        .iter()
        .enumerate()
        .filter(|(_, l)| l.origin != LineOrigin::Context)
        .map(|(i, _)| i)
        .collect();
    if change_idx.is_empty() {
        return Vec::new();
    }

    // Build the inclusive [start, end] flat-index window for each hunk,
    // merging windows whose context regions touch or overlap.
    let mut windows: Vec<(usize, usize)> = Vec::new();
    for &ci in &change_idx {
        let lo = ci.saturating_sub(context);
        let hi = (ci + context).min(flat.len() - 1);
        match windows.last_mut() {
            Some((_, prev_hi)) if lo <= *prev_hi + 1 => {
                if hi > *prev_hi {
                    *prev_hi = hi;
                }
            }
            _ => windows.push((lo, hi)),
        }
    }

    // Running 1-based line numbers on each side.
    let mut old_line: u32 = 1;
    let mut new_line: u32 = 1;
    // Cursor into `flat`; old/new counters advance as we pass each line.
    let mut cursor = 0usize;

    let mut hunks = Vec::with_capacity(windows.len());
    for (lo, hi) in windows {
        // Advance counters for the gap (context-only) lines we skip.
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
            match fl.origin {
                LineOrigin::Context => {
                    old_len += 1;
                    new_len += 1;
                }
                LineOrigin::Delete => old_len += 1,
                LineOrigin::Insert => new_len += 1,
            }
            lines.push(PatchLine {
                origin: fl.origin,
                content: fl.content.clone(),
                no_newline_at_eof: fl.no_newline_at_eof,
            });
            advance(fl, &mut old_line, &mut new_line);
        }
        cursor = hi + 1;
        // Unified-diff convention: an empty side reports start = the line
        // before the change (0 when the file is empty on that side).
        let (old_start, old_len) = normalize_range(old_start, old_len);
        let (new_start, new_len) = normalize_range(new_start, new_len);
        hunks.push(PatchHunk {
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
    match line.origin {
        LineOrigin::Context => {
            *old_line += 1;
            *new_line += 1;
        }
        LineOrigin::Delete => *old_line += 1,
        LineOrigin::Insert => *new_line += 1,
    }
}

fn normalize_range(start: u32, len: u32) -> (u32, u32) {
    if len == 0 {
        (start.saturating_sub(1), 0)
    } else {
        (start, len)
    }
}
