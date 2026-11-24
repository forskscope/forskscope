//! Compare profile list and add-profile inline form (RFC-057, RFC-009).

use dioxus::prelude::*;

use forskscope_compare::NewlineCompareMode;
use forskscope_compare::WhitespaceMode;
use forskscope_compare::diff::InlineMode;

use crate::i18n::t;
use crate::state::Store;
use crate::ui::component::select_field::{SelectField, SelectOption};

/// Inline form for adding a new compare profile (revealed on demand, RFC-057).
///
/// F168 §2.2: the visible row is unchanged apart from the "Ignore
/// whitespace" rename. "More options ▸", collapsed by default, holds only
/// the character-level-highlighting control — not a whitespace-mode
/// selector. `DiffOptions.ignore_whitespace` is a single bool the engine
/// reads as "strip every whitespace character" (`engine.rs`'s `line_key`),
/// and `WhitespaceMode` itself now has only one non-default choice
/// (`IgnoreAll`) after F191 (0.199.0) retired `IgnoreTrailing`/
/// `IgnoreBlankLines`, which never behaved as their names promised — so
/// there is nothing finer to offer here yet. `whitespace: WhitespaceMode`
/// is still the field the dialog writes, not the bool directly, for
/// whenever the engine gains a real distinction to name as a choice.
#[component]
pub fn AddProfileInline(on_done: EventHandler<()>) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let mut name = use_signal(String::new);
    #[allow(unused_mut)]
    let mut ignore_ws = use_signal(|| false);
    #[allow(unused_mut)]
    let mut ignore_case = use_signal(|| false);
    #[allow(unused_mut)]
    let mut algorithm: Signal<crate::state::DiffAlgorithmSetting> = use_signal(Default::default);
    let mut ignore_newlines = use_signal(|| false);
    #[allow(unused_mut)]
    let mut inline_mode: Signal<InlineMode> = use_signal(|| InlineMode::Lazy);
    let mut more_options = use_signal(|| false);

    rsx! {
        div { class: "add-profile-form",
            input {
                placeholder: t(lang, "profile-name"),
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                style: "flex:1;"
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_ws.read(),
                    onchange: move |e| ignore_ws.set(e.checked()) }
                span { {t(lang, "ignore-whitespace")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_case.read(),
                    onchange: move |e| ignore_case.set(e.checked()) }
                span { {t(lang, "ignore-case")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_newlines.read(),
                    onchange: move |e| ignore_newlines.set(e.checked()) }
                span { {t(lang, "ignore-line-ending-differences-crlf-lf")} }
            }
            {
                let algo_value = match *algorithm.read() {
                    crate::state::DiffAlgorithmSetting::Patience  => "patience",
                    crate::state::DiffAlgorithmSetting::Histogram => "histogram",
                    _                                              => "myers",
                };
                rsx! {
                    SelectField {
                        value: algo_value.to_string(),
                        options: vec![
                            SelectOption::new("myers", "Myers"),
                            SelectOption::new("patience", "Patience"),
                            SelectOption::new("histogram", "Histogram"),
                        ],
                        onchange: move |v: String| {
                            algorithm.set(match v.as_str() {
                                "patience"  => crate::state::DiffAlgorithmSetting::Patience,
                                "histogram" => crate::state::DiffAlgorithmSetting::Histogram,
                                _           => crate::state::DiffAlgorithmSetting::Myers,
                            });
                        },
                    }
                }
            }
            button {
                class: "more-options-toggle",
                onclick: move |_| { let v = *more_options.read(); more_options.set(!v); },
                if *more_options.read() { "▾ " {t(lang, "more-options")} }
                else                    { "▸ " {t(lang, "more-options")} }
            }
            if *more_options.read() {
                div { class: "add-profile-more-options",
                    label { r#for: "profile-inline-mode", {t(lang, "character-level-highlighting")} }
                    {
                        let inline_value = match *inline_mode.read() {
                            InlineMode::EagerForSmallHunks => "eager",
                            InlineMode::None               => "never",
                            InlineMode::Lazy               => "lazy",
                        };
                        rsx! {
                            SelectField {
                                id: "profile-inline-mode".to_string(),
                                value: inline_value.to_string(),
                                options: vec![
                                    SelectOption::new("lazy", t(lang, "when-i-turn-it-on")),
                                    SelectOption::new("eager", t(lang, "automatically-for-small-changes")),
                                    SelectOption::new("never", t(lang, "never")),
                                ],
                                onchange: move |v: String| {
                                    inline_mode.set(match v.as_str() {
                                        "eager" => InlineMode::EagerForSmallHunks,
                                        "never" => InlineMode::None,
                                        _       => InlineMode::Lazy,
                                    });
                                },
                            }
                        }
                    }
                }
            }
            button {
                disabled: name.read().trim().is_empty(),
                onclick: move |_| {
                    let n = name.read().trim().to_string();
                    if !n.is_empty() {
                        crate::state::add_profile(
                            &mut store, n,
                            if *ignore_ws.read() { WhitespaceMode::IgnoreAll } else { WhitespaceMode::Significant },
                            *ignore_case.read(),
                            *algorithm.read(),
                            if *ignore_newlines.read() {
                                NewlineCompareMode::IgnoreDifference
                            } else {
                                NewlineCompareMode::Significant
                            },
                            *inline_mode.read(),
                        );
                        on_done.call(());
                    }
                },
                {t(lang, "add")}
            }
            button {
                onclick: move |_| on_done.call(()),
                {t(lang, "cancel")}
            }
        }
    }
}
