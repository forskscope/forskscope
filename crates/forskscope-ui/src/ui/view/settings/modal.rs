//! Settings modal dialog: appearance, advanced options, and compare profiles
//! (RFC-009, RFC-057, RFC-063 C6).

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store};
use super::{lf, lv, tf, tv};
use super::profile::AddProfileInline;

#[component]
pub fn SettingsModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let cur  = store.settings.read().cloned();

    let mut show_new_profile = use_signal(|| false);
    // Progressive disclosure: Advanced hidden by default (RFC-063 C6).
    let mut show_advanced    = use_signal(|| false);

    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Settings"),
            tabindex: "-1",
            onclick: move |_| store.modal.set(Modal::None),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                    e.stop_propagation(); // RFC-060 W1: prevent bubbling to app root
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

                // ── Appearance ────────────────────────────────────────────────
                div { class: "field",
                    span { {t(lang, "Theme")} }
                    select {
                        value: tv(cur.theme),
                        onchange: move |e| {
                            store.settings.write().theme = tf(&e.value());
                            super::persist(&store.settings.read());
                        },
                        option { value: "dark",  "Dark"  }
                        option { value: "light", "Light" }
                        option { value: "night", "Night" }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Language")} }
                    select {
                        value: lv(cur.language),
                        onchange: move |e| {
                            store.settings.write().language = lf(&e.value());
                            super::persist(&store.settings.read());
                        },
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
                                super::persist(&store.settings.read());
                            }
                        }
                    }
                }
                div { class: "field",
                    span { {t(lang, "Diff font family")} }
                    select {
                        value: match cur.diff_font_family {
                            crate::state::DiffFontFamily::Monospace  => "monospace",
                            crate::state::DiffFontFamily::SansSerif  => "sans-serif",
                            crate::state::DiffFontFamily::Serif      => "serif",
                            crate::state::DiffFontFamily::CourierNew => "courier-new",
                            crate::state::DiffFontFamily::Consolas   => "consolas",
                        },
                        onchange: move |e| {
                            use crate::state::DiffFontFamily;
                            let ff = match e.value().as_str() {
                                "sans-serif"  => DiffFontFamily::SansSerif,
                                "serif"       => DiffFontFamily::Serif,
                                "courier-new" => DiffFontFamily::CourierNew,
                                "consolas"    => DiffFontFamily::Consolas,
                                _             => DiffFontFamily::Monospace,
                            };
                            store.settings.write().diff_font_family = ff;
                            super::persist(&store.settings.read());
                        },
                        option { value: "monospace",  {t(lang, "Monospace (default)")} }
                        option { value: "sans-serif",  {t(lang, "Sans-serif")} }
                        option { value: "serif",       {t(lang, "Serif")} }
                        option { value: "courier-new", "Courier New" }
                        option { value: "consolas",    "Consolas / Menlo" }
                    }
                }

                // ── Advanced disclosure toggle ─────────────────────────────────
                button {
                    class: "advanced-toggle",
                    onclick: move |_| { let v = *show_advanced.read(); show_advanced.set(!v); },
                    if *show_advanced.read() { "▾ " {t(lang, "Hide advanced")} }
                    else                     { "▸ " {t(lang, "Advanced")} }
                }

                if *show_advanced.read() {
                    div { class: "field",
                        span { {t(lang, "Enable binary comparison")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.enable_binary_comparison,
                            title: t(lang, "When off, binary files cannot be compared and are shown as non-actionable in the Explorer."),
                            onchange: move |e| {
                                store.settings.write().enable_binary_comparison = e.checked();
                                super::persist(&store.settings.read());
                            }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Explorer layout")} }
                        select {
                            value: if cur.explorer_compact { "compact" } else { "aligned" },
                            onchange: move |e| {
                                store.settings.write().explorer_compact = e.value() == "compact";
                                super::persist(&store.settings.read());
                            },
                            option { value: "aligned", {t(lang, "Aligned (default)")} }
                            option { value: "compact", {t(lang, "Compact (independent panes)")} }
                        }
                    }
                    div { class: "field",
                        span { {t(lang, "Remember Explorer directories")} }
                        input {
                            r#type: "checkbox",
                            checked: cur.remember_explorer_dirs,
                            title: t(lang, "When on, the Explorer reopens the last directory shown in each pane. When off, it always starts at your home directory."),
                            onchange: move |e| {
                                let on = e.checked();
                                {
                                    let mut s = store.settings.write();
                                    s.remember_explorer_dirs = on;
                                    // Turning the feature off clears any stored locations so
                                    // they are not silently retained on disk.
                                    if !on {
                                        s.last_left_dir = None;
                                        s.last_right_dir = None;
                                    }
                                }
                                super::persist(&store.settings.read());
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
                                    super::persist(&store.settings.read());
                                }
                            },
                            option { value: "0",  {t(lang, "0 (show all)")} }
                            option { value: "3",  {t(lang, "3 (default)")} }
                            option { value: "5",  "5"  }
                            option { value: "10", "10" }
                        }
                    }

                    // ── Ignore patterns (RFC-056) ─────────────────────────────
                    div { class: "field",
                        span { {t(lang, "Ignore file extensions")} }
                        input {
                            r#type: "text",
                            placeholder: t(lang, "o, class, tmp  (comma separated, no dot needed)"),
                            value: "{cur.ignore_extensions}",
                            oninput: move |e| {
                                store.settings.write().ignore_extensions = e.value();
                                super::persist(&store.settings.read());
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
                                super::persist(&store.settings.read());
                            }
                        }
                    }

                    // ── Compare profiles ──────────────────────────────────────
                    div { class: "field",
                        span { {t(lang, "Compare profiles")} }
                        div { class: "profile-list",
                            for (i, p) in cur.profiles.iter().enumerate() {
                                div {
                                    class: if cur.active_profile == i { "profile-row active" } else { "profile-row" },
                                    span {
                                        class: "profile-name",
                                        onclick: move |_| {
                                            store.settings.write().active_profile = i;
                                            super::persist(&store.settings.read());
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
                            if !*show_new_profile.read() {
                                button {
                                    class: "new-profile-btn",
                                    onclick: move |_| show_new_profile.set(true),
                                    {t(lang, "+ New profile")}
                                }
                            } else {
                                AddProfileInline { on_done: move |_| show_new_profile.set(false) }
                            }
                        }
                    }
                } // end show_advanced

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
