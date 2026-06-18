//! All safety and action modals (RFC-007, RFC-009, RFC-031, RFC-046).
//! Dispatched from `ModalLayer` in `settings.rs`.

use dioxus::prelude::*;

use forskscope_core::platform::PlatformInfo;
use crate::i18n::t;
use crate::state::{BatchCopySpec, BatchResultSpec, DirOp, Modal, Store, close_tab, reload_tab, swap_sides};
use crate::ui::diff::save_as;

// ─── File overwrite ───────────────────────────────────────────────────────────

#[component]
pub fn OverwriteModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "File changed on disk"),
            div { class: "modal",
                h2 { {t(lang, "File changed on disk")} }
                p { {t(lang, "The target file was modified after it was loaded. Overwrite anyway?")} }
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
                        onclick: move |_| save_as(&mut store, index, path.read().cloned()),
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Reload files"),
            div { class: "modal",
                h2 { {t(lang, "Reload files?")} }
                p { {t(lang, "Unsaved merge changes will be discarded.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { reload_tab(&mut store, index); store.modal.set(Modal::None); }, {t(lang, "Discard and Reload")} }
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Swap sides"),
            div { class: "modal",
                h2 { {t(lang, "Swap sides?")} }
                p { {t(lang, "Unsaved merge changes will be discarded when sides are swapped.")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { swap_sides(&mut store, index); store.modal.set(Modal::None); }, {t(lang, "Discard and Swap")} }
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Close comparison"),
            div { class: "modal",
                h2 { {t(lang, "Close comparison?")} }
                p { {format!("\"{}\" {}",
                    title,
                    t(lang, "has unsaved changes. Discard them and close?")
                )} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button { onclick: move |_| { close_tab(&mut store, index); store.modal.set(Modal::None); }, {t(lang, "Discard and close")} }
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Copy file"),
            div { class: "modal",
                h2 { {t(lang, "Copy this file?")} }
                div { class: "field", span { {t(lang, "From")} } code { class: "path-display", "{src}" } }
                div { class: "field", span { {t(lang, "To")} }   code { class: "path-display", "{dst}" } }
                if op.dst.exists() {
                    p { class: "notice notice-ok",
                        {t(lang, "The destination exists. A .bak backup will be created first.")}
                    }
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            use forskscope_core::dir::{BatchItem, BatchFailurePolicy, batch_copy};
                            use crate::state::BatchResultSpec;
                            let items = vec![BatchItem { src: op.src.clone(), dst: op.dst.clone() }];
                            let manifest_dir = dirs_next::data_dir()
                                .map(|d| d.join("forskscope").join("manifests"));
                            let result_spec = match batch_copy(
                                &items,
                                forskscope_core::BackupPolicy::SiblingBak,
                                BatchFailurePolicy::ContinueOnFailure,
                                manifest_dir.as_deref(),
                            ) {
                                Ok(manifest) => {
                                    let failure_details: Vec<String> = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_core::dir::EntryOutcome::Failed { error } = &e.outcome {
                                                Some(format!("{}: {}", e.dst.display(), error))
                                            } else { None }
                                        })
                                        .collect();
                                    if manifest.failed() == 0 {
                                        // Success: just show a toast and close.
                                        store.notify_success(t(store.lang(), "Copied."));
                                        store.modal.set(Modal::None);
                                        return;
                                    }
                                    BatchResultSpec {
                                        succeeded: manifest.succeeded(),
                                        failed:    manifest.failed(),
                                        skipped:   0,
                                        manifest_path: manifest.manifest_path.clone(),
                                        failure_details,
                                    }
                                }
                                Err(e) => BatchResultSpec {
                                    succeeded: 0, failed: 1, skipped: 0,
                                    manifest_path: None,
                                    failure_details: vec![e.to_string()],
                                },
                            };
                            store.modal.set(Modal::BatchResult(result_spec));
                        },
                        {t(lang, "Copy file")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Batch copy"),
            div { class: "modal",
                h2 { {format!("{} {count} {}?", t(lang, "Copy"), t(lang, "files"))} }
                p { "{label}" }
                p { class: "notice", {t(lang, "Existing files will receive a .bak backup.")} }
                p { class: "notice notice-ok",
                    {t(lang, "A manifest will be saved so copies can be reviewed or reversed.")}
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Cancel")} }
                    button {
                        onclick: move |_| {
                            use forskscope_core::dir::{BatchItem, BatchFailurePolicy, batch_copy};
                            use crate::state::BatchResultSpec;
                            let items: Vec<BatchItem> = spec.items.iter()
                                .map(|(s, d)| BatchItem { src: s.clone(), dst: d.clone() })
                                .collect();
                            let manifest_dir = dirs_next::data_dir()
                                .map(|d| d.join("forskscope").join("manifests"));
                            let result = batch_copy(
                                &items,
                                forskscope_core::BackupPolicy::SiblingBak,
                                BatchFailurePolicy::ContinueOnFailure,
                                manifest_dir.as_deref(),
                            );
                            let result_spec = match result {
                                Ok(manifest) => {
                                    let failure_details: Vec<String> = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_core::dir::EntryOutcome::Failed { error } = &e.outcome {
                                                Some(format!("{}: {}", e.dst.display(), error))
                                            } else { None }
                                        })
                                        .take(5)
                                        .collect();
                                    BatchResultSpec {
                                        succeeded: manifest.succeeded(),
                                        failed:    manifest.failed(),
                                        skipped:   manifest.entries.len() - manifest.succeeded() - manifest.failed(),
                                        manifest_path: manifest.manifest_path.clone(),
                                        failure_details,
                                    }
                                }
                                Err(e) => BatchResultSpec {
                                    succeeded: 0, failed: count, skipped: 0,
                                    manifest_path: None,
                                    failure_details: vec![e.to_string()],
                                },
                            };
                            store.modal.set(Modal::BatchResult(result_spec));
                        },
                        {t(lang, "Copy all")}
                    }
                }
            }
        }
    }
}

// ─── Batch copy result ────────────────────────────────────────────────────────

#[component]
pub fn BatchResultModal(spec: BatchResultSpec) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = if spec.all_succeeded() {
        format!("{} {} {}", t(lang, "Copied"), spec.succeeded, t(lang, "files"))
    } else {
        format!("{}: {} {}, {} {}",
            t(lang, "Copy finished"),
            spec.succeeded, t(lang, "succeeded"),
            spec.failed,    t(lang, "failed"))
    };
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "Copy result"),
            div { class: "modal",
                h2 { "{title}" }
                if let Some(ref mp) = spec.manifest_path {
                    p { class: "notice notice-ok",
                        {t(lang, "Manifest saved:")}
                    }
                    code { class: "path-display", {mp.display().to_string()} }
                }
                if !spec.failure_details.is_empty() {
                    p { class: "notice", {t(lang, "Errors:")} }
                    for detail in spec.failure_details.iter() {
                        code { class: "path-display", "{detail}" }
                    }
                }
                div { class: "actions",
                    button {
                        autofocus: true,
                        onclick: move |_| store.modal.set(Modal::None),
                        {t(lang, "Close")}
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
    let lang = store.lang();
    let info = PlatformInfo::collect();
    let diag = info.to_report();
    let d2 = diag.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "About ForskScope",
            div { class: "modal",
                h2 { "ForskScope v{info.app_version}" }
                div { class: "about-grid",
                    span { class: "about-key", {t(lang, "Version")} }   span { "{info.app_version}" }
                    span { class: "about-key", {t(lang, "Rust")} }      span { "{info.rustc_version}" }
                    span { class: "about-key", {t(lang, "OS")} }        span { "{info.os}" }
                    span { class: "about-key", {t(lang, "Arch")} }      span { "{info.arch}" }
                    span { class: "about-key", {t(lang, "CPUs")} }      span { "{info.logical_cpus}" }
                }
                div { class: "actions",
                    button { onclick: move |_| { let d = d2.clone(); spawn(async move { let _ = dioxus::document::eval(&format!("navigator.clipboard?.writeText({:?})", d)).await; }); }, {t(lang, "Copy diagnostics")} }
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "Close")} }
                }
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn save_tab_force(store: &mut Store, index: usize) {
    crate::ui::diff::save_tab(store, index, true);
}
