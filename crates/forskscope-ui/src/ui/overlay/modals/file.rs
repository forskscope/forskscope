//! File and merge-state safety modals: overwrite confirmation, save-as,
//! reload, swap sides, diff-option and encoding changes, the large-file
//! load prompt, and the save-error recovery dialog — each guards an action
//! that would otherwise discard unsaved merge work, start an expensive
//! load, or leave a failed save unexplained, without asking.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_compare::DiffOptions;
use forskscope_ui_logic::SaveErrorView;

use crate::i18n::{t, tf};
use crate::state::Lang;
use crate::state::{
    LargeLoadPrompt, LargeLoadTarget, Modal, Store, open_compare_request_with_options, reload_tab,
    reload_tab_with_options, set_diff_options, set_encoding, swap_sides,
};
use crate::ui::view::BidiText;
use crate::ui::view::diff::{SaveAsPrecheck, confirm_overwrite, precheck_save_as_target, save_as};
use crate::ui::view::diff_actions::{app_error_kind_short_id, handle_save_recovery_action};

/// `target` is the exact path the conflicting save attempted — the tab's own
/// save target for a plain save conflict, or the Save As destination for a
/// Save As conflict (review 048 C1: confirming must overwrite this path,
/// never silently fall back to whatever the tab's current save target is).
#[component]
pub fn OverwriteModal(index: usize, target: PathBuf) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let path_display = forskscope_desktop::platform::display_path(&target)
        .display()
        .to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "file-changed-on-disk"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "file-changed-on-disk")} }
                p { {t(lang, "the-target-file-was-modified-after-it-was-loaded-overwrite")} }
                code { class: "path-display", BidiText { text: path_display.clone(), lang } }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| confirm_overwrite(&mut store, index, target.clone()),
                        {t(lang, "overwrite")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "save-as"),
            div { class: "modal",
                h2 { {t(lang, "save-as")} }
                div { class: "field",
                    span { {t(lang, "path")} }
                    input { autofocus: true, value: "{path}", oninput: move |e| path.set(e.value()), style: "width:100%;" }
                }
                div { class: "actions",
                    button { onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
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
                        {t(lang, "save")}
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
    let path_display = forskscope_desktop::platform::display_path(&target)
        .display()
        .to_string();
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "save-as"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "overwrite-existing-file")} }
                p { {t(lang, "a-file-already-exists-at-this-path")} }
                code { class: "path-display", BidiText { text: path_display.clone(), lang } }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            save_as(&mut store, index, target.display().to_string());
                        },
                        {t(lang, "overwrite")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "change-diff-options"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "change-diff-options-2")} }
                p { {t(lang, "unsaved-merge-changes-will-be-discarded-when-diff-options")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            set_diff_options(&mut store, index, options);
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "discard-and-change")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "change-encoding"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "change-encoding-2")} }
                p { {t(lang, "unsaved-merge-changes-will-be-discarded-when-the-encoding")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| {
                            set_encoding(&mut store, index, label.clone());
                            store.modal.set(Modal::None);
                        },
                        {t(lang, "discard-and-change")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "reload-files"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "reload-files-2")} }
                p { {t(lang, "unsaved-merge-changes-will-be-discarded")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| { reload_tab(&mut store, index); store.modal.set(Modal::None); },
                        {t(lang, "discard-and-reload")}
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
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: t(lang, "swap-sides"), onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { {t(lang, "swap-sides-2")} }
                p { {t(lang, "unsaved-merge-changes-will-be-discarded-when-sides-are")} }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
                    button {
                        onclick: move |_| { swap_sides(&mut store, index); store.modal.set(Modal::None); },
                        {t(lang, "discard-and-swap")}
                    }
                }
            }
        }
    }
}

