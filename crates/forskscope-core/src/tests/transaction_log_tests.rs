//! Transaction log tests (RFC-015 §13 "Testing Requirements", §14 "Acceptance Criteria").
//!
//! Tests validate: push/undo/redo semantics, revision tracking, dirty state,
//! clean baseline after save, active/undone entry split, label generation,
//! redo branch discard on new push, and integration with the existing
//! `MergeSession` and `ThreeWayMergeSession` APIs.

use crate::diff::{DiffOptions, compute_diff};
use crate::merge::{
    ConflictStatus, MergeSession, SessionRevision, ThreeWayMergeSession,
    TransactionKind, TransactionLog,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn simple_log() -> TransactionLog {
    TransactionLog::new()
}

fn push_apply(log: &mut TransactionLog, hunk: u64) {
    log.push(TransactionKind::ApplyHunkLeftToRight { hunk_id: hunk });
}

// ── Revision tracking ─────────────────────────────────────────────────────────

#[test]
fn new_log_is_at_initial_revision() {
    let log = simple_log();
    assert_eq!(log.current_revision(), SessionRevision::INITIAL);
}

#[test]
fn push_increments_revision() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    assert_eq!(log.current_revision(), SessionRevision(1));
    push_apply(&mut log, 2);
    assert_eq!(log.current_revision(), SessionRevision(2));
}

#[test]
fn undo_decrements_revision() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    push_apply(&mut log, 2);
    log.record_undo();
    assert_eq!(log.current_revision(), SessionRevision(1));
    log.record_undo();
    assert_eq!(log.current_revision(), SessionRevision::INITIAL);
}

#[test]
fn redo_restores_revision() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.record_undo();
    log.record_redo();
    assert_eq!(log.current_revision(), SessionRevision(1));
}

// ── Dirty state ───────────────────────────────────────────────────────────────

#[test]
fn new_log_is_not_dirty() {
    assert!(!simple_log().is_dirty());
}

#[test]
fn dirty_after_push() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    assert!(log.is_dirty());
}

#[test]
fn mark_saved_clears_dirty() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.mark_saved();
    assert!(!log.is_dirty());
}

#[test]
fn dirty_again_after_push_past_save() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.mark_saved();
    push_apply(&mut log, 2);
    assert!(log.is_dirty());
}

#[test]
fn undo_below_save_baseline_makes_dirty_again() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    push_apply(&mut log, 2);
    log.mark_saved();
    log.record_undo(); // now behind save baseline
    assert!(log.is_dirty());
}

#[test]
fn redo_back_to_save_clears_dirty() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.mark_saved();
    log.record_undo();
    assert!(log.is_dirty());
    log.record_redo();
    assert!(!log.is_dirty(), "redo back to save baseline should clear dirty");
}

// ── Can undo / redo ───────────────────────────────────────────────────────────

#[test]
fn can_undo_after_push() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    assert!(log.can_undo());
    assert!(!log.can_redo());
}

#[test]
fn can_redo_after_undo() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.record_undo();
    assert!(!log.can_undo());
    assert!(log.can_redo());
}

// ── Redo branch discard (RFC-015 §8 rule 1) ──────────────────────────────────

#[test]
fn new_push_after_undo_discards_redo_branch() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    push_apply(&mut log, 2);
    log.record_undo(); // undo hunk 2 → redo available
    push_apply(&mut log, 3); // new operation → redo branch discarded
    assert!(!log.can_redo(), "new push must discard redo branch");
    assert_eq!(log.len(), 2, "entries = hunk1 + hunk3 (hunk2 discarded)");
}

// ── Entry visibility ──────────────────────────────────────────────────────────

#[test]
fn active_entries_excludes_undone() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    push_apply(&mut log, 2);
    log.record_undo();
    assert_eq!(log.active_entries().len(), 1);
    assert_eq!(log.undone_entries().len(), 1);
}

#[test]
fn all_entries_includes_undone() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.record_undo();
    assert_eq!(log.all_entries().len(), 1,
        "all_entries must still show undone entries for history panel");
}

#[test]
fn last_active_is_none_when_fully_undone() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.record_undo();
    assert!(log.last_active().is_none());
}

#[test]
fn last_active_is_most_recent_push() {
    let mut log = simple_log();
    push_apply(&mut log, 42);
    let entry = log.last_active().unwrap();
    assert!(matches!(entry.kind, TransactionKind::ApplyHunkLeftToRight { hunk_id: 42 }));
}

