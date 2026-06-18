//! Reusable visual primitives shared across views and overlays.
//!
//! Extraction criterion (RFC-072): used by at least two views, or one view
//! plus one overlay. Components here must not import from `crate::state`.

pub mod notice;
