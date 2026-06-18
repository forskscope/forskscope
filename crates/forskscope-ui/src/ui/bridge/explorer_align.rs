//! Re-exports the pure alignment logic from `forskscope-ui-logic`
//! so that `explorer.rs` imports are unchanged (RFC-020 §5a, RFC-059 §M5).
//! Not all re-exported types are used directly in the UI today; the facade
//! is kept complete so call sites never need to reach into the logic crate.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
