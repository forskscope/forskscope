//! RFC-076 patch 6: blocking settings/session recovery dialogs.
//!
//! Renders the `dialog` half of [`SettingsRecoveryView`]/[`SessionRecoveryView`]
//! — the `Incompatible`/`CorruptPreserved`/`Migrated(Failed)` cases; see their
//! module docs in `forskscope-ui-logic`. Title and body text are rebuilt here
//! from `t()`-translated fragments spliced around the raw interpolated value
//! (schema name, version, error detail), matching this codebase's existing
//! i18n convention for dynamic content (see `BatchCopyModal`/`BatchResultModal`
//! in `../copy.rs`) rather than translating the view-model's pre-composed
//! English `body` string, which can't be looked up by exact match once it
//! contains runtime data. The view-model's `actions` list is reused as-is:
//! which actions are safe to offer for a given outcome is a tested RFC-076
//! rule (F28/F28b), not a presentation concern this file should re-derive.
//!
//! Never dismissible by Escape — see `app.rs`'s `onkeydown` handler.

use dioxus::prelude::*;

use forskscope_core::error::{AppError, AppErrorKind};
use forskscope_core::persist::schema::PersistenceCommitError;
use forskscope_core::persist::schema::session::runtime::{
    MigrationCommitOutcome as SessionMigrationCommitOutcome, SessionRuntimeOutcome,
    SessionRuntimeResolution,
};
use forskscope_core::persist::schema::settings::runtime::{
    MigrationCommitOutcome as SettingsMigrationCommitOutcome, SettingsRuntimeOutcome,
    SettingsRuntimeResolution,
};
use forskscope_ui_logic::{
    SessionRecoveryDialogAction, SessionRecoveryView, SettingsRecoveryDialogAction,
    SettingsRecoveryView, session_recovery_action_label, settings_recovery_action_label,
};

use crate::i18n::t;
use crate::state::{AppSettings, Lang, Store, advance_recovery_queue};

/// F99/C10: a friendly toast message for a `PersistenceCommitError`,
/// instead of its raw `Display` text — this is the settings/session
/// "reset and back up the original" failure path, reached only when the
/// user's configuration is already broken, which makes a raw
/// `No such file or directory (os error 2)` the worst possible moment to
/// show one.
///
/// `PersistenceCommitError` is deliberately not `CoreError` (see its own
/// doc comment: its two variants don't map onto `CoreError`'s IO/document
/// shape), so `AppError::from_core` does not apply here. `AppError::new`
/// is the sibling constructor built for exactly this — "the kind is known
/// directly" — rather than a `CoreError`. `Conflict` maps to the existing
/// `SaveConflict` kind (closest match: the file changed on disk since it
/// was read) and `Io` to `FileWriteFailed`.
///
/// Only `message.short` is used, not `.detail`: `UserMessage`'s own doc
/// says `short` fits a toast and `detail` fits a dialog body, and
/// `store.notify` only ever shows a toast here — there is no dialog
/// surface to put `detail` in without inventing one, which is out of this
/// handoff's scope.
fn recovery_failure_message(e: &PersistenceCommitError) -> String {
    let kind = match e {
        PersistenceCommitError::Conflict => AppErrorKind::SaveConflict,
        PersistenceCommitError::Io(_) => AppErrorKind::FileWriteFailed,
    };
    AppError::new(kind, e.to_string()).message.short
}

// ── Settings ────────────────────────────────────────────────────────────────

#[component]
pub fn SettingsRecoveryModal(resolution: SettingsRuntimeResolution) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let view = SettingsRecoveryView::from_resolution(&resolution);
    let Some(dialog) = view.dialog else {
        return rsx! {};
    };
    let (title, body) = settings_dialog_text(lang, &resolution.outcome);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}",
            div { class: "modal",
                h2 { "{title}" }
                p { "{body}" }
                div { class: "actions",
                    for (i , action) in dialog.actions.iter().copied().enumerate() {
                        button {
                            key: "{action:?}",
                            autofocus: i == 0,
                            onclick: {
                                let resolution = resolution.clone();
                                move |_| settings_recovery_action(&mut store, &resolution, action)
                            },
                            {t(lang, settings_recovery_action_label(action))}
                        }
                    }
                }
            }
        }
    }
}

