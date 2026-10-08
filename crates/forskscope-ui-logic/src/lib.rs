//! Framework-independent presentation logic for ForskScope (RFC-020 §5a).
//!
//! This crate is the *view-model* layer: pure logic derived from
//! `forskscope-core` truth, with no Dioxus or GTK dependency, so it can be
//! unit-tested without a display server. Feature areas are modules:
//!
//! - [`explore`] — explorer-pane logic:
//!   - `align`: aligned-row merging for the two-pane explorer.
//!   - `classify_pair`: `classify_two_files` (F145/F77, handoff 056) — a file
//!     pair's equality classification under the automatic-digest cap.
//!   - `deep_filter`: `DeepFilter` + `DeepCompareSummary` for recursive compare,
//!     and `demote_entries_under_an_unreadable_root` (F110) — an unreadable
//!     root downgrades every entry to `Unreadable`, never a one-sided verdict.
//!   - `dir_verdict`: RFC-080 tier 1 — what a fast recursive listing can
//!     conclude about a directory pair (`Different` / `MetadataMatch` /
//!     `Unknown`), and no more.
//!   - `tier1_trigger`: the debounce state machine that decides when a tier-1
//!     walk starts (a row must be rested on; moving through rows starts none).
//!   - `tier2_verdict`: RFC-080 tier 2 — what a full, content-reading
//!     recursive comparison can conclude (`Identical` / `Different` /
//!     `Unknown`), the only tier entitled to claim identity.
//!   - `status`: `RowStatusKind` from `EqualityEvidence`, and
//!     `StatusGlyph` — the shared glyph/CSS-class/label vocabulary both
//!     the Explorer and Deep Compare render through (F82).
//!   - `sync_panes`: `mirror_target` (F151) — location-mirroring for the
//!     Explorer's "sync panes" toggle: what the other pane should navigate
//!     to, given one pane's navigation, with no filesystem access.
//! - [`compare`] — diff/compare logic:
//!   - `load_guard`: pre-diff `LoadGuard` from `FileSizeClass`.
//!   - `load_identity`: runtime tab/load tokens and completion validation.
//!   - `save_error`: `SaveErrorView` — `AppError` → dialog content.
//!   - `search_index`: in-diff match index (`advance`/`retreat`).
//!   - `startup`: `StartupRequest` CLI parsing and mergetool-to-`CompareRequest`
//!     conversion (RFC-077).
//! - [`settings`] — settings form logic:
//!   - `settings_view`: theme picker choices and the shared font-size clamp.
//!   - `field_debounce`: `FieldDebounce` (F136) — debounces a text field that
//!     is cheap to display but expensive to commit (the ignore-pattern
//!     fields, whose commit triggers an Explorer rescan).
//!   - `persistence_recovery`: `SettingsRecoveryView` — RFC-076 migration/
//!     incompatibility/corruption dialog content.
//! - [`session`] — session persistence logic:
//!   - `persistence_recovery`: `SessionRecoveryView`, the session mirror of
//!     `settings::persistence_recovery`.
//! - `update_check` — F180/F181 (handoff 073): `decide`, the pure
//!   `(current version, network outcome) -> state` mapping behind the About
//!   dialog's *Check for updates* button; `Version`, `AppChannel`,
//!   `UpdateAction`, `release_page_url`.
//!
//! Crate-root re-exports keep the common types one import away. **F75/F54:**
//! this list is checked by `cargo xtask ui-logic-connectivity` — every name
//! here must be referenced by `forskscope-ui`, or it does not belong at the
//! crate root (see that command's own doc comment for exactly what counts).
//! `conflict_nav_view` (the three-way-merge conflict-workspace rail,
//! RFC-034) and `palette_view` (the command palette, RFC-019) were deleted
//! outright, not merely un-exported — deferred post-v1 UI that was never
//! built; they return from git history with their features. (RFC-028's
//! toolbar profile picker was a separate deletion, `settings_view`'s
//! `ProfileChoice`/`profile_presets` — corrected here, review 106 §4: an
//! earlier version of this comment misattributed it to these two instead.)

pub mod compare;
pub mod explore;
pub mod session;
pub mod settings;
#[cfg(test)]
mod test_support;
pub mod update_check;

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
pub use explore::align::{AlignedRow, FlatRow, RowData, compute_aligned_rows, pair_by_index};
pub use explore::classify_pair::{EntryClassification, classify_two_files};
pub use explore::deep_filter::{
    DeepCompareSummary, DeepFilter, apply_filter, demote_entries_under_an_unreadable_root,
};
pub use explore::dir_verdict::{DirVerdict, dir_verdict};
pub use explore::status::{RowStatusKind, StatusGlyph};
pub use explore::sync_panes::mirror_target;
pub use explore::tier1_trigger::{TIER1_DEBOUNCE, Tier1Action, Tier1Trigger};
pub use explore::tier2_verdict::{Tier2Verdict, tier2_verdict};

// session
pub use session::persistence_recovery::{
    RecoveryDialogAction as SessionRecoveryDialogAction, SessionRecoveryView,
    action_label as session_recovery_action_label,
};

// settings
pub use settings::field_debounce::FieldDebounce;
pub use settings::persistence_recovery::{
    RecoveryDialogAction as SettingsRecoveryDialogAction, SettingsRecoveryView,
    action_label as settings_recovery_action_label,
};
pub use settings::settings_view::{clamp_font_size, theme_choices};

// update_check
pub use update_check::{
    AppChannel, CheckFailureReason, CheckOutcome, UpdateAction, UpdateCheckState, Version, decide,
    release_page_url, update_action,
};
