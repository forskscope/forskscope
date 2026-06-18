//! Settings view: persistence helpers, modal dispatcher, and submodules.
//!
//! - `modal.rs`   — `SettingsModal` dialog (appearance, advanced, profiles)
//! - `profile.rs` — `AddProfileInline` form

pub mod modal;
pub mod profile;

use app_json_settings::ConfigManager;
use dioxus::prelude::*;

use crate::state::{AppSettings, Lang, Modal, Store, Theme};
use crate::ui::overlay::modals::{
    AboutModal, BatchCopyModal, BatchResultModal, CloseTabModal,
    ConfirmDirOpModal, OverwriteModal, ReloadModal, SaveAsModal, SwapModal,
};
use crate::ui::overlay::keybindings::KeyboardRefModal;
use modal::SettingsModal;

// ── Persistence ───────────────────────────────────────────────────────────────

pub fn persist(settings: &AppSettings) {
    let m: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    let _ = m.save(settings);
}

pub fn load() -> AppSettings {
    let m: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    m.load_or_default().unwrap_or_default()
}

// ── Modal dispatcher ──────────────────────────────────────────────────────────

#[component]
pub fn ModalLayer() -> Element {
    let store = use_context::<Store>();
    let modal = store.modal.read().cloned();
    match modal {
        Modal::None               => rsx! {},
        Modal::Settings           => rsx! { SettingsModal {} },
        Modal::ConfirmOverwrite(i) => rsx! { OverwriteModal    { index: i } },
        Modal::SaveAs(i, path)    => rsx! { SaveAsModal       { index: i, initial_path: path } },
        Modal::ConfirmReload(i)   => rsx! { ReloadModal       { index: i } },
        Modal::ConfirmSwap(i)     => rsx! { SwapModal         { index: i } },
        Modal::ConfirmDirOp(op)  => rsx! { ConfirmDirOpModal  { op } },
        Modal::ConfirmClose(i)   => rsx! { CloseTabModal      { index: i } },
        Modal::About             => rsx! { AboutModal         {} },
        Modal::ConfirmBatchCopy(spec) => rsx! { BatchCopyModal   { spec } },
        Modal::BatchResult(spec)      => rsx! { BatchResultModal { spec } },
        Modal::KeyboardRef        => rsx! { KeyboardRefModal {} },
    }
}

// ── Type helpers (used by modal.rs and settings tests) ───────────────────────

pub(crate) fn tv(t: Theme) -> &'static str {
    match t { Theme::Dark => "dark", Theme::Light => "light", Theme::Night => "night" }
}
pub(crate) fn tf(s: &str) -> Theme {
    match s { "light" => Theme::Light, "night" => Theme::Night, _ => Theme::Dark }
}
pub(crate) fn lv(l: Lang) -> &'static str { match l { Lang::En => "en", Lang::Ja => "ja" } }
pub(crate) fn lf(s: &str) -> Lang         { match s { "ja" => Lang::Ja, _ => Lang::En } }
