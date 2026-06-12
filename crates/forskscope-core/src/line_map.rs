//! Line map and scroll synchronisation model (RFC-035).
//!
//! A [`LineMap`] is derived from a [`DiffDocument`] and represents the
//! complete aligned row sequence: each `AlignedRow` pairs at most one left
//! line with at most one right line. The component uses this to:
//!
//! - render left and right panes at the same row height (for visual alignment),
//! - implement synchronized scrolling (`scroll_anchor_for_row`),
//! - populate the mini hunk map.
//!
//! ## Design
//!
//! `LineMap` owns no text — it holds line numbers and row IDs. The component
//! reads line text from `DiffDocument::hunks` by `HunkId`. Row IDs are stable
//! within a single `DiffDocument` computation; they are re-derived on reload.

use crate::diff::{DiffDocument, HunkId, HunkKind};

// ── Row types ─────────────────────────────────────────────────────────────────

/// Stable row identity within a `LineMap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId(pub u32);

/// A reference to one side's line within a `DiffDocument`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    /// 1-based line number in the original (pre-diff) document.
    pub original_line: u32,
    /// 0-based row index in the aligned sequence.
    pub row_index: usize,
}

/// Semantic state of one aligned row (RFC-035 §"RowState").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Equal,
    Inserted,
    Deleted,
    Modified,
    Conflict,
    /// Row belongs to an equal hunk that is currently collapsed.
    Collapsed,
    Unknown,
}

impl RowState {
    /// The single-character gutter symbol for this row state.
    pub fn gutter_symbol(self) -> char {
        match self {
            Self::Equal     => '=',
            Self::Inserted  => '+',
            Self::Deleted   => '-',
            Self::Modified  => '~',
            Self::Conflict  => '!',
            Self::Collapsed => '…',
            Self::Unknown   => '?',
        }
    }

    /// `true` when this row represents a visible change.
    pub fn is_changed(self) -> bool {
        matches!(self, Self::Inserted | Self::Deleted | Self::Modified | Self::Conflict)
    }
}

/// One row in the aligned line map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignedRow {
    pub row_id:    RowId,
    /// Left side line reference. `None` for insert-only rows.
    pub left:      Option<LineSpan>,
    /// Right side line reference. `None` for delete-only rows.
    pub right:     Option<LineSpan>,
    pub state:     RowState,
    /// The hunk this row belongs to.
    pub hunk_id:   Option<HunkId>,
}

impl AlignedRow {
    /// `true` when both sides have a line (the row is full-width).
    pub fn is_paired(&self) -> bool { self.left.is_some() && self.right.is_some() }
}

// ── LineMap ───────────────────────────────────────────────────────────────────

/// The complete aligned row sequence for a diff comparison (RFC-035).
#[derive(Debug, Clone)]
pub struct LineMap {
    pub rows: Vec<AlignedRow>,
    /// Total changed rows (Inserted + Deleted + Modified + Conflict).
    pub changed_row_count: usize,
}

impl LineMap {
    /// Derive a `LineMap` from a `DiffDocument`.
    pub fn from_diff(doc: &DiffDocument) -> Self {
        let mut rows    = Vec::new();
        let mut row_idx = 0u32;
        let mut changed = 0usize;

        let mut left_line_num  = 1u32;
        let mut right_line_num = 1u32;

        for hunk in &doc.hunks {
            let state = match hunk.kind {
                HunkKind::Equal   => RowState::Equal,
                HunkKind::Insert  => RowState::Inserted,
                HunkKind::Delete  => RowState::Deleted,
                HunkKind::Replace => RowState::Modified,
            };

            for row in &hunk.rows {
                let left_span = row.left.as_ref().map(|_| {
                    let s = LineSpan { original_line: left_line_num, row_index: row_idx as usize };
                    left_line_num += 1;
                    s
                });
                let right_span = row.right.as_ref().map(|_| {
                    let s = LineSpan { original_line: right_line_num, row_index: row_idx as usize };
                    right_line_num += 1;
                    s
                });

                if state.is_changed() { changed += 1; }

                rows.push(AlignedRow {
                    row_id:  RowId(row_idx),
                    left:    left_span,
                    right:   right_span,
                    state,
                    hunk_id: Some(hunk.hunk_id),
                });
                row_idx += 1;
            }
        }

        Self { rows, changed_row_count: changed }
    }

    /// Look up a row by its `RowId`.
    pub fn row(&self, id: RowId) -> Option<&AlignedRow> {
        self.rows.get(id.0 as usize)
    }

    /// All changed rows (for hunk navigation).
    pub fn changed_rows(&self) -> impl Iterator<Item = &AlignedRow> {
        self.rows.iter().filter(|r| r.state.is_changed())
    }

    /// The first changed row at or after `row_index`.
    pub fn next_changed_row(&self, from_row_index: usize) -> Option<&AlignedRow> {
        self.rows.iter()
            .skip(from_row_index)
            .find(|r| r.state.is_changed())
    }

    /// The last changed row before `row_index`.
    pub fn prev_changed_row(&self, from_row_index: usize) -> Option<&AlignedRow> {
        self.rows.iter()
            .take(from_row_index)
            .rev()
            .find(|r| r.state.is_changed())
    }

    /// `true` when the document is identical (no changed rows).
    pub fn is_identical(&self) -> bool { self.changed_row_count == 0 }
}

// ── Scroll anchor ─────────────────────────────────────────────────────────────

/// A scroll position expressed as a row index and fractional offset within
/// the row, used to synchronise the two panes (RFC-035 §"Scroll sync").
///
/// Both panes share the same `row_index`; the fractional offset is relative
/// to the row height as a fraction in `[0.0, 1.0)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAnchor {
    /// 0-based aligned row index.
    pub row_index:         usize,
    /// Fractional offset within the row `[0.0, 1.0)`.
    pub row_fraction:      f32,
}

impl ScrollAnchor {
    pub fn at_top() -> Self {
        Self { row_index: 0, row_fraction: 0.0 }
    }

    /// Clamp `row_fraction` to `[0.0, 1.0)`.
    pub fn clamped(row_index: usize, row_fraction: f32) -> Self {
        Self {
            row_index,
            row_fraction: row_fraction.clamp(0.0, 1.0 - f32::EPSILON),
        }
    }
}

// ── Mini-map segment ──────────────────────────────────────────────────────────

/// One segment in the mini hunk map shown below the diff panes (RFC-035 §"Mini map").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniMapSegment {
    pub state:     RowState,
    /// Proportion of total rows this segment occupies (`0.0..=1.0`).
    pub weight:    u32,
    pub hunk_id:   Option<HunkId>,
}

/// Build the mini-map segment sequence from a `LineMap`.
///
/// Consecutive rows with the same `RowState` and `HunkId` are merged into
/// one segment. Weights are row counts; normalisation is the caller's job.
pub fn build_mini_map(map: &LineMap) -> Vec<MiniMapSegment> {
    let mut segments: Vec<MiniMapSegment> = Vec::new();
    for row in &map.rows {
        match segments.last_mut() {
            Some(last)
                if last.state == row.state && last.hunk_id == row.hunk_id =>
            {
                last.weight += 1;
            }
            _ => segments.push(MiniMapSegment {
                state:   row.state,
                weight:  1,
                hunk_id: row.hunk_id,
            }),
        }
    }
    segments
}
