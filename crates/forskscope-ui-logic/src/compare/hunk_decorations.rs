//! Per-row hunk decoration lookup (RFC-024, RFC-006).
//!
//! [`RowDecoration`] holds the fully-resolved CSS class, gutter symbol, and
//! ARIA label for one side of one diff row. The Dioxus `Row` component
//! currently computes these inline from `HunkKind`; this module provides the
//! view-model that derives them from `DiffDecorationSet` instead, so the
//! renderer can be upgraded to consume decoration contract types rather than
//! re-implementing diff semantics.
//!
//! ## Usage
//!
//! ```no_run
//! # use forskscope_core::diff_decoration::DiffDecorationSet;
//! # use forskscope_core::diff::DiffDocument;
//! # use forskscope_ui_logic::compare::hunk_decorations::{DecorationIndex, DiffSide};
//! # let doc: DiffDocument = unimplemented!();
//! let dec = DiffDecorationSet::from_diff(&doc, None);
//! let idx = DecorationIndex::from_set(&dec);
//! // Look up the decoration for row 3 on the left side:
//! let row = idx.get(3, DiffSide::Left);
//! println!("{} {}", row.gutter_symbol, row.css_class);
//! ```

use forskscope_core::diff_decoration::{
    DiffDecorationSet, DiffSide as CoreDiffSide, LineDecorationKind,
};

// ── Re-export so callers don't need to depend on core directly ────────────────

/// Which side of the diff a decoration applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide { Left, Right }

// ── Per-row decoration data ───────────────────────────────────────────────────

/// Fully-resolved display data for one row on one side of the diff.
///
/// All fields are `'static` — no lifetime coupling to the `DiffDecorationSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDecoration {
    /// CSS class token from `LineDecorationKind::css_class()`, e.g. `"fs-line-added"`.
    pub css_class:      &'static str,
    /// Single-character gutter symbol, e.g. `'+'`, `'-'`, `'~'`, `' '`.
    pub gutter_symbol:  char,
    /// ARIA label for screen-reader accessibility, e.g. `"added"`.
    pub aria_label:     &'static str,
    /// The underlying decoration kind.
    pub kind:           LineDecorationKind,
}

impl RowDecoration {
    fn from_kind(kind: LineDecorationKind) -> Self {
        Self {
            css_class:     kind.css_class(),
            gutter_symbol: kind.gutter_symbol(),
            aria_label:    kind.aria_label(),
            kind,
        }
    }

    fn unchanged() -> Self { Self::from_kind(LineDecorationKind::Unchanged) }
}

// ── Decoration index ──────────────────────────────────────────────────────────

/// A flat array lookup from `(row_index, side)` to `RowDecoration`.
///
/// Built once from a `DiffDecorationSet` and passed to the renderer.
/// `get(row_index, side)` is O(1) after construction.
///
/// Row indices are the same sequential integers used inside
/// `DiffDecorationSet` — 0 is the first row of the first hunk, and they
/// increment across hunk boundaries.
#[derive(Debug, Clone)]
pub struct DecorationIndex {
    left:  Vec<RowDecoration>,
    right: Vec<RowDecoration>,
}

impl DecorationIndex {
    /// Build the index from a `DiffDecorationSet`.
    ///
    /// Rows with no decoration entry are treated as `Unchanged`.
    pub fn from_set(set: &DiffDecorationSet) -> Self {
        let row_count = set
            .left.iter().map(|d| d.row_index + 1)
            .chain(set.right.iter().map(|d| d.row_index + 1))
            .max()
            .unwrap_or(0);

        let mut left  = vec![RowDecoration::unchanged(); row_count];
        let mut right = vec![RowDecoration::unchanged(); row_count];

        for d in &set.left {
            if d.side == CoreDiffSide::Left {
                if let Some(slot) = left.get_mut(d.row_index) {
                    *slot = RowDecoration::from_kind(d.kind);
                }
            }
        }
        for d in &set.right {
            if d.side == CoreDiffSide::Right {
                if let Some(slot) = right.get_mut(d.row_index) {
                    *slot = RowDecoration::from_kind(d.kind);
                }
            }
        }

        Self { left, right }
    }

    /// Look up the decoration for `row_index` on `side`.
    ///
    /// Returns `Unchanged` for any out-of-bounds row index rather than
    /// panicking — safe for untrusted row indices from the component.
    pub fn get(&self, row_index: usize, side: DiffSide) -> RowDecoration {
        let vec = match side { DiffSide::Left => &self.left, DiffSide::Right => &self.right };
        vec.get(row_index).cloned().unwrap_or_else(RowDecoration::unchanged)
    }

    /// Total number of rows tracked (same as the length of the longer side).
    pub fn row_count(&self) -> usize {
        self.left.len().max(self.right.len())
    }

