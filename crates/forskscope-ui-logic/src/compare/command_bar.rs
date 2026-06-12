//! Command bar view-model (RFC-019 §5, §6).
//!
//! [`ToolbarSection`] / [`ToolbarItem`] are the framework-independent
//! representation of what the toolbar should render. The Dioxus toolbar
//! component calls [`build_toolbar`] with the current [`CommandContext`]
//! and receives a flat, ordered list of sections — each carrying items
//! whose `enabled`, `label`, and `shortcut_hint` fields are fully resolved.
//!
//! This replaces ad-hoc `if can_save { ... }` guards scattered through
//! the diff component with a single, testable evaluation.
//!
//! ## Design
//!
//! - No Dioxus dependency. Works in a `#[test]` without a display server.
//! - [`ToolbarItem`] borrows nothing from `CommandRegistry`; all fields
//!   are owned so the toolbar component can hold a snapshot.
//! - The section/group structure maps to the toolbar's visual layout:
//!   File ops | Navigate | Merge | View.

use forskscope_core::command::{
    Availability, CommandContext, CommandDefinition, CommandRegistry,
    UnavailableReason, cmd,
};

// ── Toolbar item ──────────────────────────────────────────────────────────────

/// One button in the toolbar — a fully-resolved snapshot from the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarItem {
    /// Stable command ID (`"file.save"`, `"navigate.next_difference"`, …).
    pub command_id:    &'static str,
    /// Human-readable label for the button tooltip and aria-label.
    pub label:         &'static str,
    /// Whether the button should be enabled.
    pub enabled:       bool,
    /// If disabled, why — shown as a tooltip on the greyed button.
    pub disabled_reason: Option<&'static str>,
    /// Short keyboard hint to show beside the label, if any.
    pub shortcut_hint: Option<String>,
}

impl ToolbarItem {
    fn from_def(def: &CommandDefinition, ctx: &CommandContext) -> Self {
        let (enabled, disabled_reason) = match def.availability.evaluate(ctx) {
            Availability::Available                     => (true, None),
            Availability::Unavailable(UnavailableReason(r)) => (false, Some(r)),
        };

        let shortcut_hint = def.default_shortcuts.first().map(|s| {
            let mut parts = Vec::new();
            if s.modifiers.ctrl  { parts.push("Ctrl");  }
            if s.modifiers.alt   { parts.push("Alt");   }
            if s.modifiers.shift { parts.push("Shift"); }
            if s.modifiers.meta  { parts.push("Meta");  }
            parts.push(s.key);
            parts.join("+")
        });

        Self {
            command_id: def.id.0,
            label: def.label,
            enabled,
            disabled_reason,
            shortcut_hint,
        }
    }
}

// ── Toolbar section ───────────────────────────────────────────────────────────

/// A labelled group of toolbar items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarSection {
    /// Short section label (not shown by default; used for accessibility).
    pub label: &'static str,
    pub items: Vec<ToolbarItem>,
}

// ── Build toolbar ─────────────────────────────────────────────────────────────

/// Build the toolbar item list from `registry` and the current `ctx`.
///
/// Returns sections in display order:
/// 1. **File** — Save, Save As, Close Tab
/// 2. **Navigate** — Prev/Next Difference, Prev/Next Conflict
/// 3. **Merge** — Copy Hunk L→R, Copy Hunk R→L, Use Left, Use Right
/// 4. **Edit** — Undo, Redo
/// 5. **View** — Command Palette, Settings
pub fn build_toolbar(registry: &CommandRegistry, ctx: &CommandContext) -> Vec<ToolbarSection> {
    let item = |id: &forskscope_core::command::CommandId| -> Option<ToolbarItem> {
        registry.get(id).map(|def| ToolbarItem::from_def(def, ctx))
    };

    vec![
        ToolbarSection {
            label: "File",
            items: [&cmd::SAVE, &cmd::SAVE_AS, &cmd::CLOSE_TAB]
                .iter().filter_map(|id| item(id)).collect(),
        },
        ToolbarSection {
            label: "Navigate",
            items: [&cmd::PREV_DIFFERENCE, &cmd::NEXT_DIFFERENCE,
                    &cmd::PREV_CONFLICT,   &cmd::NEXT_CONFLICT]
                .iter().filter_map(|id| item(id)).collect(),
        },
        ToolbarSection {
            label: "Merge",
            items: [&cmd::COPY_HUNK_LEFT_RIGHT, &cmd::COPY_HUNK_RIGHT_LEFT,
                    &cmd::USE_LEFT, &cmd::USE_RIGHT]
                .iter().filter_map(|id| item(id)).collect(),
        },
        ToolbarSection {
            label: "Edit",
            items: [&cmd::UNDO, &cmd::REDO]
                .iter().filter_map(|id| item(id)).collect(),
        },
        ToolbarSection {
            label: "View",
            items: [&cmd::COMMAND_PALETTE, &cmd::OPEN_SETTINGS]
                .iter().filter_map(|id| item(id)).collect(),
        },
    ]
}

