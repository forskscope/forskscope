//! Workspace tab bar (RFC-003 §4, RFC-016).

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Store;

#[component]
pub fn TabBar() -> Element {
    let store = use_context::<Store>();
    let active = *store.active.read();
    let lang = store.lang();

    // Snapshot tab summaries to avoid holding the borrow across handlers.
    let summaries: Vec<(usize, String, bool)> = store
        .tabs
        .read()
        .iter()
        .enumerate()
        .map(|(i, tab)| (i, tab.title.clone(), tab.can_save && tab.merge.is_dirty()))
        .collect();

    rsx! {
        div { class: "tabs",
            ExplorerTab { active: active.is_none(), lang }
            for (index, title, dirty) in summaries {
                TabItem { index, title, dirty, active: active == Some(index) }
            }
        }
    }
}

#[component]
fn ExplorerTab(active: bool, lang: crate::state::Lang) -> Element {
    let mut store = use_context::<Store>();
    let class = if active { "tab active" } else { "tab" };
    rsx! {
        div { class: "{class}", onclick: move |_| store.active.set(None),
            span { class: "name", {t(lang, "Explorer")} }
        }
    }
}

#[component]
fn TabItem(index: usize, title: String, dirty: bool, active: bool) -> Element {
    let mut store = use_context::<Store>();
    let class = if active { "tab active" } else { "tab" };
    rsx! {
        div { class: "{class}", onclick: move |_| store.active.set(Some(index)),
            if dirty {
                span { class: "dirty", "●" }
            }
            span { class: "name", "{title}" }
            span {
                class: "close",
                onclick: move |evt: Event<MouseData>| {
                    evt.stop_propagation();
                    close_tab(&mut store, index);
                },
                "✕"
            }
        }
    }
}

fn close_tab(store: &mut Store, index: usize) {
    let mut tabs = store.tabs.write();
    if index >= tabs.len() {
        return;
    }
    tabs.remove(index);
    let len = tabs.len();
    drop(tabs);
    let active = *store.active.read();
    match active {
        Some(a) if a == index => {
            store.active.set(if len == 0 { None } else { Some(a.min(len - 1)) });
        }
        Some(a) if a > index => store.active.set(Some(a - 1)),
        _ => {}
    }
}
