//! Directory copy modals: single-file confirm, batch copy, and batch result.

use dioxus::prelude::*;

use crate::i18n::{t, tf};
use crate::state::{BatchCopySpec, BatchResultSpec, DirOp, Modal, Store};
use crate::ui::component::notice::{Notice, NoticeKind};
use crate::ui::view::BidiText;

#[component]
pub fn ConfirmDirOpModal(op: DirOp) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let src = op.src.display().to_string();
    let dst = op.dst.display().to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "copy-file"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "copy-this-file")} }
                div { class: "field", span { {t(lang, "from")} } code { class: "path-display", BidiText { text: src.clone(), lang } } }
                div { class: "field", span { {t(lang, "to")} }   code { class: "path-display", BidiText { text: dst.clone(), lang } } }
                if op.dst.exists() {
                    Notice { kind: NoticeKind::Ok,
                        {t(lang, "the-destination-exists-a-bak-backup-will-be-created-first")}
                    }
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            use forskscope_edit::dir::{BatchItem, BatchFailurePolicy, batch_copy};
                            let items = vec![BatchItem { src: op.src.clone(), dst: op.dst.clone() }];
                            let manifest_dir = crate::startup_mode::manifest_dir();
                            let result_spec = match batch_copy(
                                &items,
                                forskscope_edit::BackupPolicy::SiblingBak,
                                BatchFailurePolicy::ContinueOnFailure,
                                manifest_dir.as_deref(),
                            ) {
                                Ok(manifest) => {
                                    if manifest.failed() == 0 {
                                        store.notify_success(crate::i18n::TranslatableMessage::new("copied"));
                                        store.modal.set(Modal::None);
                                        return;
                                    }
                                    let failure_details = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_edit::dir::EntryOutcome::Failed { error } = &e.outcome {
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
                        {t(lang, "copy-file")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn BatchCopyModal(spec: BatchCopySpec) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let count = spec.items.len();
    let label = spec.label.clone();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "batch-copy"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {tf(lang, "batch-copy-confirm-title", &[], Some(count as i64))} }
                p { "{label}" }
                Notice { kind: NoticeKind::Info, {t(lang, "existing-files-will-receive-a-bak-backup")} }
                Notice { kind: NoticeKind::Ok,
                    {t(lang, "a-manifest-will-be-saved-so-copies-can-be-reviewed-or")}
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            use forskscope_edit::dir::{BatchItem, BatchFailurePolicy, batch_copy};
                            let items: Vec<BatchItem> = spec.items.iter()
                                .map(|(s, d)| BatchItem { src: s.clone(), dst: d.clone() })
                                .collect();
                            let manifest_dir = crate::startup_mode::manifest_dir();
                            let result_spec = match batch_copy(
                                &items,
                                forskscope_edit::BackupPolicy::SiblingBak,
                                BatchFailurePolicy::ContinueOnFailure,
                                manifest_dir.as_deref(),
                            ) {
                                Ok(manifest) => {
                                    let failure_details = manifest.entries.iter()
                                        .filter_map(|e| {
                                            if let forskscope_edit::dir::EntryOutcome::Failed { error } = &e.outcome {
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
                        {t(lang, "copy-all")}
                    }
                }
            }
        }
    }
}

#[component]
pub fn BatchResultModal(spec: BatchResultSpec) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = if spec.all_succeeded() {
        tf(
            lang,
            "batch-result-all-succeeded-title",
            &[],
            Some(spec.succeeded as i64),
        )
    } else {
        tf(
            lang,
            "batch-result-partial-title",
            &[
                ("succeeded", &spec.succeeded.to_string()),
                ("failed", &spec.failed.to_string()),
            ],
            None,
        )
    };
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "copy-result"),
            div { class: "modal",
                h2 { "{title}" }
                if let Some(ref mp) = spec.manifest_path {
                    Notice { kind: NoticeKind::Ok, {t(lang, "manifest-saved")} }
                    code { class: "path-display", BidiText { text: mp.display().to_string(), lang } }
                }
                if !spec.failure_details.is_empty() {
                    Notice { kind: NoticeKind::Error, {t(lang, "errors")} }
                    for detail in spec.failure_details.iter() {
                        code { class: "path-display", BidiText { text: detail.clone(), lang } }
                    }
                }
                div { class: "actions",
                    button {
                        autofocus: true,
                        onclick: move |_| store.modal.set(Modal::None),
                        {t(lang, "close")}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;

    use super::{BatchResultModal, ConfirmDirOpModal};
    use crate::state::{AppSettings, BatchResultSpec, DirOp, Store};

    fn text_nodes(mutations: &dioxus_core::Mutations) -> Vec<String> {
        mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// F235: a bidi control character in a pending copy's own `src`/`dst`
    /// never reaches the rendered confirmation modal raw — both
    /// `.path-display` codes show `BidiText`'s own marker instead.
    /// Falsify by reverting either `BidiText` call site in
    /// `ConfirmDirOpModal` back to a bare interpolation: the raw
    /// character then reaches a `CreateTextNode` and this fails.
    #[test]
    fn a_bidi_control_character_in_a_pending_copys_own_paths_never_reaches_the_rendered_page_raw() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                ConfirmDirOpModal {
                    op: DirOp {
                        src: "/tmp/invoice\u{202E}fdp.exe".into(),
                        dst: "/tmp/dst".into(),
                        label: "copy".to_string(),
                    },
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let texts = text_nodes(&mutations);
        for text in &texts {
            assert!(
                !text.chars().any(forskscope_compare::is_bidi_control),
                "a bidi control character reached a rendered text node: {text:?}"
            );
        }
        assert!(
            texts.iter().any(|t| t.contains("RLO")),
            "expected the RLO marker token among the rendered text nodes: {texts:?}"
        );
    }

    /// F235: the same coverage, for a batch-copy failure detail line.
    #[test]
    fn a_bidi_control_character_in_a_batch_failure_detail_never_reaches_the_rendered_page_raw() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                BatchResultModal {
                    spec: BatchResultSpec {
                        succeeded: 0,
                        failed: 1,
                        skipped: 0,
                        manifest_path: None,
                        failure_details: vec![
                            "/tmp/invoice\u{202E}fdp.exe: permission denied".to_string(),
                        ],
                    },
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let texts = text_nodes(&mutations);
        for text in &texts {
            assert!(
                !text.chars().any(forskscope_compare::is_bidi_control),
                "a bidi control character reached a rendered text node: {text:?}"
            );
        }
        assert!(
            texts.iter().any(|t| t.contains("RLO")),
            "expected the RLO marker token among the rendered text nodes: {texts:?}"
        );
    }
}
