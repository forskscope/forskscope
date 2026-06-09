//! Tab bar: Explorer tab + comparison tabs with close/dirty indicators.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, close_tab};

#[component]
pub fn TabBar() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let active = *store.active.read();
    let tab_count = store.tabs.read().len();

    rsx! {
        div { class: "tabbar",
            // Permanent Explorer tab (RFC-054: always visible first entry).
            {
                let explorer_class = if active.is_none() { "tab active" } else { "tab" };
                rsx! {
                    div {
                        class: "{explorer_class}",
                        onclick: move |_| store.active.set(None),
                        span { class: "tab-title", {t(lang, "Explorer")} }
                    }
                }
            }
            // Comparison tabs.
            for i in 0..tab_count {
                TabItem { index: i, is_active: active == Some(i) }
            }
        }
    }
}

#[component]
fn TabItem(index: usize, is_active: bool) -> Element {
    let mut store = use_context::<Store>();

    let (title, is_dirty) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return rsx!{} };
        let dirty = tab.can_save && tab.merge.is_dirty();
        (tab.title.clone(), dirty)
    };

    let class = if is_active { "tab active" } else { "tab" };

    rsx! {
        div { class: "{class}",
            span {
                class: "tab-title",
                onclick: move |_| store.active.set(Some(index)),
                if is_dirty { span { class: "dirty-dot", "●" } }
                "{title}"
            }
            button {
                class: "tab-close",
                title: "Close tab",
                aria_label: "Close {title}",
                onclick: move |_| {
                    let dirty = store.tabs.read().get(index)
                        .map(|t| t.can_save && t.merge.is_dirty())
                        .unwrap_or(false);
                    if dirty { store.modal.set(Modal::ConfirmClose(index)); }
                    else     { close_tab(&mut store, index); }
                },
                "×"
            }
        }
    }
}
