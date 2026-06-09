//! Model-backed merge session (RFC-006 merge transactions, RFC-015 undo/redo).
//!
//! This module replaces the v0.22.x pattern of mutating rendered diff data
//! in UI memory. All merge operations go through [`MergeSession`], which
//! owns the working state, the transaction log, undo/redo, dirty state,
//! and the canonical result text used by the save layer.

mod session;
mod three_way;
mod transaction;

pub use session::{HunkState, MergeHunk, MergeSession};
pub use three_way::{
    ConflictId, ConflictStatus, MergeConflict, MergeLine, MergeRegion, RegionKind,
    ThreeWayMergeSession, ThreeWayStats, diff3, render_lines, split_lines,
};
pub use transaction::MergeTransaction;
