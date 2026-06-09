//! Normalized diff model and engine (RFC-002).
//!
//! The `similar` v3 crate is an implementation detail of [`engine`]; no
//! `similar` type appears in the public model, so future engine changes do
//! not rewrite the UI or merge layers.

mod engine;
mod inline;
mod model;
mod options;

pub use engine::compute_diff;
pub use inline::{inline_diff_rows, refine_pair};
pub use model::{
    DiffDocument, DiffHunk, DiffId, DiffRow, DiffStats, DiffWarning, HunkId, HunkKind, InlineDiff,
    InlineKind, InlineSpan, LineRange, NewlineMarker, SideLine,
};
pub use options::{DiffAlgorithm, DiffOptions, InlineMode};
