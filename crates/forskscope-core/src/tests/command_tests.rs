//! Command model and registry tests (RFC-019 §5, §6, §7).

use crate::command::{
    Availability, AvailabilityRule, CommandCategory, CommandContext,
    CommandDangerLevel, CommandRegistry, Modifiers, Shortcut, cmd,
};

// ── AvailabilityRule::evaluate ────────────────────────────────────────────────

fn ctx_empty() -> CommandContext { CommandContext::default() }

fn ctx_diff_open() -> CommandContext {
    CommandContext {
        has_active_diff_tab:    true,
        has_active_compare_tab: true,
        active_tab_has_hunks:   true,
        active_hunk_exists:     true,
        right_side_is_editable: true,
        ..Default::default()
    }
}

fn ctx_dirty_saveable() -> CommandContext {
    CommandContext {
        has_active_compare_tab: true,
        active_tab_is_dirty:    true,
        active_tab_is_saveable: true,
        ..Default::default()
    }
}

#[test]
fn always_is_available_in_empty_context() {
    assert!(AvailabilityRule::Always.evaluate(&ctx_empty()).is_available());
}

#[test]
fn dirty_and_saveable_available_only_when_dirty_and_saveable() {
    assert!(!AvailabilityRule::DirtyAndSaveable.evaluate(&ctx_empty()).is_available());
    assert!( AvailabilityRule::DirtyAndSaveable.evaluate(&ctx_dirty_saveable()).is_available());
}

#[test]
fn active_diff_tab_unavailable_with_no_open_tab() {
    let r = AvailabilityRule::ActiveDiffTab.evaluate(&ctx_empty());
    assert!(!r.is_available());
    if let Availability::Unavailable(reason) = r {
        assert!(!reason.as_str().is_empty(), "unavailable reason must be non-empty");
    }
}

#[test]
fn active_diff_tab_available_with_open_tab() {
    assert!(AvailabilityRule::ActiveDiffTab.evaluate(&ctx_diff_open()).is_available());
}

#[test]
fn active_hunk_editable_requires_hunk_and_editable_side() {
    let ctx_no_hunk = CommandContext {
        has_active_diff_tab:    true,
        active_hunk_exists:     false,
        right_side_is_editable: true,
        ..Default::default()
    };
    assert!(!AvailabilityRule::ActiveHunkEditable.evaluate(&ctx_no_hunk).is_available());
    assert!( AvailabilityRule::ActiveHunkEditable.evaluate(&ctx_diff_open()).is_available());
}

#[test]
fn can_undo_and_redo_require_history() {
    assert!(!AvailabilityRule::CanUndo.evaluate(&ctx_empty()).is_available());
    assert!(!AvailabilityRule::CanRedo.evaluate(&ctx_empty()).is_available());
    let ctx = CommandContext { can_undo: true, can_redo: true, ..Default::default() };
    assert!( AvailabilityRule::CanUndo.evaluate(&ctx).is_available());
    assert!( AvailabilityRule::CanRedo.evaluate(&ctx).is_available());
}

#[test]
fn all_unavailable_reasons_are_non_empty() {
    let rules = [
        AvailabilityRule::ActiveDiffTab,
        AvailabilityRule::DirtyAndSaveable,
        AvailabilityRule::ActiveCompareTab,
        AvailabilityRule::ActiveHunkEditable,
        AvailabilityRule::HasHunks,
        AvailabilityRule::ActiveConflict,
        AvailabilityRule::AnyConflictUnresolved,
        AvailabilityRule::CanUndo,
        AvailabilityRule::CanRedo,
        AvailabilityRule::SelectedPathExists,
    ];
    for rule in rules {
        if let Availability::Unavailable(reason) = rule.evaluate(&ctx_empty()) {
            assert!(!reason.as_str().is_empty(), "{rule:?} must have non-empty reason");
        } else {
            panic!("{rule:?} should be unavailable in empty context");
        }
    }
}

// ── CommandDangerLevel ────────────────────────────────────────────────────────

#[test]
fn safe_does_not_require_confirmation() {
    assert!(!CommandDangerLevel::Safe.requires_confirmation());
}

#[test]
fn may_discard_and_destructive_require_confirmation() {
    assert!(CommandDangerLevel::MayDiscardWork.requires_confirmation());
    assert!(CommandDangerLevel::Destructive.requires_confirmation());
}

#[test]
fn danger_level_ordering_is_ascending() {
    assert!(CommandDangerLevel::Safe < CommandDangerLevel::MayDiscardWork);
    assert!(CommandDangerLevel::MayDiscardWork < CommandDangerLevel::Destructive);
}

