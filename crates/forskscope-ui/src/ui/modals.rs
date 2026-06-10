//! All safety and action modals (RFC-007, RFC-009, RFC-031, RFC-046).
//! Dispatched from `ModalLayer` in `settings.rs`.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{BatchCopySpec, DirOp, Modal, Store, close_tab, reload_tab, swap_sides};
use crate::ui::diff::save_as;

// ─── File overwrite ───────────────────────────────────────────────────────────

#[component]
pub fn OverwriteModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "File changed on disk",
            div { class: "modal",
                h2 { {t(lang, "File changed on disk")} }
                p { "The target file was modified after it was loaded. Overwrite anyway?" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { save_tab_force(&mut store, index); }, {t(lang, "Overwrite")} }
                }
            }
        }
    }
}

// ─── Save As ─────────────────────────────────────────────────────────────────

#[component]
pub fn SaveAsModal(index: usize, initial_path: String) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut path = use_signal(|| initial_path);
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Save As",
            div { class: "modal",
                h2 { "Save As" }
                div { class: "field",
                    span { "Path" }
                    input { autofocus: true, value: "{path}", oninput: move |e| path.set(e.value()), style: "width:100%;" }
                }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        disabled: path.read().trim().is_empty(),
                        onclick: move |_| save_as(&mut store, index, path.read().clone()),
                        {t(lang, "Save")}
                    }
                }
            }
        }
    }
}

// ─── Reload ──────────────────────────────────────────────────────────────────

#[component]
pub fn ReloadModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Reload files",
            div { class: "modal",
                h2 { "Reload files?" }
                p { "Unsaved merge changes will be discarded." }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { reload_tab(&mut store, index); store.modal.set(Modal::None); }, "Discard and Reload" }
                }
            }
        }
    }
}

// ─── Swap sides ───────────────────────────────────────────────────────────────

#[component]
pub fn SwapModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Swap sides",
            div { class: "modal",
                h2 { "Swap sides?" }
                p { "Unsaved merge changes will be discarded when sides are swapped." }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { swap_sides(&mut store, index); store.modal.set(Modal::None); }, "Discard and Swap" }
                }
            }
        }
    }
}

// ─── Close dirty tab ─────────────────────────────────────────────────────────

#[component]
pub fn CloseTabModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = store.tabs.read().get(index).map(|t| t.title.clone()).unwrap_or_default();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Close comparison",
            div { class: "modal",
                h2 { "Close comparison?" }
                p { "\"{title}\" has unsaved changes. Discard them and close?" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { close_tab(&mut store, index); store.modal.set(Modal::None); }, "Discard and close" }
                }
            }
        }
    }
}

// ─── Directory file copy ──────────────────────────────────────────────────────

#[component]
pub fn ConfirmDirOpModal(op: DirOp) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let src = op.src.display().to_string();
    let dst = op.dst.display().to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Copy file",
            div { class: "modal",
                h2 { "Copy file?" }
                p { "{op.label}" }
                div { class: "field", span { "From" } code { class: "path-display", "{src}" } }
                div { class: "field", span { "To"   } code { class: "path-display", "{dst}" } }
                if op.dst.exists() {
                    p { class: "notice", "Destination exists. A .bak backup will be created." }
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            match forskscope_core::dir::copy_file(&op.src, &op.dst, forskscope_core::BackupPolicy::SiblingBak) {
                                Ok(_)  => store.notify("Copied."),
                                Err(e) => store.notify(e.to_string()),
                            }
                            store.modal.set(Modal::None);
                        },
                        "Copy"
                    }
                }
            }
        }
    }
}

// ─── Batch copy ───────────────────────────────────────────────────────────────

#[component]
pub fn BatchCopyModal(spec: BatchCopySpec) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let count = spec.items.len();
    let label = spec.label.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "Batch copy",
            div { class: "modal",
                h2 { "Copy {count} files?" }
                p { "{label}" }
                p { class: "notice", "Existing files will receive a .bak backup." }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            let mut ok = 0usize; let mut err = 0usize;
                            for (src, dst) in &spec.items {
                                match forskscope_core::dir::copy_file(src, dst, forskscope_core::BackupPolicy::SiblingBak) {
                                    Ok(_)  => ok  += 1,
                                    Err(_) => err += 1,
                                }
                            }
                            store.notify(format!("Copied {ok} file{}{}.", if ok==1{""} else {"s"},
                                if err > 0 { format!(", {err} failed") } else { String::new() }));
                            store.modal.set(Modal::None);
                        },
                        "Copy all"
                    }
                }
            }
        }
    }
}

// ─── About ───────────────────────────────────────────────────────────────────

#[component]
pub fn AboutModal() -> Element {
    let mut store = use_context::<Store>();
    const VERSION: &str  = env!("CARGO_PKG_VERSION");
    const PROFILE: &str  = if cfg!(debug_assertions) { "debug" } else { "release" };
    let platform = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let diag = format!("ForskScope {VERSION}\nBuild: {PROFILE}\nPlatform: {platform}\nUI: Dioxus 0.7\nDiff engine: similar 3");
    let d2 = diag.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "About ForskScope",
            div { class: "modal",
                h2 { "ForskScope v{VERSION}" }
                div { class: "about-grid",
                    span { class: "about-key", "Version" }   span { "{VERSION}" }
                    span { class: "about-key", "Build" }     span { "{PROFILE}" }
                    span { class: "about-key", "Platform" }  span { "{platform}" }
                    span { class: "about-key", "UI" }        span { "Dioxus 0.7" }
                    span { class: "about-key", "Diff" }      span { "similar 3" }
                }
                div { class: "actions",
                    button { onclick: move |_| { let d = d2.clone(); spawn(async move { let _ = dioxus::document::eval(&format!("navigator.clipboard?.writeText({:?})", d)).await; }); }, "Copy diagnostics" }
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), "Close" }
                }
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn save_tab_force(store: &mut Store, index: usize) {
    crate::ui::diff::save_tab(store, index, true);
}
