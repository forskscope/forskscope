//! Framework-independent presentation logic for ForskScope (RFC-020 §5a).
//!
//! This crate is the *view-model* layer: pure logic derived from
//! `forskscope-core` truth, with no Dioxus or GTK dependency, so it can be
//! unit-tested without a display server. Feature areas are modules:
//!
//! - [`explore`] — explorer-pane logic (`align`: aligned-row merging).
//! - [`compare`] — diff/compare logic (`search_index`: in-diff match index).
//!
//! A `settings` area is reserved for when pure settings logic emerges.
//! Crate-root re-exports keep the common types one import away.

pub mod compare;
pub mod explore;

pub use compare::search_index::{MatchIndex, MatchPosition, MatchSide};
pub use explore::align::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
