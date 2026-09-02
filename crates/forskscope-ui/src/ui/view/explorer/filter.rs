//! Explorer filter bar (RFC-067): name-pattern input and hide-binary/identical
//! checkboxes. Also exposes `apply_filter` which narrows the aligned row list.

use std::collections::HashMap;
use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::dir::EqualityEvidence;
use forskscope_ui_logic::AlignedRow;

use super::DigestKey;
use crate::i18n::t;
use crate::state::Lang;

// ── Filter bar component ──────────────────────────────────────────────────────

#[component]
pub fn FilterBar(
    lang: Lang,
    filter_open: Signal<bool>,
    filter_query: Signal<String>,
    filter_hide_bin: Signal<bool>,
    filter_hide_eq: Signal<bool>,
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
                    onkeydown: move |e| filter_input_keydown(&e),
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

/// RFC-060/handoff 020 §5: the filter input's whole keyboard obligation is
/// swallowing every key so it cannot reach the global handler behind it
/// (e.g. Ctrl+S while typing a filter query must not save the active tab).
/// The one line this delegates to is exactly what the falsification test
/// below exercises directly, through this function — not a copy of it.
fn filter_input_keydown(e: &Event<KeyboardData>) {
    crate::keyboard::swallow_when_typing(e);
}

// ── Filter predicate ──────────────────────────────────────────────────────────

/// Apply the active filter to the aligned row list.
///
/// - `query`: lowercase name substring (empty = no filter).
/// - `hide_bin`: hide pairs where all present file sides are binary.
/// - `hide_eq`: hide pairs whose evidence is conclusively equal
///   (`EqualityEvidence::is_equal`).
/// - `binary_enabled`: when `true`, the binary gate is open; hide_bin has no effect.
pub fn apply_filter(
    rows: Vec<AlignedRow>,
    query: &str,
    hide_bin: bool,
    hide_eq: bool,
    binary_enabled: bool,
    digest_map: &HashMap<DigestKey, EqualityEvidence>,
    binary_cache: &mut Signal<HashMap<PathBuf, bool>>,
) -> Vec<AlignedRow> {
    rows.into_iter()
        .filter(|(lr, rr)| {
            // Name filter.
            let name_ok = if query.is_empty() {
                true
            } else {
                let l_match = lr
                    .as_ref()
                    .and_then(|r| r.rel_path.file_name())
                    .map(|n| n.to_string_lossy().to_lowercase().contains(query))
                    .unwrap_or(false);
                let r_match = rr
                    .as_ref()
                    .and_then(|r| r.rel_path.file_name())
                    .map(|n| n.to_string_lossy().to_lowercase().contains(query))
                    .unwrap_or(false);
                l_match || r_match
            };

            // Hide-binary filter (only meaningful when binary comparison is off).
            let bin_ok = if hide_bin && !binary_enabled {
                let mut is_bin = |path: &PathBuf| -> bool {
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
                let l_bin = lr
                    .as_ref()
                    .map(|r| !r.is_dir && is_bin(&r.abs_path))
                    .unwrap_or(false);
                let r_bin = rr
                    .as_ref()
                    .map(|r| !r.is_dir && is_bin(&r.abs_path))
                    .unwrap_or(false);
                match (lr.is_some(), rr.is_some()) {
                    (true, true) => !l_bin || !r_bin,
                    (true, false) => !l_bin,
                    (false, true) => !r_bin,
                    (false, false) => true,
                }
            } else {
                true
            };

            // Hide-identical filter. F74: a directory row is never proven
            // identical here - the Explorer doesn't examine directory
            // contents, so its evidence (`Unknown`, rendered as
            // `NotCompared`, never `Equal`) can't earn hiding either way.
            // Checking `is_dir` directly, not just the evidence, keeps this
            // exemption correct even if some future evidence ever
            // misrepresented a directory as equal again - hiding is for
            // rows *proven* identical, and a directory never is, regardless
            // of what its map entry says.
            let is_dir_row = lr.as_ref().map(|r| r.is_dir).unwrap_or(false)
                || rr.as_ref().map(|r| r.is_dir).unwrap_or(false);
            let eq_ok = if hide_eq && !is_dir_row {
                let rel = lr.as_ref().or(rr.as_ref()).map(|r| r.rel_path.clone());
                rel.map(|rel| {
                    !digest_map
                        .get(&DigestKey::Common(rel))
                        .is_some_and(EqualityEvidence::is_equal)
                })
                .unwrap_or(true)
            } else {
                true
            };

            name_ok && bin_ok && eq_ok
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
    use forskscope_ui_logic::RowData;

    use super::*;
    use crate::keyboard::test_support::key_event;
    use crate::state::with_test_store;

    /// Handoff 020 §6 test 3: `filter_input_keydown` is the exact function
    /// `FilterBar`'s `onkeydown` calls — not a copy of it — so removing its
    /// `swallow_when_typing` call must make this fail.
    #[test]
    fn filter_input_keydown_swallows_every_key() {
        let e = key_event(Key::Character("s".into()), Modifiers::CONTROL);
        assert!(e.propagates(), "test setup: a fresh event must propagate");
        filter_input_keydown(&e);
        assert!(
            !e.propagates(),
            "typing in the filter input must not let Ctrl+S (or any other \
             key) reach the global keyboard handler behind it"
        );
    }

    fn dir_row(name: &str) -> RowData {
        RowData {
            abs_path: PathBuf::from(format!("/root/{name}")),
            rel_path: PathBuf::from(name),
            is_dir: true,
            is_expanded: false,
            is_selected: false,
            depth: 0,
        }
    }

    // F74: hide-identical must never hide a directory row, whatever its
    // evidence says - a directory is never *proven* identical by this
    // Explorer, since its contents are never examined. Checks the
    // exemption against `DigestEqual` specifically (not `Unknown`, which
    // is what `classify_entry` actually produces for a directory pair and
    // is never treated as equal anyway) because the guard is meant to hold
    // regardless of what the map entry says, not because `Unknown` happens
    // to differ from an equal verdict.
    #[test]
    fn hide_identical_never_hides_a_directory_row_even_if_marked_equal() {
        with_test_store(|_store| {
            let mut binary_cache: Signal<HashMap<PathBuf, bool>> =
                Signal::new_in_scope(HashMap::new(), ScopeId::ROOT);
            let mut digest_map = HashMap::new();
            digest_map.insert(
                DigestKey::Common(PathBuf::from("same-name-dir")),
                EqualityEvidence::DigestEqual,
            );
            let rows = vec![(
                Some(dir_row("same-name-dir")),
                Some(dir_row("same-name-dir")),
            )];

            let visible = apply_filter(
                rows,
                "",
                false,
                true, // hide_eq
                true,
                &digest_map,
                &mut binary_cache,
            );

            assert_eq!(
                visible.len(),
                1,
                "a directory row must stay visible under hide-identical, \
                 regardless of its recorded state"
            );
        });
    }
}
