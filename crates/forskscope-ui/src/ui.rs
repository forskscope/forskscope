//! Dioxus UI components.
//!
//! Subgroups follow the architectural layers defined in the UI structural
//! review (v0.152.0):
//!
//! - `layout/`  — persistent app shell: header, tab bar, status bar
//! - `view/`    — main user-facing workspaces: Explorer, Diff, Settings, …
//! - `overlay/` — modal, keyboard help, and safety-guard overlays
//! - `bridge/`  — thin re-export adapters from `forskscope-ui-logic`

pub mod bridge;
pub mod layout;
pub mod overlay;
pub mod view;
