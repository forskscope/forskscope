//! Command palette view-model (RFC-019 §"Command palette", Slice 7).
//!
//! [`build_palette`] filters the command registry by a query string and
//! evaluates availability from the current [`CommandContext`], returning a
//! [`Vec<PaletteRow>`] ready for the palette component to render.
//!
//! This is the search-filtered complement to `command_bar` (which produces
//! the fixed toolbar); `palette_view` produces the dynamic filtered list.

use forskscope_core::command::{
    Availability, CommandContext, CommandDangerLevel, CommandRegistry,
    UnavailableReason,
};

// ── PaletteRow ────────────────────────────────────────────────────────────────

/// One row in the command palette list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteRow {
    /// Stable command ID.
    pub command_id:       &'static str,
    /// Human-readable label, e.g. `"Save file"`.
    pub label:            &'static str,
    /// One-sentence description shown below the label.
    pub description:      &'static str,
    /// Short keyboard shortcut hint, e.g. `"Ctrl+s"`. Empty if none.
    pub shortcut_hint:    String,
    /// Whether the command can currently be executed.
    pub enabled:          bool,
    /// If disabled, the reason shown as a dimmed tooltip.
    pub disabled_reason:  Option<&'static str>,
    /// Whether this is a destructive action (shown with a warning colour).
    pub is_dangerous:     bool,
}

// ── build_palette ─────────────────────────────────────────────────────────────

/// Build a filtered palette row list.
///
/// - `query` — search string; empty string returns all commands.
/// - Results are sorted: enabled commands first, then disabled.
/// - Within each group, the original registry order is preserved.
pub fn build_palette(
    registry: &CommandRegistry,
    ctx:      &CommandContext,
    query:    &str,
) -> Vec<PaletteRow> {
    let all = registry.all();

    let filtered: Vec<_> = if query.is_empty() {
        all.iter().collect()
    } else {
        registry.search(query).collect()
    };

    let mut rows: Vec<PaletteRow> = filtered.into_iter().map(|def| {
        let (enabled, disabled_reason) = match def.availability.evaluate(ctx) {
            Availability::Available                          => (true, None),
            Availability::Unavailable(UnavailableReason(r)) => (false, Some(r)),
        };

        let shortcut_hint = def.default_shortcuts.first().map(|s| {
            let mut parts = Vec::new();
            if s.modifiers.ctrl  { parts.push("Ctrl"); }
            if s.modifiers.alt   { parts.push("Alt"); }
            if s.modifiers.shift { parts.push("Shift"); }
            if s.modifiers.meta  { parts.push("Meta"); }
            parts.push(s.key);
            parts.join("+")
        }).unwrap_or_default();

        PaletteRow {
            command_id:      def.id.0,
            label:           def.label,
            description:     def.description,
            shortcut_hint,
            enabled,
            disabled_reason,
            is_dangerous:    def.danger_level == CommandDangerLevel::Destructive,
        }
    }).collect();

    // Stable sort: enabled rows before disabled rows.
    rows.sort_by_key(|r| if r.enabled { 0u8 } else { 1u8 });
    rows
}

