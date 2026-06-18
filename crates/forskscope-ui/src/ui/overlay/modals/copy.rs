//! Directory copy modals: single-file confirm, batch copy, and batch result.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{BatchCopySpec, BatchResultSpec, DirOp, Modal, Store};

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
                                    if manifest.failed() == 0 {
                                        store.notify_success(t(store.lang(), "Copied."));
                                        store.modal.set(Modal::None);
                                        return;
                                    }
                                    let failure_details = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_core::dir::EntryOutcome::Failed { error } = &e.outcome {
                                                Some(format!("{}: {}", e.dst.display(), error))
                                            } else { None }
                                        })
                                        .collect();
                                    BatchResultSpec {
                                        succeeded: manifest.succeeded(), failed: manifest.failed(),
                                        skipped: 0, manifest_path: manifest.manifest_path.clone(),
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

#[component]
pub fn BatchCopyModal(spec: BatchCopySpec) -> Element {
    let mut store = use_context::<Store>();
    let lang  = store.lang();
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
                            let items: Vec<BatchItem> = spec.items.iter()
                                .map(|(s, d)| BatchItem { src: s.clone(), dst: d.clone() })
                                .collect();
                            let manifest_dir = dirs_next::data_dir()
                                .map(|d| d.join("forskscope").join("manifests"));
                            let result_spec = match batch_copy(
                                &items,
                                forskscope_core::BackupPolicy::SiblingBak,
                                BatchFailurePolicy::ContinueOnFailure,
                                manifest_dir.as_deref(),
                            ) {
                                Ok(manifest) => {
                                    let failure_details = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_core::dir::EntryOutcome::Failed { error } = &e.outcome {
                                                Some(format!("{}: {}", e.dst.display(), error))
                                            } else { None }
                                        })
                                        .take(5).collect();
                                    BatchResultSpec {
                                        succeeded: manifest.succeeded(), failed: manifest.failed(),
                                        skipped: manifest.entries.len() - manifest.succeeded() - manifest.failed(),
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

#[component]
pub fn BatchResultModal(spec: BatchResultSpec) -> Element {
    let mut store = use_context::<Store>();
    let lang  = store.lang();
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
                    p { class: "notice notice-ok", {t(lang, "Manifest saved:")} }
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
