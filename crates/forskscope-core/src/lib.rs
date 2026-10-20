//! # forskscope-core
//!
//! GUI-independent core of ForskScope (RFC-001, RFC-002), mid-split under
//! RFC-093: the part of the original `forskscope-core` not yet moved to its
//! final crate — the view-model layer destined for `forskscope-ui-logic`.
//! No Dioxus, Tauri, WebView, or JavaScript type appears here.
//!
//! UI layers consume these domain objects and derive their own view models;
//! they must never become an independent source of truth (see RFC-042,
//! "Core First").

pub mod command;
pub mod conflict_nav;
pub mod diff_decoration;
pub mod line_map;

#[cfg(test)]
mod tests;
