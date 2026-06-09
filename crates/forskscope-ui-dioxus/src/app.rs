//! Application root (RFC-003 app shell).

use std::path::PathBuf;

use dioxus::prelude::*;

use crate::state::{Store, open_compare};
use crate::ui::diff::DiffWorkspace;
use crate::ui::explorer::Explorer;
use crate::ui::header::Header;
use crate::ui::settings::{ModalLayer, load};
use crate::ui::statusbar::StatusBar;
use crate::ui::tabs::TabBar;

const MAIN_CSS: &str = include_str!("../assets/main.css");

/// CLI-provided startup pair, parsed once in `main`.
pub static STARTUP_PAIR: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> =
    std::sync::OnceLock::new();

#[component]
pub fn App() -> Element {
    // Provide the shared store, seeded with persisted settings.
    let mut store = use_context_provider(|| Store::new(load()));

    // RFC-034: open a comparison from `forskscope <left> <right>` once.
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
        div { class: "app {theme_class}",
            Header {}
            TabBar {}
            div { class: "body",
                match active {
                    None => rsx! { Explorer {} },
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
