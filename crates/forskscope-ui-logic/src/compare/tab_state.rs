//! Tab state view-model — derives `CommandContext` from tab snapshot fields.
//!
//! The Dioxus `DiffWorkspace` component holds a `TabSnapshot` that carries
//! raw booleans (`can_save`, `can_undo`, `has_hunks`, …). `build_toolbar`
//! in `command_bar` needs a `CommandContext`. This module provides the
//! bridge: `context_from_snapshot(snapshot) → CommandContext`.
//!
//! Keeping this in `ui-logic` (rather than in the Dioxus component) means
//! the derivation is unit-testable without GTK.

use forskscope_core::command::CommandContext;

/// Minimal snapshot of tab state needed to derive `CommandContext`.
///
/// The Dioxus component populates this from `TabSnapshot` and passes it to
/// `context_from_snapshot`. All fields default to `false` so callers only
/// set the flags they know about.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TabStateSnapshot {
    /// A diff or compare tab is currently active.
    pub has_active_diff_tab:     bool,
    /// A compare tab is active (any kind — text, binary, xlsx).
    pub has_active_compare_tab:  bool,
    /// The active tab has unsaved changes.
    pub active_tab_is_dirty:     bool,
    /// The active tab's content can be saved (editable, has a target path).
    pub active_tab_is_saveable:  bool,
    /// The active tab contains at least one changed hunk.
    pub active_tab_has_hunks:    bool,
    /// There is a currently focused/highlighted hunk.
    pub active_hunk_exists:      bool,
    /// The right side of the active tab is editable (not read-only).
    pub right_side_is_editable:  bool,
    /// There is a focused conflict in a three-way merge session.
    pub has_active_conflict:     bool,
    /// At least one conflict in the session is unresolved.
    pub any_conflict_unresolved: bool,
    /// The undo stack has entries.
    pub can_undo:                bool,
    /// The redo stack has entries.
    pub can_redo:                bool,
    /// A path is selected in the explorer pane.
    pub selected_path_exists:    bool,
}

/// Derive a `CommandContext` from a `TabStateSnapshot`.
///
/// This is the bridge between `TabSnapshot` (Dioxus-side) and
/// `CommandContext` (core-side) so `build_toolbar` receives the correct
/// availability flags.
pub fn context_from_snapshot(snap: &TabStateSnapshot) -> CommandContext {
    CommandContext {
        has_active_diff_tab:     snap.has_active_diff_tab,
        has_active_compare_tab:  snap.has_active_compare_tab,
        active_tab_is_dirty:     snap.active_tab_is_dirty,
        active_tab_is_saveable:  snap.active_tab_is_saveable,
        active_tab_has_hunks:    snap.active_tab_has_hunks,
        active_hunk_exists:      snap.active_hunk_exists,
        right_side_is_editable:  snap.right_side_is_editable,
        has_active_conflict:     snap.has_active_conflict,
        any_conflict_unresolved: snap.any_conflict_unresolved,
        can_undo:                snap.can_undo,
        can_redo:                snap.can_redo,
        selected_path_exists:    snap.selected_path_exists,
        ..CommandContext::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::command::{AvailabilityRule, CommandRegistry};
    use crate::compare::command_bar::{build_toolbar, find_item};

    fn dirty_tab() -> TabStateSnapshot {
        TabStateSnapshot {
            has_active_diff_tab:    true,
            has_active_compare_tab: true,
            active_tab_is_dirty:    true,
            active_tab_is_saveable: true,
            active_tab_has_hunks:   true,
            right_side_is_editable: true,
            active_hunk_exists:     true,
            can_undo:               true,
            ..Default::default()
        }
    }

    #[test]
    fn default_snapshot_produces_all_false_context() {
        let ctx = context_from_snapshot(&TabStateSnapshot::default());
        assert!(!ctx.has_active_diff_tab);
        assert!(!ctx.active_tab_is_dirty);
        assert!(!ctx.can_undo);
    }

    #[test]
    fn dirty_tab_snapshot_produces_correct_context() {
        let ctx = context_from_snapshot(&dirty_tab());
        assert!(ctx.has_active_diff_tab);
        assert!(ctx.active_tab_is_dirty);
        assert!(ctx.active_tab_is_saveable);
        assert!(ctx.active_tab_has_hunks);
        assert!(ctx.can_undo);
        assert!(!ctx.can_redo);
    }

    #[test]
    fn context_wires_through_to_toolbar_correctly() {
        // End-to-end: TabStateSnapshot → CommandContext → build_toolbar → item enabled.
        let snap = dirty_tab();
        let ctx  = context_from_snapshot(&snap);
        let reg  = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &ctx);

        assert!(find_item(&sections, "file.save").unwrap().enabled,
            "save must be enabled for dirty+saveable tab");
        assert!(find_item(&sections, "edit.undo").unwrap().enabled,
            "undo must be enabled when can_undo is true");
        assert!(!find_item(&sections, "edit.redo").unwrap().enabled,
            "redo must be disabled when can_redo is false");
        assert!(find_item(&sections, "navigate.next_difference").unwrap().enabled,
            "next_difference must be enabled when tab has hunks");
    }

    #[test]
    fn empty_snapshot_disables_save_in_toolbar() {
        let ctx = context_from_snapshot(&TabStateSnapshot::default());
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &ctx);
        assert!(!find_item(&sections, "file.save").unwrap().enabled);
    }

    #[test]
    fn context_from_snapshot_is_inverse_of_availability_evaluate() {
        // Verify the context fields satisfy the AvailabilityRule contracts.
        let snap = dirty_tab();
        let ctx  = context_from_snapshot(&snap);
        assert!(AvailabilityRule::DirtyAndSaveable.evaluate(&ctx).is_available());
        assert!(AvailabilityRule::HasHunks.evaluate(&ctx).is_available());
        assert!(AvailabilityRule::ActiveHunkEditable.evaluate(&ctx).is_available());
        assert!(AvailabilityRule::CanUndo.evaluate(&ctx).is_available());
        assert!(!AvailabilityRule::CanRedo.evaluate(&ctx).is_available());
    }
}
