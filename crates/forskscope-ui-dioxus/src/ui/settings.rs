//! Settings dialog, persist/load helpers, and the ModalLayer dispatcher (RFC-009, RFC-046).

use app_json_settings::ConfigManager;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{AppSettings, Lang, Modal, Store, Theme};
use crate::ui::modals::{AboutModal, CloseTabModal, ConfirmDirOpModal, OverwriteModal,
                         ReloadModal, SaveAsModal, SwapModal};
use crate::ui::keybindings::KeyboardRefModal;

pub fn persist(settings: &AppSettings) {
    let m: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    let _ = m.save(settings);
}

pub fn load() -> AppSettings {
    let m: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    m.load_or_default().unwrap_or_default()
}

#[component]
pub fn ModalLayer() -> Element {
    let store = use_context::<Store>();
    let modal = store.modal.read().clone();
    match modal {
        Modal::None               => rsx! {},
        Modal::Settings           => rsx! { SettingsModal {} },
        Modal::ConfirmOverwrite(i) => rsx! { OverwriteModal      { index: i } },
        Modal::SaveAs(i, path)    => rsx! { SaveAsModal         { index: i, initial_path: path } },
        Modal::ConfirmReload(i)   => rsx! { ReloadModal         { index: i } },
        Modal::ConfirmSwap(i)     => rsx! { SwapModal           { index: i } },
        Modal::ConfirmDirOp(op)  => rsx! { ConfirmDirOpModal   { op } },
        Modal::ConfirmClose(i)   => rsx! { CloseTabModal       { index: i } },
        Modal::About             => rsx! { AboutModal          {} },
        Modal::ConfirmBatchCopy(spec) => rsx! { crate::ui::modals::BatchCopyModal { spec } },
        Modal::KeyboardRef        => rsx! { KeyboardRefModal {} },
    }
}

// ─── Settings ─────────────────────────────────────────────────────────────────

#[component]
fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cur = store.settings.read().clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Settings",
            div { class: "modal",
                h2 { id: "settings-title", "Settings" }
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    select {
                        value: tv(cur.theme),
                        onchange: move |e| { store.settings.write().theme = tf(&e.value()); persist(&store.settings.read()); },
                        option { value: "dark", "Dark" }
                        option { value: "light", "Light" }
                        option { value: "night", "Night" }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Language")} }
                    select {
                        value: lv(cur.language),
                        onchange: move |e| { store.settings.write().language = lf(&e.value()); persist(&store.settings.read()); },
                        option { value: "en", "English" }
                        option { value: "ja", "日本語" }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Diff font size")} }
                    input {
                        r#type: "number", min: "8", max: "32",
                        value: "{cur.diff_font_size}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<u32>() {
                                store.settings.write().diff_font_size = n.clamp(8, 32);
                                persist(&store.settings.read());
                            }
                        }
                    }
                }
                div { class: "field",
                    span { "Context lines" }
                    select {
                        value: "{cur.context_lines}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<usize>() {
                                store.settings.write().context_lines = n;
                                persist(&store.settings.read());
                            }
                        },
                        option { value: "0",  "0 (show all)" }
                        option { value: "3",  "3 (default)"  }
                        option { value: "5",  "5"            }
                        option { value: "10", "10"           }
                    }
                }
                div { class: "field",
                    span { "Compare profiles" }
                    div { class: "profile-list",
                        for (i, p) in cur.profiles.iter().enumerate() {
                            div { class: if cur.active_profile == i { "profile-row active" } else { "profile-row" },
                                span {
                                    class: "profile-name",
                                    onclick: move |_| {
                                        store.settings.write().active_profile = i;
                                        persist(&store.settings.read());
                                    },
                                    if cur.active_profile == i { "▸ " } else { "  " }
                                    "{p.name}"
                                }
                                if !p.built_in {
                                    button {
                                        class: "profile-delete",
                                        title: "Delete profile",
                                        onclick: move |_| crate::state::remove_profile(&mut store, i),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "field",
                    span { "New profile" }
                    AddProfileInline {}
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Close")} }
                }
            }
        }
    }
}

/// Inline "Add profile" sub-form inside the Settings modal.
#[component]
fn AddProfileInline() -> Element {
    let mut store = use_context::<Store>();
    let mut name        = use_signal(String::new);
    #[allow(unused_mut)] let mut ignore_ws   = use_signal(|| false);
    #[allow(unused_mut)] let mut ignore_case = use_signal(|| false);
    #[allow(unused_mut)] let mut algorithm:  Signal<crate::state::DiffAlgorithmSetting> = use_signal(Default::default);
    rsx! {
        div { class: "add-profile-form",
            input { placeholder: "Profile name", value: "{name}",
                oninput: move |e| name.set(e.value()), style: "flex:1;" }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_ws.read(),
                    onchange: move |e| ignore_ws.set(e.checked()) }
                span { "Ignore WS" }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_case.read(),
                    onchange: move |e| ignore_case.set(e.checked()) }
                span { "Ignore case" }
            }
            select {
                onchange: move |e| {
                    algorithm.set(match e.value().as_str() {
                        "patience"  => crate::state::DiffAlgorithmSetting::Patience,
                        "histogram" => crate::state::DiffAlgorithmSetting::Histogram,
                        _           => crate::state::DiffAlgorithmSetting::Myers,
                    });
                },
                option { value: "myers",     "Myers"     }
                option { value: "patience",  "Patience"  }
                option { value: "histogram", "Histogram" }
            }
            button {
                disabled: name.read().trim().is_empty(),
                onclick: move |_| {
                    let n = name.read().trim().to_string();
                    if !n.is_empty() {
                        crate::state::add_profile(&mut store, n, *ignore_ws.read(),
                            *ignore_case.read(), *algorithm.read());
                        name.set(String::new());
                        ignore_ws.set(false); ignore_case.set(false);
                        algorithm.set(Default::default());
                    }
                },
                "Add"
            }
        }
    }
}

fn tv(t: Theme) -> &'static str { match t { Theme::Dark => "dark", Theme::Light => "light", Theme::Night => "night" } }
fn tf(s: &str) -> Theme { match s { "light" => Theme::Light, "night" => Theme::Night, _ => Theme::Dark } }
fn lv(l: Lang)  -> &'static str { match l { Lang::En => "en", Lang::Ja => "ja" } }
fn lf(s: &str)  -> Lang { match s { "ja" => Lang::Ja, _ => Lang::En } }
