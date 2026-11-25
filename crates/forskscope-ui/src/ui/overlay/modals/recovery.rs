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

use forskscope_compare::error::AppErrorKind;
use forskscope_desktop::persist::schema::PersistenceCommitError;
use forskscope_desktop::persist::schema::session::runtime::{
    MigrationCommitOutcome as SessionMigrationCommitOutcome, SessionRuntimeOutcome,
    SessionRuntimeResolution,
};
use forskscope_desktop::persist::schema::settings::runtime::{
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
/// Only the short text is used, not `UserMessage`'s `.detail`: `short`
/// fits a toast and `detail` fits a dialog body, and `store.notify` only
/// ever shows a toast here — there is no dialog surface to put `detail`
/// in without inventing one, which is out of this handoff's scope.
///
/// F238 (review 238 §1.4): never reads `AppError::new(..).message.short`
/// directly any more — that's `forskscope-compare`'s hardcoded English
/// (RFC-093 crate boundaries: that crate keeps no i18n of its own).
/// `app_error_kind_short_id` maps the kind to a catalog id instead.
fn recovery_failure_message(e: &PersistenceCommitError) -> crate::i18n::TranslatableMessage {
    let kind = match e {
        PersistenceCommitError::Conflict => AppErrorKind::SaveConflict,
        PersistenceCommitError::Io(_) => AppErrorKind::FileWriteFailed,
    };
    crate::i18n::TranslatableMessage::new(crate::ui::view::diff_actions::app_error_kind_short_id(
        kind,
    ))
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
    let (title, body_id, body_args, isolate) = settings_dialog_text(lang, &resolution.outcome);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}",
            div { class: "modal",
                h2 { "{title}" }
                p { crate::ui::view::TranslatedText { lang, id: body_id, args: body_args, isolate } }
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

type DialogBody = (
    String,
    &'static str,
    Vec<(&'static str, String)>,
    Vec<&'static str>,
);

/// RFC-101 stage B (handoff 114): each outcome is one whole-sentence
/// catalog message now, never pieces joined in English word order.
/// `settings-incompatible-schema`'s wording is TODAY's, unchanged -
/// review 232 §3.2 flags it for the owner's review (reply 202a); every
/// other id here already carries the architect-approved rewrite.
///
/// Returns the body as (id, args, isolate) rather than an already-
/// substituted `String`: `schema` and `detail` carry data from the file
/// itself (a schema name, raw error text) — review 232 §3.4 — so the
/// caller renders them through `TranslatedText`, which isolates each
/// `isolate`-named placeholder in its own `<bdi>` instead of splicing it
/// into one opaque string with no markup at all.
fn settings_dialog_text(lang: Lang, outcome: &SettingsRuntimeOutcome) -> DialogBody {
    match outcome {
        SettingsRuntimeOutcome::Incompatible { schema, version } => (
            t(lang, "settings-file-is-from-a-newer-version"),
            "settings-incompatible-schema",
            vec![("schema", schema.clone()), ("version", version.to_string())],
            vec!["schema"],
        ),
        SettingsRuntimeOutcome::CorruptPreserved { detail } => (
            t(lang, "settings-file-could-not-be-read"),
            "settings-corrupt-preserved",
            vec![("detail", detail.to_string())],
            vec!["detail"],
        ),
        SettingsRuntimeOutcome::Migrated(SettingsMigrationCommitOutcome::Failed { detail }) => (
            t(lang, "settings-could-not-be-upgraded"),
            "settings-migration-failed",
            vec![("detail", detail.clone())],
            vec!["detail"],
        ),
        // Fresh/Current/Migrated(Committed|DeferredByConflict) never produce a
        // dialog — `view.dialog` is `None` and the caller returns early.
        _ => (String::new(), "", Vec::new(), Vec::new()),
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
    let (title, body_id, body_args, isolate) = session_dialog_text(lang, &resolution.outcome);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}",
            div { class: "modal",
                h2 { "{title}" }
                p { crate::ui::view::TranslatedText { lang, id: body_id, args: body_args, isolate } }
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

/// Same reasoning as [`settings_dialog_text`]: `session-incompatible-schema`
/// keeps today's wording (flagged for the owner, reply 202a); the other
/// two carry the architect-approved rewrite. `schema`/`detail` are
/// isolated in the rendered body the same way (review 232 §3.4).
fn session_dialog_text(lang: Lang, outcome: &SessionRuntimeOutcome) -> DialogBody {
    match outcome {
        SessionRuntimeOutcome::Incompatible { schema, version } => (
            t(lang, "session-file-is-from-a-newer-version"),
            "session-incompatible-schema",
            vec![("schema", schema.clone()), ("version", version.to_string())],
            vec!["schema"],
        ),
        SessionRuntimeOutcome::CorruptPreserved { detail } => (
            t(lang, "session-file-could-not-be-read"),
            "session-corrupt-preserved",
            vec![("detail", detail.to_string())],
            vec!["detail"],
        ),
        SessionRuntimeOutcome::Migrated(SessionMigrationCommitOutcome::Failed { detail }) => (
            t(lang, "session-could-not-be-upgraded"),
            "session-migration-failed",
            vec![("detail", detail.clone())],
            vec!["detail"],
        ),
        _ => (String::new(), "", Vec::new(), Vec::new()),
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
    use forskscope_desktop::persist::schema::session::PersistedSession;
    use forskscope_desktop::persist::schema::settings::PersistedSettings;

    #[test]
    fn recovery_failure_message_maps_conflict_to_a_friendly_string_not_the_raw_one() {
        let e = PersistenceCommitError::Conflict;
        let msg = recovery_failure_message(&e).render(Lang::En);
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
        let msg = recovery_failure_message(&e).render(Lang::En);
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
            let rendered = toast.message.render(Lang::En);
            assert_ne!(
                rendered,
                PersistenceCommitError::Conflict.to_string(),
                "the raw PersistenceCommitError text must not reach the user"
            );
            assert!(!rendered.is_empty());
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
            let rendered = toast.message.render(Lang::En);
            assert_ne!(
                rendered,
                PersistenceCommitError::Conflict.to_string(),
                "the raw PersistenceCommitError text must not reach the user"
            );
            assert!(!rendered.is_empty());
        });

        let _ = std::fs::remove_dir_all(&dir);
    }
}