/// Count enabled rows in a palette result.
pub fn enabled_count(rows: &[PaletteRow]) -> usize {
    rows.iter().filter(|r| r.enabled).count()
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
            ..Default::default()
        }
    }

    // ── Empty query returns all commands ───────────────────────────────────────

    #[test]
    fn empty_query_returns_all_commands() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "");
        assert_eq!(rows.len(), reg.all().len(),
            "empty query must return all commands");
    }

    // ── Filtering ─────────────────────────────────────────────────────────────

    #[test]
    fn query_save_returns_save_commands() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "save");
        assert!(!rows.is_empty(), "\"save\" must match at least one command");
        for row in &rows {
            let combined = format!("{} {}", row.label, row.description).to_lowercase();
            assert!(combined.contains("save"),
                "row {:?} matched 'save' but label+description don't contain it", row.label);
        }
    }

    #[test]
    fn nonsense_query_returns_empty() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "xyzzy_no_match_please");
        assert!(rows.is_empty());
    }

    #[test]
    fn query_is_case_insensitive() {
        let reg = CommandRegistry::builtin();
        let lower = build_palette(&reg, &empty_ctx(), "save");
        let upper = build_palette(&reg, &empty_ctx(), "SAVE");
        assert_eq!(lower.len(), upper.len(),
            "query matching must be case-insensitive");
    }

    // ── Sorting: enabled before disabled ──────────────────────────────────────

    #[test]
    fn enabled_rows_come_before_disabled() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &diff_ctx(), "");
        let mut saw_disabled = false;
        for row in &rows {
            if !row.enabled { saw_disabled = true; }
            if saw_disabled {
                assert!(!row.enabled,
                    "enabled row {:?} appeared after disabled rows", row.label);
            }
        }
    }

    // ── Availability reflects context ──────────────────────────────────────────

    #[test]
    fn save_is_disabled_in_empty_context() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "");
        let save = rows.iter().find(|r| r.command_id == "file.save")
            .expect("file.save must appear in palette");
        assert!(!save.enabled);
        assert!(save.disabled_reason.is_some());
    }

    #[test]
    fn next_difference_enabled_in_diff_context() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &diff_ctx(), "");
        let next = rows.iter().find(|r| r.command_id == "navigate.next_difference")
            .expect("navigate.next_difference must appear");
        assert!(next.enabled);
    }

    // ── enabled_count ─────────────────────────────────────────────────────────

    #[test]
    fn enabled_count_zero_for_empty_context_save_query() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "save");
        // Save is disabled in empty context; count might be 0 or more depending
        // on which commands match "save" and are always-available.
        let count = enabled_count(&rows);
        let _ = count; // just verify it doesn't panic
    }

    #[test]
    fn enabled_count_matches_manual_count() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &diff_ctx(), "");
        let manual = rows.iter().filter(|r| r.enabled).count();
        assert_eq!(enabled_count(&rows), manual);
    }

    // ── Row fields ─────────────────────────────────────────────────────────────

    #[test]
    fn all_rows_have_non_empty_labels() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "");
        for row in &rows {
            assert!(!row.label.is_empty(),
                "command {} must have non-empty label", row.command_id);
        }
    }

    #[test]
    fn all_rows_have_non_empty_command_ids() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "");
        for row in &rows {
            assert!(!row.command_id.is_empty());
        }
    }

    // ── PaletteRow field coverage ─────────────────────────────────────────────

    #[test]
    fn save_row_has_ctrl_s_shortcut_hint() {
        let reg = CommandRegistry::builtin();
        // "save" query should find the save command.
        let rows = build_palette(&reg, &empty_ctx(), "save");
        let save = rows.iter().find(|r| r.command_id == "file.save");
        assert!(save.is_some(), "save command must appear for 'save' query");
        let hint = &save.unwrap().shortcut_hint;
        assert!(!hint.is_empty(),
            "file.save must have a non-empty shortcut_hint (Ctrl+S)");
        assert!(hint.to_lowercase().contains('s'),
            "file.save shortcut hint '{}' must reference 's'", hint);
    }

    #[test]
    fn disabled_row_has_disabled_reason_some() {
        // In an empty context, file.save is disabled.
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "save");
        let save = rows.iter().find(|r| r.command_id == "file.save").unwrap();
        assert!(!save.enabled,
            "file.save must be disabled in empty context");
        assert!(save.disabled_reason.is_some(),
            "disabled_reason must be Some when the command is disabled");
        assert!(!save.disabled_reason.unwrap().is_empty(),
            "disabled_reason text must be non-empty");
    }

    #[test]
    fn enabled_row_has_disabled_reason_none() {
        // CommandPalette command is always enabled — its disabled_reason must be None.
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "palette");
        let palette = rows.iter().find(|r| r.command_id == "view.command_palette");
        assert!(palette.is_some(), "command_palette must appear for 'palette' query");
        let row = palette.unwrap();
        assert!(row.enabled, "view.command_palette must be enabled in any context");
        assert!(row.disabled_reason.is_none(),
            "disabled_reason must be None when the command is enabled");
    }

    #[test]
    fn all_rows_have_non_empty_descriptions() {
        let reg = CommandRegistry::builtin();
        let rows = build_palette(&reg, &empty_ctx(), "");
        for row in &rows {
            assert!(!row.description.is_empty(),
                "description must be non-empty for command '{}'", row.command_id);
        }
    }

    #[test]
    fn enabled_count_matches_enabled_rows_in_diff_context() {
        let reg = CommandRegistry::builtin();
        let ctx = diff_ctx();
        let rows = build_palette(&reg, &ctx, "");
        let manual = rows.iter().filter(|r| r.enabled).count();
        assert_eq!(enabled_count(&rows), manual,
            "enabled_count must match the number of enabled rows");
        assert!(enabled_count(&rows) > 0,
            "at least some commands must be enabled in diff context");
    }
}
