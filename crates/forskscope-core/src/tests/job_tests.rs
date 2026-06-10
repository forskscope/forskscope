//! Tests for `JobProgress`, `JobHandle`, and threshold policy constants
//! (RFC-013 §"Background Job Model", §"Thresholds").

use crate::job::{
    DIGEST_CONCURRENCY_LIMIT, LARGE_DIRECTORY_VIRTUAL_THRESHOLD,
    LARGE_FILE_INLINE_DIFF_BYTES, LARGE_HUNK_AUTO_EXPAND_LINES,
    VERY_LARGE_FILE_BYTES,
    JobHandle, JobKind, JobProgress,
};

// ── Threshold policy ──────────────────────────────────────────────────────────

#[test]
fn inline_diff_threshold_is_below_very_large_threshold() {
    // The inline-diff threshold must be lower than the very-large-file
    // threshold so a file can hit one without immediately hitting the other.
    assert!(
        LARGE_FILE_INLINE_DIFF_BYTES < VERY_LARGE_FILE_BYTES,
        "inline-diff threshold ({LARGE_FILE_INLINE_DIFF_BYTES}) must be < very-large threshold ({VERY_LARGE_FILE_BYTES})"
    );
}

#[test]
fn concurrency_limit_is_positive() {
    assert!(DIGEST_CONCURRENCY_LIMIT > 0);
}

#[test]
fn virtual_threshold_is_positive() {
    assert!(LARGE_DIRECTORY_VIRTUAL_THRESHOLD > 0);
}

#[test]
fn auto_expand_threshold_is_positive() {
    assert!(LARGE_HUNK_AUTO_EXPAND_LINES > 0);
}

// ── JobProgress ───────────────────────────────────────────────────────────────

fn progress(completed: u64, total: Option<u64>) -> JobProgress {
    JobProgress {
        job_id: 1,
        kind: JobKind::LineDiff,
        phase: "computing".into(),
        completed_units: completed,
        total_units: total,
        cancellable: true,
    }
}

#[test]
fn fraction_none_when_total_unknown() {
    assert!(progress(50, None).fraction().is_none());
}

#[test]
fn fraction_is_correct_for_known_total() {
    let f = progress(1, Some(4)).fraction().unwrap();
    assert!((f - 0.25).abs() < 1e-6);
}

#[test]
fn fraction_clamps_at_one_when_over_total() {
    let f = progress(10, Some(4)).fraction().unwrap();
    assert!((f - 1.0).abs() < 1e-6);
}

#[test]
fn fraction_is_one_for_zero_total() {
    let f = progress(0, Some(0)).fraction().unwrap();
    assert!((f - 1.0).abs() < 1e-6);
}

#[test]
fn is_complete_false_when_total_unknown() {
    assert!(!progress(100, None).is_complete());
}

#[test]
fn is_complete_false_when_not_done() {
    assert!(!progress(3, Some(10)).is_complete());
}

#[test]
fn is_complete_true_when_reached_total() {
    assert!(progress(10, Some(10)).is_complete());
}

#[test]
fn job_kind_labels_are_non_empty() {
    for kind in [
        JobKind::ReadFile, JobKind::DecodeFile, JobKind::LineDiff,
        JobKind::InlineDiff, JobKind::DirectoryDigest,
        JobKind::SavePreflight, JobKind::BatchCopy,
    ] {
        assert!(!kind.label().is_empty(), "label for {kind:?} must not be empty");
    }
}

// ── JobHandle ─────────────────────────────────────────────────────────────────

#[test]
fn job_handle_cancel_propagates_to_worker_token() {
    let (handle, worker_token) = JobHandle::new(42);
    assert!(!worker_token.is_cancelled());
    handle.cancel();
    assert!(worker_token.is_cancelled());
}

#[test]
fn job_handle_token_is_the_same_signal() {
    let (handle, worker_token) = JobHandle::new(7);
    worker_token.cancel(); // cancel from worker side
    assert!(handle.is_cancelled());
}

#[test]
fn job_id_is_stored_correctly() {
    let (handle, _) = JobHandle::new(99);
    assert_eq!(handle.job_id, 99);
}
