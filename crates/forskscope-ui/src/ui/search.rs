//! Inline search within a diff comparison (RFC-014 §"Text Search", RFC-059 §M4).
//!
//! `SearchCtx` is provided as a Dioxus context at the `DiffWorkspace` level.
//! The toolbar's `SearchBar` owns the query input and Prev/Next navigation;
//! the individual `Row` components read the context to apply match highlights;
//! `diff.rs` rebuilds the index on query change and triggers scroll-into-view.

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::ui::search_index::MatchIndex;

// ── SearchCtx ─────────────────────────────────────────────────────────────────

/// Shared search state provided as a Dioxus context by `DiffWorkspace`.
#[derive(Clone, Default)]
pub struct SearchCtx {
    pub query:  String,
    pub active: bool,
    /// Ordered match positions rebuilt by `DiffWorkspace` on every query change.
    pub index:  MatchIndex,
}

impl PartialEq for SearchCtx {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query && self.active == other.active
        // index is excluded from equality so signal updates aren't suppressed
        // when only the focused match changes.
    }
}

// ── SearchBar component ───────────────────────────────────────────────────────

/// Compact search bar with Prev / Next match navigation.
///
/// The parent must have called `use_context_provider` to provide
/// `Signal<SearchCtx>` before mounting this component.
#[component]
pub fn SearchBar() -> Element {
    let mut ctx: Signal<SearchCtx> = use_context::<Signal<SearchCtx>>();
    let store = use_context::<crate::state::Store>();
    let lang = store.lang();
    let active = ctx.read().active;
    if !active {
        return rsx! {};
    }

    let total   = ctx.read().index.len();
    let focused = ctx.read().index.focused_number();
    let query_empty = ctx.read().query.is_empty();

    let count_label = if query_empty || total == 0 {
        if !query_empty { t(lang, "No matches") } else { String::new() }
    } else {
        match focused {
            Some(n) => format!("{n} / {total}"),
            None    => if total == 1 { format!("{total} {}", t(lang, "match")) } else { format!("{total} {}", t(lang, "matches")) },
        }
    };

    rsx! {
        div { class: "search-bar",
            input {
                class: "search-input",
                r#type: "text",
                placeholder: t(lang, "Search…"),
                autofocus: true,
                value: "{ctx.read().query}",
                "aria-label": t(lang, "Search within diff"),
                oninput: move |e| {
                    ctx.write().query = e.value();
                    // Index rebuilt by DiffWorkspace on the next render cycle
                    // (it reads `ctx.read().query` in its snapshot computation).
                },
                onkeydown: move |e| {
                    match e.key() {
                        Key::Escape => {
                            let mut c = ctx.write();
                            c.active = false;
                            c.query.clear();
                            c.index = MatchIndex::default();
                        }
                        Key::Enter => {
                            if e.modifiers().shift() {
                                ctx.write().index.retreat();
                            } else {
                                ctx.write().index.advance();
                            }
                            scroll_to_focused(&ctx.read());
                        }
                        _ => {}
                    }
                },
            }

            // Prev / Next buttons
            button {
                id: "search-prev-btn",
                class: "search-nav",
                disabled: total == 0,
                title: t(lang, "Previous match (Shift+Enter)"),
                "aria-label": t(lang, "Previous match"),
                onclick: move |_| {
                    ctx.write().index.retreat();
                    scroll_to_focused(&ctx.read());
                },
                "▲"
            }
            button {
                id: "search-next-btn",
                class: "search-nav",
                disabled: total == 0,
                title: t(lang, "Next match (Enter / F3)"),
                "aria-label": t(lang, "Next match"),
                onclick: move |_| {
                    ctx.write().index.advance();
                    scroll_to_focused(&ctx.read());
                },
                "▼"
            }

            if !count_label.is_empty() {
                span {
                    class: if total == 0 && !query_empty { "search-count no-matches" }
                           else { "search-count" },
                    "aria-live": "polite",
                    "{count_label}"
                }
            }

            button {
                class: "search-close",
                "aria-label": t(lang, "Close search bar"),
                onclick: move |_| {
                    let mut c = ctx.write();
                    c.active = false;
                    c.query.clear();
                    c.index = MatchIndex::default();
                },
                "×"
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Scroll the focused match's hunk element into view via document.eval.
pub fn scroll_to_focused(ctx: &SearchCtx) {
    if let Some(pos) = ctx.index.focused() {
        let id = pos.hunk_elem_id.clone();
        spawn(async move {
            let _ = dioxus::document::eval(
                &format!("document.getElementById({id:?})?.scrollIntoView({{block:'nearest',behavior:'smooth'}});")
            ).await;
        });
    }
}

/// Whether a line's content matches the current active query.
/// Case-insensitive substring match. `false` when inactive or query is empty.
pub fn line_matches(ctx: &SearchCtx, content: &str) -> bool {
    if !ctx.active || ctx.query.is_empty() { return false; }
    content.to_ascii_lowercase().contains(&ctx.query.to_ascii_lowercase())
}
