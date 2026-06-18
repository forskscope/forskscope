//! Explorer footer: targets label (progressive guidance) and Compare button
//! (RFC-069).

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::{Lang, Store, open_compare, open_dir_compare};
use super::{CompareAction, PickKind, compare_action};
use crate::ui::view::dir_pane::short_name;

#[component]
pub fn ExplorerFooter(
    lang:       Lang,
    left_pick:  Signal<Option<PickKind>>,
    right_pick: Signal<Option<PickKind>>,
) -> Element {
    let mut store = use_context::<Store>();
    let lp = left_pick.read().clone();
    let rp = right_pick.read().clone();
    let action = compare_action(&lp, &rp);
    let can_compare = action != CompareAction::None;
    let compare_tooltip = match &action {
        CompareAction::Files(..) => t(lang, "Compare selected files"),
        CompareAction::Dirs(..)  => t(lang, "Compare selected directories"),
        CompareAction::None      => t(lang, "Select a file or directory on each side to compare"),
    };

    rsx! {
        div { class: "explorer-footer",
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
                            span { class: "targets-pick", {short_name(l.path())} }
                            span { class: "targets-sep", " ↔ " }
                            span { class: "targets-hint", {t(lang, "Choose a file or folder on the right")} }
                        },
                        (None, Some(r)) => rsx! {
                            span { class: "targets-hint", {t(lang, "Choose a file or folder on the left")} }
                            span { class: "targets-sep", " ↔ " }
                            span { class: "targets-pick", {short_name(r.path())} }
                        },
                        (Some(l), Some(r)) => rsx! {
                            span { class: "targets-pick", {short_name(l.path())} }
                            span { class: "targets-sep", " ↔ " }
                            span { class: "targets-pick", {short_name(r.path())} }
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
