//! LineMap, AlignedRow, ScrollAnchor, and mini-map tests (RFC-035).

use crate::diff::{DiffOptions, compute_diff};
use crate::line_map::{
    LineMap, RowState, ScrollAnchor, build_mini_map,
};

fn map(old: &str, new: &str) -> LineMap {
    let doc = compute_diff(old, new, DiffOptions::default());
    LineMap::from_diff(&doc)
}

// ── RowState ──────────────────────────────────────────────────────────────────

#[test]
fn row_state_gutter_symbols_are_distinct() {
    let states = [
        RowState::Equal, RowState::Inserted, RowState::Deleted,
        RowState::Modified, RowState::Conflict, RowState::Collapsed,
        RowState::Unknown,
    ];
    let syms: std::collections::HashSet<char> =
        states.iter().map(|s| s.gutter_symbol()).collect();
    assert_eq!(syms.len(), states.len(), "all gutter symbols must be distinct");
}

#[test]
fn is_changed_is_true_for_insert_delete_modified_conflict() {
    assert!(!RowState::Equal.is_changed());
    assert!( RowState::Inserted.is_changed());
    assert!( RowState::Deleted.is_changed());
    assert!( RowState::Modified.is_changed());
    assert!( RowState::Conflict.is_changed());
    assert!(!RowState::Collapsed.is_changed());
    assert!(!RowState::Unknown.is_changed());
}

// ── LineMap from identical document ──────────────────────────────────────────

#[test]
fn identical_docs_produce_no_changed_rows() {
    let m = map("hello\nworld\n", "hello\nworld\n");
    assert!(m.is_identical());
    assert_eq!(m.changed_row_count, 0);
    assert!(!m.rows.is_empty(), "rows exist even for identical docs");
}

// ── LineMap from insert ───────────────────────────────────────────────────────

#[test]
fn insert_adds_right_only_row() {
    let m = map("a\n", "a\nb\n");
    let inserted = m.rows.iter().any(|r| r.state == RowState::Inserted);
    assert!(inserted, "insert hunk must produce an Inserted row");
    assert_eq!(m.changed_row_count, 1);
}

#[test]
fn inserted_row_has_no_left_span() {
    let m = map("a\n", "a\nb\n");
    let inserted_row = m.rows.iter().find(|r| r.state == RowState::Inserted).unwrap();
    assert!(inserted_row.left.is_none(), "pure insert row must have no left span");
    assert!(inserted_row.right.is_some(), "pure insert row must have right span");
}

// ── LineMap from delete ───────────────────────────────────────────────────────

#[test]
fn delete_adds_left_only_row() {
    let m = map("a\nb\n", "a\n");
    let deleted = m.rows.iter().any(|r| r.state == RowState::Deleted);
    assert!(deleted);
    let deleted_row = m.rows.iter().find(|r| r.state == RowState::Deleted).unwrap();
    assert!(deleted_row.right.is_none(), "pure delete row must have no right span");
}

// ── LineMap from replace ──────────────────────────────────────────────────────

#[test]
fn replace_produces_modified_rows() {
    let m = map("hello\n", "world\n");
    let modified_count = m.rows.iter().filter(|r| r.state == RowState::Modified).count();
    assert!(modified_count > 0, "replace hunk must produce Modified rows");
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[test]
fn next_changed_row_finds_first_change_from_start() {
    let m = map("a\n", "a\nb\n");
    let row = m.next_changed_row(0);
    assert!(row.is_some(), "must find a changed row");
    assert!(row.unwrap().state.is_changed());
}

#[test]
fn prev_changed_row_returns_none_before_any_change() {
    let m = map("a\n", "a\nb\n");
    // Row 0 is Equal ("a\n"), change is at row 1.
    let row = m.prev_changed_row(1);
    assert!(row.is_none() || !row.unwrap().state.is_changed(),
        "prev from before first change must return None");
}

#[test]
fn changed_rows_iterator_count_matches_changed_row_count() {
    let m = map("a\nb\nc\n", "a\nB\nC\n");
    assert_eq!(m.changed_rows().count(), m.changed_row_count);
}

// ── AlignedRow pairing ────────────────────────────────────────────────────────

#[test]
fn equal_rows_are_paired() {
    let m = map("a\nb\n", "a\nc\n");
    let equal_rows: Vec<_> = m.rows.iter().filter(|r| r.state == RowState::Equal).collect();
    assert!(equal_rows.iter().all(|r| r.is_paired()),
        "equal rows must have both left and right spans");
}

// ── ScrollAnchor ──────────────────────────────────────────────────────────────

#[test]
fn scroll_anchor_at_top_is_row_zero() {
    let a = ScrollAnchor::at_top();
    assert_eq!(a.row_index, 0);
    assert_eq!(a.row_fraction, 0.0);
}

#[test]
fn scroll_anchor_clamped_clamps_fraction() {
    let a = ScrollAnchor::clamped(5, 1.5);
    assert!(a.row_fraction < 1.0, "fraction must be clamped below 1.0");
    let b = ScrollAnchor::clamped(5, -0.5);
    assert_eq!(b.row_fraction, 0.0, "negative fraction must be clamped to 0.0");
}

// ── Mini-map ──────────────────────────────────────────────────────────────────

#[test]
fn build_mini_map_merges_consecutive_equal_rows() {
    let m = map("a\nb\nc\n", "a\nb\nc\n");
    let segments = build_mini_map(&m);
    assert_eq!(segments.len(), 1, "all-equal diff must produce one segment");
    assert_eq!(segments[0].state, RowState::Equal);
}

#[test]
fn build_mini_map_splits_on_state_change() {
    let m = map("a\n", "b\n");
    let segments = build_mini_map(&m);
    // A replace hunk with no surrounding context = just the Modified segment.
    assert!(segments.iter().any(|s| s.state == RowState::Modified),
        "replace hunk must appear as Modified segment in mini-map");
}

#[test]
fn mini_map_weights_sum_to_total_row_count() {
    let m = map("a\nb\nc\n", "a\nB\nC\n");
    let segments = build_mini_map(&m);
    let total_weight: u32 = segments.iter().map(|s| s.weight).sum();
    assert_eq!(total_weight as usize, m.rows.len(),
        "segment weights must sum to total row count");
}

#[test]
fn empty_diff_produces_no_segments() {
    let m = map("", "");
    let segments = build_mini_map(&m);
    assert!(segments.is_empty());
}
