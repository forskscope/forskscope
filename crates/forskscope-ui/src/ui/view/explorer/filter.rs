//! Explorer filter bar (RFC-067): name-pattern input and hide-binary/identical
//! checkboxes. Also exposes `apply_filter` which narrows the aligned row list.

use std::collections::HashMap;
use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_ui_logic::AlignedRow;

use crate::i18n::t;
use crate::state::Lang;
use crate::ui::view::dir_pane::DigestState;
use super::{DigestKey};

// ── Filter bar component ──────────────────────────────────────────────────────

#[component]
pub fn FilterBar(
    lang:            Lang,
    filter_open:     Signal<bool>,
    filter_query:    Signal<String>,
    filter_hide_bin: Signal<bool>,
    filter_hide_eq:  Signal<bool>,
) -> Element {
    rsx! {
        div { class: "filter-bar-row",
            button {
                class: if *filter_open.read() { "filter-toggle active" } else { "filter-toggle" },
                title: t(lang, "Filter items"),
                aria_label: t(lang, "Filter items"),
                onclick: move |_| { let v = *filter_open.read(); filter_open.set(!v); },
                "⊞"
            }
            if *filter_open.read() {
                input {
                    class: "filter-input",
                    r#type: "search",
                    placeholder: t(lang, "Filter by name…"),
                    value: "{filter_query}",
                    oninput: move |e| filter_query.set(e.value()),
                    onkeydown: move |e| { e.stop_propagation(); },
                }
                label { class: "filter-check",
                    input { r#type: "checkbox", checked: *filter_hide_bin.read(),
                        onchange: move |e| filter_hide_bin.set(e.checked()) }
                    span { {t(lang, "Hide binary")} }
                }
                label { class: "filter-check",
                    input { r#type: "checkbox", checked: *filter_hide_eq.read(),
                        onchange: move |e| filter_hide_eq.set(e.checked()) }
                    span { {t(lang, "Hide identical")} }
                }
                if !filter_query.read().is_empty() || *filter_hide_bin.read() || *filter_hide_eq.read() {
                    button {
                        class: "filter-clear",
                        title: t(lang, "Clear filter"),
                        onclick: move |_| {
                            filter_query.set(String::new());
                            filter_hide_bin.set(false);
                            filter_hide_eq.set(false);
                        },
                        "✕"
                    }
                }
            }
        }
    }
}

// ── Filter predicate ──────────────────────────────────────────────────────────

/// Apply the active filter to the aligned row list.
///
/// - `query`: lowercase name substring (empty = no filter).
/// - `hide_bin`: hide pairs where all present file sides are binary.
/// - `hide_eq`: hide pairs whose digest is `DigestState::Equal`.
/// - `binary_enabled`: when `true`, the binary gate is open; hide_bin has no effect.
pub fn apply_filter(
    rows:           Vec<AlignedRow>,
    query:          &str,
    hide_bin:       bool,
    hide_eq:        bool,
    binary_enabled: bool,
    digest_map:     &HashMap<DigestKey, DigestState>,
    binary_cache:   &mut Signal<HashMap<PathBuf, bool>>,
) -> Vec<AlignedRow> {
    rows.into_iter().filter(|(lr, rr)| {
        // Name filter.
        let name_ok = if query.is_empty() { true } else {
            let l_match = lr.as_ref().and_then(|r| r.rel_path.file_name())
                .map(|n| n.to_string_lossy().to_lowercase().contains(query))
                .unwrap_or(false);
            let r_match = rr.as_ref().and_then(|r| r.rel_path.file_name())
                .map(|n| n.to_string_lossy().to_lowercase().contains(query))
                .unwrap_or(false);
            l_match || r_match
        };

        // Hide-binary filter (only meaningful when binary comparison is off).
        let bin_ok = if hide_bin && !binary_enabled {
            let is_bin = |path: &PathBuf| -> bool {
                let cached = binary_cache.read().get(path).copied();
                cached.unwrap_or_else(|| {
                    let b = matches!(
                        forskscope_core::file_kind::classify(path),
                        Ok(forskscope_core::file_kind::FileKind::Binary)
                    );
                    binary_cache.write().insert(path.clone(), b);
                    b
                })
            };
            let l_bin = lr.as_ref().map(|r| !r.is_dir && is_bin(&r.abs_path)).unwrap_or(false);
            let r_bin = rr.as_ref().map(|r| !r.is_dir && is_bin(&r.abs_path)).unwrap_or(false);
            match (lr.is_some(), rr.is_some()) {
                (true,  true)  => !l_bin || !r_bin,
                (true,  false) => !l_bin,
                (false, true)  => !r_bin,
                (false, false) => true,
            }
        } else { true };

        // Hide-identical filter.
        let eq_ok = if hide_eq {
            let rel = lr.as_ref().or(rr.as_ref()).map(|r| r.rel_path.clone());
            rel.map(|rel| !matches!(
                digest_map.get(&DigestKey::Common(rel)),
                Some(DigestState::Equal)
            )).unwrap_or(true)
        } else { true };

        name_ok && bin_ok && eq_ok
    }).collect()
}
