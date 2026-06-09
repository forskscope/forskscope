//! Settings dialog and safety confirm dialog (RFC-009, RFC-007).

use app_json_settings::ConfigManager;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{AppSettings, Lang, Modal, Store, Theme};
use crate::ui::diff::save_tab;

/// Persist current settings to the OS config directory.
pub fn persist(settings: &AppSettings) {
    let manager: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    let _ = manager.save(settings);
}

/// Load persisted settings, or defaults on first run.
pub fn load() -> AppSettings {
    let manager: ConfigManager<AppSettings> = ConfigManager::new().with_filename("settings.json");
    manager.load_or_default().unwrap_or_default()
}

#[component]
pub fn ModalLayer() -> Element {
    let store = use_context::<Store>();
    let modal = store.modal.read().clone();
    match modal {
        Modal::None => rsx! {},
        Modal::Settings => rsx! { SettingsModal {} },
        Modal::ConfirmOverwrite(index) => rsx! { OverwriteModal { index } },
    }
}

#[component]
fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let current = store.settings.read().clone();

    rsx! {
        div { class: "scrim",
            div { class: "modal",
                h2 { {t(lang, "Settings")} }
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    select {
                        value: theme_value(current.theme),
                        onchange: move |e| {
                            store.settings.write().theme = theme_from(&e.value());
                            persist(&store.settings.read());
                        },
                        option { value: "dark", "Dark" }
                        option { value: "light", "Light" }
                        option { value: "night", "Night" }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Language")} }
                    select {
                        value: lang_value(current.language),
                        onchange: move |e| {
                            store.settings.write().language = lang_from(&e.value());
                            persist(&store.settings.read());
                        },
                        option { value: "en", "English" }
                        option { value: "ja", "日本語" }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Diff font size")} }
                    input {
                        r#type: "number", min: "8", max: "32",
                        value: "{current.diff_font_size}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<u32>() {
                                store.settings.write().diff_font_size = n.clamp(8, 32);
                                persist(&store.settings.read());
                            }
                        }
                    }
                }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "Close")} }
                }
            }
        }
    }
}

#[component]
fn OverwriteModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim",
            div { class: "modal",
                h2 { {t(lang, "File changed on disk")} }
                p { "The target file was modified after it was loaded. Overwrite anyway?" }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| save_tab(&mut store, index, true),
                        {t(lang, "Overwrite")}
                    }
                }
            }
        }
    }
}

fn theme_value(t: Theme) -> &'static str {
    match t {
        Theme::Dark => "dark",
        Theme::Light => "light",
        Theme::Night => "night",
    }
}
fn theme_from(s: &str) -> Theme {
    match s {
        "light" => Theme::Light,
        "night" => Theme::Night,
        _ => Theme::Dark,
    }
}
fn lang_value(l: Lang) -> &'static str {
    match l {
        Lang::En => "en",
        Lang::Ja => "ja",
    }
}
fn lang_from(s: &str) -> Lang {
    match s {
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}
