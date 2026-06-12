//! Command model and registry (RFC-019).
//!
//! `CommandDefinition` is the single source of truth for every user-visible
//! action in ForskScope: its `id`, human-readable `label`, keyboard `Shortcut`s,
//! `CommandCategory`, and `AvailabilityRule`.
//!
//! The toolbar, keyboard handler, and command palette all read from the same
//! `CommandRegistry` rather than implementing their own ad-hoc availability
//! logic. The UI derives button-enabled state and menu-item disabled-reason
//! by calling `AvailabilityRule::evaluate(ctx)`.
//!
//! ## Design (RFC-019 §5, §6, §7)
//!
//! - Commands are pure data; no closures or callbacks are stored here.
//!   Execution is the UI layer's responsibility.
//! - `AvailabilityRule` is evaluated at render time against `CommandContext`,
//!   which carries the minimal state snapshot the rule needs.
//! - Shortcut resolution order (§7): modal-specific > editor-focus >
//!   global app > WebView default. Enforcement is the UI layer's job;
//!   this module only records each command's `default_shortcuts`.

// ── Command identity ──────────────────────────────────────────────────────────

/// Stable string identifier for a command (RFC-019 §5, §"Must Stabilize").
///
/// IDs use a `category.action` dotted namespace, e.g. `"file.save"`,
/// `"merge.copy_left_to_right"`. Never localised; used as keys in keybinding
/// config and undo labels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub &'static str);

impl CommandId {
    pub const fn new(id: &'static str) -> Self { Self(id) }
    pub fn as_str(&self) -> &str { self.0 }
}

// ── Well-known command IDs ────────────────────────────────────────────────────

/// All built-in command IDs, grouped by category (RFC-019 §11).
pub mod cmd {
    use super::CommandId;

    // File
    pub const OPEN_FILE_PAIR:       CommandId = CommandId::new("file.open_file_pair");
    pub const OPEN_DIR_PAIR:        CommandId = CommandId::new("file.open_dir_pair");
    pub const SAVE:                 CommandId = CommandId::new("file.save");
    pub const SAVE_AS:              CommandId = CommandId::new("file.save_as");
    pub const CLOSE_TAB:            CommandId = CommandId::new("file.close_tab");
    pub const QUIT:                 CommandId = CommandId::new("file.quit");

    // Edit
    pub const UNDO:                 CommandId = CommandId::new("edit.undo");
    pub const REDO:                 CommandId = CommandId::new("edit.redo");
    pub const FIND:                 CommandId = CommandId::new("edit.find");

    // Navigate
    pub const NEXT_DIFFERENCE:      CommandId = CommandId::new("navigate.next_difference");
    pub const PREV_DIFFERENCE:      CommandId = CommandId::new("navigate.prev_difference");
    pub const NEXT_CONFLICT:        CommandId = CommandId::new("navigate.next_conflict");
    pub const PREV_CONFLICT:        CommandId = CommandId::new("navigate.prev_conflict");

    // Compare
    pub const RELOAD_TAB:           CommandId = CommandId::new("compare.reload_tab");
    pub const SWAP_SIDES:           CommandId = CommandId::new("compare.swap_sides");
    pub const OPEN_COMPARE:         CommandId = CommandId::new("compare.open_compare");

    // Merge
    pub const COPY_HUNK_LEFT_RIGHT: CommandId = CommandId::new("merge.copy_left_to_right");
    pub const COPY_HUNK_RIGHT_LEFT: CommandId = CommandId::new("merge.copy_right_to_left");
    pub const COPY_ALL_LEFT_RIGHT:  CommandId = CommandId::new("merge.copy_all_left_to_right");
    pub const COPY_ALL_RIGHT_LEFT:  CommandId = CommandId::new("merge.copy_all_right_to_left");
    pub const USE_LEFT:             CommandId = CommandId::new("merge.use_left");
    pub const USE_RIGHT:            CommandId = CommandId::new("merge.use_right");
    pub const USE_BOTH:             CommandId = CommandId::new("merge.use_both");
    pub const IGNORE_CONFLICT:      CommandId = CommandId::new("merge.ignore_conflict");
    pub const REVERT_HUNK:          CommandId = CommandId::new("merge.revert_hunk");

