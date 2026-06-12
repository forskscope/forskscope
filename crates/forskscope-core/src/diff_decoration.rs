//! Diff decoration model (RFC-024).
//!
//! A [`DiffDecorationSet`] is derived from a [`DiffDocument`] and describes
//! *what to render* without prescribing *how to render it*. The Dioxus diff
//! component receives a `DiffDecorationSet` and maps it to CSS classes and
//! gutter symbols — no diff logic lives in the component.
//!
//! ## Design
//!
//! - All fields are indices into the document's line space; the component
//!   holds the actual text in `DiffDocument::hunks`.
//! - Every `LineDecorationKind` has a stable CSS class token and a gutter
//!   symbol for non-colour accessibility (RFC-024 §"Visual contract").
//! - `InlineDecoration` column ranges are byte-offset based, consistent
//!   with `InlineSpan`.

use crate::diff::{DiffDocument, DiffWarning, HunkId, HunkKind, InlineKind};

// ── Side ──────────────────────────────────────────────────────────────────────

/// Which pane a decoration applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Left,
    Right,
}

// ── Line decoration ───────────────────────────────────────────────────────────

/// Semantic state for one line (RFC-024 §"Semantic decoration model").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDecorationKind {
    /// Unchanged — identical on both sides.
    Unchanged,
    /// Line was added on the right side.
    Added,
    /// Line was deleted from the left side.
    Deleted,
    /// Line content changed (part of a Replace hunk).
    Modified,
    /// A placeholder row with no content on this side (the other side has content).
    EmptyCounterpart,
    /// Conflict line requiring resolution.
    Conflict,
    /// A merge operation has been applied to this line.
    MergeApplied,
}

impl LineDecorationKind {
    /// Stable CSS class token for this kind (RFC-024 §"Class contract").
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Unchanged       => "fs-line-unchanged",
            Self::Added           => "fs-line-added",
            Self::Deleted         => "fs-line-deleted",
            Self::Modified        => "fs-line-modified",
            Self::EmptyCounterpart => "fs-line-empty-counterpart",
            Self::Conflict        => "fs-line-conflict",
            Self::MergeApplied    => "fs-line-merge-applied",
        }
    }

    /// Single-character gutter symbol (RFC-024 §"Non-colour indicator").
    pub fn gutter_symbol(self) -> char {
        match self {
            Self::Unchanged        => ' ',
            Self::Added            => '+',
            Self::Deleted          => '-',
            Self::Modified         => '~',
            Self::EmptyCounterpart => '·',
            Self::Conflict         => '!',
            Self::MergeApplied     => '✓',
        }
    }

    /// ARIA label for screen reader accessibility (RFC-009 §7).
    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Unchanged        => "unchanged",
            Self::Added            => "added",
            Self::Deleted          => "deleted",
            Self::Modified         => "modified",
            Self::EmptyCounterpart => "empty counterpart",
            Self::Conflict         => "conflict",
            Self::MergeApplied     => "merge applied",
        }
    }
}

/// Decoration for one line in the diff view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDecoration {
    pub side:       DiffSide,
    /// 0-based row index in the aligned row sequence.
    pub row_index:  usize,
    pub kind:       LineDecorationKind,
    /// The hunk this decoration belongs to, if any.
    pub hunk_id:    Option<HunkId>,
}

// ── Inline decoration ─────────────────────────────────────────────────────────

/// Semantic state for one character-level inline span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDecorationKind {
    InsertedChars,
    DeletedChars,
    ReplacedChars,
    WhitespaceOnly,
}

impl InlineDecorationKind {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::InsertedChars  => "fs-inline-inserted",
            Self::DeletedChars   => "fs-inline-deleted",
            Self::ReplacedChars  => "fs-inline-replaced",
            Self::WhitespaceOnly => "fs-inline-whitespace",
        }
    }
}

/// A character-level decoration span on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineDecoration {
    pub side:      DiffSide,
    /// 0-based row index.
    pub row_index: usize,
    /// Byte offset start within the line content.
    pub start_col: usize,
    /// Byte offset end (exclusive) within the line content.
    pub end_col:   usize,
    pub kind:      InlineDecorationKind,
}

// ── Hunk decoration ───────────────────────────────────────────────────────────

/// Decoration metadata for one hunk (used by the hunk navigator and mini-map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkDecoration {
    pub hunk_id:         HunkId,
    pub start_row_index: usize,
    pub end_row_index:   usize,
    /// `true` when this is the focused/current hunk.
    pub is_focused:      bool,
}

// ── Warning decoration ────────────────────────────────────────────────────────

/// A non-fatal warning banner to show above or below the diff view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecorationWarning {
    pub message: String,
    pub kind:    DecorationWarningKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationWarningKind {
    LargeFile,
    DeadlineExpired,
    InlineSkipped,
}

// ── DiffDecorationSet ─────────────────────────────────────────────────────────

