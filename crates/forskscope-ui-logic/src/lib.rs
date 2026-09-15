//! Framework-independent presentation logic for ForskScope (RFC-020 §5a).
//!
//! This crate is the *view-model* layer: pure logic derived from
//! `forskscope-core` truth, with no Dioxus or GTK dependency, so it can be
//! unit-tested without a display server. Feature areas are modules:
//!
//! - [`explore`] — explorer-pane logic:
//!   - `align`: aligned-row merging for the two-pane explorer.
//!   - `deep_filter`: `DeepFilter` + `DeepCompareSummary` for recursive compare.
//!   - `status`: `RowStatusKind` from `EqualityEvidence`, and
//!     `StatusGlyph` — the shared glyph/CSS-class/label vocabulary both
//!     the Explorer and Deep Compare render through (F82).
//! - [`compare`] — diff/compare logic:
//!   - `load_guard`: pre-diff `LoadGuard` from `FileSizeClass`.
//!   - `load_identity`: runtime tab/load tokens and completion validation.
//!   - `save_error`: `SaveErrorView` — `AppError` → dialog content.
//!   - `search_index`: in-diff match index (`advance`/`retreat`).
//!   - `startup`: `StartupRequest` CLI parsing and mergetool-to-`CompareRequest`
//!     conversion (RFC-077).
//! - [`settings`] — settings form logic:
//!   - `settings_view`: theme picker choices and the shared font-size clamp.
//!   - `persistence_recovery`: `SettingsRecoveryView` — RFC-076 migration/
//!     incompatibility/corruption dialog content.
//! - [`session`] — session persistence logic:
//!   - `persistence_recovery`: `SessionRecoveryView`, the session mirror of
//!     `settings::persistence_recovery`.
//!
//! Crate-root re-exports keep the common types one import away. **F75/F54:**
//! this list is checked by `cargo xtask ui-logic-connectivity` — every name
//! here must be referenced by `forskscope-ui`, or it does not belong at the
//! crate root (see that command's own doc comment for exactly what counts).
//! `conflict_nav_view` and `palette_view` (RFC-028's toolbar profile picker,
//! the command-palette and three-way-merge conflict-workspace view-models)
//! were deleted outright, not merely un-exported — deferred post-v1 UI that
//! was never built; they return from git history with their features.

pub mod compare;
pub mod explore;
pub mod session;
pub mod settings;

// compare
pub use compare::load_guard::{LoadGuard, guard_for_sizes};
pub use compare::load_identity::{
    CompareTabId, CompareTabIdAllocator, CompletionDecision, LoadGeneration, LoadIdentityError,
    LoadIdentitySnapshot, LoadToken, completion_decision,
};
pub use compare::save_error::SaveErrorView;
pub use compare::search_index::MatchIndex;
pub use compare::startup::{CompareRequest, SaveDestination, StartupRequest, parse_startup_args};

// explore
pub use explore::align::{AlignedRow, FlatRow, RowData, compute_aligned_rows};
pub use explore::deep_filter::{DeepCompareSummary, DeepFilter, apply_filter};
pub use explore::status::{RowStatusKind, StatusGlyph};

// session
pub use session::persistence_recovery::{
    RecoveryDialogAction as SessionRecoveryDialogAction, SessionRecoveryView,
    action_label as session_recovery_action_label,
};

// settings
pub use settings::persistence_recovery::{
    RecoveryDialogAction as SettingsRecoveryDialogAction, SettingsRecoveryView,
    action_label as settings_recovery_action_label,
};
pub use settings::settings_view::{clamp_font_size, theme_choices};
