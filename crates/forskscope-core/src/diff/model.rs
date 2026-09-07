//! Normalized diff model (RFC-002 §5, §6).
//!
//! These types are the contract consumed by the merge layer and UI. They
//! deliberately contain no `similar` type.

use crate::fnv1a64;

use super::options::DiffOptions;

/// Deterministic hunk identity within one `DiffDocument`.
pub type HunkId = u64;

/// 1-based inclusive line range on one side; `len == 0` marks an anchor
/// position with no lines on that side (pure insert/delete).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,
    pub len: u32,
}

impl LineRange {
    pub fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }
}

/// Hunk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind {
    Equal,
    Insert,
    Delete,
    Replace,
}

impl HunkKind {
    pub fn is_change(self) -> bool {
        !matches!(self, Self::Equal)
    }
}

/// Original line terminator of a side line, preserved for round-trip saves
/// (RFC-010 of the roadmap: newline semantics are significant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineMarker {
    Lf,
    CrLf,
    Cr,
    /// Last line without a trailing newline.
    None,
}

impl NewlineMarker {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
            Self::None => "",
        }
    }
}

/// One line on one side of a diff row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideLine {
    /// 1-based line number in the original document.
    pub original_line_number: Option<u32>,
    /// Line content without its terminator.
    pub content: String,
    pub newline: NewlineMarker,
}

/// Inline span classification (decoration only, never document truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    Equal,
    Insert,
    Delete,
}

/// One inline decoration span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub kind: InlineKind,
    pub text: String,
}

/// Inline decorations for one diff row.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineDiff {
    pub left_spans: Vec<InlineSpan>,
    pub right_spans: Vec<InlineSpan>,
}

/// One visual row pairing at most one left line with at most one right line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    pub left: Option<SideLine>,
    pub right: Option<SideLine>,
    pub inline: Option<InlineDiff>,
}

/// One contiguous hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub hunk_id: HunkId,
    pub kind: HunkKind,
    pub left_range: LineRange,
    pub right_range: LineRange,
    pub rows: Vec<DiffRow>,
}

/// Aggregate statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiffStats {
    pub hunks_total: usize,
    pub hunks_changed: usize,
    pub lines_inserted: usize,
    pub lines_deleted: usize,
}

/// Non-fatal diff observations surfaced to the user (RFC-002 §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffWarning {
    LargeFilePolicyApplied,
    DeadlineExpired,
    InlineSkippedHunkTooLarge,
}

/// The normalized result of one diff computation.
#[derive(Debug, Clone)]
pub struct DiffDocument {
    pub options: DiffOptions,
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
    pub warnings: Vec<DiffWarning>,
}

impl DiffDocument {
    /// An empty placeholder used while a tab is in the Loading state (RFC-065).
    pub fn empty() -> Self {
        Self {
            options: DiffOptions::default(),
            hunks: Vec::new(),
            stats: DiffStats::default(),
            warnings: Vec::new(),
        }
    }

    pub fn hunk(&self, hunk_id: HunkId) -> Option<&DiffHunk> {
        self.hunks.iter().find(|h| h.hunk_id == hunk_id)
    }

    /// `true` when both documents are identical.
    pub fn is_identical(&self) -> bool {
        self.stats.hunks_changed == 0
    }
}

/// Deterministic hunk identity, unique **within the one `DiffDocument`
/// this hunk belongs to, and nowhere further** (RFC-086 §4):
/// `hash(ordinal, kind, left_range, right_range)`. Recomputing the same
/// document with unchanged options therefore yields identical ids — the
/// property `MergeSession`'s undo/redo log depends on — but two different
/// documents may legitimately produce equal ids. That is safe only
/// because ids are ever compared within one `MergeSession`, which owns
/// exactly one document's hunks (`MergeSession::from_diff` asserts
/// uniqueness within that scope in debug builds); comparing ids **across**
/// documents or sessions is not a supported operation.
pub(super) fn hunk_id_for(
    ordinal: usize,
    kind: HunkKind,
    left: LineRange,
    right: LineRange,
) -> HunkId {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&(ordinal as u64).to_le_bytes());
    buf.push(match kind {
        HunkKind::Equal => 0,
        HunkKind::Insert => 1,
        HunkKind::Delete => 2,
        HunkKind::Replace => 3,
    });
    for v in [left.start, left.len, right.start, right.len] {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    fnv1a64(&buf)
}
