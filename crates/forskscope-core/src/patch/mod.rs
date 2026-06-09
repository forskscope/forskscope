//! Patch export (RFC-039 §"Export Patch", §"Patch Review").
//!
//! This module builds and serializes [`PatchDocument`]s from the existing
//! diff model. It covers the *export* half of RFC-039:
//!
//! - [`patch_from_file_diff`] — single-file unified diff;
//! - [`patch_from_directories`] — directory-scope patch from a recursive
//!   comparison;
//! - [`to_unified`] — deterministic unified-diff serialization.
//!
//! The guarded *apply* workflow (preflight, backup-protected writes) remains
//! proposed in RFC-039 and is intentionally not implemented here, so patch
//! export never performs writes to the user's tree.

mod build;
mod directory;
mod model;
mod unified;

pub use build::{PatchOptions, hunks_from_diff, patch_from_file_diff};
pub use directory::patch_from_directories;
pub use model::{
    LineOrigin, PatchDocument, PatchFileChange, PatchFormat, PatchHunk, PatchLine, PatchSummary,
};
pub use unified::to_unified;
