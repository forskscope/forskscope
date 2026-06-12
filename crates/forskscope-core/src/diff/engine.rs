//! Diff engine over `similar` v3 (RFC-002).
//!
//! Lines are split by the engine itself, preserving `\n`, `\r\n`, and `\r`
//! terminators so that newline-style changes are visible and merge results
//! round-trip the original line endings. The diff runs over full lines
//! (terminator included); ignore options apply to a normalized key.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use similar::{Algorithm, DiffOp, TextDiffConfig};

use super::inline::inline_diff_rows;
use super::model::{
    DiffDocument, DiffHunk, DiffRow, DiffStats, DiffWarning, HunkKind, LineRange, NewlineMarker,
    SideLine, hunk_id_for,
};
use super::options::{DiffAlgorithm, DiffOptions, InlineMode};

static DIFF_COUNTER: AtomicU64 = AtomicU64::new(1);

/// One split source line: content without terminator plus its marker.
#[derive(Debug, Clone)]
struct SrcLine {
    content: String,
    newline: NewlineMarker,
}

/// Split text into lines, preserving terminator information.
fn split_lines(text: &str) -> Vec<SrcLine> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                out.push(SrcLine {
                    content: text[start..i].to_string(),
                    newline: NewlineMarker::Lf,
                });
                i += 1;
                start = i;
            }
            b'\r' => {
                let (marker, step) = if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    (NewlineMarker::CrLf, 2)
                } else {
                    (NewlineMarker::Cr, 1)
                };
                out.push(SrcLine {
                    content: text[start..i].to_string(),
                    newline: marker,
                });
                i += step;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        out.push(SrcLine {
            content: text[start..].to_string(),
            newline: NewlineMarker::None,
        });
    }
    out
}

/// Comparison key for one line under the active ignore options.
fn line_key(line: &SrcLine, options: &DiffOptions) -> String {
    // Start with content only (never the newline) when ignoring newline
    // differences; otherwise include the newline so LF≠CRLF.
    let mut key = if options.ignore_newlines {
        line.content.clone()
    } else {
        format!("{}{}", line.content, line.newline.as_str())
    };
    if options.ignore_whitespace {
        key.retain(|c| !c.is_whitespace());
    }
    if options.ignore_case {
        key = key.to_lowercase();
    }
    key
}

fn map_algorithm(a: DiffAlgorithm) -> Algorithm {
    match a {
        DiffAlgorithm::Myers => Algorithm::Myers,
        DiffAlgorithm::Patience => Algorithm::Patience,
        DiffAlgorithm::Lcs => Algorithm::Lcs,
        DiffAlgorithm::Histogram => Algorithm::Histogram,
    }
}

/// Compute the normalized [`DiffDocument`] for two texts.
pub fn compute_diff(left_text: &str, right_text: &str, options: DiffOptions) -> DiffDocument {
    let mut effective = options;
    let mut warnings = Vec::new();

    let total_bytes = (left_text.len() + right_text.len()) as u64;
    if total_bytes > options.max_file_bytes_for_full_diff {
        effective.inline_mode = InlineMode::None;
        warnings.push(DiffWarning::LargeFilePolicyApplied);
    }

    let left_lines = split_lines(left_text);
    let right_lines = split_lines(right_text);
    let left_keys: Vec<String> = left_lines.iter().map(|l| line_key(l, &effective)).collect();
    let right_keys: Vec<String> = right_lines.iter().map(|l| line_key(l, &effective)).collect();
    let left_refs: Vec<&str> = left_keys.iter().map(String::as_str).collect();
    let right_refs: Vec<&str> = right_keys.iter().map(String::as_str).collect();

    let mut config = TextDiffConfig::default();
    config.algorithm(map_algorithm(effective.algorithm));
    let started = Instant::now();
    if let Some(ms) = effective.deadline_ms {
        config.deadline(started + Duration::from_millis(ms));
    }
    let diff = config.diff_slices(&left_refs, &right_refs);
    let ops: Vec<DiffOp> = diff.ops().to_vec();
    if let Some(ms) = effective.deadline_ms
        && started.elapsed() > Duration::from_millis(ms)
    {
        warnings.push(DiffWarning::DeadlineExpired);
    }

    let diff_id = DIFF_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hunks = Vec::with_capacity(ops.len());
    let mut stats = DiffStats::default();

    for (ordinal, op) in ops.iter().enumerate() {
        let (kind, old_range, new_range) = classify_op(op);
        let left_range = LineRange::new(old_range.start as u32 + 1, old_range.len() as u32);
        let right_range = LineRange::new(new_range.start as u32 + 1, new_range.len() as u32);
        let rows = build_rows(
            kind,
            &left_lines,
            &right_lines,
            old_range,
            new_range,
            &mut stats,
        );
        let mut hunk = DiffHunk {
            hunk_id: hunk_id_for(diff_id, ordinal, kind, left_range, right_range),
            kind,
            left_range,
            right_range,
            rows,
        };
        if kind == HunkKind::Replace
            && effective.inline_mode == InlineMode::EagerForSmallHunks
            && !inline_diff_rows(&mut hunk, effective.max_inline_chars_per_hunk)
        {
            warnings.push(DiffWarning::InlineSkippedHunkTooLarge);
        }
        stats.hunks_total += 1;
        if kind.is_change() {
            stats.hunks_changed += 1;
        }
        hunks.push(hunk);
    }

    DiffDocument {
        diff_id,
        options,
        hunks,
        stats,
        warnings,
    }
}

fn classify_op(op: &DiffOp) -> (HunkKind, std::ops::Range<usize>, std::ops::Range<usize>) {
    match *op {
        DiffOp::Equal {
            old_index,
            new_index,
            len,
        } => (
            HunkKind::Equal,
            old_index..old_index + len,
            new_index..new_index + len,
        ),
        DiffOp::Delete {
            old_index,
            old_len,
            new_index,
        } => (
            HunkKind::Delete,
            old_index..old_index + old_len,
            new_index..new_index,
        ),
        DiffOp::Insert {
            old_index,
            new_index,
            new_len,
        } => (
            HunkKind::Insert,
            old_index..old_index,
            new_index..new_index + new_len,
        ),
        DiffOp::Replace {
            old_index,
            old_len,
            new_index,
            new_len,
        } => (
            HunkKind::Replace,
            old_index..old_index + old_len,
            new_index..new_index + new_len,
        ),
    }
}

fn side_line(lines: &[SrcLine], index: usize) -> SideLine {
    let line = &lines[index];
    SideLine {
        original_line_number: Some(index as u32 + 1),
        content: line.content.clone(),
        newline: line.newline,
    }
}

fn build_rows(
    kind: HunkKind,
    left_lines: &[SrcLine],
    right_lines: &[SrcLine],
    old_range: std::ops::Range<usize>,
    new_range: std::ops::Range<usize>,
    stats: &mut DiffStats,
) -> Vec<DiffRow> {
    let rows_len = old_range.len().max(new_range.len());
    let mut rows = Vec::with_capacity(rows_len);
    for i in 0..rows_len {
        let left = old_range
            .clone()
            .nth(i)
            .map(|idx| side_line(left_lines, idx));
        let right = new_range
            .clone()
            .nth(i)
            .map(|idx| side_line(right_lines, idx));
        if kind.is_change() {
            if right.is_some() {
                stats.lines_inserted += 1;
            }
            if left.is_some() {
                stats.lines_deleted += 1;
            }
        }
        rows.push(DiffRow {
            left,
            right,
            inline: None,
        });
    }
    if kind == HunkKind::Equal {
        // Equal hunks duplicate identical content; deletions/insertions stay 0.
    }
    rows
}
