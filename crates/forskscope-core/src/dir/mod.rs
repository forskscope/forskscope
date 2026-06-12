//! Directory listing, digest comparison, file copy, and recursive diff (RFC-005, RFC-008, RFC-037).

mod batch;
mod copy;
mod digest;
mod index;
mod listing;
mod merge_plan;
mod recursive;

pub use batch::{
    BatchFailurePolicy, BatchItem, BatchManifest, EntryOutcome, ManifestEntry, OperationId,
    batch_copy, restore_from_manifest,
};
pub use copy::{CopyOutcome, copy_file};
pub use digest::{dir_digest_equal, file_digest_equal};
pub use index::{
    ContentDigest, DirectoryEntryRecord, DirectoryIndex, EntryType,
    EqualityEvidence, IndexRevision, PairedEntry, PairedEntrySet,
    pair_entries,
};
pub use listing::{DirectoryListing, FileEntry, list_dir};
pub use merge_plan::{
    CopyDirection, DirectoryMergeAction, EntrySelection, FileOutcome, OperationPlan,
    OperationPlanId, OperationPreflight, PlanExecutionReport, PlannedFileOperation,
    RiskSummary, execute_plan, plan_operations,
};
pub use recursive::{
    RecEntry, RecStatus,
    list_recursive_for_display, list_recursive_for_display_with_cancel,
    recursive_diff, recursive_diff_with_cancel,
};
