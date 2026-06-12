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

// ── JobStatus lifecycle (RFC-008 §8) ──────────────────────────────────────────

use crate::job::{JobRegistry, JobStatus, JobStatusRecord};

#[test]
fn job_status_queued_is_active_not_terminal() {
    assert!( JobStatus::Queued.is_active());
    assert!(!JobStatus::Queued.is_terminal());
    assert!(!JobStatus::Queued.is_success());
}

#[test]
fn job_status_running_is_active() {
    assert!(JobStatus::Running.is_active());
    assert!(!JobStatus::Running.is_terminal());
}

#[test]
fn job_status_completed_is_terminal_and_success() {
    assert!(!JobStatus::Completed.is_active());
    assert!( JobStatus::Completed.is_terminal());
    assert!( JobStatus::Completed.is_success());
}

#[test]
fn job_status_cancelled_is_terminal_not_success() {
    assert!( JobStatus::Cancelled.is_terminal());
    assert!(!JobStatus::Cancelled.is_success());
}

#[test]
fn job_status_failed_is_terminal_not_success() {
    assert!( JobStatus::Failed("err".into()).is_terminal());
    assert!(!JobStatus::Failed("err".into()).is_success());
}

// ── JobStatusRecord lifecycle transitions ──────────────────────────────────────

fn record() -> JobStatusRecord { JobStatusRecord::new(1, JobKind::DirectoryDigest) }

#[test]
fn new_record_starts_queued() {
    let r = record();
    assert_eq!(r.status, JobStatus::Queued);
}

#[test]
fn start_transitions_queued_to_running() {
    let mut r = record();
    r.start();
    assert_eq!(r.status, JobStatus::Running);
}

#[test]
fn start_on_already_running_is_noop() {
    let mut r = record();
    r.start();
    r.start(); // second call should not change state
    assert_eq!(r.status, JobStatus::Running);
}

#[test]
fn complete_transitions_running_to_completed() {
    let mut r = record();
    r.start();
    r.complete();
    assert_eq!(r.status, JobStatus::Completed);
}

#[test]
fn cancel_transitions_running_to_cancelled() {
    let mut r = record();
    r.start();
    r.cancel();
    assert_eq!(r.status, JobStatus::Cancelled);
}

#[test]
fn fail_transitions_running_to_failed() {
    let mut r = record();
    r.start();
    r.fail("disk error");
    assert!(matches!(r.status, JobStatus::Failed(ref m) if m == "disk error"));
}

#[test]
fn complete_on_terminal_is_noop() {
    let mut r = record();
    r.start();
    r.cancel();
    r.complete(); // must not change from Cancelled to Completed
    assert_eq!(r.status, JobStatus::Cancelled);
}

#[test]
fn cancel_on_terminal_is_noop() {
    let mut r = record();
    r.start();
    r.complete();
    r.cancel(); // must not change from Completed to Cancelled
    assert_eq!(r.status, JobStatus::Completed);
}

// ── JobRegistry ───────────────────────────────────────────────────────────────

#[test]
fn registry_register_and_get() {
    let mut reg = JobRegistry::default();
    reg.register(1, JobKind::DirectoryDigest);
    assert!(reg.get(&1).is_some());
    assert_eq!(reg.len(), 1);
}

#[test]
fn registry_active_returns_only_active_jobs() {
    let mut reg = JobRegistry::default();
    reg.register(1, JobKind::DirectoryDigest);
    reg.register(2, JobKind::DirectoryDigest);
    // Complete job 2
    reg.get_mut(&2).unwrap().start();
    reg.get_mut(&2).unwrap().complete();

    let active: Vec<_> = reg.active().collect();
    assert_eq!(active.len(), 1, "only one job is still active");
    assert_eq!(active[0].job_id, 1);
}

#[test]
fn registry_prune_terminal_removes_completed_jobs() {
    let mut reg = JobRegistry::default();
    reg.register(1, JobKind::DirectoryDigest);
    reg.register(2, JobKind::DirectoryDigest);
    reg.get_mut(&1).unwrap().start();
    reg.get_mut(&1).unwrap().complete();

    reg.prune_terminal();
    assert_eq!(reg.len(), 1, "only the queued job remains after pruning");
    assert!(reg.get(&2).is_some(), "queued job must survive prune");
    assert!(reg.get(&1).is_none(), "completed job must be pruned");
}
