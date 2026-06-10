//! Search match index and traversal engine (RFC-014 §"Text Search", RFC-059 §M4).
//!
//! This module owns the *pure data* part of in-diff search: building an
//! ordered list of every matching (hunk_id, row_index, side) position from
//! a snapshot of hunks and a query, and stepping through that list with
//! wrapping. It has no Dioxus dependency and is fully unit-testable.
//!
//! The rendering layer (`search.rs`, `hunk.rs`) reads `SearchCtx`, which
//! wraps this type, and the `ScrollTarget` it produces is consumed by
//! `diff.rs` to call `scrollIntoView`.

use std::collections::HashSet;

// ── Public types ──────────────────────────────────────────────────────────────

/// Which side(s) of a row matched the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSide { Left, Right, Both }

/// One search match: the DOM element id used for scroll-into-view, plus
/// the hunk / row coordinates for highlight bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPosition {
    /// The CSS `id` of the hunk element containing this row: `"h-{hunk_id}"`.
    pub hunk_elem_id: String,
    pub hunk_id:      u64,
    pub row_index:    usize,
    pub side:         MatchSide,
}

/// The flat ordered list of all matches for the current query.
#[derive(Debug, Clone, Default)]
pub struct MatchIndex {
    positions:     Vec<MatchPosition>,
    focused_index: Option<usize>,
}

impl MatchIndex {
    /// Build a match index from an iterator of `(hunk_id, rows)` tuples.
    ///
    /// `rows` is a slice of `(left_content, right_content)` pairs, where
    /// either side is `None` for an empty (insert/delete) half.
    pub fn build<'a>(
        hunks: impl Iterator<Item = (u64, &'a [(Option<&'a str>, Option<&'a str>)])>,
        query: &str,
    ) -> Self {
        if query.is_empty() {
            return Self::default();
        }
        let q = query.to_ascii_lowercase();
        let mut positions = Vec::new();

        for (hunk_id, rows) in hunks {
            let hunk_elem_id = format!("h-{hunk_id}");
            for (row_index, (left, right)) in rows.iter().enumerate() {
                let lm = left.map(|c| c.to_ascii_lowercase().contains(&q)).unwrap_or(false);
                let rm = right.map(|c| c.to_ascii_lowercase().contains(&q)).unwrap_or(false);
                let side = match (lm, rm) {
                    (true,  true)  => Some(MatchSide::Both),
                    (true,  false) => Some(MatchSide::Left),
                    (false, true)  => Some(MatchSide::Right),
                    (false, false) => None,
                };
                if let Some(side) = side {
                    positions.push(MatchPosition { hunk_elem_id: hunk_elem_id.clone(),
                        hunk_id, row_index, side });
                }
            }
        }

        // Focus the first match automatically.
        let focused_index = if positions.is_empty() { None } else { Some(0) };
        Self { positions, focused_index }
    }

    /// Total number of matches.
    pub fn len(&self) -> usize { self.positions.len() }
    pub fn is_empty(&self) -> bool { self.positions.is_empty() }

    /// 1-based focused match number for display ("3 / 12").
    pub fn focused_number(&self) -> Option<usize> {
        self.focused_index.map(|i| i + 1)
    }

    /// The currently focused position, if any.
    pub fn focused(&self) -> Option<&MatchPosition> {
        self.focused_index.and_then(|i| self.positions.get(i))
    }

    /// Advance to the next match (wrapping). Returns the new focused position.
    pub fn next(&mut self) -> Option<&MatchPosition> {
        if self.positions.is_empty() { return None; }
        self.focused_index = Some(match self.focused_index {
            None    => 0,
            Some(i) => (i + 1) % self.positions.len(),
        });
        self.focused()
    }

    /// Move to the previous match (wrapping). Returns the new focused position.
    pub fn prev(&mut self) -> Option<&MatchPosition> {
        if self.positions.is_empty() { return None; }
        self.focused_index = Some(match self.focused_index {
            None    => 0,
            Some(0) => self.positions.len() - 1,
            Some(i) => i - 1,
        });
        self.focused()
    }

    /// Reset focus to the first match (e.g. after a query change).
    pub fn reset_focus(&mut self) {
        self.focused_index = if self.positions.is_empty() { None } else { Some(0) };
    }

    /// Set of hunk IDs that contain at least one match — used by the
    /// renderer to ensure matching hunks are auto-expanded.
    pub fn matching_hunk_ids(&self) -> HashSet<u64> {
        self.positions.iter().map(|p| p.hunk_id).collect()
    }

