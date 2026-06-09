//! Tab bar: close buttons, dirty-state indicator, active highlighting.

use dioxus::prelude::*;

use crate::state::{Modal, Store, close_tab};

#[component]
pub fn TabBar() -> Element {
    let store = use_context::<Store>();
    let active = *store.active.read();
    let tab_count = store.tabs.read().len();

    rsx! {
        div { class: "tabbar",
            for i in 0..tab_count {
                TabItem {
                    index: i,
                    is_active: active == Some(i),
                }
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
                // Dirty indicator: a filled dot before the title.
                if is_dirty {
                    span { class: "dirty-dot", title: "Unsaved changes", "●" }
                }
                "{title}"
            }
            button {
                class: "tab-close",
                title: "Close comparison",
                aria_label: "Close {title}",
                onclick: move |evt| {
                    evt.stop_propagation();
                    let dirty = store.tabs.read().get(index)
                        .map(|t| t.can_save && t.merge.is_dirty())
                        .unwrap_or(false);
                    if dirty {
                        store.modal.set(Modal::ConfirmClose(index));
                    } else {
                        close_tab(&mut store, index);
                    }
                },
                "×"
            }
        }
    }
}
