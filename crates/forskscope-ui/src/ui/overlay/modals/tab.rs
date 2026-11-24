//! Close-dirty-tab confirmation modal.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Modal, Store, close_tab};
use crate::ui::view::BidiText;

#[component]
pub fn CloseTabModal(index: usize) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let title = store
        .tabs
        .read()
        .get(index)
        .map(|t| t.title.clone())
        .unwrap_or_default();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "close-comparison"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "close-comparison-2")} }
                p {
                    "\""
                    BidiText { text: title.clone(), lang }
                    "\" "
                    {t(lang, "has-unsaved-changes-discard-them-and-close")}
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            close_tab(&mut store, index);
                            store.modal.set(Modal::None);
                            // RFC-087: "focus moves to the tab that became
                            // selected" applies here too. Safe to call from
                            // this about-to-unmount modal because
                            // `focus_handle` (`keyboard.rs`) uses
                            // `spawn_forever`, which is not tied to this
                            // scope's lifecycle — see its doc for why a
                            // plain `spawn` here lost the race.
                            store.focus_active_tab();
                        },
                        {t(lang, "discard-and-close")}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;
    use forskscope_compare::{
        BomPresence, DiffOptions, FileKind, LoadedDocument, NewlineStyle, TextDocument,
        TextEncoding, compute_diff,
    };
    use forskscope_edit::MergeSession;
    use forskscope_edit::compare_prep::SaveCapability;
    use forskscope_ui_logic::{CompareTabId, LoadGeneration};

    use super::CloseTabModal;
    use crate::state::tab::CompareLaunchMode;
    use crate::state::{AppSettings, CompareTab, Store, TabState};

    fn text_doc(content: &str) -> LoadedDocument {
        LoadedDocument {
            file_id: None,
            fingerprint_at_load: None,
            kind: FileKind::Text,
            bytes_len: content.len() as u64,
            text: Some(TextDocument {
                content: content.to_string(),
                encoding: TextEncoding::utf8(),
                newline_style: NewlineStyle::Lf,
                had_decode_errors: false,
                bom: BomPresence::Absent,
                raw_bytes: Vec::new(),
            }),
            warnings: Vec::new(),
        }
    }

    fn bidi_titled_tab() -> CompareTab {
        let left = text_doc("one\ntwo\n");
        let right = text_doc("one\nTWO\n");
        let diff_options = DiffOptions::default();
        let diff = compute_diff(left.diff_text(), right.diff_text(), diff_options);
        let merge = MergeSession::from_diff(&diff);

        CompareTab {
            id: CompareTabId::new(1).unwrap(),
            load_generation: LoadGeneration::new(1).unwrap(),
            title: "invoice\u{202E}fdp.exe".to_string(),
            left_path: None,
            right_path: None,
            state: TabState::Ready,
            left_doc: left,
            right_doc: right,
            merge,
            diff,
            diff_options,
            can_save: true,
            save_capability: SaveCapability::Saveable,
            spreadsheet_warnings: Vec::new(),
            char_mode: false,
            word_wrap: false,
            focused_change: 0,
            save_target: None,
            launch_mode: CompareLaunchMode::Normal,
        }
    }

    /// F235: a bidi control character in the tab's own title never
    /// reaches the close-confirmation modal raw — `BidiText` splits the
    /// title into its own `"invoice"`/`"RLO"`/`"fdp.exe"` text nodes, and
    /// the trailing translated sentence stays a separate, intact text
    /// node (the surrounding quote marks are static template text, not
    /// their own dynamic `CreateTextNode` mutations — not asserted here,
    /// same limitation this file's own sibling tests already note for
    /// static attributes). Falsify by reverting `CloseTabModal`'s
    /// `BidiText { text: title.clone(), lang }` back to a bare
    /// `"{title}"`: the raw character then reaches a `CreateTextNode`
    /// and this fails.
    #[test]
    fn a_bidi_control_character_in_the_closing_tabs_own_title_never_reaches_the_rendered_page_raw()
    {
        fn root() -> Element {
            let store = use_context_provider(|| {
                let mut store = Store::new(AppSettings::default(), Default::default(), false);
                store.tabs.write().push(bidi_titled_tab());
                store
            });
            let _ = store;
            rsx! { CloseTabModal { index: 0usize } }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let texts: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();
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
        assert!(
            texts
                .iter()
                .any(|t| t.contains("has unsaved changes. Discard them and close?")),
            "the trailing sentence must stay plain translated text, got {texts:?}"
        );
    }
}
