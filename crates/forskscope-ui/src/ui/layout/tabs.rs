//! Tab bar: Explorer tab + file comparison tabs + directory compare tabs
//! (RFC-087): a real WAI-ARIA **tabs** pattern, automatic activation.
//!
//! The Explorer, every file comparison tab, and every directory compare tab
//! form one ordered list (`state::tab_nav`): the Explorer first, then file
//! tabs, then directory tabs, matching render order. Only the active tab
//! has `tabindex="0"` (roving tabindex) — the list is one `Tab` stop, and
//! ←/→/Home/End move *and select* within it (automatic activation: there is
//! no separate "focused but not yet selected" state). `Delete` closes the
//! focused tab through the exact path its own × button (and `Ctrl+W`) use.

use dioxus::html::input_data::keyboard_types::Key;
use dioxus::prelude::*;

use crate::i18n::t;
use crate::keyboard::focus_handle;
use crate::state::{self, Store, tab_nav};

#[component]
pub fn TabBar() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let active_file = *store.active.read();
    let active_dir = *store.active_dir.read();
    let file_count = store.tabs.read().len();
    let dir_count = store.dir_tabs.read().len();
    let total = tab_nav::tab_count(file_count, dir_count);

    let explorer_active = active_file.is_none() && active_dir.is_none();

    rsx! {
        crate::keyboard::ControlZone { class: "tabbar",
            div {
                class: "tablist",
                role: "tablist",
                aria_label: t(lang, "Open tabs"),
                TabItem {
                    position: 0, total, file_count, dir_count, is_active: explorer_active,
                    aria_label: t(lang, "Explorer"),
                    onclick: move |_| state::activate_tab_at(&mut store, 0, file_count, dir_count),
                    span { class: "tab-title", {t(lang, "Explorer")} }
                }
                for i in 0..file_count {
                    FileTabItem {
                        index: i, position: 1 + i, total, file_count, dir_count,
                        is_active: active_file == Some(i) && active_dir.is_none(),
                    }
                }
                for j in 0..dir_count {
                    DirTabItem {
                        index: j, position: 1 + file_count + j, total, file_count, dir_count,
                        is_active: active_dir == Some(j),
                    }
                }
            }
        }
    }
}

/// The one `role="tab"` element every tab kind renders — markup, roving
/// tabindex, focus capture and the shared nav/Delete keydown handling live
/// here once; each kind supplies only its label and its own close action.
///
/// `close_button` renders as the tab's **sibling**, inside a plain,
/// role-less `.tab-slot` wrapper — never as the tab's DOM descendant. A
/// wrapper with no ARIA role is pruned from the accessibility tree, so the
/// tab still reaches the tablist as a direct accessible child; a plain
/// `<div>` carries no such exemption for *clickability*. Live AT-SPI
/// testing this release found that WebKitGTK's `do_action` resolves a
/// composite element's default action by walking its DOM descendants for
/// *any* clickable one, not by consulting ARIA roles or `aria-hidden` —
/// with the × button nested inside `role="tab"`, `do_action` on the tab
/// fired the close button instead of selecting the tab. Moving it out of
/// the DOM subtree, not merely out of the accessibility tree, is what
/// actually fixes it.
#[component]
fn TabItem(
    position: usize,
    total: usize,
    file_count: usize,
    dir_count: usize,
    is_active: bool,
    aria_label: String,
    onclick: EventHandler<()>,
    #[props(default)] on_delete: Option<EventHandler<()>>,
    #[props(default)] close_button: Option<Element>,
    children: Element,
) -> Element {
    let mut store = use_context::<Store>();

    rsx! {
        div {
            class: "tab-slot",
            div {
                id: "tab-{position}",
                role: "tab",
                class: if is_active { "tab active" } else { "tab" },
                tabindex: if is_active { "0" } else { "-1" },
                aria_selected: if is_active { "true" } else { "false" },
                aria_label: "{aria_label}",
                onmounted: move |e| { store.tab_handles.write().insert(position, e.data()); },
                onclick: move |_| onclick.call(()),
                onkeydown: move |e: Event<KeyboardData>| {
                    let target = match e.key() {
                        Key::ArrowLeft => Some(tab_nav::prev_position(position, total)),
                        Key::ArrowRight => Some(tab_nav::next_position(position, total)),
                        Key::Home => Some(tab_nav::home_position()),
                        Key::End => Some(tab_nav::end_position(total)),
                        Key::Delete => {
                            e.stop_propagation();
                            if let Some(on_delete) = on_delete {
                                on_delete.call(());
                            }
                            return;
                        }
                        _ => None,
                    };
                    if let Some(target) = target {
                        e.stop_propagation();
                        e.prevent_default();
                        state::activate_tab_at(&mut store, target, file_count, dir_count);
                        focus_handle(store.tab_handles.read().get(&target).cloned());
                    }
                },
                {children}
            }
            if let Some(btn) = close_button { {btn} }
        }
    }
}

