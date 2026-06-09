//! Merge transaction record (RFC-015).

use crate::diff::{DiffRow, HunkId, HunkKind};

use super::session::HunkState;

/// One reversible merge operation. The transaction stores everything needed
/// to restore the hunk exactly, so undo never reconstructs state from the
/// view layer.
#[derive(Debug, Clone)]
pub struct MergeTransaction {
    pub hunk_id: HunkId,
    /// Rows of the hunk before the operation.
    pub previous_rows: Vec<DiffRow>,
    pub previous_kind: HunkKind,
    pub previous_state: HunkState,
}