/// Count enabled items across all sections.
pub fn enabled_count(sections: &[ToolbarSection]) -> usize {
    sections.iter().flat_map(|s| &s.items).filter(|i| i.enabled).count()
}

/// Find a toolbar item by command ID.
pub fn find_item<'a>(sections: &'a [ToolbarSection], id: &str) -> Option<&'a ToolbarItem> {
    sections.iter()
        .flat_map(|s| &s.items)
        .find(|i| i.command_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::command::{CommandContext, CommandRegistry};

    fn empty_ctx() -> CommandContext { CommandContext::default() }

    fn diff_ctx() -> CommandContext {
        CommandContext {
            has_active_diff_tab:    true,
            has_active_compare_tab: true,
            active_tab_has_hunks:   true,
            active_hunk_exists:     true,
            right_side_is_editable: true,
            can_undo:               true,
            can_redo:               false,
            ..Default::default()
        }
    }

    fn dirty_ctx() -> CommandContext {
        CommandContext {
            has_active_compare_tab: true,
            active_tab_is_dirty:    true,
            active_tab_is_saveable: true,
            ..Default::default()
        }
    }

    #[test]
    fn build_toolbar_returns_five_sections() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        assert_eq!(sections.len(), 5, "must have 5 sections");
    }

    #[test]
    fn section_labels_are_correct() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        let labels: Vec<&str> = sections.iter().map(|s| s.label).collect();
        assert_eq!(labels, vec!["File", "Navigate", "Merge", "Edit", "View"]);
    }

    #[test]
    fn save_is_disabled_in_empty_context() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        let save = find_item(&sections, "file.save").unwrap();
        assert!(!save.enabled, "Save must be disabled when nothing is open");
        assert!(save.disabled_reason.is_some());
    }

    #[test]
    fn save_is_enabled_when_dirty_and_saveable() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &dirty_ctx());
        let save = find_item(&sections, "file.save").unwrap();
        assert!(save.enabled, "Save must be enabled when dirty and saveable");
    }

    #[test]
    fn next_difference_enabled_when_hunks_exist() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &diff_ctx());
        let next = find_item(&sections, "navigate.next_difference").unwrap();
        assert!(next.enabled);
    }

    #[test]
    fn copy_hunk_enabled_when_active_hunk_and_editable() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &diff_ctx());
        let copy = find_item(&sections, "merge.copy_left_to_right").unwrap();
        assert!(copy.enabled);
    }

    #[test]
    fn undo_enabled_redo_disabled_in_diff_ctx() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &diff_ctx());
        assert!( find_item(&sections, "edit.undo").unwrap().enabled);
        assert!(!find_item(&sections, "edit.redo").unwrap().enabled);
    }

    #[test]
    fn command_palette_always_enabled() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        let palette = find_item(&sections, "view.command_palette").unwrap();
        assert!(palette.enabled, "command palette must always be available");
    }

    #[test]
    fn save_item_has_ctrl_s_shortcut_hint() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &dirty_ctx());
        let save = find_item(&sections, "file.save").unwrap();
        assert_eq!(save.shortcut_hint.as_deref(), Some("Ctrl+s"));
    }

    #[test]
    fn enabled_count_zero_in_empty_context() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        // In empty context only Always-availability commands are enabled:
        // command_palette and open_settings.
        assert!(enabled_count(&sections) >= 2, "at least the Always commands must be enabled");
    }

    #[test]
    fn find_item_returns_none_for_unknown_id() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &empty_ctx());
        assert!(find_item(&sections, "nonexistent.command").is_none());
    }

    #[test]
    fn all_toolbar_items_have_non_empty_labels() {
        let reg = CommandRegistry::builtin();
        let sections = build_toolbar(&reg, &diff_ctx());
        for section in &sections {
            for item in &section.items {
                assert!(!item.label.is_empty(),
                    "item {} must have a non-empty label", item.command_id);
            }
        }
    }
}
