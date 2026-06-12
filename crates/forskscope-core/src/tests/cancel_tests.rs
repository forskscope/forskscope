//! Tests for the lightweight cancellation token (RFC-037, RFC-008).

use crate::cancel::CancellationToken;

// ── Basic state machine ───────────────────────────────────────────────────────

#[test]
fn new_token_is_not_cancelled() {
    let t = CancellationToken::new();
    assert!(!t.is_cancelled());
}

#[test]
fn cancel_sets_cancelled() {
    let t = CancellationToken::new();
    t.cancel();
    assert!(t.is_cancelled());
}

#[test]
fn cancel_is_idempotent() {
    let t = CancellationToken::new();
    t.cancel();
    t.cancel(); // second call must not panic
    assert!(t.is_cancelled());
}

// ── Clone propagation ─────────────────────────────────────────────────────────

#[test]
fn clone_observes_cancel_from_original() {
    let original = CancellationToken::new();
    let worker = original.clone();
    assert!(!worker.is_cancelled(), "clone must start uncancelled");
    original.cancel();
    assert!(worker.is_cancelled(), "clone must observe cancel from original");
}

#[test]
fn original_observes_cancel_from_clone() {
    let original = CancellationToken::new();
    let worker = original.clone();
    worker.cancel();
    assert!(original.is_cancelled(), "original must observe cancel from clone");
}

#[test]
fn multiple_clones_all_observe_cancel() {
    let t = CancellationToken::new();
    let c1 = t.clone();
    let c2 = t.clone();
    let c3 = c1.clone(); // clone of clone
    t.cancel();
    assert!(c1.is_cancelled());
    assert!(c2.is_cancelled());
    assert!(c3.is_cancelled());
}

#[test]
fn cancel_from_any_clone_propagates_to_all() {
    let t = CancellationToken::new();
    let c1 = t.clone();
    let c2 = t.clone();
    c2.cancel(); // cancel from a non-original clone
    assert!(t.is_cancelled());
    assert!(c1.is_cancelled());
}

// ── Default ───────────────────────────────────────────────────────────────────

#[test]
fn default_token_is_not_cancelled() {
    let t = CancellationToken::default();
    assert!(!t.is_cancelled());
}

// ── Debug ─────────────────────────────────────────────────────────────────────

#[test]
fn debug_format_does_not_panic() {
    let t = CancellationToken::new();
    let _ = format!("{t:?}");
    t.cancel();
    let _ = format!("{t:?}");
}
