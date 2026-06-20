//! Corpus-driven three-way merge correctness tests (RFC-033).
//!
//! These tests load fixture triples from `tests/fixtures/merge/` and verify
//! that `ThreeWayMergeSession` produces correct conflict detection,
//! auto-merge, and result text for documented real-world scenarios.
//!
//! Each scenario uses a `base_*.txt` / `left_*.txt` / `right_*.txt` triple.
//! See `tests/fixtures/README.md` for descriptions of each triple.

use std::fs;
use std::path::Path;

use forskscope_core::merge::ThreeWayMergeSession;
use forskscope_core::ConflictId;

fn load(path: &str) -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set by cargo test");
    let full = Path::new(&manifest)
        .join("../../tests/fixtures/merge")
        .join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"))
}

fn session(scenario: &str) -> ThreeWayMergeSession {
    ThreeWayMergeSession::from_texts(
        &load(&format!("base_{scenario}.txt")),
        &load(&format!("left_{scenario}.txt")),
        &load(&format!("right_{scenario}.txt")),
    )
}

// ── No conflicts: non-overlapping changes ─────────────────────────────────────

#[test]
fn noconflict_fixture_has_no_conflicts() {
    let s = session("noconflict");
    assert_eq!(s.stats().conflicts_total, 0,
        "non-overlapping changes must auto-merge without conflicts");
}

#[test]
fn noconflict_result_contains_both_changes() {
    let s = session("noconflict");
    // Left changed "alpha" → "ALPHA"; right changed "delta" → "DELTA"
    let result = s.result_text();
    assert!(result.contains("ALPHA"), "result must contain left-side change ALPHA");
    assert!(result.contains("DELTA"), "result must contain right-side change DELTA");
}

#[test]
fn noconflict_can_save_immediately() {
    let s = session("noconflict");
    assert!(s.can_save(), "no-conflict session must be saveable without resolution");
}

// ── Conflict: both sides change the same line differently ─────────────────────

#[test]
fn conflict_fixture_has_one_conflict() {
    let s = session("conflict");
    assert_eq!(s.stats().conflicts_total, 1,
        "divergent single-line change must produce exactly one conflict");
}

#[test]
fn conflict_blocks_save_until_resolved() {
    let s = session("conflict");
    assert!(!s.can_save(), "session with unresolved conflict must not be saveable");
}

#[test]
fn conflict_resolve_left_enables_save_and_uses_left_content() {
    let mut s = session("conflict");
    let id: ConflictId = s.conflicts().iter().next().unwrap().id;
    s.resolve_left(id).expect("resolve_left must succeed");
    assert!(s.can_save(), "session must be saveable after resolving conflict");
    let result = s.result_text();
    assert!(result.contains("LEFT"), "result must contain left-side resolution");
    assert!(!result.contains("RIGHT"), "result must not contain right-side after resolving left");
}

#[test]
fn conflict_resolve_right_uses_right_content() {
    let mut s = session("conflict");
    let id: ConflictId = s.conflicts().iter().next().unwrap().id;
    s.resolve_right(id).expect("resolve_right must succeed");
    let result = s.result_text();
    assert!(result.contains("RIGHT"), "result must contain right-side resolution");
}

// ── Both-same: identical changes on both sides auto-merge ─────────────────────

#[test]
fn both_same_fixture_has_no_conflicts() {
    let s = session("both_same");
    assert_eq!(s.stats().conflicts_total, 0,
        "identical changes on both sides must auto-merge without conflict");
}

#[test]
fn both_same_result_contains_shared_change() {
    let s = session("both_same");
    let result = s.result_text();
    assert!(result.contains("BRAVO"),
        "result must contain the shared change (BRAVO)");
}

// ── Left insert: one side inserts, other unchanged ────────────────────────────

#[test]
fn left_insert_fixture_auto_merges() {
    let s = session("left_insert");
    assert_eq!(s.stats().conflicts_total, 0,
        "insert on one side only must auto-merge");
    let result = s.result_text();
    assert!(result.contains("bravo"),
        "result must contain left-side insertion (bravo)");
}

// ── CRLF: line terminators preserved through merge ───────────────────────────

#[test]
fn crlf_fixture_preserves_crlf_in_result() {
    let s = session("crlf");
    let result = s.result_text();
    // The base and right both use CRLF; result must preserve CRLF
    assert!(result.contains("\r\n"),
        "CRLF line endings must be preserved through merge");
}

#[test]
fn crlf_fixture_contains_left_change() {
    let s = session("crlf");
    // Left changed "bravo\r\n" → "BRAVO\r\n"
    let result = s.result_text();
    assert!(result.to_ascii_uppercase().contains("BRAVO"),
        "CRLF fixture result must contain left-side change");
}

// ── Multiple conflicts ────────────────────────────────────────────────────────

#[test]
fn multi_fixture_has_three_conflicts() {
    let s = session("multi");
    assert_eq!(s.stats().conflicts_total, 3,
        "multi fixture has three divergent lines → three conflicts");
}

#[test]
fn multi_fixture_all_conflicts_unresolved_blocks_save() {
    let s = session("multi");
    assert!(!s.can_save());
}

#[test]
fn multi_fixture_resolve_all_enables_save() {
    let mut s = session("multi");
    let ids: Vec<ConflictId> = s.conflicts().iter().map(|c| c.id).collect();
    assert_eq!(ids.len(), 3);
    for id in ids {
        s.resolve_left(id).expect("resolve_left must succeed");
    }
    assert!(s.can_save(), "all resolved → can_save must be true");
}

#[test]
fn multi_fixture_result_after_all_left_resolutions_matches_left() {
    let mut s = session("multi");
    let ids: Vec<ConflictId> = s.conflicts().iter().map(|c| c.id).collect();
    for id in ids {
        s.resolve_left(id).expect("resolve_left must succeed");
    }
    let result = s.result_text();
    // Left has A, C, E (uppercase single chars)
    assert!(result.contains('A'), "result must contain 'A' from left");
    assert!(result.contains('C'), "result must contain 'C' from left");
    assert!(result.contains('E'), "result must contain 'E' from left");
}