/// The title, body and confirm label of the large-file prompt (F122).
///
/// The "File is large" tier is localised here and says what the product does
/// for the kind of pair: a text pair may be slow, its line diff is bounded at
/// five seconds and says so when approximate, and character highlighting is
/// off; a spreadsheet pair is exact or refused by the size bound, never
/// approximate. The "File too large" tier (over 64 MiB, metadata only) keeps
/// `ui-logic`'s English text unchanged.
pub(crate) fn large_load_text(lang: Lang, prompt: &LargeLoadPrompt) -> (String, String, String) {
    if prompt.too_large {
        return (
            prompt.title.clone(),
            prompt.body.clone(),
            prompt.confirm_label.clone(),
        );
    }
    let mib = (forskscope_compare::job::PerformanceLimits::default().medium_text_threshold_bytes
        / (1024 * 1024))
        .to_string();
    let body = if prompt.spreadsheet {
        tf(
            lang,
            "one-or-both-workbooks-exceed-the-recommended-comparison",
            &[("n", &mib)],
            None,
        )
    } else {
        tf(
            lang,
            "one-or-both-files-exceed-the-recommended-diff-limit-n-mib",
            &[("n", &mib)],
            None,
        )
    };
    (t(lang, "file-is-large"), body, t(lang, "diff-anyway"))
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
    let (title, body, confirm_label) = large_load_text(lang, &prompt);
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}", onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { "{title}" }
                p { "{body}" }
                div { class: "actions",
                    button { autofocus: true, onclick: move |_| store.modal.set(Modal::None), {t(lang, "cancel")} }
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
/// action is "first". `view.body` and the button labels still come from
/// `UserMessage`/`action_label`, English-only (`.detail` and the
/// recovery-button labels are a separate, larger system, out of review
/// 238 §1.4's scope) — but the dialog's own title (F238) is translated
/// here from `view.kind`, never `view.title` (`forskscope-ui-logic`'s
/// hardcoded English, kept only so a caller not displaying this view
/// still has the English text available).
#[component]
pub fn SaveErrorModal(index: usize, target: PathBuf, view: SaveErrorView) -> Element {
    let mut store = use_context::<Store>();
    let title = t(store.lang(), app_error_kind_short_id(view.kind));
    rsx! {
        div { class: "scrim", role: "dialog", aria_modal: "true", aria_label: "{title}", onmounted: super::focus_autofocus_button,
            div { class: "modal",
                h2 { "{title}" }
                p { "{view.body}" }
                if let Some(path) = &view.path {
                    code { class: "path-display", BidiText { text: path.clone(), lang: Lang::En } }
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

#[cfg(test)]
mod large_load_text_tests {
    use super::*;
    use crate::state::LargeLoadTarget;

    fn prompt(spreadsheet: bool, too_large: bool) -> LargeLoadPrompt {
        LargeLoadPrompt {
            target: LargeLoadTarget::Reload(0),
            opts: DiffOptions::default(),
            title: "ui-logic title".into(),
            body: "ui-logic body".into(),
            confirm_label: "ui-logic label".into(),
            too_large,
            spreadsheet,
        }
    }

    /// F122 B: the prompt must not promise an approximation for a workbook —
    /// the comparison is exact or refused (F117) — and must not for text
    /// either: what actually happens is a slow diff whose line diff is bounded
    /// at five seconds and says so.
    #[test]
    fn the_prompt_says_what_the_product_does_for_each_kind() {
        let (_, text, _) = large_load_text(Lang::En, &prompt(false, false));
        assert!(!text.contains("produce an approximate result"), "{text}");
        assert!(
            text.contains("5 seconds") && text.contains("highlighting"),
            "{text}"
        );

        let (_, book, _) = large_load_text(Lang::En, &prompt(true, false));
        assert!(book.contains("never approximate"), "{book}");
        assert!(!book.contains("line diff"), "{book}");
        assert_ne!(text, book);
    }

    #[test]
    fn the_size_in_the_prompt_is_the_guard_threshold_and_both_languages_differ() {
        let mib = (forskscope_compare::job::PerformanceLimits::default()
            .medium_text_threshold_bytes
            / (1024 * 1024))
            .to_string();
        for spreadsheet in [false, true] {
            let (t_en, en, l_en) = large_load_text(Lang::En, &prompt(spreadsheet, false));
            let (t_ja, ja, l_ja) = large_load_text(Lang::Ja, &prompt(spreadsheet, false));
            assert!(
                en.contains(&format!("({mib} MiB)")) && !en.contains("{n}"),
                "{en}"
            );
            assert!(
                ja.contains(&format!("（{mib} MiB）")) && !ja.contains("{n}"),
                "{ja}"
            );
            assert_ne!(en, ja, "the Japanese text must actually be used");
            assert_ne!(t_en, t_ja);
            assert_ne!(l_en, l_ja);
        }
    }

    /// A drift guard: the English text here and `ui-logic`'s own text for a
    /// text pair must stay the same sentence, or the two would say different
    /// things about the same prompt.
    #[test]
    fn the_localised_text_pair_body_matches_ui_logic_s_english() {
        let guard = forskscope_ui_logic::guard_for_sizes(5 * 1024 * 1024, 1024);
        let forskscope_ui_logic::LoadGuard::ConfirmPrompt { body, .. } = guard else {
            panic!("a 5 MiB file must prompt");
        };
        let (_, mine, _) = large_load_text(Lang::En, &prompt(false, false));
        assert_eq!(mine, body);
    }

    #[test]
    fn the_over_64_mib_prompt_is_ui_logic_s_text_unchanged() {
        let p = prompt(false, true);
        let (title, body, label) = large_load_text(Lang::Ja, &p);
        assert_eq!((title, body, label), (p.title, p.body, p.confirm_label));
    }
}

#[cfg(test)]
mod f235_bidi_name_markup_tests {
    use dioxus::prelude::*;

    use super::{OverwriteModal, SaveErrorModal};
    use crate::state::{AppSettings, Store};
    use forskscope_ui_logic::SaveErrorView;

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

    /// F235: a bidi control character in an overwrite target's own path
    /// never reaches the rendered confirmation modal raw — `.path-
    /// display` shows `BidiText`'s own marker instead. Falsify by
    /// reverting `OverwriteModal`'s `BidiText { text: path_display.clone(),
    /// lang }` back to a bare `"{path_display}"`: the raw character then
    /// reaches a `CreateTextNode` and this fails.
    #[test]
    fn a_bidi_control_character_in_an_overwrite_targets_own_path_never_reaches_the_rendered_page_raw()
     {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                OverwriteModal { index: 0usize, target: "/tmp/invoice\u{202E}fdp.exe".into() }
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

    /// F235: the same coverage, for `SaveErrorModal`'s own `view.path` —
    /// `path`/`BidiText` stay hardcoded `Lang::En` (not translated); only
    /// the dialog's title is (F238, via `view.kind`).
    #[test]
    fn a_bidi_control_character_in_a_save_errors_own_path_never_reaches_the_rendered_page_raw() {
        fn root() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            rsx! {
                SaveErrorModal {
                    index: 0usize,
                    target: "/tmp/invoice\u{202E}fdp.exe".into(),
                    view: SaveErrorView {
                        kind: forskscope_compare::error::AppErrorKind::InternalFault,
                        title: "Could not save".to_string(),
                        body: "Something went wrong.".to_string(),
                        path: Some("/tmp/invoice\u{202E}fdp.exe".to_string()),
                        buttons: Vec::new(),
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
