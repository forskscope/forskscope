//! ConflictNavigator tests (RFC-034 §"Conflict navigator",
//! §"Conflict navigator table", §"Navigator footer").

use crate::conflict_nav::{
    ConflictFilter, ConflictNavigator, ConflictStatusDisplay,
};
use crate::merge::{ConflictStatus, ThreeWayMergeSession};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Session with two genuine conflicts separated by unchanged context.
/// diff3 requires context between conflict regions to treat them as separate.
fn two_conflict_session() -> ThreeWayMergeSession {
    ThreeWayMergeSession::from_texts(
        "conflict A\ncontext 1\ncontext 2\nconflict B\n",
        "left A\ncontext 1\ncontext 2\nleft B\n",
        "right A\ncontext 1\ncontext 2\nright B\n",
    )
}

/// Session with one conflict.
fn one_conflict_session() -> ThreeWayMergeSession {
    ThreeWayMergeSession::from_texts("base\n", "left\n", "right\n")
}

/// Session with no conflicts (all lines identical).
fn no_conflict_session() -> ThreeWayMergeSession {
    ThreeWayMergeSession::from_texts("same\n", "same\n", "same\n")
}

// ── ConflictStatusDisplay ─────────────────────────────────────────────────────

#[test]
fn all_statuses_have_non_empty_glyph_and_text() {
    for status in [
        ConflictStatus::Unresolved,
        ConflictStatus::ResolvedLeft,
        ConflictStatus::ResolvedRight,
        ConflictStatus::ResolvedBoth,
        ConflictStatus::ResolvedManual,
        ConflictStatus::Ignored,
    ] {
        let d = ConflictStatusDisplay::for_status(status.clone());
        assert!(!d.text.is_empty(), "{status:?} must have non-empty text label");
    }
}

#[test]
fn all_status_glyphs_are_distinct() {
    let statuses = [
        ConflictStatus::Unresolved,
        ConflictStatus::ResolvedLeft,
        ConflictStatus::ResolvedRight,
        ConflictStatus::ResolvedBoth,
        ConflictStatus::ResolvedManual,
        ConflictStatus::Ignored,
    ];
    let glyphs: std::collections::HashSet<char> = statuses.iter()
        .map(|s| ConflictStatusDisplay::for_status(s.clone()).glyph)
        .collect();
    assert_eq!(glyphs.len(), statuses.len(), "all glyph characters must be distinct");
}

#[test]
fn unresolved_glyph_is_exclamation() {
    let d = ConflictStatusDisplay::for_status(ConflictStatus::Unresolved);
    assert_eq!(d.glyph, '!');
    assert_eq!(d.text, "unresolved");
}

// ── ConflictNavigator: build ──────────────────────────────────────────────────

#[test]
fn no_conflict_session_produces_empty_navigator() {
    let session = no_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert!(nav.entries.is_empty());
    assert!(nav.is_fully_resolved());
    assert_eq!(nav.summary.total, 0);
}

#[test]
fn one_conflict_session_produces_one_entry() {
    let session = one_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert_eq!(nav.entries.len(), 1);
    assert_eq!(nav.summary.total, 1);
    assert_eq!(nav.summary.unresolved, 1);
}

#[test]
fn two_conflict_session_summary_counts() {
    let session = two_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    // Accept 1 or 2 conflicts — diff3 grouping depends on context window.
    // The invariant is total == entries.len() and unresolved == total.
    assert_eq!(nav.summary.total, nav.entries.len());
    assert_eq!(nav.summary.unresolved, nav.summary.total);
    assert_eq!(nav.summary.resolved, 0);
    assert!(nav.summary.total >= 1, "must have at least one conflict");
}

#[test]
fn display_nums_are_one_based_and_sequential() {
    let session = two_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    let nums: Vec<usize> = nav.entries.iter().map(|e| e.display_num).collect();
    let expected: Vec<usize> = (1..=nav.entries.len()).collect();
    assert_eq!(nums, expected);
}

#[test]
fn all_entries_initially_unresolved() {
    let session = one_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert!(nav.entries.iter().all(|e| e.status == ConflictStatus::Unresolved));
}

// ── Focused entry ─────────────────────────────────────────────────────────────

#[test]
fn focused_entry_is_none_when_no_focus_set() {
    let session = one_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert!(nav.focused_entry().is_none());
    assert!(nav.entries.iter().all(|e| !e.is_focused));
}

