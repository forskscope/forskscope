//! Inline search within a diff comparison (RFC-006 §D-003).
//!
//! `SearchCtx` is provided as a Dioxus context at the `DiffWorkspace`
//! level; both the toolbar's `SearchBar` and the individual `Row`
//! components read it so matches are highlighted without prop-drilling.

use dioxus::prelude::*;


/// Shared search state provided by `DiffWorkspace`.
#[derive(Clone, PartialEq, Default)]
pub struct SearchCtx {
    pub query:  String,
    pub active: bool,
}

/// Compact search bar rendered inside the diff toolbar when activated.
///
/// The parent must have called `use_context_provider` to provide
/// `Signal<SearchCtx>` before mounting this component.
#[component]
pub fn SearchBar(match_count: usize) -> Element {
    let mut ctx: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let active = ctx.read().active;
    if !active {
        return rsx! {};
    }
    let count_label = if ctx.read().query.is_empty() {
        String::new()
    } else {
        format!("{match_count} match{}", if match_count == 1 { "" } else { "es" })
    };
    rsx! {
        div { class: "search-bar",
            input {
                class: "search-input",
                placeholder: "Search…",
                autofocus: true,
                value: "{ctx.read().query}",
                oninput: move |e| {
                    ctx.write().query = e.value();
                },
                onkeydown: move |e| {
                    if e.key() == dioxus::html::input_data::keyboard_types::Key::Escape {
                        let mut c = ctx.write();
                        c.active = false;
                        c.query.clear();
                    }
                },
            }
            if !count_label.is_empty() {
                span { class: "search-count", "{count_label}" }
            }
            button {
                class: "search-close",
                onclick: move |_| {
                    let mut c = ctx.write();
                    c.active = false;
                    c.query.clear();
                },
                "×"
            }
        }
    }
}

/// Whether the given line content matches the current active search query.
/// Case-insensitive. Returns `false` when search is inactive or query is empty.
pub fn line_matches(ctx: &SearchCtx, content: &str) -> bool {
    if !ctx.active || ctx.query.is_empty() {
        return false;
    }
    let q = ctx.query.to_ascii_lowercase();
    content.to_ascii_lowercase().contains(&q)
}