fn settings_dialog_text(lang: Lang, outcome: &SettingsRuntimeOutcome) -> (String, String) {
    match outcome {
        SettingsRuntimeOutcome::Incompatible { schema, version } => (
            t(lang, "Settings file is from a newer version"),
            format!(
                "{} \"{}\" {} {}, {} {}",
                t(lang, "This settings file uses"),
                schema,
                t(lang, "schema version"),
                version,
                t(
                    lang,
                    "which this version of ForskScope does not understand. The file has not been modified."
                ),
                t(lang, "Changes you make this session will not be saved."),
            ),
        ),
        SettingsRuntimeOutcome::CorruptPreserved { detail } => (
            t(lang, "Settings file could not be read"),
            format!(
                "{}: {}. {}",
                t(
                    lang,
                    "The settings file is preserved but could not be parsed"
                ),
                detail,
                t(
                    lang,
                    "Changes you make this session will not be saved unless you reset it."
                ),
            ),
        ),
        SettingsRuntimeOutcome::Migrated(SettingsMigrationCommitOutcome::Failed { detail }) => (
            t(lang, "Settings could not be upgraded"),
            format!(
                "{} ({}). {}",
                t(
                    lang,
                    "Your settings were read and are in use for this session, but they could not be saved in the new format"
                ),
                detail,
                t(lang, "Changes will not be saved until this is resolved."),
            ),
        ),
        // Fresh/Current/Migrated(Committed|DeferredByConflict) never produce a
        // dialog — `view.dialog` is `None` and the caller returns early.
        _ => (String::new(), String::new()),
    }
}

fn settings_recovery_action(
    store: &mut Store,
    resolution: &SettingsRuntimeResolution,
    action: SettingsRecoveryDialogAction,
) {
    match action {
        SettingsRecoveryDialogAction::Exit => {
            dioxus_desktop::window().close();
        }
        SettingsRecoveryDialogAction::ContinueWithTemporaryDefaults
        | SettingsRecoveryDialogAction::ContinueWithoutSaving => {
            advance_recovery_queue(store);
        }
        SettingsRecoveryDialogAction::ResetAndBackupOriginal => {
            if let Some(raw) = &resolution.raw_bytes {
                match crate::ui::view::settings::reset_settings_with_backup(&resolution.value, raw)
                {
                    Ok(()) => {
                        store.settings_v2_base.set(resolution.value.clone());
                        store.settings.set(AppSettings::from_v2(&resolution.value));
                        store.settings_write_disabled.set(false);
                    }
                    Err(e) => store.notify(recovery_failure_message(&e)),
                }
            }
            advance_recovery_queue(store);
        }
    }
}

// ── Session ─────────────────────────────────────────────────────────────────

#[component]
pub fn SessionRecoveryModal(resolution: SessionRuntimeResolution) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let view = SessionRecoveryView::from_resolution(&resolution);
    let Some(dialog) = view.dialog else {
        return rsx! {};
    };
    let (title, body) = session_dialog_text(lang, &resolution.outcome);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}",
            div { class: "modal",
                h2 { "{title}" }
                p { "{body}" }
                div { class: "actions",
                    for (i , action) in dialog.actions.iter().copied().enumerate() {
                        button {
                            key: "{action:?}",
                            autofocus: i == 0,
                            onclick: {
                                let resolution = resolution.clone();
                                move |_| session_recovery_action(&mut store, &resolution, action)
                            },
                            {t(lang, session_recovery_action_label(action))}
                        }
                    }
                }
            }
        }
    }
}

fn session_dialog_text(lang: Lang, outcome: &SessionRuntimeOutcome) -> (String, String) {
    match outcome {
        SessionRuntimeOutcome::Incompatible { schema, version } => (
            t(lang, "Session file is from a newer version"),
            format!(
                "{} \"{}\" {} {}, {} {}",
                t(lang, "This session file uses"),
                schema,
                t(lang, "schema version"),
                version,
                t(
                    lang,
                    "which this version of ForskScope does not understand. The file has not been modified."
                ),
                t(lang, "Changes you make this session will not be saved."),
            ),
        ),
        SessionRuntimeOutcome::CorruptPreserved { detail } => (
            t(lang, "Session file could not be read"),
            format!(
                "{}: {}. {}",
                t(
                    lang,
                    "The session file is preserved but could not be parsed"
                ),
                detail,
                t(
                    lang,
                    "Changes you make this session will not be saved unless you reset it."
                ),
            ),
        ),
        SessionRuntimeOutcome::Migrated(SessionMigrationCommitOutcome::Failed { detail }) => (
            t(lang, "Session could not be upgraded"),
            format!(
                "{} ({}). {}",
                t(
                    lang,
                    "Your session was read and is in use for this run, but it could not be saved in the new format"
                ),
                detail,
                t(lang, "Changes will not be saved until this is resolved."),
            ),
        ),
        _ => (String::new(), String::new()),
    }
}

