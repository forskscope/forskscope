//! Folder operations that write — batch copy, plan and execute (RFC-008,
//! RFC-037). The read half (listing, recursive diff, digests) is
//! `forskscope_compare::dir` (RFC-093 §1).

mod batch;
mod copy;
mod merge_plan;

pub use batch::{
    BatchFailurePolicy, BatchItem, BatchManifest, EntryOutcome, ManifestEntry, OperationId,
    batch_copy, restore_from_manifest,
};
pub use copy::{CopyOutcome, copy_file};
pub use merge_plan::{
    CopyDirection, DirectoryMergeAction, EntrySelection, FileOutcome, OperationPlan,
    OperationPlanId, OperationPreflight, PlanExecutionReport, PlannedFileOperation, RiskSummary,
    execute_plan, plan_operations,
};
