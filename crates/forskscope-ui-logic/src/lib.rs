//! Framework-independent presentation logic for ForskScope (RFC-020 §5a).
//!
//! This crate is the *view-model* layer: pure logic derived from
//! `forskscope-core` truth, with no Dioxus or GTK dependency, so it can be
//! unit-tested without a display server. Feature areas are modules:
//!
//! - [`explore`] — explorer-pane logic:
//!   - `align`: aligned-row merging for the two-pane explorer.
//!   - `deep_filter`: `DeepFilter` + `DeepCompareSummary` for recursive compare.
//!   - `status`: `RowStatusKind`/`StatusRow` from `EqualityEvidence`.
//! - [`compare`] — diff/compare logic:
//!   - `command_bar`: `ToolbarSection` list from `CommandRegistry` + `CommandContext`.
//!   - `search_index`: in-diff match index (`advance`/`retreat`).
//!   - `summary`: `CompareStatusSummary` and `DiffNavigationState`.
//!   - `tab_state`: `TabStateSnapshot` → `CommandContext` bridge.
//!
//! Crate-root re-exports keep the common types one import away.

pub mod compare;
pub mod explore;

pub use compare::command_bar::{
    ToolbarItem, ToolbarSection, build_toolbar, enabled_count, find_item,
};
pub use compare::search_index::{MatchIndex, MatchPosition, MatchSide};
pub use compare::hunk_decorations::{DecorationIndex, DiffSide, RowDecoration};
pub use compare::load_guard::{LoadGuard, guard_for_sizes, guard_for_sizes_with_limits};
pub use compare::conflict_nav_view::{ConflictNavView, ConflictRailRow};
pub use compare::palette_view::{PaletteRow, build_palette, enabled_count as palette_enabled_count};
pub use compare::save_error::{RecoveryButton, SaveErrorView, action_label};
pub use compare::scroll_sync::ScrollSyncState;
pub use compare::summary::{CompareStatusSummary, DiffNavigationState};
pub use compare::tab_state::{TabStateSnapshot, context_from_snapshot};
pub use explore::align::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
pub use explore::deep_filter::{DeepCompareSummary, DeepFilter, apply_filter};
pub use explore::status::{RowStatusKind, StatusRow};
