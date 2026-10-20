//! # forskscope-desktop
//!
//! ForskScope's desktop integration (RFC-093 §1): settings/session
//! persistence and its display types, external tools, Git, file watching,
//! platform detection, and environment pass-through. The only crate besides
//! the GUI that may write its own files (settings/session) or spawn a
//! process (external tools, Git, the system opener).

pub mod env_passthrough;
pub mod external_tool;
pub mod persist;
pub mod platform;
pub mod settings;
pub mod vcs;
pub mod watcher;

#[cfg(test)]
mod tests;
