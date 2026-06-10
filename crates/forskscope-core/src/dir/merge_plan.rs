//! Directory merge and batch operation planner (RFC-022).
//!
//! Turns a directory comparison result (`Vec<RecEntry>`) into a concrete,
//! previewable, executable operation plan. The plan must be reviewed before
//! execution — ForskScope never silently overwrites files.
//!
//! ## Overview
//!
//! 1. Run [`recursive_diff`] to get `Vec<RecEntry>`.
//! 2. Call [`plan_operations`] with a direction and selection to get a
//!    [`OperationPlan`].
//! 3. Present [`OperationPlan::risk_summary`] to the user for confirmation.
//! 4. On confirmation call [`execute_plan`] — it delegates to [`batch_copy`]
//!    for I/O and returns a [`PlanExecutionReport`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dir::batch::{BatchFailurePolicy, BatchItem, batch_copy};
use crate::dir::recursive::RecEntry;
use crate::dir::RecStatus;
use crate::save::BackupPolicy;

// ── Public types ──────────────────────────────────────────────────────────────

/// Direction for a batch copy operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyDirection {
    /// Copy from left root to right root (synchronise right from left).
    LeftToRight,
    /// Copy from right root to left root (synchronise left from right).
    RightToLeft,
}

/// Which entries from the scan to include in a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntrySelection {
    /// All changed, one-sided, and different-type entries.
    #[default]
    AllNonEqual,
    /// Only entries that differ in content (changed files).
    ChangedOnly,
    /// Only entries present solely on the source side (new files).
    SourceOnlyEntries,
}

/// A single planned file operation.
#[derive(Debug, Clone)]
pub struct PlannedFileOperation {
    pub rel_path:  PathBuf,
    pub action:    DirectoryMergeAction,
    pub source:    Option<PathBuf>,
    pub target:    Option<PathBuf>,
    pub preflight: OperationPreflight,
}

/// Pre-execution checks for one operation (RFC-022 §"Operation plan model").
#[derive(Debug, Clone)]
pub struct OperationPreflight {
    /// The target path already exists and will be overwritten.
    pub target_exists:   bool,
    /// The target is not writable (best-effort check at plan time).
    pub target_writable: bool,
    /// A backup should be created before overwriting.
    pub backup_required: bool,
    /// Estimated bytes to copy (0 for directory creation).
    pub estimated_bytes: u64,
}

/// High-level risk summary shown in the batch review dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RiskSummary {
    pub total_files:   usize,
    pub new_files:     usize,
    pub overwrites:    usize,
    pub estimated_bytes: u64,
    /// Number of files where the target is not writable at plan time.
    pub permission_blocks: usize,
}

/// A unique identifier for one operation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPlanId(pub String);

impl OperationPlanId {
    fn new() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self(format!("plan-{secs}-{}", std::process::id()))
    }
}

/// A previewable, executable directory merge plan (RFC-022 §"Operation plan model").
#[derive(Debug, Clone)]
pub struct OperationPlan {
    pub id:           OperationPlanId,
    pub left_root:    PathBuf,
    pub right_root:   PathBuf,
    pub direction:    CopyDirection,
    pub operations:   Vec<PlannedFileOperation>,
    pub risk_summary: RiskSummary,
}

impl OperationPlan {
    /// `true` when every operation in the plan has a writable target.
    pub fn is_safe_to_execute(&self) -> bool {
        self.risk_summary.permission_blocks == 0
    }
}

/// What happened to one file during plan execution.
#[derive(Debug, Clone)]
pub enum FileOutcome {
    Copied { bytes: u64, backup_created: bool },
    Skipped { reason: String },
    Failed { error: String },
}

/// The result of running [`execute_plan`].
#[derive(Debug, Clone)]
pub struct PlanExecutionReport {
    pub plan_id:    OperationPlanId,
    pub succeeded:  usize,
    pub failed:     usize,
    pub skipped:    usize,
    pub outcomes:   Vec<(PathBuf, FileOutcome)>,
}

// ── DirectoryMergeAction ──────────────────────────────────────────────────────

/// A single actionable directory merge operation (RFC-022 §"Directory merge actions").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryMergeAction {
    /// Copy a file from the left root to the right root.
    CopyLeftToRight,
    /// Copy a file from the right root to the left root.
    CopyRightToLeft,
    /// Skip this entry — no operation.
    Skip,
}

// ── Planner ───────────────────────────────────────────────────────────────────