fn session_recovery_action(
    store: &mut Store,
    resolution: &SessionRuntimeResolution,
    action: SessionRecoveryDialogAction,
) {
    match action {
        SessionRecoveryDialogAction::Exit => {
            dioxus_desktop::window().close();
        }
        SessionRecoveryDialogAction::ContinueWithTemporaryDefaults
        | SessionRecoveryDialogAction::ContinueWithoutSaving => {
            advance_recovery_queue(store);
        }
        SessionRecoveryDialogAction::ResetAndBackupOriginal => {
            if let Some(raw) = &resolution.raw_bytes {
                match crate::state::session::reset_session_with_backup(&resolution.value, raw) {
                    Ok(()) => {
                        store.session_write_disabled.set(false);
                    }
                    Err(e) => store.notify(recovery_failure_message(&e)),
                }
            }
            advance_recovery_queue(store);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ConfigRootOverrideGuard, with_test_store};
    use forskscope_core::persist::schema::session::PersistedSession;
    use forskscope_core::persist::schema::settings::PersistedSettings;

    #[test]
    fn recovery_failure_message_maps_conflict_to_a_friendly_string_not_the_raw_one() {
        let e = PersistenceCommitError::Conflict;
        let msg = recovery_failure_message(&e);
        assert_ne!(
            msg,
            e.to_string(),
            "the raw PersistenceCommitError::Conflict text must not reach the user"
        );
        assert!(!msg.is_empty());
    }

    #[test]
    fn recovery_failure_message_maps_io_to_a_friendly_string_not_the_raw_one() {
        let e = PersistenceCommitError::Io("No such file or directory (os error 2)".into());
        let msg = recovery_failure_message(&e);
        assert_ne!(
            msg,
            e.to_string(),
            "the raw OS error text must not reach the user"
        );
        assert!(!msg.is_empty());
    }

    // F99/C10 falsification: reverting `settings_recovery_action`'s
    // `Err(e)` arm back to `store.notify(e.to_string())` must fail this
    // test. It drives the real function end to end against a genuine
    // `PersistenceCommitError::Conflict` — the overridden config
    // directory holds different bytes than `raw_bytes` claims were read,
    // exactly `verify_unchanged`'s real failure path — not a synthetic
    // error value, and asserts the toast the user actually sees is the
    // mapped message, not the raw one.
    #[test]
    fn settings_reset_conflict_notifies_the_mapped_message_not_the_raw_one() {
        let dir = std::env::temp_dir().join(format!(
            "fsk-recovery-settings-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let settings_dir = dir.join("forskscope");
        std::fs::create_dir_all(&settings_dir).unwrap();
        std::fs::write(settings_dir.join("settings.json"), b"{\"on_disk\":true}").unwrap();
        let _guard = ConfigRootOverrideGuard::set(dir.clone());

        let resolution = SettingsRuntimeResolution {
            value: PersistedSettings::default(),
            write_disabled: true,
            outcome: SettingsRuntimeOutcome::Fresh,
            raw_bytes: Some(b"{\"read_earlier\":true}".to_vec()),
        };

        with_test_store(|store| {
            settings_recovery_action(
                store,
                &resolution,
                SettingsRecoveryDialogAction::ResetAndBackupOriginal,
            );
            let toast = store
                .toast
                .read()
                .clone()
                .expect("a toast must be shown on a genuine reset failure");
            assert_ne!(
                toast.message,
                PersistenceCommitError::Conflict.to_string(),
                "the raw PersistenceCommitError text must not reach the user"
            );
            assert!(!toast.message.is_empty());
        });

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Same shape as the settings test above, for `session_recovery_action`
    // — a separate call site sharing `recovery_failure_message`, and the
    // handoff named both `recovery.rs:142` and `:252` as needing the fix.
    #[test]
    fn session_reset_conflict_notifies_the_mapped_message_not_the_raw_one() {
        let dir = std::env::temp_dir().join(format!(
            "fsk-recovery-session-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let session_dir = dir.join("forskscope");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.json"), b"{\"on_disk\":true}").unwrap();
        let _guard = ConfigRootOverrideGuard::set(dir.clone());

        let resolution = SessionRuntimeResolution {
            value: PersistedSession::default(),
            write_disabled: true,
            outcome: SessionRuntimeOutcome::Fresh,
            raw_bytes: Some(b"{\"read_earlier\":true}".to_vec()),
        };

        with_test_store(|store| {
            session_recovery_action(
                store,
                &resolution,
                SessionRecoveryDialogAction::ResetAndBackupOriginal,
            );
            let toast = store
                .toast
                .read()
                .clone()
                .expect("a toast must be shown on a genuine reset failure");
            assert_ne!(
                toast.message,
                PersistenceCommitError::Conflict.to_string(),
                "the raw PersistenceCommitError text must not reach the user"
            );
            assert!(!toast.message.is_empty());
        });

        let _ = std::fs::remove_dir_all(&dir);
    }
}
