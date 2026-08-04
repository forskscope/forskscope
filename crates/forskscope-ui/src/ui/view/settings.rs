//! Settings view: persistence helpers, modal dispatcher, and submodules.
//!
//! - `modal.rs`   — `SettingsModal` dialog (appearance, advanced, profiles)
//! - `profile.rs` — `AddProfileInline` form

pub mod modal;
pub mod profile;

use dioxus::prelude::*;
use forskscope_core::persist::schema::PersistenceCommitError;
use forskscope_core::persist::schema::settings::runtime::{
    SettingsRuntimeResolution, resolve_and_commit,
};
use forskscope_core::persist::schema::settings::{PersistedSettings, SettingsRepository};
use forskscope_ui_logic::SettingsRecoveryView;

use crate::state::{AppSettings, Lang, Modal, Notice, Store, Theme, config_file_path};
use crate::ui::overlay::keybindings::KeyboardRefModal;
use crate::ui::overlay::modals::{
    AboutModal, BatchCopyModal, BatchResultModal, CloseTabModal, ConfirmDirOpModal, OverwriteModal,
    ReloadModal, SaveAsModal, SessionRecoveryModal, SettingsRecoveryModal, SwapModal,
};
use modal::SettingsModal;

#[cfg(test)]
mod tests;

// ── Persistence (RFC-076) ─────────────────────────────────────────────────────

fn repository() -> SettingsRepository {
    SettingsRepository::new(config_file_path("settings.json"))
}

/// Merges `store.settings`'s UI-editable fields onto the cached canonical
/// value and writes it, unless `store.settings_write_disabled` is set — a
/// future/corrupt/unwritable source this run could not establish is safe to
/// overwrite (RFC-076 "persistence_write_disabled").
pub fn persist(mut store: Store) {
    if *store.settings_write_disabled.read() {
        return;
    }
    let merged = build_save_payload(&store.settings.read(), &store.settings_v2_base.read());
    store.settings_v2_base.set(merged.clone());
    persist_settings(&merged, &repository());
}

/// The Store-independent half of [`persist`]: what gets written and where.
/// Split out so a test can exercise it against a temp-path repository
/// without needing a running Dioxus runtime to construct a `Store`.
pub fn build_save_payload(settings: &AppSettings, base: &PersistedSettings) -> PersistedSettings {
    settings.merge_into_v2(base)
}

/// Writes `payload` via `repo` — the exact repository call `persist` makes,
/// exposed for direct testing (handoff §6: "targeted tests proving the
/// actual UI startup and save functions use the new repositories").
pub fn persist_settings(payload: &PersistedSettings, repo: &SettingsRepository) {
    let _ = repo.save(payload);
}

/// Loads settings via the RFC-076 repository, durably committing any legacy
/// migration. Returns the UI-facing view alongside the full resolution, so
/// callers can decide what (if anything) to tell the user and whether
/// writes should start out disabled.
pub fn load() -> (AppSettings, SettingsRuntimeResolution) {
    load_settings(&repository())
}

/// The repository-explicit half of [`load`], exposed for direct testing.
pub fn load_settings(repo: &SettingsRepository) -> (AppSettings, SettingsRuntimeResolution) {
    let resolution = resolve_and_commit(repo);
    let settings = AppSettings::from_v2(&resolution.value);
    (settings, resolution)
}

/// Maps a settings load's resolution to a one-time startup toast: an
/// informational notice for a durably-committed migration. An outcome that
/// needs a blocking dialog (`Incompatible`/`CorruptPreserved`/
/// `Migrated(Failed)`) is no longer surfaced as a toast — see
/// [`recovery_modal`], patch 6's replacement for the interim toast this
/// function used to also produce.
pub fn recovery_notice(resolution: &SettingsRuntimeResolution) -> Option<Notice> {
    let view = SettingsRecoveryView::from_resolution(resolution);
    view.migration_notice.map(|n| Notice::success(n.message))
}

/// The blocking-dialog counterpart of [`recovery_notice`] (RFC-076 patch 6):
/// `Some` exactly when [`SettingsRecoveryView::dialog`] is set, wrapping
/// `resolution` for [`crate::ui::overlay::modals::SettingsRecoveryModal`] to
/// render and act on.
pub fn recovery_modal(resolution: &SettingsRuntimeResolution) -> Option<Modal> {
    let view = SettingsRecoveryView::from_resolution(resolution);
    view.dialog
        .is_some()
        .then(|| Modal::SettingsRecovery(resolution.clone()))
}

/// Explicit, user-confirmed reset of a `Corrupt` settings file
/// (`RecoveryDialogAction::ResetAndBackupOriginal`): backs up
/// `original_bytes` then writes `value`. Thin wrapper so the recovery modal
/// doesn't need to know the repository's on-disk path.
pub fn reset_settings_with_backup(
    value: &PersistedSettings,
    original_bytes: &[u8],
) -> Result<(), PersistenceCommitError> {
    reset_settings(value, original_bytes, &repository())
}

/// The repository-explicit half of [`reset_settings_with_backup`], exposed
/// for direct testing (same split as [`load`]/[`load_settings`]).
pub fn reset_settings(
    value: &PersistedSettings,
    original_bytes: &[u8],
    repo: &SettingsRepository,
) -> Result<(), PersistenceCommitError> {
    repo.reset_with_backup(value, original_bytes).map(|_| ())
}

// ── Modal dispatcher ──────────────────────────────────────────────────────────

#[component]
pub fn ModalLayer() -> Element {
    let store = use_context::<Store>();
    let modal = store.modal.read().cloned();
    match modal {
        Modal::None => rsx! {},
        Modal::Settings => rsx! { SettingsModal {} },
        Modal::ConfirmOverwrite(i, target) => rsx! { OverwriteModal { index: i, target } },
        Modal::SaveAs(i, path) => rsx! { SaveAsModal       { index: i, initial_path: path } },
        Modal::ConfirmReload(i) => rsx! { ReloadModal       { index: i } },
        Modal::ConfirmSwap(i) => rsx! { SwapModal         { index: i } },
        Modal::ConfirmDirOp(op) => rsx! { ConfirmDirOpModal  { op } },
        Modal::ConfirmClose(i) => rsx! { CloseTabModal      { index: i } },
        Modal::About => rsx! { AboutModal         {} },
        Modal::ConfirmBatchCopy(spec) => rsx! { BatchCopyModal   { spec } },
        Modal::BatchResult(spec) => rsx! { BatchResultModal { spec } },
        Modal::KeyboardRef => rsx! { KeyboardRefModal {} },
        Modal::SettingsRecovery(resolution) => rsx! { SettingsRecoveryModal { resolution } },
        Modal::SessionRecovery(resolution) => rsx! { SessionRecoveryModal { resolution } },
    }
}

// ── Type helpers (used by modal.rs and settings tests) ───────────────────────

pub(crate) fn tv(t: Theme) -> &'static str {
    match t {
        Theme::Dark => "dark",
        Theme::Light => "light",
        Theme::Night => "night",
    }
}
pub(crate) fn tf(s: &str) -> Theme {
    match s {
        "light" => Theme::Light,
        "night" => Theme::Night,
        _ => Theme::Dark,
    }
}
pub(crate) fn lv(l: Lang) -> &'static str {
    match l {
        Lang::En => "en",
        Lang::Ja => "ja",
    }
}
pub(crate) fn lf(s: &str) -> Lang {
    match s {
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}
