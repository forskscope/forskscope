//! Keyboard reference modal (RFC-030): a compact table of all shortcuts.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Lang, Modal, Store};

#[component]
pub fn KeyboardRefModal() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "keyboard-shortcuts"),
            tabindex: "-1",
            onclick: move |_| store.modal.set(Modal::None),
            onkeydown: move |e: Event<KeyboardData>| {
                if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                    store.modal.set(Modal::None);
                }
            },
            div { class: "modal modal-wide", onclick: move |e| e.stop_propagation(),
                h2 { {t(lang, "keyboard-shortcuts")} }
                div { class: "kb-section",
                    h3 { {t(lang, "diff-view")} }
                    div { class: "kb-table",
                        KbRow { keys: "F7 / F8",       desc: t(lang, "previous-next-change") }
                        KbRow { keys: "Enter",          desc: t(lang, "apply-focused-change-left-right") }
                        KbRow { keys: "Ctrl + Z",       desc: t(lang, "undo-last-merge") }
                        KbRow { keys: "Ctrl + Y",       desc: t(lang, "redo-last-undone-merge") }
                        KbRow { keys: "Ctrl + S",       desc: t(lang, "save-merge-result") }
                        KbRow { keys: "Ctrl + F",       desc: t(lang, "open-close-inline-search") }
                        KbRow { keys: "F3 / Shift+F3",  desc: t(lang, "next-previous-search-match") }
                    }
                }
                div { class: "kb-section",
                    h3 { {t(lang, "navigation")} }
                    div { class: "kb-table",
                        KbRow { keys: "F6",             desc: t(lang, "switch-focused-pane-left-right") }
                        KbRow { keys: "↑ / ↓",         desc: t(lang, "move-focus-in-explorer-list") }
                        KbRow { keys: "Enter",          desc: t(lang, "expand-or-collapse-the-focused-folder-does-nothing-on-a-file") }
                        KbRow { keys: "Space",          desc: t(lang, "select-file-as-comparison-candidate") }
                        KbRow { keys: "Alt + ↑",        desc: t(lang, "go-up-one-directory-focused-pane") }
                        KbRow { keys: "Alt + Home",     desc: t(lang, "go-to-home-directory-focused-pane") }
                        KbRow { keys: "Ctrl + O",       desc: t(lang, "open-a-folder-focused-pane") }
                        KbRow { keys: "Alt + ← / Alt + →", desc: t(lang, "back-forward-directory-history-focused-pane") }
                    }
                }
                div { class: "kb-section",
                    h3 { {t(lang, "deep-compare")} }
                    div { class: "kb-table",
                        KbRow { keys: "↑ / ↓",  desc: t(lang, "move-the-selected-row") }
                        KbRow { keys: "Enter",   desc: t(lang, "open-the-comparison-for-the-selected-row-if-available") }
                    }
                }
                div { class: "kb-section",
                    h3 { {t(lang, "tabs")} }
                    div { class: "kb-table",
                        KbRow { keys: "Ctrl + Tab / Ctrl + PageDown",         desc: t(lang, "next-tab-from-anywhere-wraps-includes-explorer") }
                        KbRow { keys: "Ctrl + Shift + Tab / Ctrl + PageUp",   desc: t(lang, "previous-tab-from-anywhere-wraps-includes-explorer") }
                        KbRow { keys: "← / →",          desc: t(lang, "previous-next-tab-tab-list-focused") }
                        KbRow { keys: "Home / End",     desc: t(lang, "first-last-tab-tab-list-focused") }
                        KbRow { keys: "Delete",         desc: t(lang, "close-the-focused-tab-tab-list-focused") }
                    }
                }
                div { class: "kb-section",
                    h3 { {t(lang, "app")} }
                    div { class: "kb-table",
                        KbRow { keys: "Ctrl + W",       desc: t(lang, "close-the-active-comparison-tab") }
                        KbRow { keys: "Ctrl + /",       desc: t(lang, "this-keyboard-reference") }
                        KbRow { keys: "Escape",         desc: t(lang, "close-modal-search-bar") }
                        KbRow { keys: "Ctrl + = / Ctrl + +", desc: t(lang, "increase-text-size") }
                        KbRow { keys: "Ctrl + -",       desc: t(lang, "decrease-text-size") }
                        // Review 203 §3: "Ctrl + O" (open a folder, above) and
                        // "Ctrl + 0" read the same in this dialog's monospace
                        // font — the people Text size is for are the likeliest
                        // to confuse them. The "(zero)"/"（ゼロ）" annotation is
                        // language-specific, so this needs two literal KbRow
                        // calls (one per branch), not one conditional `keys`
                        // value `cargo xtask keyboard-docs`'s own text scanner
                        // could see through either way - see its own doc
                        // comment on why a trailing parenthetical doesn't count
                        // as part of the key's identity.
                        if lang == Lang::Ja {
                            KbRow { keys: "Ctrl + 0（ゼロ）", desc: t(lang, "reset-text-size-to-100") }
                        } else {
                            KbRow { keys: "Ctrl + 0 (zero)", desc: t(lang, "reset-text-size-to-100") }
                        }
                    }
                }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "close")} }
                }
            }
        }
    }
}

#[component]
fn KbRow(keys: &'static str, desc: String) -> Element {
    rsx! {
        div { class: "kb-row",
            kbd { class: "kb-key", "{keys}" }
            span { class: "kb-desc", "{desc}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppSettings;

    // F100 falsification 3: this file had no existing test shape for "does
    // the help modal document a shortcut" — saying so rather than
    // inventing one silently, per the handoff. Built the same
    // rendered-output-inspection technique F99's PathBar aria-label test
    // used (no `dioxus-ssr` in this workspace), extended to text content:
    // `Mutation::CreateTextNode` instead of `SetAttribute`. Falsify by
    // removing either new `KbRow` above and the corresponding assertion
    // fails.
    #[test]
    fn help_modal_documents_the_new_pathbar_shortcuts() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                KeyboardRefModal {}
            }
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

        for expected in [
            "Alt + Home",
            "Go to home directory (focused pane)",
            "Ctrl + O",
            "Open a folder (focused pane)",
            "Alt + ← / Alt + →",
            "Back / forward directory history (focused pane)",
        ] {
            assert!(
                texts.iter().any(|t| t == expected),
                "expected {expected:?} among the rendered help-modal text {texts:?}"
            );
        }
    }
}
