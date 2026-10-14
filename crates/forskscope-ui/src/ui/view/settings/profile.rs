//! Compare profile list and add-profile inline form (RFC-057, RFC-009).

use dioxus::prelude::*;

use forskscope_core::NewlineCompareMode;
use forskscope_core::WhitespaceMode;
use forskscope_core::diff::InlineMode;

use crate::i18n::t;
use crate::state::Store;

/// Inline form for adding a new compare profile (revealed on demand, RFC-057).
///
/// F168 §2.2: the visible row is unchanged apart from the "Ignore
/// whitespace" rename. "More options ▸", collapsed by default, holds only
/// the character-level-highlighting control — not a whitespace-mode
/// selector. `DiffOptions.ignore_whitespace` is a single bool the engine
/// reads as "strip every whitespace character" (`engine.rs`'s `line_key`);
/// `IgnoreTrailing` and `IgnoreBlankLines` would behave identically to
/// `IgnoreAll` today, so offering them as distinct choices would say
/// something untrue about what picking one does. `whitespace: WhitespaceMode`
/// already carries losslessly (§2.1) for whenever the engine gains that
/// distinction; naming it here as a choice must wait for that.
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
                placeholder: t(lang, "Profile name"),
                value: "{name}",
                oninput: move |e| name.set(e.value()),
                style: "flex:1;"
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_ws.read(),
                    onchange: move |e| ignore_ws.set(e.checked()) }
                span { {t(lang, "Ignore whitespace")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_case.read(),
                    onchange: move |e| ignore_case.set(e.checked()) }
                span { {t(lang, "Ignore case")} }
            }
            label { class: "profile-check",
                input { r#type: "checkbox", checked: *ignore_newlines.read(),
                    onchange: move |e| ignore_newlines.set(e.checked()) }
                span { {t(lang, "Ignore line-ending differences (CRLF/LF)")} }
            }
            select {
                onchange: move |e| {
                    algorithm.set(match e.value().as_str() {
                        "patience"  => crate::state::DiffAlgorithmSetting::Patience,
                        "histogram" => crate::state::DiffAlgorithmSetting::Histogram,
                        _           => crate::state::DiffAlgorithmSetting::Myers,
                    });
                },
                option { value: "myers",     "Myers"     }
                option { value: "patience",  "Patience"  }
                option { value: "histogram", "Histogram" }
            }
            button {
                class: "more-options-toggle",
                onclick: move |_| { let v = *more_options.read(); more_options.set(!v); },
                if *more_options.read() { "▾ " {t(lang, "More options")} }
                else                    { "▸ " {t(lang, "More options")} }
            }
            if *more_options.read() {
                div { class: "add-profile-more-options",
                    label { r#for: "profile-inline-mode", {t(lang, "Character-level highlighting")} }
                    select {
                        id: "profile-inline-mode",
                        onchange: move |e| {
                            inline_mode.set(match e.value().as_str() {
                                "eager" => InlineMode::EagerForSmallHunks,
                                "never" => InlineMode::None,
                                _       => InlineMode::Lazy,
                            });
                        },
                        option { value: "lazy",  selected: *inline_mode.read() == InlineMode::Lazy,
                            {t(lang, "When I turn it on")} }
                        option { value: "eager", selected: *inline_mode.read() == InlineMode::EagerForSmallHunks,
                            {t(lang, "Automatically for small changes")} }
                        option { value: "never", selected: *inline_mode.read() == InlineMode::None,
                            {t(lang, "Never")} }
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
                {t(lang, "Add")}
            }
            button {
                onclick: move |_| on_done.call(()),
                {t(lang, "Cancel")}
            }
        }
    }
}
