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
                    Err(e) => store.notify(e.to_string()),
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
                    Err(e) => store.notify(e.to_string()),
                }
            }
            advance_recovery_queue(store);
        }
    }
}
