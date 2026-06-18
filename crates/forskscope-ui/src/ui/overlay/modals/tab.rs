//! Close-dirty-tab confirmation modal.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, close_tab};

#[component]
pub fn CloseTabModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang  = store.lang();
    let title = store.tabs.read().get(index).map(|t| t.title.clone()).unwrap_or_default();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Close comparison"),
            div { class: "modal",
                h2 { {t(lang, "Close comparison?")} }
                p { {format!("\"{}\" {}",
                    title,
                    t(lang, "has unsaved changes. Discard them and close?")
                )} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| { close_tab(&mut store, index); store.modal.set(Modal::None); },
                        {t(lang, "Discard and close")}
                    }
                }
            }
        }
    }
}
