use crate::diff::{DiffOptions, HunkKind, compute_diff};
use crate::merge::{HunkState, MergeSession};

fn replace_session() -> MergeSession {
    let diff = compute_diff("keep\nold\nkeep2\n", "keep\nnew\nkeep2\n", DiffOptions::default());
    MergeSession::from_diff(&diff)
}

#[test]
fn new_session_is_clean_and_mirrors_right_side() {
    let session = replace_session();
    assert!(!session.is_dirty());
    assert_eq!(session.result_text(), "keep\nnew\nkeep2\n");
    assert_eq!(session.pending_changes(), 1);
}

#[test]
fn apply_left_to_right_updates_result_and_dirty_state() {
    let mut session = replace_session();
    let hunk_id = session
        .hunks()
        .iter()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap()
        .hunk_id;
    session.apply_left_to_right(hunk_id).unwrap();
    assert!(session.is_dirty());
    assert_eq!(session.result_text(), "keep\nold\nkeep2\n");
    let applied = session.hunks().iter().find(|h| h.hunk_id == hunk_id).unwrap();
    assert_eq!(applied.state, HunkState::AppliedLeftToRight);
    assert_eq!(session.pending_changes(), 0);
}

#[test]
fn undo_and_redo_restore_exact_state() {
    let mut session = replace_session();
    let hunk_id = session
        .hunks()
        .iter()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap()
        .hunk_id;
    session.apply_left_to_right(hunk_id).unwrap();

    session.undo().unwrap();
    assert_eq!(session.result_text(), "keep\nnew\nkeep2\n");
    assert!(!session.is_dirty());

    session.redo().unwrap();
    assert_eq!(session.result_text(), "keep\nold\nkeep2\n");
    assert!(session.is_dirty());
}

#[test]
fn applying_an_unknown_hunk_id_is_rejected() {
    let mut session = replace_session();
    assert!(session.apply_left_to_right(0xdead_beef).is_err());
}

#[test]
fn double_apply_is_rejected() {
    let mut session = replace_session();
    let hunk_id = session
        .hunks()
        .iter()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap()
        .hunk_id;
    session.apply_left_to_right(hunk_id).unwrap();
    assert!(session.apply_left_to_right(hunk_id).is_err());
}

#[test]
fn mark_saved_clears_dirty_state() {
    let mut session = replace_session();
    let hunk_id = session
        .hunks()
        .iter()
        .find(|h| h.kind == HunkKind::Replace)
        .unwrap()
        .hunk_id;
    session.apply_left_to_right(hunk_id).unwrap();
    assert!(session.is_dirty());
    session.mark_saved();
    assert!(!session.is_dirty());
}

// ── New tests for v0.32.0 ─────────────────────────────────────────────────────

#[test]
fn result_text_after_full_apply_equals_left_side() {
    let mut s = replace_session();
    // The session was built from ("old\n", "new\n"). Apply all hunks.
    let ids: Vec<u64> = s.hunks().iter().filter(|h| h.is_pending_change())
        .map(|h| h.hunk_id).collect();
    for id in ids { s.apply_left_to_right(id).unwrap(); }
    assert_eq!(s.result_text(), "keep\nold\nkeep2\n",
        "after applying all hunks the result should equal the left side");
}

#[test]
fn result_text_before_any_apply_equals_right_side() {
    let s = replace_session();
    assert_eq!(s.result_text(), "keep\nnew\nkeep2\n",
        "before any apply, result is the original right-side content");
}

#[test]
fn partial_apply_preserves_unapplied_hunks() {
    let left  = "line1\nold\nline3\n";
    let right = "line1\nnew\nline3\n";
    use crate::diff::DiffOptions;
    use crate::merge::MergeSession;
    let diff = crate::compute_diff(left, right, DiffOptions::default());
    let mut s = MergeSession::from_diff(&diff);
    assert_eq!(s.result_text(), right, "starts as right side");
    let hunk_id = s.hunks().iter().find(|h| h.is_pending_change()).map(|h| h.hunk_id).unwrap();
    s.apply_left_to_right(hunk_id).unwrap();
    assert_eq!(s.result_text(), left, "after applying the one hunk, result is left side");
}

#[test]
fn session_dirty_after_apply_clean_after_mark_saved() {
    let mut s = replace_session();
    assert!(!s.is_dirty());
    let id = s.hunks().iter().find(|h| h.is_pending_change()).map(|h| h.hunk_id).unwrap();
    s.apply_left_to_right(id).unwrap();
    assert!(s.is_dirty());
    s.mark_saved();
    assert!(!s.is_dirty());
}

#[test]
fn undo_after_apply_restores_result_text() {
    let mut s = replace_session();
    let id = s.hunks().iter().find(|h| h.is_pending_change()).map(|h| h.hunk_id).unwrap();
    let original = s.result_text();
    s.apply_left_to_right(id).unwrap();
    assert_ne!(s.result_text(), original);
    s.undo().unwrap();
    assert_eq!(s.result_text(), original, "undo must fully restore the original result");
}

#[test]
fn redo_after_undo_reapplies_the_change() {
    let mut s = replace_session();
    let id = s.hunks().iter().find(|h| h.is_pending_change()).map(|h| h.hunk_id).unwrap();
    s.apply_left_to_right(id).unwrap();
    let after_apply = s.result_text();
    s.undo().unwrap();
    assert!(s.can_redo(), "redo should be available after undo");
    s.redo().unwrap();
    assert_eq!(s.result_text(), after_apply, "redo must reproduce the applied state");
}

#[test]
fn can_undo_is_false_on_fresh_session() {
    let s = replace_session();
    assert!(!s.can_undo());
    assert!(!s.can_redo());
}

#[test]
fn multiple_independent_hunks_can_each_be_undone() {
    let left  = "a\nX\nb\nY\nc\n";
    let right = "a\nX2\nb\nY2\nc\n";
    use crate::{DiffOptions, MergeSession, compute_diff};
    let diff = compute_diff(left, right, DiffOptions::default());
    let mut s = MergeSession::from_diff(&diff);
    let ids: Vec<u64> = s.hunks().iter().filter(|h| h.is_pending_change())
        .map(|h| h.hunk_id).collect();
    assert_eq!(ids.len(), 2);
    for id in &ids { s.apply_left_to_right(*id).unwrap(); }
    assert_eq!(s.result_text(), left);
    // Undo both, in reverse order.
    s.undo().unwrap();
    s.undo().unwrap();
    assert_eq!(s.result_text(), right, "fully undone result must equal original right side");
}
