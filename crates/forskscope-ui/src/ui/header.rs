//! Global header — brand only; Explorer tab lives in the tab bar (RFC-054).
//! The About button moved to the Settings dialog header (RFC-057).

use dioxus::prelude::*;

use crate::state::{Modal, Store};

#[component]
pub fn Header() -> Element {
    let mut store = use_context::<Store>();

    rsx! {
        div { class: "header",
            span { class: "brand", "ForskScope" }
            span { class: "spacer" }
            button {
                onclick: move |_| store.modal.set(Modal::Settings),
                "Settings"
            }
            button {
                onclick: move |_| store.modal.set(Modal::KeyboardRef),
                title: "Keyboard shortcuts (Ctrl+/)", "?"
            }
        }
    }
}
