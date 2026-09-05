//! File and merge-state safety modals: overwrite confirmation, save-as,
//! reload, swap sides, diff-option and encoding changes, the large-file
//! load prompt, and the save-error recovery dialog — each guards an action
//! that would otherwise discard unsaved merge work, start an expensive
//! load, or leave a failed save unexplained, without asking.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::DiffOptions;
use forskscope_ui_logic::SaveErrorView;

use crate::i18n::t;
use crate::state::{
    LargeLoadPrompt, LargeLoadTarget, Modal, Store, open_compare_request_with_options, reload_tab,
    reload_tab_with_options, set_diff_options, set_encoding, swap_sides,
};
use crate::ui::view::diff::{SaveAsPrecheck, confirm_overwrite, precheck_save_as_target, save_as};
use crate::ui::view::diff_actions::handle_save_recovery_action;

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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "File changed on disk"), onmounted: super::focus_autofocus_button,
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
                            // Classified via inspect_save_target (review 050
                            // §3.2), not a plain existence check, so a
                            // destination that can never be written to
                            // (a directory, binary, ...) is reported
                            // immediately instead of asking to "overwrite"
                            // something the next step would refuse anyway.
                            match precheck_save_as_target(&store, index, &target) {
                                SaveAsPrecheck::New => save_as(&mut store, index, typed),
                                SaveAsPrecheck::Overwrite => {
                                    store.modal.set(Modal::ConfirmSaveAsOverwrite(index, target));
                                }
                                SaveAsPrecheck::Blocked(message) => store.notify(message),
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Save As"), onmounted: super::focus_autofocus_button,
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

/// Confirmed via [`Modal::ConfirmDiffOptionChange`] — installs `options` and
/// recomputes the diff, discarding applied merge work and the undo/redo
/// stack (F40), same discard-and-proceed pattern as [`SwapModal`].
#[component]
pub fn ConfirmDiffOptionChangeModal(index: usize, options: DiffOptions) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Change diff options"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "Change diff options?")} }
                p { {t(lang, "Unsaved merge changes will be discarded when diff options change.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            set_diff_options(&mut store, index, options);
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "Discard and Change")}
                    }
                }
            }
        }
    }
}

/// RFC-083 §3: guards `set_encoding`'s own `recompute_diff` call, the same
/// discard-and-proceed pattern as [`ConfirmDiffOptionChangeModal`].
#[component]
pub fn ConfirmEncodingChangeModal(index: usize, label: String) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Change encoding"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "Change encoding?")} }
                p { {t(lang, "Unsaved merge changes will be discarded when the encoding changes.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            set_encoding(&mut store, index, label.clone());
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "Discard and Change")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Reload files"), onmounted: super::focus_autofocus_button,
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Swap sides"), onmounted: super::focus_autofocus_button,
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

/// F84: confirmed via `Modal::ConfirmLargeLoad` (`LoadGuard::ConfirmPrompt`,
/// RFC-013 §"Large file prompt") — nothing has been loaded yet. Confirming
/// resumes `prompt.target` with `prompt.opts`, calling the `_with_options`
/// entry points directly rather than the checked `open_compare_request`/
/// `reload_tab` — the guard already ran once to produce these exact
/// (inline-suppressed) options; running it again here would either repeat
/// this same prompt or silently discard the suppression the user just
/// accepted.
#[component]
pub fn LargeLoadModal(prompt: LargeLoadPrompt) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = prompt.title.clone();
    let body = prompt.body.clone();
    let confirm_label = prompt.confirm_label.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}", onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { "{title}" }
                p { "{body}" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            let prompt = prompt.clone();
                            store.modal.set(Modal::None);
                            match prompt.target {
                                LargeLoadTarget::Open(request) => {
                                    open_compare_request_with_options(&mut store, request, prompt.opts);
                                }
                                LargeLoadTarget::Reload(index) => {
                                    reload_tab_with_options(&mut store, index, prompt.opts);
                                }
                            }
                        },
                        "{confirm_label}"
                    }
                }
            }
        }
    }
}

/// F52: confirmed via `Modal::SaveError` — a non-conflict save failure
/// (`diff_actions::handle_result`'s `Err(e)` arm; a `CoreError::Conflict`
/// never reaches this dialog, see `OverwriteModal`). `view.buttons` is
/// already in the order and primary-button selection `SaveErrorView`
/// decided; this component renders it as-is rather than re-deriving which
/// action is "first". Not run through `t()`: `SaveErrorView`'s text comes
/// from `UserMessage`/`action_label`, English-only by the same precedent
/// `handle_result`'s old `Err(e) => store.notify(e.to_string())` arm set.
#[component]
pub fn SaveErrorModal(index: usize, target: PathBuf, view: SaveErrorView) -> Element {
    let mut store = use_context::<Store>();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{view.title}", onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { "{view.title}" }
                p { "{view.body}" }
                if let Some(path) = &view.path {
                    code { class: "path-display", "{path}" }
                }
                div { class: "actions",
                    for button in view.buttons.iter() {
                        button {
                            key: "{button.action:?}",
                            autofocus: button.is_primary,
                            onclick: {
                                let target = target.clone();
                                let action = button.action;
                                move |_| handle_save_recovery_action(&mut store, index, target.clone(), action)
                            },
                            "{button.label}"
                        }
                    }
                }
            }
        }
    }
}
