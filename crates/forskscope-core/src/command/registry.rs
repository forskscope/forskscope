//! Command registry and builtin command definitions (RFC-019).

use super::{
    AvailabilityRule, CommandCategory,
    CommandDangerLevel, CommandDefinition, CommandId, Modifiers, Shortcut,
};
use super::cmd;

// ── Command registry ──────────────────────────────────────────────────────────

/// All command definitions, indexed by `CommandId` (RFC-019 §5).
#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: Vec<CommandDefinition>,
}

impl CommandRegistry {
    /// Build the registry with all built-in command definitions.
    pub fn builtin() -> Self {
        use Modifiers as M;
        let mut r = Self::default();

        // ── File ──────────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::OPEN_FILE_PAIR, label: "Open File Pair…",
            description: "Open two files for comparison",
            category: CommandCategory::File,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "o")],
            availability: AvailabilityRule::Always,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::SAVE, label: "Save",
            description: "Save the merge result",
            category: CommandCategory::File,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "s")],
            availability: AvailabilityRule::DirtyAndSaveable,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::SAVE_AS, label: "Save As…",
            description: "Save the merge result to a new file",
            category: CommandCategory::File,
            default_shortcuts: vec![Shortcut::new(M::CTRL_SHIFT, "s")],
            availability: AvailabilityRule::ActiveCompareTab,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::CLOSE_TAB, label: "Close Tab",
            description: "Close the active comparison tab",
            category: CommandCategory::File,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "w")],
            availability: AvailabilityRule::ActiveCompareTab,
            danger_level: CommandDangerLevel::MayDiscardWork,
        });

        // ── Edit ──────────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::UNDO, label: "Undo",
            description: "Undo the last merge action or edit",
            category: CommandCategory::Edit,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "z")],
            availability: AvailabilityRule::CanUndo,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::REDO, label: "Redo",
            description: "Redo the last undone action",
            category: CommandCategory::Edit,
            default_shortcuts: vec![Shortcut::new(M::CTRL_SHIFT, "z")],
            availability: AvailabilityRule::CanRedo,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::FIND, label: "Find…",
            description: "Search within the active comparison",
            category: CommandCategory::Edit,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "f")],
            availability: AvailabilityRule::ActiveDiffTab,
            danger_level: CommandDangerLevel::Safe,
        });

        // ── Navigate ──────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::NEXT_DIFFERENCE, label: "Next Difference",
            description: "Move to the next changed hunk",
            category: CommandCategory::Navigate,
            default_shortcuts: vec![Shortcut::new(M::NONE, "F8")],
            availability: AvailabilityRule::HasHunks,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::PREV_DIFFERENCE, label: "Previous Difference",
            description: "Move to the previous changed hunk",
            category: CommandCategory::Navigate,
            default_shortcuts: vec![Shortcut::new(M::NONE, "F7")],
            availability: AvailabilityRule::HasHunks,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::NEXT_CONFLICT, label: "Next Conflict",
            description: "Move to the next unresolved conflict",
            category: CommandCategory::Navigate,
            default_shortcuts: vec![],
            availability: AvailabilityRule::AnyConflictUnresolved,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::PREV_CONFLICT, label: "Previous Conflict",
            description: "Move to the previous unresolved conflict",
            category: CommandCategory::Navigate,
            default_shortcuts: vec![],
            availability: AvailabilityRule::AnyConflictUnresolved,
            danger_level: CommandDangerLevel::Safe,
        });

        // ── Compare ───────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::RELOAD_TAB, label: "Reload",
            description: "Reload both files and recompute the diff",
            category: CommandCategory::Compare,
            default_shortcuts: vec![Shortcut::new(M::CTRL, "r")],
            availability: AvailabilityRule::ActiveDiffTab,
            danger_level: CommandDangerLevel::MayDiscardWork,
        });
        r.add(CommandDefinition {
            id: cmd::SWAP_SIDES, label: "Swap Sides",
            description: "Swap left and right files",
            category: CommandCategory::Compare,
            default_shortcuts: vec![],
            availability: AvailabilityRule::ActiveDiffTab,
            danger_level: CommandDangerLevel::MayDiscardWork,
        });

        // ── Merge ─────────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::COPY_HUNK_LEFT_RIGHT, label: "Copy Hunk Left → Right",
            description: "Apply the focused hunk from left to right",
            category: CommandCategory::Merge,
            default_shortcuts: vec![Shortcut::new(M::ALT, "ArrowRight")],
            availability: AvailabilityRule::ActiveHunkEditable,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::COPY_HUNK_RIGHT_LEFT, label: "Copy Hunk Right → Left",
            description: "Apply the focused hunk from right to left",
            category: CommandCategory::Merge,
            default_shortcuts: vec![Shortcut::new(M::ALT, "ArrowLeft")],
            availability: AvailabilityRule::ActiveHunkEditable,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::USE_LEFT, label: "Use Left",
            description: "Resolve the active conflict using the left version",
            category: CommandCategory::Merge,
            default_shortcuts: vec![],
            availability: AvailabilityRule::ActiveConflict,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::USE_RIGHT, label: "Use Right",
            description: "Resolve the active conflict using the right version",
            category: CommandCategory::Merge,
            default_shortcuts: vec![],
            availability: AvailabilityRule::ActiveConflict,
            danger_level: CommandDangerLevel::Safe,
        });

        // ── View ──────────────────────────────────────────────────────────
        r.add(CommandDefinition {
            id: cmd::COMMAND_PALETTE, label: "Command Palette…",
            description: "Open the command palette",
            category: CommandCategory::View,
            default_shortcuts: vec![Shortcut::new(M::CTRL_SHIFT, "p")],
            availability: AvailabilityRule::Always,
            danger_level: CommandDangerLevel::Safe,
        });
        r.add(CommandDefinition {
            id: cmd::OPEN_SETTINGS, label: "Settings…",
            description: "Open the settings dialog",
            category: CommandCategory::Settings,
            default_shortcuts: vec![Shortcut::new(M::CTRL, ",")],
            availability: AvailabilityRule::Always,
            danger_level: CommandDangerLevel::Safe,
        });

        r
    }

    pub fn add(&mut self, def: CommandDefinition) {
        self.commands.push(def);
    }

    pub fn get(&self, id: &CommandId) -> Option<&CommandDefinition> {
        self.commands.iter().find(|c| &c.id == id)
    }

    pub fn all(&self) -> &[CommandDefinition] {
        &self.commands
    }

    pub fn by_category(&self, cat: CommandCategory) -> impl Iterator<Item = &CommandDefinition> {
        self.commands.iter().filter(move |c| c.category == cat)
    }

    /// Filter by query string — matches label and description, case-insensitive.
    pub fn search<'a>(&'a self, query: &'a str) -> impl Iterator<Item = &'a CommandDefinition> {
        let q = query.to_ascii_lowercase();
        self.commands.iter().filter(move |c| {
            let q = q.as_str();
            c.label.to_ascii_lowercase().contains(q)
                || c.description.to_ascii_lowercase().contains(q)
        })
    }

    /// Find the command bound to a given shortcut (first match).
    pub fn find_by_shortcut(&self, s: &Shortcut) -> Option<&CommandDefinition> {
        self.commands.iter()
            .find(|c| c.default_shortcuts.iter().any(|sc| sc == s))
    }

    pub fn len(&self) -> usize { self.commands.len() }
    pub fn is_empty(&self) -> bool { self.commands.is_empty() }
}
