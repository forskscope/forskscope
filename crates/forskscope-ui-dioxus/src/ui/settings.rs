//! Settings dialog and safety modals with accessibility attributes (RFC-009, RFC-046).

use app_json_settings::ConfigManager;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{AppSettings, Lang, Modal, Store, Theme, reload_tab, swap_sides};
use crate::ui::diff::save_as;

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
        Modal::ConfirmOverwrite(i) => rsx! { OverwriteModal   { index: i } },
        Modal::SaveAs(i, path)    => rsx! { SaveAsModal      { index: i, initial_path: path } },
        Modal::ConfirmReload(i)   => rsx! { ReloadModal      { index: i } },
        Modal::ConfirmSwap(i)     => rsx! { SwapModal        { index: i } },
        Modal::ConfirmDirOp(op)  => rsx! { DirOpModal       { op } },
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
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Close")} }
                }
            }
        }
    }
}

// ─── Safety modals ────────────────────────────────────────────────────────────

#[component]
fn OverwriteModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "File changed on disk",
            div { class: "modal",
                h2 { {t(lang, "File changed on disk")} }
                p { "The target file was modified after it was loaded. Overwrite anyway?" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { save_tab_force(&mut store, index); }, {t(lang, "Overwrite")} }
                }
            }
        }
    }
}

#[component]
fn SaveAsModal(index: usize, initial_path: String) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut path = use_signal(|| initial_path);
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Save As",
            div { class: "modal",
                h2 { "Save As" }
                div { class: "field",
                    span { "Path" }
                    input {
                        autofocus: true,
                        value: "{path}", oninput: move |e| path.set(e.value()),
                        style: "width:100%;",
                    }
                }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        disabled: path.read().trim().is_empty(),
                        onclick: move |_| save_as(&mut store, index, path.read().clone()),
                        {t(lang, "Save")}
                    }
                }
            }
        }
    }
}

#[component]
fn ReloadModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Reload files",
            div { class: "modal",
                h2 { "Reload files?" }
                p { "Unsaved merge changes will be discarded." }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| { reload_tab(&mut store, index); store.modal.set(Modal::None); },
                        "Discard and Reload"
                    }
                }
            }
        }
    }
}

#[component]
fn SwapModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Swap sides",
            div { class: "modal",
                h2 { "Swap sides?" }
                p { "Unsaved merge changes will be discarded when sides are swapped." }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| { swap_sides(&mut store, index); store.modal.set(Modal::None); },
                        "Discard and Swap"
                    }
                }
            }
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn save_tab_force(store: &mut Store, index: usize) {
    use crate::ui::diff::save_tab;
    save_tab(store, index, true);
}

fn tv(t: Theme) -> &'static str { match t { Theme::Dark => "dark", Theme::Light => "light", Theme::Night => "night" } }
fn tf(s: &str) -> Theme { match s { "light" => Theme::Light, "night" => Theme::Night, _ => Theme::Dark } }
fn lv(l: Lang) -> &'static str { match l { Lang::En => "en", Lang::Ja => "ja" } }
fn lf(s: &str) -> Lang { match s { "ja" => Lang::Ja, _ => Lang::En } }

/// Confirm a directory file-copy operation (RFC-031 safety model for dir ops).
#[component]
fn DirOpModal(op: crate::state::DirOp) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let src  = op.src.display().to_string();
    let dst  = op.dst.display().to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Copy file",
            div { class: "modal",
                h2 { "Copy file?" }
                p { "{op.label}" }
                div { class: "field",
                    span { "From" }
                    code { class: "path-display", "{src}" }
                }
                div { class: "field",
                    span { "To" }
                    code { class: "path-display", "{dst}" }
                }
                if op.dst.exists() {
                    p { class: "notice", "Destination exists. A .bak backup will be created." }
                }
                div { class: "actions",
                    button {
                        autofocus: true,
                        onclick: move |_| store.modal.set(Modal::None),
                        {t(lang, "Cancel")}
                    }
                    button {
                        onclick: move |_| {
                            match forskscope_core::dir::copy_file(
                                &op.src, &op.dst, forskscope_core::BackupPolicy::SiblingBak
                            ) {
                                Ok(_)  => store.notify("Copied.".to_string()),
                                Err(e) => store.notify(e.to_string()),
                            }
                            store.modal.set(Modal::None);
                        },
                        "Copy"
                    }
                }
            }
        }
    }
}
