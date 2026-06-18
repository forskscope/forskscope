//! Thin UI-facing adapters and re-exports from `forskscope-ui-logic`.
//! This module must not contain Dioxus components or visual decisions.

pub mod command_bar;
pub mod compare_summary;
pub mod conflict_nav;
pub mod deep_filter;
pub mod explore_status;
pub mod explorer_align;
pub mod hunk_decorations;
pub mod load_guard;
pub mod palette_view;
pub mod save_error;
pub mod scroll_sync;
pub mod search_index;
pub mod settings_view;
pub mod tab_state;