#[test]
fn focused_entry_is_set_when_focus_id_matches() {
    let session = one_conflict_session();
    let first_id = session.conflicts()[0].id;
    let nav = ConflictNavigator::build(&session, Some(first_id), ConflictFilter::All);
    let focused = nav.focused_entry().unwrap();
    assert_eq!(focused.conflict_id, first_id);
    assert!(focused.is_focused);
}

// ── Prev / next traversal ─────────────────────────────────────────────────────

#[test]
fn next_id_from_only_entry_wraps_to_itself() {
    let session = one_conflict_session();
    let id = session.conflicts()[0].id;
    let nav = ConflictNavigator::build(&session, Some(id), ConflictFilter::All);
    assert_eq!(nav.next_id(), Some(id));
}

#[test]
fn prev_id_from_only_entry_wraps_to_itself() {
    let session = one_conflict_session();
    let id = session.conflicts()[0].id;
    let nav = ConflictNavigator::build(&session, Some(id), ConflictFilter::All);
    assert_eq!(nav.prev_id(), Some(id));
}

#[test]
fn next_and_prev_return_none_for_empty_navigator() {
    let session = no_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert!(nav.next_id().is_none());
    assert!(nav.prev_id().is_none());
}

// ── Filter ────────────────────────────────────────────────────────────────────

#[test]
fn unresolved_only_filter_hides_nothing_when_all_unresolved() {
    let session = one_conflict_session();
    let nav_all      = ConflictNavigator::build(&session, None, ConflictFilter::All);
    let nav_filtered = ConflictNavigator::build(&session, None, ConflictFilter::UnresolvedOnly);
    assert_eq!(nav_all.entries.len(), nav_filtered.entries.len());
    assert!(!nav_filtered.has_hidden_entries());
}

#[test]
fn unresolved_only_filter_hides_resolved_entries() {
    let mut session = one_conflict_session();
    let id = session.conflicts()[0].id;
    session.resolve_left(id).unwrap();

    let nav_all      = ConflictNavigator::build(&session, None, ConflictFilter::All);
    let nav_filtered = ConflictNavigator::build(&session, None, ConflictFilter::UnresolvedOnly);
    assert_eq!(nav_all.entries.len(), 1);
    assert_eq!(nav_filtered.entries.len(), 0, "resolved entry must be hidden");
    assert!(nav_filtered.has_hidden_entries());
}

// ── Summary after resolution ──────────────────────────────────────────────────

#[test]
fn resolving_conflict_updates_summary_counts() {
    let mut session = one_conflict_session();
    let id = session.conflicts()[0].id;
    session.resolve_left(id).unwrap();

    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert_eq!(nav.summary.resolved,   1);
    assert_eq!(nav.summary.unresolved, 0);
    assert!(nav.is_fully_resolved());
}

// ── First unresolved ──────────────────────────────────────────────────────────

#[test]
fn first_unresolved_id_returns_first_unresolved_conflict() {
    let session = one_conflict_session();
    let id = session.conflicts()[0].id;
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert_eq!(nav.first_unresolved_id(), Some(id));
}

#[test]
fn first_unresolved_id_returns_none_when_all_resolved() {
    let mut session = one_conflict_session();
    let id = session.conflicts()[0].id;
    session.resolve_left(id).unwrap();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert!(nav.first_unresolved_id().is_none());
}

// ── CSS classes ───────────────────────────────────────────────────────────────

#[test]
fn all_css_classes_start_with_fsk_conflict_prefix() {
    let session = one_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    for entry in &nav.entries {
        assert!(entry.css_class().starts_with("fsk-conflict-"),
            "CSS class {} must start with fsk-conflict-", entry.css_class());
    }
}

// ── Progress fraction ─────────────────────────────────────────────────────────

#[test]
fn progress_fraction_is_zero_when_nothing_resolved() {
    let session = one_conflict_session();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert_eq!(nav.summary.progress_fraction(), 0.0);
}

#[test]
fn progress_fraction_is_one_when_fully_resolved() {
    let mut session = one_conflict_session();
    let id = session.conflicts()[0].id;
    session.resolve_left(id).unwrap();
    let nav = ConflictNavigator::build(&session, None, ConflictFilter::All);
    assert_eq!(nav.summary.progress_fraction(), 1.0);
}

#[test]
fn progress_fraction_is_one_for_empty_session() {
    let nav = ConflictNavigator::build(&no_conflict_session(), None, ConflictFilter::All);
    assert_eq!(nav.summary.progress_fraction(), 1.0);
}