/// Build an operation plan from a directory scan result.
///
/// Only copy operations are planned (no deletions — deletions require a
/// stronger confirmation model that is UI-layer responsibility).
pub fn plan_operations(
    entries:    &[RecEntry],
    left_root:  &Path,
    right_root: &Path,
    direction:  CopyDirection,
    selection:  EntrySelection,
) -> OperationPlan {
    let id = OperationPlanId::new();
    let mut operations = Vec::new();
    let mut risk = RiskSummary::default();

    for entry in entries {
        // Skip entries that don't qualify under the selection filter.
        let include = match (selection, entry.status) {
            (_, RecStatus::Equal | RecStatus::Computing) => false,
            (EntrySelection::ChangedOnly, s) => s == RecStatus::Changed,
            (EntrySelection::SourceOnlyEntries, s) => match direction {
                CopyDirection::LeftToRight => s == RecStatus::LeftOnly,
                CopyDirection::RightToLeft => s == RecStatus::RightOnly,
            },
            (EntrySelection::AllNonEqual, _) => true,
        };
        if !include { continue; }

        // Determine source and target based on direction and status.
        let (source, target, action) = match (direction, entry.status) {
            (CopyDirection::LeftToRight, RecStatus::Changed | RecStatus::LeftOnly) => {
                let src = left_root.join(&entry.rel_path);
                let tgt = right_root.join(&entry.rel_path);
                (Some(src), Some(tgt), DirectoryMergeAction::CopyLeftToRight)
            }
            (CopyDirection::RightToLeft, RecStatus::Changed | RecStatus::RightOnly) => {
                let src = right_root.join(&entry.rel_path);
                let tgt = left_root.join(&entry.rel_path);
                (Some(src), Some(tgt), DirectoryMergeAction::CopyRightToLeft)
            }
            // Entry exists only on the non-source side: skip in this direction.
            _ => {
                operations.push(PlannedFileOperation {
                    rel_path:  entry.rel_path.clone(),
                    action:    DirectoryMergeAction::Skip,
                    source:    None,
                    target:    None,
                    preflight: OperationPreflight {
                        target_exists:   false,
                        target_writable: true,
                        backup_required: false,
                        estimated_bytes: 0,
                    },
                });
                continue;
            }
        };

        let preflight = compute_preflight(target.as_deref(), entry);
        risk.total_files += 1;
        if preflight.target_exists  { risk.overwrites += 1; }
        else                        { risk.new_files  += 1; }
        if !preflight.target_writable { risk.permission_blocks += 1; }
        risk.estimated_bytes += preflight.estimated_bytes;

        operations.push(PlannedFileOperation {
            rel_path: entry.rel_path.clone(),
            action,
            source,
            target,
            preflight,
        });
    }

    OperationPlan {
        id,
        left_root:  left_root.to_path_buf(),
        right_root: right_root.to_path_buf(),
        direction,
        operations,
        risk_summary: risk,
    }
}

fn compute_preflight(target: Option<&Path>, entry: &RecEntry) -> OperationPreflight {
    let (target_exists, target_writable) = match target {
        None => (false, false),
        Some(p) => {
            let exists = p.exists();
            let writable = if exists {
                p.metadata().map(|m| !m.permissions().readonly()).unwrap_or(false)
            } else {
                // Check parent directory is writable.
                p.parent()
                    .and_then(|par| par.metadata().ok())
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(true)  // assume writable when parent doesn't exist yet
            };
            (exists, writable)
        }
    };
    let estimated_bytes = entry.left_size.or(entry.right_size).unwrap_or(0);
    OperationPreflight {
        target_exists,
        target_writable,
        backup_required: target_exists,
        estimated_bytes,
    }
}

// ── Executor ──────────────────────────────────────────────────────────────────

/// Execute a previewed operation plan.
///
/// Delegates to [`batch_copy`] for actual I/O. Returns a
/// [`PlanExecutionReport`] regardless of partial failure.
pub fn execute_plan(
    plan:           &OperationPlan,
    backup:         BackupPolicy,
    failure_policy: BatchFailurePolicy,
) -> PlanExecutionReport {
    let copy_ops: Vec<PlannedFileOperation> = plan.operations.iter()
        .filter(|op| op.action != DirectoryMergeAction::Skip)
        .filter(|op| op.source.is_some() && op.target.is_some())
        .cloned()
        .collect();

    // Ensure target parent directories exist before bulk copy.
    for op in &copy_ops {
        if let Some(target) = &op.target {
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
    }

    let items: Vec<BatchItem> = copy_ops.iter()
        .filter_map(|op| {
            Some(BatchItem {
                src: op.source.clone()?,
                dst: op.target.clone()?,
            })
        })
        .collect();

    let skipped_count = plan.operations.iter()
        .filter(|op| op.action == DirectoryMergeAction::Skip)
        .count();

    let manifest = batch_copy(&items, backup, failure_policy, None)
        .unwrap_or_else(|e| {
            // batch_copy itself only errors on manifest write, which we're
            // not doing (manifest_dir=None). This path should be unreachable,
            // but we handle it defensively.
            panic!("batch_copy returned Err with no manifest_dir: {e}")
        });

    let outcomes: Vec<(PathBuf, FileOutcome)> = copy_ops.iter().zip(manifest.entries.iter())
        .map(|(op, entry)| {
            let outcome = match &entry.outcome {
                crate::dir::batch::EntryOutcome::Copied { bytes, backup_path } =>
                    FileOutcome::Copied {
                        bytes: *bytes,
                        backup_created: backup_path.is_some(),
                    },
                crate::dir::batch::EntryOutcome::Skipped { reason } =>
                    FileOutcome::Skipped { reason: reason.clone() },
                crate::dir::batch::EntryOutcome::Failed { error } =>
                    FileOutcome::Failed { error: error.clone() },
            };
            (op.rel_path.clone(), outcome)
        })
        .collect();

    PlanExecutionReport {
        plan_id:   plan.id.clone(),
        succeeded: manifest.succeeded(),
        failed:    manifest.failed(),
        skipped:   skipped_count,
        outcomes,
    }
}