    /// `true` if `hunk_id` / `row_index` is the currently focused match.
    pub fn is_focused(&self, hunk_id: u64, row_index: usize) -> bool {
        self.focused().map(|p| p.hunk_id == hunk_id && p.row_index == row_index)
            .unwrap_or(false)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rows<'a>(contents: &[(&'a str, &'a str)]) -> Vec<(Option<&'a str>, Option<&'a str>)> {
        contents.iter().map(|(l, r)| (Some(*l), Some(*r))).collect()
    }

    fn index_for(query: &str, hunks: &[(u64, Vec<(Option<&str>, Option<&str>)>)]) -> MatchIndex {
        MatchIndex::build(
            hunks.iter().map(|(id, rs)| (*id, rs.as_slice())),
            query,
        )
    }

    #[test]
    fn empty_query_produces_empty_index() {
        let hunks = vec![(1u64, rows(&[("hello", "world")]))];
        let idx = index_for("", &hunks);
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn non_matching_query_produces_empty_index() {
        let hunks = vec![(1u64, rows(&[("hello", "world")]))];
        let idx = index_for("zzz", &hunks);
        assert!(idx.is_empty());
    }

    #[test]
    fn single_match_is_focused_automatically() {
        let hunks = vec![(1u64, rows(&[("hello world", "unchanged")]))];
        let idx = index_for("hello", &hunks);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.focused_number(), Some(1));
        let pos = idx.focused().unwrap();
        assert_eq!(pos.hunk_id, 1);
        assert_eq!(pos.row_index, 0);
        assert_eq!(pos.side, MatchSide::Left);
    }

    #[test]
    fn match_is_case_insensitive() {
        let hunks = vec![(1u64, rows(&[("HELLO", "World")]))];
        let idx = index_for("hello", &hunks);
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn both_sides_match_produces_both_variant() {
        let hunks = vec![(1u64, rows(&[("foo bar", "foo baz")]))];
        let idx = index_for("foo", &hunks);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.focused().unwrap().side, MatchSide::Both);
    }

    #[test]
    fn matches_span_multiple_hunks_in_order() {
        let hunks = vec![
            (10u64, rows(&[("alpha line", "unchanged"), ("beta line", "unchanged")])),
            (20u64, rows(&[("unchanged", "alpha here")])),
        ];
        let idx = index_for("alpha", &hunks);
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.positions[0].hunk_id, 10);
        assert_eq!(idx.positions[0].row_index, 0);
        assert_eq!(idx.positions[1].hunk_id, 20);
    }

    #[test]
    fn next_advances_and_wraps() {
        let hunks = vec![(1u64, rows(&[("a match", "a match"), ("a match", "no")]))];
        let mut idx = index_for("match", &hunks);
        assert_eq!(idx.focused_number(), Some(1));
        idx.next();
        assert_eq!(idx.focused_number(), Some(2));
        idx.next(); // wraps to 1
        assert_eq!(idx.focused_number(), Some(1));
    }

    #[test]
    fn prev_goes_backward_and_wraps() {
        let hunks = vec![(1u64, rows(&[("needle", "needle"), ("needle", "needle")]))];
        let mut idx = index_for("needle", &hunks);
        // Focus starts at 1; going prev wraps to 4 (or len).
        idx.prev();
        assert_eq!(idx.focused_number(), Some(idx.len()));
    }

    #[test]
    fn reset_focus_returns_to_first_match() {
        let hunks = vec![(1u64, rows(&[("x", "x"), ("x", "x")]))];
        let mut idx = index_for("x", &hunks);
        idx.next(); idx.next();
        idx.reset_focus();
        assert_eq!(idx.focused_number(), Some(1));
    }

    #[test]
    fn matching_hunk_ids_returns_correct_set() {
        let hunks = vec![
            (5u64, rows(&[("match here", "no")])),
            (9u64, rows(&[("nothing", "nothing")])),
        ];
        let idx = index_for("match", &hunks);
        let ids = idx.matching_hunk_ids();
        assert!(ids.contains(&5));
        assert!(!ids.contains(&9));
    }

    #[test]
    fn is_focused_correctly_identifies_focused_row() {
        let hunks = vec![(7u64, rows(&[("find me", "also find me")]))];
        let idx = index_for("find", &hunks);
        assert!(idx.is_focused(7, 0));
        assert!(!idx.is_focused(7, 1));
        assert!(!idx.is_focused(8, 0));
    }

    #[test]
    fn none_side_row_produces_no_entry() {
        // A row where only the left side exists (insert row: right is None).
        let hunks = vec![(1u64, vec![(Some("find me"), None)])];
        let idx = index_for("find", &hunks);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.focused().unwrap().side, MatchSide::Left);
    }

    #[test]
    fn hunk_elem_id_format_is_correct() {
        let hunks = vec![(42u64, rows(&[("target", "no")]))];
        let idx = index_for("target", &hunks);
        assert_eq!(idx.focused().unwrap().hunk_elem_id, "h-42");
    }
}
