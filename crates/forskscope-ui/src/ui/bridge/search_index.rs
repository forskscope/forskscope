//! Re-exports the pure search-index logic from `forskscope-ui-logic`
//! so UI components get a single import path (RFC-020 §5a, RFC-014 §M4).
//! Not all re-exported types are used directly in the UI today; the facade
//! is kept complete so call sites never need to reach into the logic crate.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{MatchIndex, MatchPosition, MatchSide};
