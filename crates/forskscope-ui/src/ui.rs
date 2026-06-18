//! Dioxus UI components.
//!
//! Subgroups follow the architectural layers defined in the UI structural
//! review (v0.152.0):
//!
//! - `component/` — reusable visual primitives (RFC-072)
//! - `layout/`    — persistent app shell: header, tab bar, status bar
//! - `view/`      — main user-facing workspaces: Explorer, Diff, Settings, …
//! - `overlay/`   — modal, keyboard help, and safety-guard overlays

pub mod component;
pub mod layout;
pub mod overlay;
pub mod view;
