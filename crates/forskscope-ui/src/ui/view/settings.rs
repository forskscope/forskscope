//! Settings view: persistence helpers, modal dispatcher, and submodules.
//!
//! - `modal.rs`   — `SettingsModal` dialog (appearance, advanced, profiles)
//! - `profile.rs` — `AddProfileInline` form

pub mod modal;
pub mod profile;

use dioxus::prelude::*;
use forskscope_core::persist::v2::settings::runtime::{
    SettingsRuntimeResolution, resolve_and_commit,
};
use forskscope_core::persist::v2::settings::{PersistedSettingsV2, SettingsRepository};
use forskscope_ui_logic::SettingsRecoveryView;

use crate::state::{AppSettings, Lang, Modal, Notice, Store, Theme, config_file_path};
use crate::ui::overlay::keybindings::KeyboardRefModal;
use crate::ui::overlay::modals::{
    AboutModal, BatchCopyModal, BatchResultModal, CloseTabModal, ConfirmDirOpModal, OverwriteModal,
    ReloadModal, SaveAsModal, SwapModal,
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
pub fn build_save_payload(
    settings: &AppSettings,
    base: &PersistedSettingsV2,
) -> PersistedSettingsV2 {
    settings.merge_into_v2(base)
}

/// Writes `payload` via `repo` — the exact repository call `persist` makes,
/// exposed for direct testing (handoff §6: "targeted tests proving the
/// actual UI startup and save functions use the new repositories").
pub fn persist_settings(payload: &PersistedSettingsV2, repo: &SettingsRepository) {
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

/// Maps a settings load's resolution to a one-time startup toast, if any —
/// an informational notice for a durably-committed migration, or a warning
/// for an outcome that leaves writes disabled. Reuses `forskscope-ui-logic`'s
/// already-tested recovery copy rather than duplicating message text; the
/// full recovery dialog (Exit/Continue/Reset actions) is patch 5's job.
pub fn recovery_notice(resolution: &SettingsRuntimeResolution) -> Option<Notice> {
    let view = SettingsRecoveryView::from_resolution(resolution);
    if let Some(notice) = view.migration_notice {
        return Some(Notice::success(notice.message));
    }
    if let Some(dialog) = view.dialog {
        return Some(Notice::error(dialog.body));
    }
    None
}

// ── Modal dispatcher ──────────────────────────────────────────────────────────

#[component]
pub fn ModalLayer() -> Element {
    let store = use_context::<Store>();
    let modal = store.modal.read().cloned();
    match modal {
        Modal::None => rsx! {},
        Modal::Settings => rsx! { SettingsModal {} },
        Modal::ConfirmOverwrite(i) => rsx! { OverwriteModal    { index: i } },
        Modal::SaveAs(i, path) => rsx! { SaveAsModal       { index: i, initial_path: path } },
        Modal::ConfirmReload(i) => rsx! { ReloadModal       { index: i } },
        Modal::ConfirmSwap(i) => rsx! { SwapModal         { index: i } },
        Modal::ConfirmDirOp(op) => rsx! { ConfirmDirOpModal  { op } },
        Modal::ConfirmClose(i) => rsx! { CloseTabModal      { index: i } },
        Modal::About => rsx! { AboutModal         {} },
        Modal::ConfirmBatchCopy(spec) => rsx! { BatchCopyModal   { spec } },
        Modal::BatchResult(spec) => rsx! { BatchResultModal { spec } },
        Modal::KeyboardRef => rsx! { KeyboardRefModal {} },
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