    // Search
    pub const FIND_NEXT:            CommandId = CommandId::new("search.find_next");
    pub const FIND_PREV:            CommandId = CommandId::new("search.find_prev");

    // View
    pub const TOGGLE_EXPLORER:      CommandId = CommandId::new("view.toggle_explorer");
    pub const TOGGLE_DIAGNOSTICS:   CommandId = CommandId::new("view.toggle_diagnostics");
    pub const COMMAND_PALETTE:      CommandId = CommandId::new("view.command_palette");

    // Settings
    pub const OPEN_SETTINGS:        CommandId = CommandId::new("settings.open");

    // External
    pub const OPEN_PARENT_FOLDER:   CommandId = CommandId::new("external.open_parent_folder");
    pub const OPEN_FILE_EXTERNAL:   CommandId = CommandId::new("external.open_file");
}

// ── Category ──────────────────────────────────────────────────────────────────

/// Command category for the command palette and menu structure (RFC-019 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandCategory {
    File,
    Edit,
    View,
    Navigate,
    Compare,
    Merge,
    Search,
    Settings,
    External,
    Diagnostics,
}

impl CommandCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::File        => "File",
            Self::Edit        => "Edit",
            Self::View        => "View",
            Self::Navigate    => "Navigate",
            Self::Compare     => "Compare",
            Self::Merge       => "Merge",
            Self::Search      => "Search",
            Self::Settings    => "Settings",
            Self::External    => "External",
            Self::Diagnostics => "Diagnostics",
        }
    }
}

// ── Danger level ──────────────────────────────────────────────────────────────

/// How much care is required before executing this command (RFC-019 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CommandDangerLevel {
    /// Normal operation; no extra confirmation.
    #[default]
    Safe,
    /// May discard unsaved work; confirm if tab is dirty.
    MayDiscardWork,
    /// Overwrites files or performs irreversible filesystem changes.
    Destructive,
}

impl CommandDangerLevel {
    pub fn requires_confirmation(self) -> bool {
        !matches!(self, Self::Safe)
    }
}

// ── Shortcut ──────────────────────────────────────────────────────────────────

/// Modifier keys for a keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl:  bool,
    pub alt:   bool,
    pub shift: bool,
    pub meta:  bool,  // Cmd on macOS, Super on Linux
}

impl Modifiers {
    pub const NONE:       Self = Self { ctrl: false, alt: false, shift: false, meta: false };
    pub const CTRL:       Self = Self { ctrl: true,  alt: false, shift: false, meta: false };
    pub const ALT:        Self = Self { ctrl: false, alt: true,  shift: false, meta: false };
    pub const CTRL_SHIFT: Self = Self { ctrl: true,  alt: false, shift: true,  meta: false };

    pub fn is_none(self) -> bool { !self.ctrl && !self.alt && !self.shift && !self.meta }
}

/// One keyboard shortcut (RFC-019 §7).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub modifiers: Modifiers,
    /// Key name, e.g. `"s"`, `"F7"`, `"Tab"`, `"ArrowRight"`.
    pub key:       &'static str,
}

impl Shortcut {
    pub const fn new(modifiers: Modifiers, key: &'static str) -> Self {
        Self { modifiers, key }
    }
}

// ── Availability rule ─────────────────────────────────────────────────────────