// ── CommandCategory ───────────────────────────────────────────────────────────

#[test]
fn all_categories_have_non_empty_label() {
    for cat in [
        CommandCategory::File, CommandCategory::Edit, CommandCategory::View,
        CommandCategory::Navigate, CommandCategory::Compare, CommandCategory::Merge,
        CommandCategory::Search, CommandCategory::Settings, CommandCategory::External,
        CommandCategory::Diagnostics,
    ] {
        assert!(!cat.label().is_empty(), "{cat:?} must have non-empty label");
    }
}

// ── CommandRegistry ───────────────────────────────────────────────────────────

#[test]
fn builtin_registry_is_non_empty() {
    let reg = CommandRegistry::builtin();
    assert!(!reg.is_empty());
    assert!(reg.len() > 10, "should have many built-in commands");
}

#[test]
fn all_builtin_ids_are_unique() {
    let reg = CommandRegistry::builtin();
    let ids: std::collections::HashSet<_> = reg.all().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), reg.len(), "all command IDs must be unique");
}

#[test]
fn all_builtin_labels_are_non_empty() {
    let reg = CommandRegistry::builtin();
    assert!(reg.all().iter().all(|c| !c.label.is_empty()),
        "all commands must have non-empty labels");
}

#[test]
fn get_finds_existing_command() {
    let reg = CommandRegistry::builtin();
    let cmd = reg.get(&cmd::SAVE).unwrap();
    assert_eq!(cmd.id, cmd::SAVE);
    assert_eq!(cmd.category, CommandCategory::File);
}

#[test]
fn get_returns_none_for_unknown_id() {
    use crate::command::CommandId;
    let reg = CommandRegistry::builtin();
    assert!(reg.get(&CommandId::new("nonexistent.command")).is_none());
}

#[test]
fn by_category_returns_only_matching_commands() {
    let reg = CommandRegistry::builtin();
    let merge_cmds: Vec<_> = reg.by_category(CommandCategory::Merge).collect();
    assert!(!merge_cmds.is_empty());
    assert!(merge_cmds.iter().all(|c| c.category == CommandCategory::Merge));
}

#[test]
fn search_matches_label_case_insensitive() {
    let reg = CommandRegistry::builtin();
    let results: Vec<_> = reg.search("save").collect();
    assert!(results.iter().any(|c| c.id == cmd::SAVE));
}

#[test]
fn search_empty_query_returns_all_commands() {
    let reg = CommandRegistry::builtin();
    assert_eq!(reg.search("").count(), reg.len());
}

#[test]
fn search_nonmatching_query_returns_nothing() {
    let reg = CommandRegistry::builtin();
    assert_eq!(reg.search("xyzzy_not_a_real_command_label").count(), 0);
}

// ── Shortcut ──────────────────────────────────────────────────────────────────

#[test]
fn find_by_shortcut_finds_ctrl_s_as_save() {
    let reg = CommandRegistry::builtin();
    let shortcut = Shortcut::new(Modifiers::CTRL, "s");
    let found = reg.find_by_shortcut(&shortcut);
    assert!(found.is_some(), "Ctrl+S must be bound to a command");
    assert_eq!(found.unwrap().id, cmd::SAVE);
}

#[test]
fn find_by_shortcut_returns_none_for_unbound_key() {
    let reg = CommandRegistry::builtin();
    let unbound = Shortcut::new(Modifiers::CTRL, "9");
    assert!(reg.find_by_shortcut(&unbound).is_none());
}

#[test]
fn modifiers_none_is_all_false() {
    assert!(Modifiers::NONE.is_none());
    assert!(!Modifiers::CTRL.is_none());
}

// ── Save command availability wires through context ───────────────────────────

#[test]
fn save_command_is_available_only_when_dirty_and_saveable() {
    let reg = CommandRegistry::builtin();
    let save_cmd = reg.get(&cmd::SAVE).unwrap();
    assert!(!save_cmd.is_available(&ctx_empty()));
    assert!( save_cmd.is_available(&ctx_dirty_saveable()));
}

#[test]
fn undo_command_is_available_only_when_can_undo() {
    let reg = CommandRegistry::builtin();
    let undo = reg.get(&cmd::UNDO).unwrap();
    assert!(!undo.is_available(&ctx_empty()));
    let ctx = CommandContext { can_undo: true, ..Default::default() };
    assert!(undo.is_available(&ctx));
}