/// The complete decoration set for one diff view (RFC-024).
///
/// Derived from a [`DiffDocument`] via [`DiffDecorationSet::from_diff`].
/// The Dioxus component receives this and maps it to CSS classes + gutter
/// symbols without performing any diff logic.
#[derive(Debug, Clone, Default)]
pub struct DiffDecorationSet {
    pub left:     Vec<LineDecoration>,
    pub right:    Vec<LineDecoration>,
    pub inline:   Vec<InlineDecoration>,
    pub hunks:    Vec<HunkDecoration>,
    pub warnings: Vec<DecorationWarning>,
}

impl DiffDecorationSet {
    /// Derive the decoration set from a `DiffDocument`.
    ///
    /// `focused_hunk_id` marks the currently navigated hunk, if any.
    pub fn from_diff(
        doc: &DiffDocument,
        focused_hunk_id: Option<HunkId>,
    ) -> Self {
        let mut set = DiffDecorationSet::default();

        let mut row_index = 0usize;

        for hunk in &doc.hunks {
            let hunk_start = row_index;

            for row in &hunk.rows {
                let (lkind, rkind) = match hunk.kind {
                    HunkKind::Equal => (
                        LineDecorationKind::Unchanged,
                        LineDecorationKind::Unchanged,
                    ),
                    HunkKind::Insert => (
                        LineDecorationKind::EmptyCounterpart,
                        LineDecorationKind::Added,
                    ),
                    HunkKind::Delete => (
                        LineDecorationKind::Deleted,
                        LineDecorationKind::EmptyCounterpart,
                    ),
                    HunkKind::Replace => (
                        if row.left.is_some() {
                            LineDecorationKind::Modified
                        } else {
                            LineDecorationKind::EmptyCounterpart
                        },
                        if row.right.is_some() {
                            LineDecorationKind::Modified
                        } else {
                            LineDecorationKind::EmptyCounterpart
                        },
                    ),
                };

                if row.left.is_some() || hunk.kind == HunkKind::Insert {
                    set.left.push(LineDecoration {
                        side:      DiffSide::Left,
                        row_index,
                        kind:      lkind,
                        hunk_id:   Some(hunk.hunk_id),
                    });
                }
                if row.right.is_some() || hunk.kind == HunkKind::Delete {
                    set.right.push(LineDecoration {
                        side:      DiffSide::Right,
                        row_index,
                        kind:      rkind,
                        hunk_id:   Some(hunk.hunk_id),
                    });
                }

                // Inline decorations from InlineDiff spans.
                if let Some(inline) = &row.inline {
                    let mut left_col  = 0usize;
                    let mut right_col = 0usize;

                    for span in &inline.left_spans {
                        let end = left_col + span.text.len();
                        if span.kind != InlineKind::Equal {
                            let ikind = match span.kind {
                                InlineKind::Delete => InlineDecorationKind::DeletedChars,
                                InlineKind::Insert => InlineDecorationKind::InsertedChars,
                                InlineKind::Equal  => unreachable!(),
                            };
                            set.inline.push(InlineDecoration {
                                side:      DiffSide::Left,
                                row_index,
                                start_col: left_col,
                                end_col:   end,
                                kind:      ikind,
                            });
                        }
                        left_col = end;
                    }

                    for span in &inline.right_spans {
                        let end = right_col + span.text.len();
                        if span.kind != InlineKind::Equal {
                            let ikind = match span.kind {
                                InlineKind::Insert => InlineDecorationKind::InsertedChars,
                                InlineKind::Delete => InlineDecorationKind::DeletedChars,
                                InlineKind::Equal  => unreachable!(),
                            };
                            set.inline.push(InlineDecoration {
                                side:      DiffSide::Right,
                                row_index,
                                start_col: right_col,
                                end_col:   end,
                                kind:      ikind,
                            });
                        }
                        right_col = end;
                    }
                }

                row_index += 1;
            }

            let hunk_end = row_index.saturating_sub(1);
            if hunk.kind.is_change() {
                set.hunks.push(HunkDecoration {
                    hunk_id:         hunk.hunk_id,
                    start_row_index: hunk_start,
                    end_row_index:   hunk_end,
                    is_focused:      focused_hunk_id == Some(hunk.hunk_id),
                });
            }
        }

        // Warnings from the document.
        for w in &doc.warnings {
            let (msg, kind) = match w {
                DiffWarning::LargeFilePolicyApplied =>
                    ("Large file — inline diff disabled.", DecorationWarningKind::LargeFile),
                DiffWarning::DeadlineExpired =>
                    ("Diff timed out — result may be approximate.", DecorationWarningKind::DeadlineExpired),
                DiffWarning::InlineSkippedHunkTooLarge =>
                    ("Some hunks were too large for inline diff.", DecorationWarningKind::InlineSkipped),
            };
            set.warnings.push(DecorationWarning {
                message: msg.into(),
                kind,
            });
        }

        set
    }

    /// Number of changed hunks in this decoration set.
    pub fn changed_hunk_count(&self) -> usize { self.hunks.len() }

    /// `true` when there are no changed lines.
    pub fn is_empty(&self) -> bool { self.hunks.is_empty() }
}
