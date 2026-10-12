//! Explorer filter bar (RFC-067): name-pattern input and hide-binary/identical
//! checkboxes. Also exposes `apply_filter` which narrows the aligned row list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use forskscope_core::dir::EqualityEvidence;
use forskscope_ui_logic::{AlignedRow, FlatRow};

use super::DigestKey;
use crate::i18n::t;
use crate::state::Lang;

// ── Filter bar component ──────────────────────────────────────────────────────

/// F187: whether a filter is narrowing what the tree shows right now — true
/// whenever a query is typed or either *Hide* option is ticked, regardless
/// of whether the filter bar itself is open or closed. The one function the
/// render and the tests both call (F169/review 151): the *Filter* button's
/// class/dot and accessible name are driven by this, never by `filter_open`
/// directly — a filtered row that stays hidden behind a closed, unmarked
/// button is F166's kind of fault, the screen saying something untrue.
pub(crate) fn filter_in_effect(query: &str, hide_bin: bool, hide_eq: bool) -> bool {
    !query.is_empty() || hide_bin || hide_eq
}

#[component]
pub fn FilterBar(
    lang: Lang,
    filter_open: Signal<bool>,
    filter_query: Signal<String>,
    filter_hide_bin: Signal<bool>,
    filter_hide_eq: Signal<bool>,
    /// F151 (issue #148): while on, the two panes show the same location.
    /// Always shown, at the end of the row (owner finding, handoff 074: it
    /// has nothing of its own to disclose, so it does not sit inside
    /// Filter's own group) - matching the Filter button's own keyboard
    /// treatment (none beyond the click itself) rather than giving sync a
    /// binding Filter does not have (handoff 062 §2).
    sync_locations: Signal<bool>,
) -> Element {
    let in_effect = filter_in_effect(
        &filter_query.read(),
        *filter_hide_bin.read(),
        *filter_hide_eq.read(),
    );
    rsx! {
        div { class: "filter-bar-row",
            // Owner finding, handoff 074 verification: Filter and its own
            // expanded controls (the input, the two checkboxes, Clear) must
            // sit next to each other — they used to have *Move together*
            // between them, an unrelated toggle interrupting one control's
            // own disclosure. Filter's whole group comes first now; Move
            // together, with nothing of its own to disclose, comes last.
            button {
                class: "filter-toggle",
                aria_expanded: if *filter_open.read() { "true" } else { "false" },
                title: t(lang, if in_effect { "Filter (on)" } else { "Filter" }),
                aria_label: t(lang, if in_effect { "Filter (on)" } else { "Filter" }),
                onclick: move |_| { let v = *filter_open.read(); filter_open.set(!v); },
                {t(lang, "Filter")}
                // F187: always rendered, so nothing else on the row shifts
                // sideways when a filter turns on or off — only its own
                // colour changes, never its presence.
                span { class: if in_effect { "filter-dot on" } else { "filter-dot" }, aria_hidden: "true" }
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
            button {
                class: "filter-toggle",
                aria_pressed: if *sync_locations.read() { "true" } else { "false" },
                title: t(lang, "When one pane moves to a folder, the other moves to the matching folder, if it has one."),
                aria_label: t(lang, "Move together"),
                onclick: move |_| { let v = *sync_locations.read(); sync_locations.set(!v); },
                {t(lang, "Move together")}
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

// ── Entry predicates and pair predicates (review 150 §3) ─────────────────────
//
// A filter predicate must say whether it is about an **entry** (one path on one
// side) or a **pair** (a row, two entries). The two layouts mean different things by a
// row: aligned pairs entries of the same name, so a pair is one entry seen twice;
// compact pairs by position, so a pair is two unrelated entries. Entry predicates are
// therefore applied to each side before pairing (`filter_flat`), and only aligned,
// whose pairs are same-named, applies the pair predicates in `apply_filter`.

/// Entry: the lowercase `query` is a substring of the entry's file name. An empty
/// query matches everything.
fn entry_name_matches(rel: &Path, query: &str) -> bool {
    query.is_empty()
        || rel
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase().contains(query))
            .unwrap_or(false)
}

/// Entry: whether the file at `path` is binary, cached per path.
fn is_binary_cached(path: &PathBuf, binary_cache: &mut Signal<HashMap<PathBuf, bool>>) -> bool {
    let cached = binary_cache.read().get(path).copied();
    cached.unwrap_or_else(|| {
        let b = matches!(
            forskscope_core::file_kind::classify(path),
            Ok(forskscope_core::file_kind::FileKind::Binary)
        );
        binary_cache.write().insert(path.clone(), b);
        b
    })
}

/// Entry predicates, applied to one side's visible entries before compact packs them by
/// position. Each entry is judged alone: a name it does not match is hidden, whatever
/// its neighbour in the other pane is. A filtered-out entry leaves no gap.
///
/// - Name: the entry's name contains `query`.
/// - Hide binary (only when binary comparison is off): a binary file is hidden.
/// - Hide identical: an entry whose `Common` evidence `is_equal()` is hidden. Directories
///   are exempt (F74): the Explorer never proves a directory identical.
#[allow(clippy::too_many_arguments)]
pub fn filter_flat(
    rows: &[FlatRow],
    root: &Path,
    query: &str,
    hide_bin: bool,
    hide_eq: bool,
    binary_enabled: bool,
    digest_map: &HashMap<DigestKey, EqualityEvidence>,
    binary_cache: &mut Signal<HashMap<PathBuf, bool>>,
) -> Vec<FlatRow> {
    rows.iter()
        .filter(|(abs, is_dir, ..)| {
            let Ok(rel) = abs.strip_prefix(root) else {
                return false;
            };
            entry_name_matches(rel, query)
                && !(hide_bin && !binary_enabled && !is_dir && is_binary_cached(abs, binary_cache))
                && !(hide_eq
                    && !is_dir
                    && digest_map
                        .get(&DigestKey::Common(rel.to_path_buf()))
                        .is_some_and(EqualityEvidence::is_equal))
        })
        .cloned()
        .collect()
}

/// Every row the tree shows, for either layout, in the one order each layout needs
/// (review 150 §3; review 151 §3). This is the only function that builds rows: the
/// app calls it, and the tests call it too, so the composition under test is the
/// composition that runs.
///
/// Compact filters each side's entries (entry predicates), then packs them by position.
/// Aligned pairs same-named entries, then filters the pairs (pair predicates).
#[allow(clippy::too_many_arguments)]
pub fn rows_for_layout(
    compact: bool,
    left_flat: &[FlatRow],
    right_flat: &[FlatRow],
    roots: (&Path, &Path),
    query: &str,
    hide: (bool, bool),
    binary_enabled: bool,
    digest_map: &HashMap<DigestKey, EqualityEvidence>,
    binary_cache: &mut Signal<HashMap<PathBuf, bool>>,
) -> Vec<AlignedRow> {
    let (l_root, r_root) = roots;
    let (hide_bin, hide_eq) = hide;
    if compact {
        let left = filter_flat(
            left_flat,
            l_root,
            query,
            hide_bin,
            hide_eq,
            binary_enabled,
            digest_map,
            binary_cache,
        );
        let right = filter_flat(
            right_flat,
            r_root,
            query,
            hide_bin,
            hide_eq,
            binary_enabled,
            digest_map,
            binary_cache,
        );
        forskscope_ui_logic::pair_by_index(&left, &right, l_root, r_root)
    } else {
        let rows = forskscope_ui_logic::compute_aligned_rows(left_flat, right_flat, l_root, r_root);
        apply_filter(
            rows,
            query,
            hide_bin,
            hide_eq,
            binary_enabled,
            digest_map,
            binary_cache,
        )
    }
}

/// Apply the active filter to the aligned row list. **A pair predicate**: each row is
/// judged as a pair of same-named entries, which is true of aligned rows and of no
/// compact row (compact filters entries first, through [`filter_flat`]).
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
            // Name filter: the same name on either side keeps the row.
            let name_ok = query.is_empty()
                || lr
                    .as_ref()
                    .is_some_and(|r| entry_name_matches(&r.rel_path, query))
                || rr
                    .as_ref()
                    .is_some_and(|r| entry_name_matches(&r.rel_path, query));

            // Hide-binary filter (only meaningful when binary comparison is off).
            let bin_ok = if hide_bin && !binary_enabled {
                let l_bin = lr
                    .as_ref()
                    .map(|r| !r.is_dir && is_binary_cached(&r.abs_path, binary_cache))
                    .unwrap_or(false);
                let r_bin = rr
                    .as_ref()
                    .map(|r| !r.is_dir && is_binary_cached(&r.abs_path, binary_cache))
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

    // ── F187: filter_in_effect ───────────────────────────────────────────────

    #[test]
    fn no_query_and_no_hide_option_is_not_in_effect() {
        assert!(!filter_in_effect("", false, false));
    }

    #[test]
    fn a_query_is_in_effect() {
        assert!(filter_in_effect("x", false, false));
    }

    #[test]
    fn hide_binary_alone_is_in_effect() {
        assert!(filter_in_effect("", true, false));
    }

    #[test]
    fn hide_identical_alone_is_in_effect() {
        assert!(filter_in_effect("", false, true));
    }

    /// The render must call this exact function — not re-derive the same
    /// answer from `filter_open` — so a `FilterBar` with the bar **closed**
    /// but a filter **in effect** still renders the "on" state. Falsify by
    /// making the button's class/label depend on `filter_open` again (as it
    /// did before F187): with the bar closed, this fails.
    #[test]
    fn the_filter_button_shows_on_even_while_the_bar_is_closed() {
        fn root() -> Element {
            rsx! {
                FilterBar {
                    lang: Lang::En,
                    filter_open: Signal::new_in_scope(false, ScopeId::ROOT),
                    filter_query: Signal::new_in_scope("pattern".to_string(), ScopeId::ROOT),
                    filter_hide_bin: Signal::new_in_scope(false, ScopeId::ROOT),
                    filter_hide_eq: Signal::new_in_scope(false, ScopeId::ROOT),
                    sync_locations: Signal::new_in_scope(false, ScopeId::ROOT),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let aria_labels: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-label",
                    value: dioxus_core::AttributeValue::Text(s),
                    ..
                } => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert!(
            aria_labels.iter().any(|l| l == "Filter (on)"),
            "expected an aria-label \"Filter (on)\" with the bar closed and a \
             query typed, among {aria_labels:?}"
        );
        assert!(
            !aria_labels.iter().any(|l| l == "Filter"),
            "the bar-closed, no-filter label must not appear once a filter \
             is in effect: {aria_labels:?}"
        );
    }

    /// The converse: nothing typed, nothing ticked — the plain label, not
    /// "(on)" — whether the bar is open or closed.
    #[test]
    fn the_filter_button_shows_the_plain_label_when_nothing_is_active() {
        fn root() -> Element {
            rsx! {
                FilterBar {
                    lang: Lang::En,
                    filter_open: Signal::new_in_scope(true, ScopeId::ROOT),
                    filter_query: Signal::new_in_scope(String::new(), ScopeId::ROOT),
                    filter_hide_bin: Signal::new_in_scope(false, ScopeId::ROOT),
                    filter_hide_eq: Signal::new_in_scope(false, ScopeId::ROOT),
                    sync_locations: Signal::new_in_scope(false, ScopeId::ROOT),
                }
            }
        }
        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();
        let aria_labels: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "aria-label",
                    value: dioxus_core::AttributeValue::Text(s),
                    ..
                } => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert!(
            aria_labels.iter().any(|l| l == "Filter"),
            "expected the plain \"Filter\" aria-label, among {aria_labels:?}"
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

    fn file_row(name: &str) -> RowData {
        RowData {
            is_dir: false,
            ..dir_row(name)
        }
    }

    // RFC-080 §2: a tier-1 match is not equality, so *hide identical* keeps it
    // visible. Falsify by making `EqualityEvidence::is_equal` true for
    // `MetadataMatch`: the file row below is hidden and this fails.
    #[test]
    fn hide_identical_keeps_a_tier_1_match_visible() {
        with_test_store(|_store| {
            let mut binary_cache: Signal<HashMap<PathBuf, bool>> =
                Signal::new_in_scope(HashMap::new(), ScopeId::ROOT);
            let mut digest_map = HashMap::new();
            for name in ["dir", "file"] {
                digest_map.insert(
                    DigestKey::Common(PathBuf::from(name)),
                    EqualityEvidence::MetadataMatch,
                );
            }
            let rows = vec![
                (Some(dir_row("dir")), Some(dir_row("dir"))),
                (Some(file_row("file")), Some(file_row("file"))),
            ];
            let visible = apply_filter(rows, "", false, true, true, &digest_map, &mut binary_cache);
            assert_eq!(visible.len(), 2, "a tier-1 match must stay visible");
        });
    }

    // ── Compact: entries are filtered before they are paired (review 150 §1, §3) ──

    fn plain(root: &str, names: &[&str]) -> Vec<FlatRow> {
        names
            .iter()
            .map(|n| (PathBuf::from(root).join(n), false, false, false, 0))
            .collect()
    }

    /// The production composition, called with the arguments the app passes. It only
    /// supplies the binary cache and the roots; the rows are `rows_for_layout`'s.
    #[allow(clippy::too_many_arguments)]
    fn shown(
        compact: bool,
        left: &[FlatRow],
        right: &[FlatRow],
        roots: (&Path, &Path),
        query: &str,
        hide: (bool, bool),
        binary_enabled: bool,
        digest_map: &HashMap<DigestKey, EqualityEvidence>,
    ) -> Vec<AlignedRow> {
        let mut cache: Signal<HashMap<PathBuf, bool>> =
            Signal::new_in_scope(HashMap::new(), ScopeId::ROOT);
        rows_for_layout(
            compact,
            left,
            right,
            roots,
            query,
            hide,
            binary_enabled,
            digest_map,
            &mut cache,
        )
    }

    fn right_names(rows: &[AlignedRow]) -> Vec<String> {
        rows.iter()
            .filter_map(|(_, r)| {
                r.as_ref()
                    .map(|d| d.rel_path.to_string_lossy().into_owned())
            })
            .collect()
    }

    fn left_names(rows: &[AlignedRow]) -> Vec<String> {
        rows.iter()
            .filter_map(|(l, _)| {
                l.as_ref()
                    .map(|d| d.rel_path.to_string_lossy().into_owned())
            })
            .collect()
    }

    /// Review 150 §1, the name probe, through the production composition: the left entry
    /// matches the query and its positional neighbour does not, so the right entry must
    /// not be shown. Falsify inside `rows_for_layout` (compact pairs first, then filters
    /// the pairs): zulu.txt is shown beside alpha.txt, and this fails.
    #[test]
    fn compact_name_filter_does_not_show_an_entry_because_its_neighbour_matched() {
        with_test_store(|_store| {
            let rows = shown(
                true,
                &plain("/l", &["alpha.txt"]),
                &plain("/r", &["zulu.txt"]),
                (Path::new("/l"), Path::new("/r")),
                "alpha",
                (false, false),
                true,
                &HashMap::new(),
            );
            assert!(
                right_names(&rows).is_empty(),
                "zulu.txt must stay hidden: {rows:?}"
            );
            assert_eq!(left_names(&rows), vec!["alpha.txt".to_string()]);
        });
    }

    /// Review 150 §1, the hide-identical probe, through the production composition: the left
    /// entry is identical and hidden, and the right entry, a positional neighbour, is changed
    /// and must stay visible. Falsify inside `rows_for_layout` (judge the pair by its left
    /// evidence): changed.txt is hidden with its identical neighbour, and this fails.
    #[test]
    fn compact_hide_identical_keeps_a_changed_entry_whatever_its_neighbour() {
        with_test_store(|_store| {
            let mut digest = HashMap::new();
            digest.insert(
                DigestKey::Common(PathBuf::from("same.txt")),
                EqualityEvidence::DigestEqual,
            );
            digest.insert(
                DigestKey::Common(PathBuf::from("changed.txt")),
                EqualityEvidence::DigestDifferent,
            );
            let rows = shown(
                true,
                &plain("/l", &["same.txt"]),
                &plain("/r", &["changed.txt"]),
                (Path::new("/l"), Path::new("/r")),
                "",
                (false, true),
                true,
                &digest,
            );
            assert_eq!(
                right_names(&rows),
                vec!["changed.txt".to_string()],
                "changed.txt must stay visible: {rows:?}"
            );
        });
    }

    /// Hide binary in compact, through the production composition: a binary file on the left
    /// is hidden, and its text neighbour on the right stays. Real files on disk, because the
    /// binary check reads them.
    #[test]
    fn compact_hide_binary_hides_a_binary_entry_beside_a_text_one() {
        with_test_store(|_store| {
            let dir = std::env::temp_dir().join(format!("fsk-compact-bin-{}", std::process::id()));
            let (l, r) = (dir.join("l"), dir.join("r"));
            std::fs::create_dir_all(&l).unwrap();
            std::fs::create_dir_all(&r).unwrap();
            std::fs::write(l.join("data.bin"), [0u8, 159, 146, 150, 0, 1]).unwrap();
            std::fs::write(r.join("notes.txt"), "plain text\n").unwrap();
            let left = vec![(l.join("data.bin"), false, false, false, 0)];
            let right = vec![(r.join("notes.txt"), false, false, false, 0)];
            let rows = shown(
                true,
                &left,
                &right,
                (&l, &r),
                "",
                (true, false),
                false,
                &HashMap::new(),
            );
            assert!(
                left_names(&rows).is_empty(),
                "the binary file must be hidden: {rows:?}"
            );
            assert_eq!(right_names(&rows), vec!["notes.txt".to_string()]);
            let _ = std::fs::remove_dir_all(&dir);
        });
    }

    /// Packing, through the production composition: a filtered-out entry leaves no gap.
    /// Filtering out the second of three left entries leaves the left column as entries 1
    /// and 3, adjacent, and the right column keeps all three by position.
    #[test]
    fn compact_filtered_entries_pack_without_a_gap() {
        with_test_store(|_store| {
            let mut digest = HashMap::new();
            digest.insert(
                DigestKey::Common(PathBuf::from("b")),
                EqualityEvidence::DigestEqual,
            );
            let rows = shown(
                true,
                &plain("/l", &["a", "b", "c"]),
                &plain("/r", &["x", "y", "z"]),
                (Path::new("/l"), Path::new("/r")),
                "",
                (false, true),
                true,
                &digest,
            );
            assert_eq!(
                left_names(&rows),
                vec!["a".to_string(), "c".to_string()],
                "no gap where b was: {rows:?}"
            );
            assert_eq!(
                rows.len(),
                3,
                "the right column still has all three, paired by position"
            );
        });
    }

    /// Aligned, through the same production composition: the same name on both sides is one
    /// row and is shown; a name that matches the query on neither side is hidden with its
    /// row. The aligned branch is judged by pairs, and this is the test that checks it.
    #[test]
    fn aligned_filter_keeps_same_named_pairs_and_hides_the_rest_through_the_production_path() {
        with_test_store(|_store| {
            let rows = shown(
                false,
                &plain("/l", &["alpha.txt", "beta.txt"]),
                &plain("/r", &["alpha.txt", "gamma.txt"]),
                (Path::new("/l"), Path::new("/r")),
                "alpha",
                (false, false),
                true,
                &HashMap::new(),
            );
            assert_eq!(rows.len(), 1, "only the alpha pair remains: {rows:?}");
            assert_eq!(left_names(&rows), vec!["alpha.txt".to_string()]);
            assert_eq!(right_names(&rows), vec!["alpha.txt".to_string()]);
        });
    }
}
