//! Explorer footer: targets label (progressive guidance) and Compare button
//! (RFC-069).

use dioxus::prelude::*;

use super::{CompareAction, PickKind, compare_action};
use crate::i18n::t;
use crate::state::{Lang, Store, open_compare, open_dir_compare};
use crate::ui::view::BidiText;
use crate::ui::view::dir_pane::{short_name, status_kind_label};
use forskscope_ui_logic::RowStatusKind;

#[component]
pub fn ExplorerFooter(
    lang: Lang,
    left_pick: Signal<Option<PickKind>>,
    right_pick: Signal<Option<PickKind>>,
    /// RFC-097 stage 3 §2: the status glyphs currently on screen in either
    /// pane, each with the `is_dir` its wording was drawn from. Computed by
    /// the caller (`explorer.rs::visible_status_kinds`) from the same live
    /// state the rows render their own badges from.
    status_key: Vec<(RowStatusKind, bool)>,
) -> Element {
    let mut store = use_context::<Store>();
    let lp = left_pick.read().clone();
    let rp = right_pick.read().clone();
    let action = compare_action(&lp, &rp);
    let can_compare = action != CompareAction::None;
    let compare_tooltip = match &action {
        CompareAction::Files(..) => t(lang, "Compare selected files"),
        CompareAction::Dirs(..) => t(lang, "Compare selected directories"),
        CompareAction::None => t(lang, "Select a file or directory on each side to compare"),
    };

    rsx! {
        div { class: "explorer-footer",
            // RFC-097 stage 3 §2: a plain, non-live line — not role="status",
            // since announcing it on every scroll/filter change would be
            // noise (the rows' own `role="img"` labels already speak for
            // themselves). Its own row, above the targets/Compare row, so it
            // never competes with Compare for width at 200%.
            if !status_key.is_empty() {
                div { class: "explorer-status-key",
                    for (i, (kind, is_dir)) in status_key.iter().enumerate() {
                        if i > 0 {
                            span { class: "status-key-sep", " · " }
                        }
                        span { class: "status-key-item",
                            span {
                                class: "status-key-glyph {kind.css_class()}",
                                aria_hidden: "true",
                                "{kind.glyph()}"
                            }
                            // F230: a non-breaking space, not a plain one -
                            // `.status-key-item` no longer forbids wrapping
                            // (a long label, Japanese especially, needs to
                            // wrap across lines rather than overflow), so
                            // this is what keeps the glyph glued to its own
                            // label's first character instead of landing
                            // alone at the end of a line.
                            "\u{a0}"
                            {status_kind_label(*kind, lang, *is_dir)}
                        }
                    }
                }
            }
            div { class: "explorer-footer-main",
                div { class: "targets-label",
                    {
                        let lp = left_pick.read();
                        let rp = right_pick.read();
                        match (&*lp, &*rp) {
                            (None, None) => rsx! {
                                span { class: "targets-hint",
                                    {t(lang, "Choose a file or folder on each side to compare")}
                                }
                            },
                            (Some(l), None) => rsx! {
                                span { class: "targets-pick", BidiText { text: short_name(l.path()), lang } }
                                span { class: "targets-sep", " ↔ " }
                                span { class: "targets-hint", {t(lang, "Choose a file or folder on the right")} }
                            },
                            (None, Some(r)) => rsx! {
                                span { class: "targets-hint", {t(lang, "Choose a file or folder on the left")} }
                                span { class: "targets-sep", " ↔ " }
                                span { class: "targets-pick", BidiText { text: short_name(r.path()), lang } }
                            },
                            (Some(l), Some(r)) => rsx! {
                                span { class: "targets-pick", BidiText { text: short_name(l.path()), lang } }
                                span { class: "targets-sep", " ↔ " }
                                span { class: "targets-pick", BidiText { text: short_name(r.path()), lang } }
                            },
                        }
                    }
                }
                button {
                    class: "compare-btn",
                    disabled: !can_compare,
                    title: compare_tooltip.clone(),
                    aria_label: compare_tooltip.clone(),
                    onclick: move |_| {
                        let lp = left_pick.read().clone();
                        let rp = right_pick.read().clone();
                        match compare_action(&lp, &rp) {
                            CompareAction::Files(l, r) => open_compare(&mut store, l, r),
                            CompareAction::Dirs(l, r)  => open_dir_compare(&mut store, l, r),
                            CompareAction::None => {}
                        }
                    },
                    {t(lang, "Compare")} " ▶"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppSettings;

    fn root(status_key: Vec<(RowStatusKind, bool)>) -> Element {
        use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
        let left_pick = Signal::new_in_scope(None, ScopeId::ROOT);
        let right_pick = Signal::new_in_scope(None, ScopeId::ROOT);
        rsx! {
            ExplorerFooter { lang: Lang::En, left_pick, right_pick, status_key }
        }
    }

    fn texts(mutations: &dioxus_core::Mutations) -> Vec<String> {
        mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// RFC-097 stage 3 §2: an empty key renders no `.explorer-status-key`
    /// line at all - checked by the absence of any status-row label text
    /// (`status_kind_label`'s own words), since the empty `if` branch
    /// itself is static template structure this harness cannot see
    /// directly. Falsify by dropping the `if !status_key.is_empty()` guard:
    /// nothing changes here (there is nothing to iterate either way), so
    /// this test alone would not catch that regression - the point is
    /// narrower: an empty key must not crash or fabricate an entry.
    #[test]
    fn an_empty_key_shows_no_status_line() {
        let mut vdom = VirtualDom::new_with_props(root, Vec::new());
        let mutations = vdom.rebuild_to_vec();
        assert!(
            !texts(&mutations)
                .iter()
                .any(|t| t == "Different" || t == "Identical — contents compared"),
            "an empty status_key must render no row label"
        );
    }

    /// The key lists each given kind's own row wording, in order, and
    /// nothing else — never all nine regardless of what was passed in.
    /// Falsify by hard-coding `is_dir: false` in `status_kind_label`'s
    /// call: the dir-form wording for `MetadataMatch` would then
    /// disappear, failing the second assertion.
    #[test]
    fn the_key_lists_exactly_the_given_kinds_with_their_own_wording() {
        let mut vdom = VirtualDom::new_with_props(
            root,
            vec![
                (RowStatusKind::Different, false),
                (RowStatusKind::MetadataMatch, true),
            ],
        );
        let mutations = vdom.rebuild_to_vec();
        let all_text = texts(&mutations).join(" | ");
        assert!(all_text.contains("Different"), "got {all_text:?}");
        assert!(
            all_text.contains("Names and sizes match; contents not compared"),
            "a dir-form MetadataMatch must use the dir wording, got {all_text:?}"
        );
        assert!(
            !all_text.contains("Size matches; contents not compared"),
            "must not also show the file-form wording for the same entry, got {all_text:?}"
        );
    }

    /// F235: a bidi control character in a picked target's own file name
    /// never reaches the rendered "Targets" line raw — `.targets-pick`
    /// shows `BidiText`'s own marker instead. Falsify by reverting any
    /// one of `ExplorerFooter`'s `BidiText { text: short_name(...), lang }`
    /// call sites back to a bare interpolation: the raw character then
    /// reaches a `CreateTextNode` and this fails.
    #[test]
    fn a_bidi_control_character_in_a_picked_targets_own_name_never_reaches_the_rendered_page_raw() {
        fn root_with_pick() -> Element {
            use_context_provider(|| Store::new(AppSettings::default(), Default::default(), false));
            let left_pick = Signal::new_in_scope(
                Some(super::PickKind::File(std::path::PathBuf::from(
                    "/tmp/invoice\u{202E}fdp.exe",
                ))),
                ScopeId::ROOT,
            );
            let right_pick = Signal::new_in_scope(None, ScopeId::ROOT);
            rsx! {
                ExplorerFooter { lang: Lang::En, left_pick, right_pick, status_key: Vec::new() }
            }
        }
        let mut vdom = VirtualDom::new(root_with_pick);
        let mutations = vdom.rebuild_to_vec();
        let all_texts = texts(&mutations);
        for text in &all_texts {
            assert!(
                !text.chars().any(forskscope_compare::is_bidi_control),
                "a bidi control character reached a rendered text node: {text:?}"
            );
        }
        assert!(
            all_texts.iter().any(|t| t.contains("RLO")),
            "expected the RLO marker token among the rendered text nodes: {all_texts:?}"
        );
    }
}
