//! Pure UI data logic — no Dioxus, no GTK dependency (RFC-059 §M5, RFC-014 §M4).
//!
//! This crate hosts GUI-independent algorithms used by `forskscope-ui-dioxus`
//! so they can be unit-tested without a display server:
//!
//! - [`align`] — merges two flat visible-row lists into an aligned structure
//!   for the two-pane Explorer (`compute_aligned_rows`).
//! - [`search_index`] — builds and traverses an ordered match index for
//!   in-diff text search with next/prev navigation (`MatchIndex`).

pub mod align;
pub mod search_index;

pub use align::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
pub use search_index::{MatchIndex, MatchPosition, MatchSide};
