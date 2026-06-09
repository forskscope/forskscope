//! Application root with global keyboard shortcuts (RFC-003, RFC-019).

use std::path::PathBuf;

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;

use crate::state::{Store, open_compare};
use crate::ui::diff::{DiffWorkspace, move_focus, save_tab};
use crate::ui::explorer::Explorer;
use crate::ui::header::Header;
use crate::ui::settings::{ModalLayer, load};
use crate::ui::statusbar::StatusBar;
use crate::ui::tabs::TabBar;

const MAIN_CSS: &str = include_str!("../assets/main.css");

pub static STARTUP_PAIR: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> =
    std::sync::OnceLock::new();

#[component]
pub fn App() -> Element {
    let mut store = use_context_provider(|| Store::new(load()));

    use_hook(|| {
        if let Some(Some((left, right))) = STARTUP_PAIR.get() {
            open_compare(&mut store, left.clone(), right.clone());
        }
    });

    let theme_class = store.settings.read().theme.css_class();
    let active = *store.active.read();
    let toast = store.toast.read().clone();

    rsx! {
        style { {MAIN_CSS} }
        div {
            class: "app {theme_class}",
            // Enable keyboard events on the root container.
            tabindex: "-1",
            onkeydown: move |e: Event<KeyboardData>| {
                let Some(index) = *store.active.read() else { return };
                let mods = e.modifiers();
                match e.key() {
                    Key::F7 => move_focus(&mut store, index, -1),
                    Key::F8 => move_focus(&mut store, index,  1),
                    Key::Character(ref s) if mods.contains(Modifiers::CONTROL) => {
                        match s.to_ascii_lowercase().as_str() {
                            "s" => save_tab(&mut store, index, false),
                            "z" => { let _ = store.tabs.write().get_mut(index).map(|t| t.merge.undo()); }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            },
            Header {}
            TabBar {}
            div { class: "body",
                match active {
                    None        => rsx! { Explorer {} },
                    Some(index) => rsx! { DiffWorkspace { index } },
                }
            }
            StatusBar {}
            ModalLayer {}
            if let Some(message) = toast {
                div {
                    class: "toast",
                    onclick: move |_| store.toast.set(None),
                    "{message}"
                }
            }
        }
    }
}
