//! Settings-persistence recovery view-model (RFC-076 §"User-facing
//! behavior", implementation sequence step 4: "Add runtime adapter tests
//! before changing `App`").
//!
//! [`SettingsRecoveryView::from_resolution`] maps a core
//! [`SettingsRuntimeResolution`] — the decision about what value this run
//! uses and whether a migration was durably committed — into what a
//! settings-recovery dialog needs to render: a one-time migration notice, or
//! a blocking incompatibility/corruption dialog with ordered recovery
//! actions. Same "core decides, ui-logic renders" split as
//! [`crate::compare::save_error::SaveErrorView`].
//!
//! Patch 3 boundary: nothing here is called by `App` yet — `forskscope-ui`
//! still calls `app_json_settings::ConfigManager` directly until patch 4.

use forskscope_core::persist::v2::settings::runtime::{
    SettingsRuntimeOutcome, SettingsRuntimeResolution,
};

/// A one-time notice that a migration was durably written. Only produced
/// once the write actually landed — an uncommitted migration (review 037
/// N1's race) says nothing, since it will simply be retried next run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationNotice {
    pub message: String,
}

/// Ordered actions for a blocking recovery dialog. `ChooseAnotherLocation`
/// is not offered: RFC-076 lists it only "if that capability is later
/// approved", and it is not implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDialogAction {
    Exit,
    ContinueWithTemporaryDefaults,
    ResetAndBackupOriginal,
}

/// A blocking dialog for a future-version or corrupt settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryDialogView {
    pub title: String,
    pub body: String,
    pub actions: Vec<RecoveryDialogAction>,
}

/// Everything the settings-recovery UI needs, derived from one
/// [`SettingsRuntimeResolution`]. At most one of `migration_notice`/`dialog`
/// is ever set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRecoveryView {
    pub migration_notice: Option<MigrationNotice>,
    pub dialog: Option<RecoveryDialogView>,
}

impl SettingsRecoveryView {
    pub fn from_resolution(resolution: &SettingsRuntimeResolution) -> Self {
        match &resolution.outcome {
            SettingsRuntimeOutcome::Fresh | SettingsRuntimeOutcome::Current => Self {
                migration_notice: None,
                dialog: None,
            },
            SettingsRuntimeOutcome::Migrated { committed, .. } => Self {
                migration_notice: committed.then(|| MigrationNotice {
                    message: "Your settings were upgraded to the current format.".into(),
                }),
                dialog: None,
            },
            SettingsRuntimeOutcome::Incompatible { schema, version } => Self {
                migration_notice: None,
                dialog: Some(RecoveryDialogView {
                    title: "Settings file is from a newer version".into(),
                    body: format!(
                        "This settings file uses \"{schema}\" schema version {version}, which this version of ForskScope does not understand. The file has not been modified."
                    ),
                    actions: vec![
                        RecoveryDialogAction::Exit,
                        RecoveryDialogAction::ContinueWithTemporaryDefaults,
                    ],
                }),
            },
            SettingsRuntimeOutcome::CorruptPreserved { detail } => Self {
                migration_notice: None,
                dialog: Some(RecoveryDialogView {
                    title: "Settings file could not be read".into(),
                    body: format!(
                        "The settings file is preserved but could not be parsed: {detail}."
                    ),
                    actions: vec![
                        RecoveryDialogAction::ContinueWithTemporaryDefaults,
                        RecoveryDialogAction::ResetAndBackupOriginal,
                    ],
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::persist::v2::PersistenceError;
    use forskscope_core::persist::v2::settings::PersistedSettingsV2;

    fn resolution(
        outcome: SettingsRuntimeOutcome,
        write_disabled: bool,
    ) -> SettingsRuntimeResolution {
        SettingsRuntimeResolution {
            value: PersistedSettingsV2::default(),
            write_disabled,
            outcome,
        }
    }

    #[test]
    fn fresh_has_no_notice_and_no_dialog() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::Fresh,
            false,
        ));
        assert!(view.migration_notice.is_none());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn current_has_no_notice_and_no_dialog() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::Current,
            false,
        ));
        assert!(view.migration_notice.is_none());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn committed_migration_shows_a_notice_and_no_dialog() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::Migrated {
                backup_path: Some("/tmp/settings.json.pre-v2.bak".into()),
                committed: true,
            },
            false,
        ));
        assert!(view.migration_notice.is_some());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn uncommitted_migration_shows_no_notice() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::Migrated {
                backup_path: None,
                committed: false,
            },
            false,
        ));
        assert!(
            view.migration_notice.is_none(),
            "an uncommitted migration will simply retry next run; nothing to tell the user yet"
        );
        assert!(view.dialog.is_none());
    }

    #[test]
    fn incompatible_shows_exit_and_continue_but_not_reset() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::Incompatible {
                schema: "settings".into(),
                version: 99,
            },
            true,
        ));
        let dialog = view.dialog.expect("must produce a dialog");
        assert!(dialog.actions.contains(&RecoveryDialogAction::Exit));
        assert!(
            dialog
                .actions
                .contains(&RecoveryDialogAction::ContinueWithTemporaryDefaults)
        );
        assert!(
            !dialog
                .actions
                .contains(&RecoveryDialogAction::ResetAndBackupOriginal),
            "a future file must never be offered for reset — it may be valid to a newer build"
        );
        assert!(dialog.body.contains("99"));
    }

    #[test]
    fn corrupt_shows_continue_and_reset_but_not_exit() {
        let view = SettingsRecoveryView::from_resolution(&resolution(
            SettingsRuntimeOutcome::CorruptPreserved {
                detail: PersistenceError::MalformedJson,
            },
            true,
        ));
        let dialog = view.dialog.expect("must produce a dialog");
        assert!(
            dialog
                .actions
                .contains(&RecoveryDialogAction::ContinueWithTemporaryDefaults)
        );
        assert!(
            dialog
                .actions
                .contains(&RecoveryDialogAction::ResetAndBackupOriginal)
        );
        assert!(!dialog.actions.contains(&RecoveryDialogAction::Exit));
    }
}
