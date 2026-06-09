//! Global header (RFC-003 §global controls).

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store};

#[component]
pub fn Header() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    rsx! {
        div { class: "header",
            span { class: "brand", "ForskScope" }
            span { class: "spacer" }
            button {
                onclick: move |_| store.active.set(None),
                {t(lang, "Explorer")}
            }
            button {
                onclick: move |_| store.modal.set(Modal::Settings),
                {t(lang, "Settings")}
            }
            button {
                onclick: move |_| store.modal.set(Modal::About),
                title: "About ForskScope",
                "?"
            }
        }
    }
}
