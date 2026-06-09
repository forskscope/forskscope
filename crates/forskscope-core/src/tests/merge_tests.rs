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