/// The state snapshot evaluated by [`AvailabilityRule`] (RFC-019 §6).
///
/// Populated by the UI at render time from the session model.
#[derive(Debug, Clone, Default)]
pub struct CommandContext {
    pub has_active_diff_tab:     bool,
    pub has_active_compare_tab:  bool,
    pub active_tab_is_dirty:     bool,
    pub active_tab_is_saveable:  bool,
    pub active_tab_has_hunks:    bool,
    pub active_hunk_exists:      bool,
    pub right_side_is_editable:  bool,
    pub has_active_conflict:     bool,
    pub any_conflict_unresolved: bool,
    pub can_undo:                bool,
    pub can_redo:                bool,
    pub selected_path_exists:    bool,
    pub explorer_is_visible:     bool,
}

/// Why a command is unavailable (shown as tooltip on disabled controls).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnavailableReason(pub &'static str);

impl UnavailableReason {
    pub fn as_str(&self) -> &str { self.0 }
}

/// Result of evaluating command availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// The command can be executed.
    Available,
    /// The command cannot be executed; carry the human-readable reason.
    Unavailable(UnavailableReason),
}

impl Availability {
    pub fn is_available(&self) -> bool { matches!(self, Self::Available) }
}

/// The availability rule for a command — evaluated against [`CommandContext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityRule {
    /// Always available.
    Always,
    /// Available only when a diff/compare tab is active.
    ActiveDiffTab,
    /// Available only when the active tab has unsaved changes and is saveable.
    DirtyAndSaveable,
    /// Available only when the active tab has text content (for Save As).
    ActiveCompareTab,
    /// Available only when there is a current hunk and right side is editable.
    ActiveHunkEditable,
    /// Available only when the active tab has any diff hunks.
    HasHunks,
    /// Available only when there is an active (focused) conflict.
    ActiveConflict,
    /// Available only when any conflict is unresolved.
    AnyConflictUnresolved,
    /// Available only when undo is possible.
    CanUndo,
    /// Available only when redo is possible.
    CanRedo,
    /// Available only when a path is selected in the explorer.
    SelectedPathExists,
}

impl AvailabilityRule {
    /// Evaluate the rule against the current command context.
    pub fn evaluate(self, ctx: &CommandContext) -> Availability {
        let (ok, reason) = match self {
            Self::Always => (true, ""),
            Self::ActiveDiffTab =>
                (ctx.has_active_diff_tab, "No comparison is open"),
            Self::DirtyAndSaveable =>
                (ctx.active_tab_is_dirty && ctx.active_tab_is_saveable,
                 "Nothing to save"),
            Self::ActiveCompareTab =>
                (ctx.has_active_compare_tab, "No file is open"),
            Self::ActiveHunkEditable =>
                (ctx.active_hunk_exists && ctx.right_side_is_editable,
                 "No editable hunk is focused"),
            Self::HasHunks =>
                (ctx.active_tab_has_hunks, "No differences found"),
            Self::ActiveConflict =>
                (ctx.has_active_conflict, "No conflict is selected"),
            Self::AnyConflictUnresolved =>
                (ctx.any_conflict_unresolved, "All conflicts are resolved"),
            Self::CanUndo =>
                (ctx.can_undo, "Nothing to undo"),
            Self::CanRedo =>
                (ctx.can_redo, "Nothing to redo"),
            Self::SelectedPathExists =>
                (ctx.selected_path_exists, "No file is selected"),
        };
        if ok {
            Availability::Available
        } else {
            Availability::Unavailable(UnavailableReason(reason))
        }
    }
}

// ── Command definition ────────────────────────────────────────────────────────

/// Complete definition of one user-visible command (RFC-019 §5).
#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub id:               CommandId,
    pub label:            &'static str,
    pub description:      &'static str,
    pub category:         CommandCategory,
    pub default_shortcuts: Vec<Shortcut>,
    pub availability:     AvailabilityRule,
    pub danger_level:     CommandDangerLevel,
}

impl CommandDefinition {
    /// Evaluate whether this command is currently available.
    pub fn is_available(&self, ctx: &CommandContext) -> bool {
        self.availability.evaluate(ctx).is_available()
    }
}

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
