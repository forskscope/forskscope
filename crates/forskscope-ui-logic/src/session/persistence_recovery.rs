//! Session-persistence recovery view-model (RFC-076 §"User-facing
//! behavior"). Mirrors [`crate::settings::persistence_recovery`]; see its
//! module doc for the full rationale.

use forskscope_core::persist::schema::session::runtime::{
    MigrationCommitOutcome, SessionRuntimeOutcome, SessionRuntimeResolution,
};

/// A one-time notice that a migration was durably written. Only produced
/// once the write actually landed — a commit deferred by conflict (review
/// 037 N1's race) says nothing, since it will simply be retried next run.
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
    /// Proceed using `resolution.value`, which is temporary defaults for
    /// `Incompatible`/`CorruptPreserved` (review 038: the file is preserved
    /// unread/unparsed).
    ContinueWithTemporaryDefaults,
    /// Proceed using `resolution.value`, which for `Migrated(Failed)` is the
    /// correctly migrated session — not defaults (review 039 N1: reusing
    /// `ContinueWithTemporaryDefaults` here would tell the user they lost
    /// their session when they have not).
    ContinueWithoutSaving,
    ResetAndBackupOriginal,
}

/// Short button label for a [`RecoveryDialogAction`] (mirrors
/// [`crate::compare::save_error::action_label`]'s established pattern).
pub fn action_label(action: RecoveryDialogAction) -> &'static str {
    match action {
        RecoveryDialogAction::Exit => "Exit",
        RecoveryDialogAction::ContinueWithTemporaryDefaults => "Continue with defaults",
        RecoveryDialogAction::ContinueWithoutSaving => "Continue without saving",
        RecoveryDialogAction::ResetAndBackupOriginal => "Reset and back up",
    }
}

/// A blocking dialog for a future-version, corrupt, or unwritable session
/// file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryDialogView {
    pub title: String,
    pub body: String,
    pub actions: Vec<RecoveryDialogAction>,
}

/// Everything the session-recovery UI needs, derived from one
/// [`SessionRuntimeResolution`]. At most one of `migration_notice`/`dialog`
/// is ever set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecoveryView {
    pub migration_notice: Option<MigrationNotice>,
    pub dialog: Option<RecoveryDialogView>,
}

impl SessionRecoveryView {
    pub fn from_resolution(resolution: &SessionRuntimeResolution) -> Self {
        match &resolution.outcome {
            SessionRuntimeOutcome::Fresh | SessionRuntimeOutcome::Current => Self {
                migration_notice: None,
                dialog: None,
            },
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::Committed { .. }) => Self {
                migration_notice: Some(MigrationNotice {
                    message: "Your session was upgraded to the current format.".into(),
                }),
                dialog: None,
            },
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::DeferredByConflict) => Self {
                migration_notice: None,
                dialog: None,
            },
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::Failed { detail }) => Self {
                migration_notice: None,
                dialog: Some(RecoveryDialogView {
                    title: "Session could not be upgraded".into(),
                    body: format!(
                        "Your session was read and is in use for this run, but it could not be saved in the new format ({detail}). Changes will not be saved until this is resolved."
                    ),
                    actions: vec![
                        RecoveryDialogAction::Exit,
                        RecoveryDialogAction::ContinueWithoutSaving,
                    ],
                }),
            },
            SessionRuntimeOutcome::Incompatible { schema, version } => Self {
                migration_notice: None,
                dialog: Some(RecoveryDialogView {
                    title: "Session file is from a newer version".into(),
                    body: format!(
                        "This session file uses \"{schema}\" schema version {version}, which this version of ForskScope does not understand. The file has not been modified. Changes you make this session will not be saved."
                    ),
                    actions: vec![
                        RecoveryDialogAction::Exit,
                        RecoveryDialogAction::ContinueWithTemporaryDefaults,
                    ],
                }),
            },
            SessionRuntimeOutcome::CorruptPreserved { detail } => Self {
                migration_notice: None,
                dialog: Some(RecoveryDialogView {
                    title: "Session file could not be read".into(),
                    body: format!(
                        "The session file is preserved but could not be parsed: {detail}. Changes you make this session will not be saved unless you reset it."
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
    use forskscope_core::persist::schema::PersistenceError;
    use forskscope_core::persist::schema::session::PersistedSession;

    #[test]
    fn all_recovery_dialog_actions_have_non_empty_labels() {
        for action in [
            RecoveryDialogAction::Exit,
            RecoveryDialogAction::ContinueWithTemporaryDefaults,
            RecoveryDialogAction::ContinueWithoutSaving,
            RecoveryDialogAction::ResetAndBackupOriginal,
        ] {
            assert!(!action_label(action).is_empty());
        }
    }

    fn resolution(
        outcome: SessionRuntimeOutcome,
        write_disabled: bool,
    ) -> SessionRuntimeResolution {
        SessionRuntimeResolution {
            value: PersistedSession::default(),
            write_disabled,
            outcome,
            raw_bytes: None,
        }
    }

    #[test]
    fn fresh_has_no_notice_and_no_dialog() {
        let view =
            SessionRecoveryView::from_resolution(&resolution(SessionRuntimeOutcome::Fresh, false));
        assert!(view.migration_notice.is_none());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn current_has_no_notice_and_no_dialog() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::Current,
            false,
        ));
        assert!(view.migration_notice.is_none());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn committed_migration_shows_a_notice_and_no_dialog() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::Committed {
                backup_path: Some("/tmp/session.json.pre-v2.bak".into()),
            }),
            false,
        ));
        assert!(view.migration_notice.is_some());
        assert!(view.dialog.is_none());
    }

    #[test]
    fn deferred_by_conflict_shows_no_notice_and_no_dialog() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::DeferredByConflict),
            true,
        ));
        assert!(
            view.migration_notice.is_none(),
            "a conflict-deferred migration will simply retry next run; nothing to tell the user yet"
        );
        assert!(view.dialog.is_none());
    }

    #[test]
    fn failed_migration_commit_shows_a_dialog_not_silence() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::Migrated(MigrationCommitOutcome::Failed {
                detail: "permission denied".into(),
            }),
            true,
        ));
        assert!(
            view.migration_notice.is_none(),
            "a failed commit is not a success and must not produce a success-shaped notice"
        );
        let dialog = view.dialog.expect(
            "a persistent commit failure recurs every launch and must be surfaced, not silent",
        );
        assert!(dialog.body.contains("permission denied"));
        assert!(dialog.actions.contains(&RecoveryDialogAction::Exit));
        assert!(
            dialog
                .actions
                .contains(&RecoveryDialogAction::ContinueWithoutSaving),
            "the migrated session is correct and in use, not defaults — the action must say so"
        );
        assert!(
            !dialog
                .actions
                .contains(&RecoveryDialogAction::ContinueWithTemporaryDefaults),
            "review 039 N1: this label would falsely imply the user's session was lost"
        );
    }

    #[test]
    fn incompatible_shows_exit_and_continue_but_not_reset() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::Incompatible {
                schema: "session".into(),
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
        assert!(
            dialog.body.to_lowercase().contains("will not be saved"),
            "F28: the dialog must state that changes will not persist"
        );
    }

    #[test]
    fn corrupt_shows_continue_and_reset_but_not_exit() {
        let view = SessionRecoveryView::from_resolution(&resolution(
            SessionRuntimeOutcome::CorruptPreserved {
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
        assert!(
            dialog.body.to_lowercase().contains("will not be saved"),
            "F28: the dialog must state that changes will not persist"
        );
    }
}
