//! Keyboard reference modal (RFC-030): a compact table of all shortcuts.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store};

#[component]
pub fn KeyboardRefModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Keyboard shortcuts",
            tabindex: "-1",
            onclick: move |_| store.modal.set(Modal::None),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                    store.modal.set(Modal::None);
                }
            },
            div { class: "modal modal-wide", onclick: move |e| e.stop_propagation(),
                h2 { "Keyboard shortcuts" }
                div { class: "kb-section",
                    h3 { "Diff view" }
                    div { class: "kb-table",
                        KbRow { keys: "F7 / F8",       desc: "Previous / next change" }
                        KbRow { keys: "Enter",          desc: "Apply focused change (left → right)" }
                        KbRow { keys: "Ctrl + Z",       desc: "Undo last merge" }
                        KbRow { keys: "Ctrl + Y",       desc: "Redo last undone merge" }
                        KbRow { keys: "Ctrl + S",       desc: "Save merge result" }
                        KbRow { keys: "Ctrl + F",       desc: "Open / close inline search" }
                    }
                }
                div { class: "kb-section",
                    h3 { "Navigation" }
                    div { class: "kb-table",
                        KbRow { keys: "↑ / ↓",         desc: "Move focus in explorer list" }
                        KbRow { keys: "Enter",          desc: "Open directory / compare same-name file" }
                        KbRow { keys: "Space",          desc: "Select file as comparison candidate" }
                        KbRow { keys: "Alt + ↑",        desc: "Go up one directory" }
                        KbRow { keys: "Alt + ↑",        desc: "Go up one directory" }
                        KbRow { keys: "◀ / ▶ buttons",  desc: "Back / forward directory history" }
                    }
                }
                div { class: "kb-section",
                    h3 { "App" }
                    div { class: "kb-table",
                        KbRow { keys: "Ctrl + W",       desc: "Close the active comparison tab" }
                        KbRow { keys: "Ctrl + /",       desc: "This keyboard reference" }
                        KbRow { keys: "Escape",         desc: "Close modal / search bar" }
                    }
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Close")} }
                }
            }
        }
    }
}

#[component]
fn KbRow(keys: &'static str, desc: &'static str) -> Element {
    rsx! {
        div { class: "kb-row",
            kbd { class: "kb-key", "{keys}" }
            span { class: "kb-desc", "{desc}" }
        }
    }
}
