//! Background job progress model and large-file threshold policy (RFC-013,
//! RFC-008 §"Background Job Model").
//!
//! This module defines:
//!
//! - **Threshold constants** — the soft/hard byte and line limits that
//!   govern how expensive operations are bounded (RFC-013 §"Thresholds").
//! - **`JobKind`** — which operation a background job is performing.
//! - **`JobProgress`** — a snapshot of a running job's state, emitted
//!   periodically to the UI so it can show progress without blocking.
//! - **`JobHandle`** — a `CancellationToken` wrapper that also carries
//!   the `JobId` so the UI can correlate progress updates with cancellations.
//!
//! The job *execution* model (spawning, scheduling) lives in the UI layer;
//! core owns only the data types and the policy constants.

pub mod limits;
pub use limits::{FileSizeClass, PerformanceLimits};

use crate::cancel::CancellationToken;

// ── Threshold policy (RFC-013 §"Thresholds") ─────────────────────────────────

/// Byte threshold above which inline character-level diff is disabled
/// automatically (RFC-013 §"Thresholds", §7.3). Matches the existing
/// `DiffOptions::max_file_bytes_for_full_diff` default.
pub const LARGE_FILE_INLINE_DIFF_BYTES: u64 = 512 * 1024; // 512 KB

/// Byte threshold above which a file is considered "very large" and the
/// diff is further constrained (deadline shortened, only line-level diff).
pub const VERY_LARGE_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

/// Line count above which collapsed equal-hunk expansion is not offered
/// automatically (to avoid rendering thousands of lines at once).
pub const LARGE_HUNK_AUTO_EXPAND_LINES: usize = 10_000;

/// Number of directory entries above which the deep-compare view switches
/// to virtual/windowed rendering (RFC-013 §7.2, RFC-037 future).
pub const LARGE_DIRECTORY_VIRTUAL_THRESHOLD: usize = 5_000;

/// Maximum number of in-flight per-file digest tasks before the directory
/// compare flow applies back-pressure (RFC-037 §"Cancellation").
pub const DIGEST_CONCURRENCY_LIMIT: usize = 32;

// ── Job model (RFC-013 §"Background Job Model", RFC-008) ─────────────────────

/// Stable identifier for one background job within a session.
pub type JobId = u64;

/// Which kind of operation a background job is performing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    ReadFile,
    DecodeFile,
    LineDiff,
    InlineDiff,
    DirectoryDigest,
    SavePreflight,
    BatchCopy,
}

impl JobKind {
    /// Human-readable label for progress UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::ReadFile       => "Reading file",
            Self::DecodeFile     => "Decoding file",
            Self::LineDiff       => "Computing diff",
            Self::InlineDiff     => "Computing inline diff",
            Self::DirectoryDigest => "Comparing directory",
            Self::SavePreflight  => "Checking save conditions",
            Self::BatchCopy      => "Copying files",
        }
    }
}

/// A progress snapshot emitted by a background job (RFC-013 §"Background
/// Job Model"). The UI polls or subscribes to these to show progress bars,
/// status text, and cancellation buttons.
///
/// `completed_units` and `total_units` use the same unit as the job's kind
/// (e.g. bytes for file I/O, file count for directory digest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProgress {
    pub job_id:          JobId,
    pub kind:            JobKind,
    /// Short description of the current phase within the job.
    pub phase:           String,
    pub completed_units: u64,
    /// `None` when the total is not yet known (e.g. during a directory walk).
    pub total_units:     Option<u64>,
    /// Whether this job responds to `CancellationToken::cancel`.
    pub cancellable:     bool,
}

impl JobProgress {
    /// 0.0–1.0 completion fraction, or `None` when total is unknown.
    pub fn fraction(&self) -> Option<f32> {
        self.total_units.map(|t| {
            if t == 0 {
                1.0
            } else {
                (self.completed_units as f32 / t as f32).clamp(0.0, 1.0)
            }
        })
    }

    /// `true` when `completed_units >= total_units` (and total is known).
    pub fn is_complete(&self) -> bool {
        self.total_units
            .map(|t| self.completed_units >= t)
            .unwrap_or(false)
    }
}

/// A handle pairing a [`JobId`] with a [`CancellationToken`].
///
/// The UI holds the handle; the worker holds a clone of the token.
/// Dropping the handle does *not* cancel automatically — call
/// [`JobHandle::cancel`] explicitly.
#[derive(Debug, Clone)]
pub struct JobHandle {
    pub job_id: JobId,
    token:      CancellationToken,
}

