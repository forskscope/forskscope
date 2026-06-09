//! Three-way merge tests (RFC-033 §"Data Model", §"Save Policy").
//!
//! These validate the design contract: one-sided changes auto-merge,
//! identical two-sided changes deduplicate, divergent two-sided changes
//! become conflicts, resolution operations and undo/redo behave, the result
//! reconstructs correctly with line endings preserved, and the save policy
//! blocks while conflicts remain unresolved.

use crate::merge::{ConflictStatus, ThreeWayMergeSession};

fn session(base: &str, left: &str, right: &str) -> ThreeWayMergeSession {
    ThreeWayMergeSession::from_texts(base, left, right)
}

#[test]
fn no_changes_yields_base_and_no_conflicts() {
    let s = session("a\nb\nc\n", "a\nb\nc\n", "a\nb\nc\n");
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.result_text(), "a\nb\nc\n");
    assert!(s.can_save());
}

#[test]
fn left_only_change_auto_merges_to_left() {
    let base = "a\nb\nc\n";
    let left = "a\nB\nc\n";
    let right = "a\nb\nc\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.stats().auto_merged, 1);
    assert_eq!(s.result_text(), "a\nB\nc\n");
    assert!(s.can_save());
}

#[test]
fn right_only_change_auto_merges_to_right() {
    let base = "a\nb\nc\n";
    let left = "a\nb\nc\n";
    let right = "a\nb\nC\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.result_text(), "a\nb\nC\n");
}

#[test]
fn nonoverlapping_changes_both_apply() {
    // Left edits line 1, right edits line 3 — independent, no conflict.
    let base = "one\ntwo\nthree\n";
    let left = "ONE\ntwo\nthree\n";
    let right = "one\ntwo\nTHREE\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.result_text(), "ONE\ntwo\nTHREE\n");
}

#[test]
fn identical_changes_on_both_sides_deduplicate() {
    let base = "a\nb\nc\n";
    let left = "a\nX\nc\n";
    let right = "a\nX\nc\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0, "same edit must not conflict");
    assert_eq!(s.result_text(), "a\nX\nc\n");
}

#[test]
fn divergent_changes_produce_a_conflict() {
    let base = "a\nb\nc\n";
    let left = "a\nLEFT\nc\n";
    let right = "a\nRIGHT\nc\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 1);
    assert_eq!(s.stats().conflicts_unresolved, 1);
    assert!(!s.can_save(), "unresolved conflict must block save");
}

#[test]
fn resolve_left_picks_left_content_and_unblocks_save() {
    let s_base = "a\nb\nc\n";
    let mut s = session(s_base, "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_left(id).unwrap();
    assert!(s.conflict(id).unwrap().status == ConflictStatus::ResolvedLeft);
    assert!(s.is_fully_resolved());
    assert!(s.can_save());
    assert_eq!(s.result_text(), "a\nLEFT\nc\n");
}

#[test]
fn resolve_right_picks_right_content() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_right(id).unwrap();
    assert_eq!(s.result_text(), "a\nRIGHT\nc\n");
}

#[test]
fn resolve_both_takes_left_then_right() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_both(id).unwrap();
    assert_eq!(s.result_text(), "a\nLEFT\nRIGHT\nc\n");
}

#[test]
fn ignore_takes_base_content() {
    let mut s = session("a\nORIG\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.ignore(id).unwrap();
    assert_eq!(s.result_text(), "a\nORIG\nc\n");
}

#[test]
fn resolve_manual_inserts_custom_text() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_manual(id, "CUSTOM\n").unwrap();
    assert_eq!(s.result_text(), "a\nCUSTOM\nc\n");
}

#[test]
fn undo_and_redo_restore_resolution_state() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_left(id).unwrap();
    assert!(s.is_fully_resolved());

    s.undo().unwrap();
    assert!(!s.is_fully_resolved(), "undo should restore unresolved state");
    assert_eq!(s.conflict(id).unwrap().status, ConflictStatus::Unresolved);

    s.redo().unwrap();
    assert!(s.is_fully_resolved());
    assert_eq!(s.conflict(id).unwrap().status, ConflictStatus::ResolvedLeft);
}

#[test]
fn dirty_tracks_resolution_and_save_baseline() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    assert!(!s.is_dirty());
    s.resolve_left(id).unwrap();
    assert!(s.is_dirty());
    s.mark_saved();
    assert!(!s.is_dirty());
}

#[test]
fn multiple_conflicts_have_distinct_ids() {
    let base = "a\nb\nc\nd\ne\n";
    let left = "a\nL1\nc\nL2\ne\n";
    let right = "a\nR1\nc\nR2\ne\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 2);
    let ids: Vec<_> = s.conflicts().iter().map(|c| c.id).collect();
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn crlf_line_endings_are_preserved_through_merge() {
    let base = "a\r\nb\r\nc\r\n";
    let left = "a\r\nB\r\nc\r\n";
    let right = "a\r\nb\r\nc\r\n";
    let s = session(base, left, right);
    assert_eq!(s.result_text(), "a\r\nB\r\nc\r\n");
}

#[test]
fn insertion_on_one_side_auto_merges() {
    let base = "a\nc\n";
    let left = "a\nb\nc\n"; // left inserts "b"
    let right = "a\nc\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.result_text(), "a\nb\nc\n");
}

#[test]
fn deletion_on_one_side_auto_merges() {
    let base = "a\nb\nc\n";
    let left = "a\nc\n"; // left deletes "b"
    let right = "a\nb\nc\n";
    let s = session(base, left, right);
    assert_eq!(s.stats().conflicts_total, 0);
    assert_eq!(s.result_text(), "a\nc\n");
}

#[test]
fn unknown_conflict_id_is_rejected() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    assert!(s.resolve_left(0xdead_beef).is_err());
}

#[test]
fn reset_returns_conflict_to_unresolved() {
    let mut s = session("a\nb\nc\n", "a\nLEFT\nc\n", "a\nRIGHT\nc\n");
    let id = s.conflicts()[0].id;
    s.resolve_left(id).unwrap();
    s.reset(id).unwrap();
    assert_eq!(s.conflict(id).unwrap().status, ConflictStatus::Unresolved);
    assert!(!s.can_save());
}
