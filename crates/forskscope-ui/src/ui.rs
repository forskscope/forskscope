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

// ── Bridge re-exports for backward-compatible crate::ui::X paths ─────────────
// TODO(v0.153): migrate call sites to crate::ui::bridge::X then remove these.
pub use bridge::command_bar;
pub use bridge::compare_summary;
pub use bridge::conflict_nav;
pub use bridge::deep_filter;
pub use bridge::explore_status;
pub use bridge::explorer_align;
pub use bridge::hunk_decorations;
pub use bridge::load_guard;
pub use bridge::palette_view;
pub use bridge::save_error;
pub use bridge::scroll_sync;
pub use bridge::search_index;
pub use bridge::settings_view;
pub use bridge::tab_state;

// ── View re-exports for backward-compatible crate::ui::X paths ───────────────
// TODO(v0.153): migrate call sites to crate::ui::view::X then remove these.
pub use view::deep_compare;
pub use view::diff;
pub use view::diff_actions;
pub use view::dir_pane;
pub use view::explorer;
pub use view::hunk;
pub use view::search;
pub use view::settings;

// ── Layout re-exports for backward-compatible crate::ui::X paths ─────────────
// TODO(v0.153): migrate call sites to crate::ui::layout::X then remove these.
pub use layout::header;
pub use layout::statusbar;
pub use layout::tabs;

// ── Overlay re-exports for backward-compatible crate::ui::X paths ────────────
// TODO(v0.153): migrate call sites to crate::ui::overlay::X then remove these.
pub use overlay::keybindings;
pub use overlay::modals;
