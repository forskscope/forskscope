//! # forskscope-core
//!
//! GUI-independent core of ForskScope (RFC-001, RFC-002), mid-split under
//! RFC-093: the part of the original `forskscope-core` not yet moved to its
//! final crate. No Dioxus, Tauri, WebView, or JavaScript type appears here.
//!
//! UI layers consume these domain objects and derive their own view models;
//! they must never become an independent source of truth (see RFC-042,
//! "Core First").

pub mod command;
pub mod conflict_nav;
pub mod diff_decoration;
pub mod env_passthrough;
pub mod external_tool;
pub mod line_map;
pub mod persist;
pub mod platform;
pub mod settings;
pub mod vcs;
pub mod watcher;

#[cfg(test)]
mod tests;
