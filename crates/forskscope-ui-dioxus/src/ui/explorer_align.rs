//! Re-exports the pure alignment logic from `forskscope-explorer-align`
//! so that `explorer.rs` imports are unchanged (RFC-059 §M5).
pub use forskscope_explorer_align::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