impl JobHandle {
    pub fn new(job_id: JobId) -> (Self, CancellationToken) {
        let token = CancellationToken::new();
        let handle = Self { job_id, token: token.clone() };
        (handle, token)
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Borrow the underlying cancellation token (e.g. to pass into a
    /// blocking task without moving the handle).
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}

// ── Job lifecycle state machine (RFC-008 §6–§7) ───────────────────────────────

/// The lifecycle state of a background job (RFC-008 §8 "Explorer Integration
/// Flow").
///
/// Transitions are strictly forward (no state can go backward) except that
/// `Running` may be cancelled to produce `Cancelled`.
///
/// ```text
/// Queued → Running → Completed
///               ↘ Cancelled
///               ↘ Failed(message)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    /// The job is waiting for an available worker slot.
    Queued,
    /// The job is actively running.
    Running,
    /// The job completed successfully.
    Completed,
    /// The job was cancelled (user or navigation triggered).
    Cancelled,
    /// The job failed; carries a human-readable reason.
    Failed(String),
}

impl JobStatus {
    /// `true` when the job is still pending or active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }

    /// `true` when the job has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        !self.is_active()
    }

    /// `true` when the job completed without error or cancellation.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// A record binding a `JobId` to its current [`JobStatus`] and last-known
/// progress (RFC-008 §6 "Progress Reporting").
#[derive(Debug, Clone)]
pub struct JobStatusRecord {
    pub job_id:   JobId,
    pub kind:     JobKind,
    pub status:   JobStatus,
    pub progress: JobProgress,
}

impl JobStatusRecord {
    pub fn new(job_id: JobId, kind: JobKind) -> Self {
        let progress = JobProgress {
            job_id,
            kind,
            phase:           String::new(),
            completed_units: 0,
            total_units:     None,
            cancellable:     true,
        };
        Self {
            job_id,
            kind,
            status:   JobStatus::Queued,
            progress,
        }
    }

    /// Advance to `Running`.  No-op when already in a terminal state.
    pub fn start(&mut self) {
        if self.status == JobStatus::Queued {
            self.status = JobStatus::Running;
        }
    }

    /// Update progress mid-run.
    pub fn update_progress(&mut self, progress: JobProgress) {
        self.progress = progress;
    }

    /// Mark as completed.  No-op when already terminal.
    pub fn complete(&mut self) {
        if !self.status.is_terminal() {
            self.status = JobStatus::Completed;
            self.progress.completed_units = self.progress.total_units.unwrap_or(1);
        }
    }

    /// Mark as cancelled.  No-op when already terminal.
    pub fn cancel(&mut self) {
        if !self.status.is_terminal() {
            self.status = JobStatus::Cancelled;
        }
    }

    /// Mark as failed.  No-op when already terminal.
    pub fn fail(&mut self, message: impl Into<String>) {
        if !self.status.is_terminal() {
            self.status = JobStatus::Failed(message.into());
        }
    }
}

/// A simple in-memory registry of all active and recently-completed job
/// records, used by the UI to display the progress indicator panel.
///
/// Records for terminal jobs can be pruned by the UI after display.
#[derive(Debug, Default)]
pub struct JobRegistry {
    records: Vec<JobStatusRecord>,
}

impl JobRegistry {
    /// Register a new job and return a mutable reference.
    pub fn register(&mut self, job_id: JobId, kind: JobKind) -> &mut JobStatusRecord {
        self.records.push(JobStatusRecord::new(job_id, kind));
        self.records.last_mut().unwrap()
    }

    /// Look up a job by ID.
    pub fn get(&self, job_id: &JobId) -> Option<&JobStatusRecord> {
        self.records.iter().find(|r| &r.job_id == job_id)
    }

    /// Look up a job mutably.
    pub fn get_mut(&mut self, job_id: &JobId) -> Option<&mut JobStatusRecord> {
        self.records.iter_mut().find(|r| &r.job_id == job_id)
    }

    /// All active (queued or running) jobs.
    pub fn active(&self) -> impl Iterator<Item = &JobStatusRecord> {
        self.records.iter().filter(|r| r.status.is_active())
    }

    /// Remove all terminal job records.
    pub fn prune_terminal(&mut self) {
        self.records.retain(|r| r.status.is_active());
    }

    pub fn len(&self) -> usize { self.records.len() }
    pub fn is_empty(&self) -> bool { self.records.is_empty() }
}