#[component]
fn FileTabItem(
    index: usize,
    position: usize,
    total: usize,
    file_count: usize,
    dir_count: usize,
    is_active: bool,
) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let (title, is_dirty, is_loading) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else {
            return rsx! {};
        };
        let dirty = tab.can_save && tab.merge.is_dirty();
        let loading = tab.state == crate::state::TabState::Loading;
        (tab.title.clone(), dirty, loading)
    };

    // RFC-087 §1: "not colour alone" — a dot or a spinner is decorative
    // (`aria_hidden`); the accessible name itself carries the state in
    // words, the same split F187's filter dot established.
    let label = if is_loading {
        format!("{title}, {}", t(lang, "loading"))
    } else if is_dirty {
        format!("{title}, {}", t(lang, "unsaved"))
    } else {
        title.clone()
    };

    rsx! {
        TabItem {
            position, total, file_count, dir_count, is_active,
            aria_label: label,
            onclick: move |_| state::activate_tab_at(&mut store, position, file_count, dir_count),
            on_delete: move |_| {
                state::request_close_file_tab(&mut store, index);
                store.focus_active_tab();
            },
            close_button: rsx! {
                button {
                    class: "tab-close",
                    tabindex: "-1",
                    // RFC-087 §1: "stay mouse-only" - see `TabItem`'s own
                    // doc comment for why this button is a sibling of the
                    // tab, not nested inside it.
                    aria_hidden: "true",
                    title: t(lang, "Close tab"),
                    onclick: move |e| {
                        e.stop_propagation();
                        // Loading tabs close without a dirty-check (RFC-065) —
                        // `request_close_file_tab` already makes that call.
                        state::request_close_file_tab(&mut store, index);
                    },
                    "×"
                }
            },
            span { class: "tab-title",
                if is_loading { span { class: "tab-loading-spinner", aria_hidden: "true", "⟳ " } }
                else if is_dirty { span { class: "dirty-dot", aria_hidden: "true", "●" } }
                "{title}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;
    use forskscope_core::compare_prep::SaveCapability;
    use forskscope_core::{
        BomPresence, DiffOptions, FileKind, LoadedDocument, MergeSession, NewlineStyle,
        TextDocument, TextEncoding, compute_diff,
    };
    use forskscope_ui_logic::{CompareTabId, LoadGeneration};

    use super::TabBar;
    use crate::state::tab::CompareLaunchMode;
    use crate::state::{AppSettings, CompareTab, Store, TabState};

    fn text_doc(content: &str) -> LoadedDocument {
        LoadedDocument {
            file_id: None,
            fingerprint_at_load: None,
            kind: FileKind::Text,
            bytes_len: content.len() as u64,
            text: Some(TextDocument {
                content: content.to_string(),
                encoding: TextEncoding::utf8(),
                newline_style: NewlineStyle::Lf,
                had_decode_errors: false,
                bom: BomPresence::Absent,
                raw_bytes: Vec::new(),
            }),
            warnings: Vec::new(),
        }
    }

    /// A dirty tab titled "t" — mirrors `state::tab::tests::dirty_tab`,
    /// duplicated locally since that helper is private to its own module.
    fn dirty_tab() -> CompareTab {
        let left = text_doc("one\ntwo\n");
        let right = text_doc("one\nTWO\n");
        let diff_options = DiffOptions::default();
        let diff = compute_diff(left.diff_text(), right.diff_text(), diff_options);
        let hunk_id = diff
            .hunks
            .iter()
            .find(|h| h.kind.is_change())
            .expect("fixture must contain a change")
            .hunk_id;
        let mut merge = MergeSession::from_diff(&diff);
        merge.apply_left_to_right(hunk_id).unwrap();

        CompareTab {
            id: CompareTabId::new(1).unwrap(),
            load_generation: LoadGeneration::new(1).unwrap(),
            title: "t".into(),
            left_path: None,
            right_path: None,
            state: TabState::Ready,
            left_doc: left,
            right_doc: right,
            merge,
            diff,
            diff_options,
            can_save: true,
            save_capability: SaveCapability::Saveable,
            spreadsheet_warnings: Vec::new(),
            char_mode: false,
            word_wrap: false,
            focused_change: 0,
            save_target: None,
            launch_mode: CompareLaunchMode::Normal,
        }
    }

    /// RFC-087 §1: a dirty tab's accessible name says "unsaved", only the
    /// active tab is a `Tab` stop (roving `tabindex`), and `aria-selected`
    /// marks which one that is. Falsify by removing any of: the ", unsaved"
    /// suffix in `FileTabItem`'s label, the `tabindex` ternary, or the
    /// `aria_selected` ternary in `TabItem` — each breaks a different
    /// assertion below.
    ///
    /// Does NOT cover `role="tab"`/`role="tablist"` themselves: those are
    /// static template content (literal strings in the `rsx!`), which
    /// produce no `Mutation::SetAttribute` for this harness to see — same
    /// limitation `settings::modal::tests::the_about_control_keeps_its_accessible_name`
    /// documents. Confirmed by reading the source (`tabs.rs`'s `rsx!`
    /// blocks) and by the live AT-SPI tree in the review response.
    #[test]
    fn the_tab_bar_marks_selection_and_a_dirty_tabs_name() {
        fn root() -> Element {
            let store = use_context_provider(|| {
                let mut store = Store::new(AppSettings::default(), Default::default(), false);
                store.tabs.write().push(dirty_tab());
                store.active.set(Some(0));
                store
            });
            let _ = store;
            rsx! { TabBar {} }
        }

        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        let attrs: Vec<(String, String)> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute { name, value, .. } => {
                    Some((name.to_string(), format!("{value:?}")))
                }
                _ => None,
            })
            .collect();

        let selected: Vec<&str> = attrs
            .iter()
            .filter(|(n, _)| n == "aria-selected")
            .map(|(_, v)| v.as_str())
            .collect();
        assert!(
            selected.iter().any(|v| v.contains("true")),
            "the active file tab must carry aria-selected=true, got {selected:?}"
        );
        assert!(
            selected.iter().any(|v| v.contains("false")),
            "the Explorer, not active here, must carry aria-selected=false, got {selected:?}"
        );

        let tabindexes: Vec<&str> = attrs
            .iter()
            .filter(|(n, _)| n == "tabindex")
            .map(|(_, v)| v.as_str())
            .collect();
        assert!(
            tabindexes
                .iter()
                .any(|v| v.contains('0') && !v.contains("-1")),
            "the active tab must be the one Tab stop (tabindex=0), got {tabindexes:?}"
        );
        assert!(
            tabindexes.iter().any(|v| v.contains("-1")),
            "the Explorer, not active here, must be tabindex=-1, got {tabindexes:?}"
        );

        let labels: Vec<&str> = attrs
            .iter()
            .filter(|(n, _)| n == "aria-label")
            .map(|(_, v)| v.as_str())
            .collect();
        assert!(
            labels.iter().any(|v| v.contains("unsaved")),
            "a dirty tab's accessible name must say it is unsaved, got {labels:?}"
        );
    }
}

#[component]
fn DirTabItem(
    index: usize,
    position: usize,
    total: usize,
    file_count: usize,
    dir_count: usize,
    is_active: bool,
) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    let title = {
        let tabs = store.dir_tabs.read();
        let Some((l, r)) = tabs.get(index) else {
            return rsx! {};
        };
        let ln = l
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let rn = r
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if ln == rn {
            ln
        } else {
            format!("{ln} ↔ {rn}")
        }
    };

    rsx! {
        TabItem {
            position, total, file_count, dir_count, is_active,
            aria_label: title.clone(),
            onclick: move |_| state::activate_tab_at(&mut store, position, file_count, dir_count),
            on_delete: move |_| {
                crate::state::close_dir_tab(&mut store, index);
                store.focus_active_tab();
            },
            close_button: rsx! {
                button {
                    class: "tab-close",
                    tabindex: "-1",
                    // RFC-087 §1 / see `TabItem`'s doc comment.
                    aria_hidden: "true",
                    title: t(lang, "Close tab"),
                    onclick: move |e| {
                        e.stop_propagation();
                        crate::state::close_dir_tab(&mut store, index);
                    },
                    "×"
                }
            },
            span { class: "tab-title", "📁 {title}" }
        }
    }
}
