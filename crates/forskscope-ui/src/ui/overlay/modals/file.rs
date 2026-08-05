//! File safety modals: overwrite confirmation, save-as, reload, and swap sides.

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, reload_tab, swap_sides};
use crate::ui::view::diff::{confirm_overwrite, save_as};

/// `target` is the exact path the conflicting save attempted — the tab's own
/// save target for a plain save conflict, or the Save As destination for a
/// Save As conflict (review 048 C1: confirming must overwrite this path,
/// never silently fall back to whatever the tab's current save target is).
#[component]
pub fn OverwriteModal(index: usize, target: PathBuf) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let path_display = target.display().to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "File changed on disk"),
            div { class: "modal",
                h2 { {t(lang, "File changed on disk")} }
                p { {t(lang, "The target file was modified after it was loaded. Overwrite anyway?")} }
                code { class: "path-display", "{path_display}" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| confirm_overwrite(&mut store, index, target.clone()),
                        {t(lang, "Overwrite")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn SaveAsModal(index: usize, initial_path: String) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut path = use_signal(|| initial_path);
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Save As"),
            div { class: "modal",
                h2 { {t(lang, "Save As")} }
                div { class: "field",
                    span { {t(lang, "Path")} }
                    input { autofocus: true, value: "{path}", oninput: move |e| path.set(e.value()), style: "width:100%;" }
                }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        disabled: path.read().trim().is_empty(),
                        onclick: move |_| {
                            let typed = path.read().cloned();
                            let target = PathBuf::from(&typed);
                            // RFC-077 test design: an existing Save As
                            // destination needs confirmation *before* any
                            // write is attempted — not just a reactive
                            // conflict dialog if a race happens to occur.
                            // A plain existence check is enough here; the
                            // real safety boundary is still `build_request`/
                            // `save_text`'s fresh precondition check.
                            if target.exists() {
                                store.modal.set(Modal::ConfirmSaveAsOverwrite(index, target));
                            } else {
                                save_as(&mut store, index, typed);
                            }
                        },
                        {t(lang, "Save")}
                    }
                }
            }
        }
    }
}

/// Confirmed via [`Modal::ConfirmSaveAsOverwrite`] — an existing Save As
/// destination the user explicitly chose to overwrite. Proceeds to the real
/// `save_as`, which still runs its own fresh precondition check (RFC-077:
/// selecting a path never itself constructs `Force`).
#[component]
pub fn ConfirmSaveAsOverwriteModal(index: usize, target: PathBuf) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let path_display = target.display().to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Save As"),
            div { class: "modal",
                h2 { {t(lang, "Overwrite existing file?")} }
                p { {t(lang, "A file already exists at this path.")} }
                code { class: "path-display", "{path_display}" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            save_as(&mut store, index, target.display().to_string());
                        },
                        {t(lang, "Overwrite")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn ReloadModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Reload files"),
            div { class: "modal",
                h2 { {t(lang, "Reload files?")} }
                p { {t(lang, "Unsaved merge changes will be discarded.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| { reload_tab(&mut store, index); store.modal.set(Modal::None); },
                        {t(lang, "Discard and Reload")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn SwapModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Swap sides"),
            div { class: "modal",
                h2 { {t(lang, "Swap sides?")} }
                p { {t(lang, "Unsaved merge changes will be discarded when sides are swapped.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| { swap_sides(&mut store, index); store.modal.set(Modal::None); },
                        {t(lang, "Discard and Swap")}
                    }
                }
            }
        }
    }
}
