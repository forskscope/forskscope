//! Application root with global keyboard shortcuts and accessibility (RFC-003, RFC-019, RFC-046).

use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;

use crate::state::{Store, open_compare, open_dir_compare, close_dir_tab, restore_session, save_session};
use crate::ui::diff::{DiffWorkspace, apply_focused_hunk, move_focus, save_tab};
use crate::ui::explorer::Explorer;
use crate::ui::header::Header;
use crate::ui::settings::{ModalLayer, load};
use crate::ui::statusbar::StatusBar;
use crate::ui::tabs::TabBar;

const MAIN_CSS: &str = include_str!("../assets/main.css");

pub static STARTUP_PAIR: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> =
    std::sync::OnceLock::new();
/// If set, the active tab's save target is overridden to this path after
/// the initial comparison opens (git mergetool mode).
pub static STARTUP_MERGED: std::sync::OnceLock<Option<PathBuf>> =
    std::sync::OnceLock::new();

#[component]
pub fn App() -> Element {
    let mut store = use_context_provider(|| Store::new(load()));

    use_hook(|| {
        if let Some(Some((left, right))) = STARTUP_PAIR.get() {
            open_compare(&mut store, left.clone(), right.clone());
            // git mergetool mode: redirect save target to the merged path.
            if let Some(Some(merged)) = STARTUP_MERGED.get() {
                let idx = store.tabs.read().len().saturating_sub(1);
                let merge_label = crate::i18n::t(store.lang(), "merge");
                if let Some(tab) = store.tabs.write().get_mut(idx) {
                    tab.right_path = Some(merged.clone());
                    tab.right_doc.fingerprint_at_load = None;
                    tab.title = format!("{} ({})", tab.title, merge_label);
                }
            }
        } else {
            // No explicit startup pair — restore the previous session (RFC-035).
            restore_session(&mut store);
        }
    });

    // Persist the session whenever the tab list changes.
    use_effect(move || {
        let _tabs = store.tabs.read(); // subscribe to the tabs signal
        save_session(&store);
    });

    // Update the OS window title to reflect the active comparison.
    use_effect(move || {
        let title = match *store.active.read() {
            Some(i) => store.tabs.read().get(i)
                .map(|t| format!("ForskScope — {}", t.title))
                .unwrap_or_else(|| "ForskScope".into()),
            None => "ForskScope".into(),
        };
        spawn(async move {
            let _ = dioxus::document::eval(
                &format!("document.title = {:?}", title)
            ).await;
        });
    });

    let theme_class = store.settings.read().theme.css_class();
    let active = *store.active.read();
    let toast = store.toast.read().cloned();

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app {theme_class}",
            id: "app-root",
            tabindex: "-1",
            onmounted: move |_| {
                spawn(async move {
                    let _ = dioxus::document::eval(
                        "document.getElementById('app-root')?.focus();"
                    ).await;
                });
            },
            onkeydown: move |e: Event<KeyboardData>| {
                // Escape closes any open modal regardless of whether a tab is active.
                if e.key() == Key::Escape {
                    let modal = store.modal.read().cloned();
                    if !matches!(modal, crate::state::Modal::None) {
                        store.modal.set(crate::state::Modal::None);
                        return;
                    }
                }
                let Some(index) = *store.active.read() else { return };
                let mods = e.modifiers();
                match e.key() {
                    Key::F7 => move_focus(&mut store, index, -1),
                    Key::F8 => move_focus(&mut store, index,  1),
                    // F3 / Shift+F3: next / previous search match
                    Key::F3 => {
                        let shift = mods.contains(Modifiers::SHIFT);
                        spawn(async move {
                            let id = if shift { "search-prev-btn" } else { "search-next-btn" };
                            let _ = dioxus::document::eval(
                                &format!("document.getElementById('{id}')?.click();")
                            ).await;
                        });
                    }
                    Key::Enter => apply_focused_hunk(&mut store, index),
                    Key::Character(ref s) if mods.contains(Modifiers::CONTROL) => {
                        match s.to_ascii_lowercase().as_str() {
                            "s" => save_tab(&mut store, index, false),
                            "z" => { let _ = store.tabs.write().get_mut(index).map(|t| t.merge.undo()); }
                            "y" => { let _ = store.tabs.write().get_mut(index).map(|t| t.merge.redo()); }
                            "w" => {
                                // Ctrl+W: close the active tab, with dirty-state guard.
                                let dirty = store.tabs.read().get(index)
                                    .map(|t| t.can_save && t.merge.is_dirty())
                                    .unwrap_or(false);
                                if dirty {
                                    store.modal.set(crate::state::Modal::ConfirmClose(index));
                                } else {
                                    crate::state::close_tab(&mut store, index);
                                }
                            }
                            "/" => store.modal.set(crate::state::Modal::KeyboardRef),
                            // Ctrl+F: the search bar inside DiffWorkspace handles its own
                            // context; we use document::eval to click the search button.
                            "f" => {
                                spawn(async move {
                                    let _ = dioxus::document::eval(
                                        "document.getElementById('search-open-btn')?.click();"
                                    ).await;
                                });
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            },
            Header {}
            TabBar {}
            div { class: "body",
                match (active, *store.active_dir.read()) {
                    (_, Some(dir_idx)) => {
                        let dir_tabs = store.dir_tabs.read();
                        if let Some((l, r)) = dir_tabs.get(dir_idx).cloned() {
                            let lang = store.lang();
                            drop(dir_tabs);
                            rsx! { crate::ui::deep_compare::DeepCompareView { left_root: l, right_root: r, lang } }
                        } else {
                            rsx! { Explorer {} }
                        }
                    }
                    (None, None)        => rsx! { Explorer {} },
                    (Some(index), None) => rsx! { DiffWorkspace { index } },
                }
            }
            StatusBar {}
            ModalLayer {}
            if let Some(message) = toast {
                div {
                    class: "toast",
                    role: "status",
                    aria_live: "polite",
                    onclick: move |_| store.toast.set(None),
                    "{message}"
                }
            }
        }
    }
}
