//! Tab bar: Explorer tab + file comparison tabs + directory compare tabs.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, close_tab, close_dir_tab};

#[component]
pub fn TabBar() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let active_file = *store.active.read();
    let active_dir  = *store.active_dir.read();
    let file_count = store.tabs.read().len();
    let dir_count  = store.dir_tabs.read().len();

    let explorer_active = active_file.is_none() && active_dir.is_none();

    rsx! {
        div { class: "tabbar",
            // Permanent Explorer tab.
            {
                let cls = if explorer_active { "tab active" } else { "tab" };
                rsx! {
                    div {
                        class: "{cls}",
                        onclick: move |_| {
                            store.active.set(None);
                            store.active_dir.set(None);
                        },
                        span { class: "tab-title", {t(lang, "Explorer")} }
                    }
                }
            }
            // File comparison tabs.
            for i in 0..file_count {
                FileTabItem { index: i, is_active: active_file == Some(i) && active_dir.is_none() }
            }
            // Directory compare tabs.
            for i in 0..dir_count {
                DirTabItem { index: i, is_active: active_dir == Some(i) }
            }
        }
    }
}

#[component]
fn FileTabItem(index: usize, is_active: bool) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let (title, is_dirty, is_loading) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return rsx!{} };
        let dirty    = tab.can_save && tab.merge.is_dirty();
        let loading  = tab.state == crate::state::TabState::Loading;
        (tab.title.clone(), dirty, loading)
    };

    let class = if is_active { "tab active" } else { "tab" };

    rsx! {
        div { class: "{class}",
            span {
                class: "tab-title",
                onclick: move |_| {
                    store.active.set(Some(index));
                    store.active_dir.set(None);
                },
                if is_loading { span { class: "tab-loading-spinner", "⟳ " } }
                else if is_dirty { span { class: "dirty-dot", "●" } }
                "{title}"
            }
            button {
                class: "tab-close",
                title: t(lang, "Close tab"),
                aria_label: format!("{} {title}", t(lang, "Close")),
                onclick: move |_| {
                    // Loading tabs close without a dirty-check (RFC-065).
                    if is_loading {
                        crate::state::close_tab(&mut store, index);
                        return;
                    }
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

#[component]
fn DirTabItem(index: usize, is_active: bool) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let title = {
        let tabs = store.dir_tabs.read();
        let Some((l, r)) = tabs.get(index) else { return rsx!{} };
        let ln = l.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let rn = r.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if ln == rn { ln } else { format!("{ln} ↔ {rn}") }
    };

    let class = if is_active { "tab active" } else { "tab" };

    rsx! {
        div { class: "{class}",
            span {
                class: "tab-title",
                onclick: move |_| {
                    store.active.set(None);
                    store.active_dir.set(Some(index));
                },
                "📁 {title}"
            }
            button {
                class: "tab-close",
                title: t(lang, "Close tab"),
                aria_label: format!("{} {title}", t(lang, "Close")),
                onclick: move |_| close_dir_tab(&mut store, index),
                "×"
            }
        }
    }
}