    /// `true` when the decoration set was built from an empty (identical) diff.
    pub fn is_empty(&self) -> bool { self.left.is_empty() && self.right.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::diff::{DiffOptions, compute_diff};
    use forskscope_core::diff_decoration::{DiffDecorationSet, LineDecorationKind};

    fn index_from_texts(left: &str, right: &str) -> DecorationIndex {
        let doc  = compute_diff(left, right, DiffOptions::default());
        let decs = DiffDecorationSet::from_diff(&doc, None);
        DecorationIndex::from_set(&decs)
    }

    // ── Empty / identical diff ─────────────────────────────────────────────────

    #[test]
    fn identical_texts_produce_only_unchanged_decorations() {
        let idx = index_from_texts("same\n", "same\n");
        // Equal hunks produce Unchanged decorations on both sides,
        // so the index is not empty — but no row is Added/Deleted/Modified.
        for i in 0..idx.row_count() {
            assert_eq!(idx.get(i, DiffSide::Left).kind,  LineDecorationKind::Unchanged,
                "row {i} left must be Unchanged for identical texts");
            assert_eq!(idx.get(i, DiffSide::Right).kind, LineDecorationKind::Unchanged,
                "row {i} right must be Unchanged for identical texts");
        }
    }

    #[test]
    fn empty_diff_get_returns_unchanged() {
        let idx = index_from_texts("same\n", "same\n");
        let row = idx.get(0, DiffSide::Left);
        assert_eq!(row.kind, LineDecorationKind::Unchanged);
    }

    // ── Added lines ────────────────────────────────────────────────────────────

    #[test]
    fn inserted_line_right_side_has_added_kind() {
        let idx = index_from_texts("", "hello\n");
        let row = idx.get(0, DiffSide::Right);
        assert_eq!(row.kind, LineDecorationKind::Added);
    }

    #[test]
    fn inserted_line_has_plus_gutter_symbol() {
        let idx = index_from_texts("", "hello\n");
        let row = idx.get(0, DiffSide::Right);
        assert_eq!(row.gutter_symbol, '+');
    }

    #[test]
    fn inserted_line_css_class_starts_with_fs_prefix() {
        let idx = index_from_texts("", "hello\n");
        let row = idx.get(0, DiffSide::Right);
        assert!(row.css_class.starts_with("fs-"), "css class must have fs- prefix: {}", row.css_class);
    }

    #[test]
    fn inserted_line_has_non_empty_aria_label() {
        let idx = index_from_texts("", "hello\n");
        let row = idx.get(0, DiffSide::Right);
        assert!(!row.aria_label.is_empty());
    }

    // ── Deleted lines ──────────────────────────────────────────────────────────

    #[test]
    fn deleted_line_left_side_has_deleted_kind() {
        let idx = index_from_texts("hello\n", "");
        let row = idx.get(0, DiffSide::Left);
        assert_eq!(row.kind, LineDecorationKind::Deleted);
    }

    #[test]
    fn deleted_line_has_minus_gutter_symbol() {
        let idx = index_from_texts("hello\n", "");
        let row = idx.get(0, DiffSide::Left);
        assert_eq!(row.gutter_symbol, '-');
    }

    // ── Modified lines ─────────────────────────────────────────────────────────

    #[test]
    fn replaced_line_both_sides_have_modified_kind() {
        let idx = index_from_texts("old\n", "new\n");
        let left  = idx.get(0, DiffSide::Left);
        let right = idx.get(0, DiffSide::Right);
        assert_eq!(left.kind,  LineDecorationKind::Modified);
        assert_eq!(right.kind, LineDecorationKind::Modified);
    }

    #[test]
    fn modified_line_has_tilde_gutter_symbol() {
        let idx = index_from_texts("old\n", "new\n");
        assert_eq!(idx.get(0, DiffSide::Left).gutter_symbol, '~');
        assert_eq!(idx.get(0, DiffSide::Right).gutter_symbol, '~');
    }

    // ── Multi-hunk diffs ───────────────────────────────────────────────────────

    #[test]
    fn multi_hunk_row_count_covers_all_rows() {
        let left  = "a\nb\nc\nd\ne\n";
        let right = "a\nX\nc\nY\ne\n";
        let idx = index_from_texts(left, right);
        assert!(idx.row_count() >= 5, "must cover all 5 rows");
    }

    #[test]
    fn unchanged_rows_in_multi_hunk_are_unchanged() {
        let left  = "a\nb\nc\n";
        let right = "a\nX\nc\n";
        let idx = index_from_texts(left, right);
        // Row 0 ("a") is unchanged on both sides
        assert_eq!(idx.get(0, DiffSide::Left).kind,  LineDecorationKind::Unchanged);
        assert_eq!(idx.get(0, DiffSide::Right).kind, LineDecorationKind::Unchanged);
    }

    // ── Out-of-bounds safety ───────────────────────────────────────────────────

    #[test]
    fn out_of_bounds_row_returns_unchanged_not_panic() {
        let idx = index_from_texts("a\n", "b\n");
        let row = idx.get(9999, DiffSide::Left);
        assert_eq!(row.kind, LineDecorationKind::Unchanged);
    }

    // ── RowDecoration contract ─────────────────────────────────────────────────

    #[test]
    fn unchanged_row_decoration_has_space_gutter() {
        let row = RowDecoration::unchanged();
        assert_eq!(row.gutter_symbol, ' ');
        assert_eq!(row.kind, LineDecorationKind::Unchanged);
    }

    #[test]
    fn row_decoration_fields_match_kind_methods() {
        for kind in [
            LineDecorationKind::Unchanged,
            LineDecorationKind::Added,
            LineDecorationKind::Deleted,
            LineDecorationKind::Modified,
            LineDecorationKind::EmptyCounterpart,
            LineDecorationKind::Conflict,
            LineDecorationKind::MergeApplied,
        ] {
            let row = RowDecoration::from_kind(kind);
            assert_eq!(row.css_class,     kind.css_class(),     "{kind:?} css_class mismatch");
            assert_eq!(row.gutter_symbol, kind.gutter_symbol(), "{kind:?} gutter_symbol mismatch");
            assert_eq!(row.aria_label,    kind.aria_label(),    "{kind:?} aria_label mismatch");
        }
    }
}
