//! Three-way (base-aware) merge model (RFC-033).
//!
//! This module adds an explicit, conflict-safe three-way merge alongside
//! the existing two-way [`MergeSession`](super::MergeSession). It is
//! GUI-independent and owns product truth: the reconciled region list,
//! structured conflict records with durable IDs, resolution operations
//! with undo/redo, the merged result text, and the save-block predicate.
//!
//! The conflict-resolution *workspace* (RFC-034) and editor-driven manual
//! edits (RFC-032) are UI concerns layered on top of this model and remain
//! out of scope here.

mod engine;
mod line;
mod session;

pub use engine::{MergeRegion, RegionKind, diff3};
pub use line::{MergeLine, render_lines, split_lines};
pub use session::{
    ConflictId, ConflictStatus, MergeConflict, ThreeWayMergeSession, ThreeWayStats,
};
