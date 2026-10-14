//! Close-dirty-tab confirmation modal.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, close_tab};

#[component]
pub fn CloseTabModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = store
        .tabs
        .read()
        .get(index)
        .map(|t| t.title.clone())
        .unwrap_or_default();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Close comparison"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "Close comparison?")} }
                p { {format!("\"{}\" {}",
                    title,
                    t(lang, "has unsaved changes. Discard them and close?")
                )} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            close_tab(&mut store, index);
                            store.modal.set(Modal::None);
                            // RFC-087: "focus moves to the tab that became
                            // selected" applies here too. Safe to call from
                            // this about-to-unmount modal because
                            // `focus_handle` (`keyboard.rs`) uses
                            // `spawn_forever`, which is not tied to this
                            // scope's lifecycle — see its doc for why a
                            // plain `spawn` here lost the race.
                            store.focus_active_tab();
                        },
                        {t(lang, "Discard and close")}
                    }
                }
            }
        }
    }
}
