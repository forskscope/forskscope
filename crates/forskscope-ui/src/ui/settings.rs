//! Settings dialog, persist/load helpers, and the ModalLayer dispatcher (RFC-009, RFC-046, RFC-057).

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

// ── Settings modal ────────────────────────────────────────────────────────────

#[component]
fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cur  = store.settings.read().clone();
    // Progressive disclosure state for New Profile form (RFC-057).
    let mut show_new_profile = use_signal(|| false);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Settings",
            tabindex: "-1",
            onclick: move |_| store.modal.set(Modal::None),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                    store.modal.set(Modal::None);
                }
            },
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                // Header row: title + About button (RFC-057).
                div { class: "modal-header-row",
                    h2 { id: "settings-title", {t(lang, "Settings")} }
                    button {
                        class: "about-btn",
                        title: "About ForskScope",
                        onclick: move |_| store.modal.set(Modal::About),
                        "ℹ"
                    }
                }

                // ── Appearance ────────────────────────────────────
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    select {
                        value: tv(cur.theme),
                        onchange: move |e| { store.settings.write().theme = tf(&e.value()); persist(&store.settings.read()); },
                        option { value: "dark",  "Dark"  }
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
                        option { value: "ja", "日本語"   }
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
                    span { {t(lang, "Context lines")} }
                    select {
                        value: "{cur.context_lines}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<usize>() {
                                store.settings.write().context_lines = n;
                                persist(&store.settings.read());
                            }
                        },
                        option { value: "0",  {t(lang, "0 (show all)")} }
                        option { value: "3",  {t(lang, "3 (default)")} }
                        option { value: "5",  "5"            }
                        option { value: "10", "10"           }
                    }
                }

                // ── Ignore patterns (RFC-056) ─────────────────────
                div { class: "field",
                    span { {t(lang, "Ignore file extensions")} }
                    input {
                        r#type: "text",
                        placeholder: t(lang, "o, class, tmp  (comma separated, no dot needed)"),
                        value: "{cur.ignore_extensions}",
                        oninput: move |e| {
                            store.settings.write().ignore_extensions = e.value();
                            persist(&store.settings.read());
                        }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Ignore directory names")} }
                    input {
                        r#type: "text",
                        placeholder: t(lang, "target, node_modules, *.cache  (* wildcard allowed)"),
                        value: "{cur.ignore_dirs}",
                        oninput: move |e| {
                            store.settings.write().ignore_dirs = e.value();
                            persist(&store.settings.read());
                        }
                    }
                }

                // ── Compare profiles ──────────────────────────────
                div { class: "field",
                    span { {t(lang, "Compare profiles")} }
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
                                        title: t(lang, "Delete profile"),
                                        onclick: move |_| crate::state::remove_profile(&mut store, i),
                                        "×"
                                    }
                                }
                            }
                        }
                        // New profile: disclosed on demand (RFC-057).
                        if !*show_new_profile.read() {
                            button {
                                class: "new-profile-btn",
                                onclick: move |_| show_new_profile.set(true),
                                {t(lang, "+ New profile")}
                            }
                        } else {
                            AddProfileInline {
                                on_done: move |_| show_new_profile.set(false),
                            }
                        }
                    }
                }

                div { class: "actions",
                    button {
                        autofocus: true,
                        onclick: move |_| store.modal.set(Modal::None),
                        {t(lang, "Close")}
                    }
                }
            }
        }
    }
}

// ── Add-profile inline form (RFC-057: hidden by default) ──────────────────────

#[component]
fn AddProfileInline(on_done: EventHandler<()>) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut name        = use_signal(String::new);
    #[allow(unused_mut)] let mut ignore_ws   = use_signal(|| false);
    #[allow(unused_mut)] let mut ignore_case = use_signal(|| false);
    #[allow(unused_mut)] let mut algorithm: Signal<crate::state::DiffAlgorithmSetting> =
        use_signal(Default::default);

    rsx! {
        div { class: "add-profile-form",
            input {
                placeholder: t(lang, "Profile name"),
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                style: "flex:1;"
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_ws.read(),
                    onchange: move |e| ignore_ws.set(e.checked()) }
                span { {t(lang, "Ignore WS")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_case.read(),
                    onchange: move |e| ignore_case.set(e.checked()) }
                span { {t(lang, "Ignore case")} }
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
                        on_done.call(());
                    }
                },
                {t(lang, "Add")}
            }
            button {
                onclick: move |_| on_done.call(()),
                {t(lang, "Cancel")}
            }
        }
    }
}

fn tv(t: Theme) -> &'static str { match t { Theme::Dark => "dark", Theme::Light => "light", Theme::Night => "night" } }
fn tf(s: &str) -> Theme { match s { "light" => Theme::Light, "night" => Theme::Night, _ => Theme::Dark } }
fn lv(l: Lang)  -> &'static str { match l { Lang::En => "en", Lang::Ja => "ja" } }
fn lf(s: &str)  -> Lang { match s { "ja" => Lang::Ja, _ => Lang::En } }