// ── active_ops_since_save ─────────────────────────────────────────────────────

#[test]
fn active_ops_since_save_counts_correctly() {
    let mut log = simple_log();
    push_apply(&mut log, 1);
    log.mark_saved();
    push_apply(&mut log, 2);
    push_apply(&mut log, 3);
    assert_eq!(log.active_ops_since_save(), 2);
}

// ── Labels ───────────────────────────────────────────────────────────────────

#[test]
fn transaction_kind_labels_are_non_empty() {
    let kinds = [
        TransactionKind::ApplyHunkLeftToRight { hunk_id: 1 },
        TransactionKind::RevertHunk { hunk_id: 2 },
        TransactionKind::ApplyAllLeftToRight,
        TransactionKind::ResolveConflictLeft { conflict_id: 0 },
        TransactionKind::ResolveConflictRight { conflict_id: 0 },
        TransactionKind::ResolveConflictBoth { conflict_id: 0 },
        TransactionKind::ResolveConflictManual { conflict_id: 0 },
        TransactionKind::IgnoreConflict { conflict_id: 0 },
        TransactionKind::ReopenConflict { conflict_id: 0 },
        TransactionKind::ManualTextEdit,
        TransactionKind::ApplyExternalPatch,
    ];
    for k in &kinds {
        assert!(!k.label().is_empty(), "label must not be empty for {k:?}");
    }
}

#[test]
fn entry_stores_label_from_kind() {
    let mut log = simple_log();
    log.push(TransactionKind::ApplyHunkLeftToRight { hunk_id: 7 });
    let entry = log.last_active().unwrap();
    assert!(entry.label.contains("7"), "label should contain hunk id");
}

// ── Integration with MergeSession ────────────────────────────────────────────
// (RFC-015 §13: "Copy one hunk and undo", "Copy hunk, undo, redo")

#[test]
fn log_tracks_merge_session_apply_and_undo() {
    let left  = "a\nB\nc\n";
    let right = "a\nb\nc\n";
    let diff = compute_diff(left, right, DiffOptions::default());
    let mut session = MergeSession::from_diff(&diff);
    let mut log = TransactionLog::new();

    // Find the changed hunk and apply it.
    let hunk_id = session.hunks().iter()
        .find(|h| h.is_pending_change())
        .map(|h| h.hunk_id)
        .expect("expected a pending change");

    session.apply_left_to_right(hunk_id).unwrap();
    log.push(TransactionKind::ApplyHunkLeftToRight { hunk_id });

    assert!(session.is_dirty());
    assert!(log.is_dirty());
    assert_eq!(log.active_entries().len(), 1);

    // Undo in both.
    session.undo().unwrap();
    log.record_undo();

    assert!(!session.is_dirty(), "session should not be dirty after undo");
    assert!(!log.is_dirty(), "log should not be dirty after undo");
    assert_eq!(log.active_entries().len(), 0);
    assert!(log.can_redo());
}

#[test]
fn log_tracks_three_way_resolve_and_undo() {
    let mut session = ThreeWayMergeSession::from_texts(
        "a\nb\nc\n",
        "a\nLEFT\nc\n",
        "a\nRIGHT\nc\n",
    );
    let mut log = TransactionLog::new();

    let conflict_id = session.conflicts()[0].id;
    session.resolve_left(conflict_id).unwrap();
    log.push(TransactionKind::ResolveConflictLeft { conflict_id });

    assert!(session.can_save());
    assert!(log.is_dirty());

    session.undo().unwrap();
    log.record_undo();

    assert!(!session.can_save(), "unresolved after undo");
    assert!(!log.is_dirty());
    assert_eq!(session.conflicts()[0].status, ConflictStatus::Unresolved);
}

#[test]
fn save_marks_clean_baseline_in_both_session_and_log() {
    let left  = "a\nB\nc\n";
    let right = "a\nb\nc\n";
    let diff = compute_diff(left, right, DiffOptions::default());
    let mut session = MergeSession::from_diff(&diff);
    let mut log = TransactionLog::new();

    let hunk_id = session.hunks().iter()
        .find(|h| h.is_pending_change())
        .map(|h| h.hunk_id).unwrap();
    session.apply_left_to_right(hunk_id).unwrap();
    log.push(TransactionKind::ApplyHunkLeftToRight { hunk_id });

    session.mark_saved();
    log.mark_saved();

    assert!(!session.is_dirty());
    assert!(!log.is_dirty());
    assert_eq!(log.saved_revision(), log.current_revision());
}
