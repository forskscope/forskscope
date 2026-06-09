//! Status bar (RFC-003). Calm, single line; passive metadata only.
//!
//! "Less is more": the status bar carries the quiet context (which files,
//! which encoding) and a single local-only trust marker. Action counts and
//! save state live next to the diff and on the tab, not duplicated here.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Store;

#[component]
pub fn StatusBar() -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let active = *store.active.read();

    let context = active.and_then(|i| {
        let tabs = store.tabs.read();
        tabs.get(i).map(|tab| {
            let left = file_name(&tab.left_path);
            let right = file_name(&tab.right_path);
            let dirty = tab.can_save && tab.merge.is_dirty();
            (format!("{left} ↔ {right}"), tab.right_label(), dirty)
        })
    });

    rsx! {
        div { class: "statusbar",
            if let Some((pair, enc, dirty)) = context {
                span { "{pair}" }
                span { "{enc}" }
                if dirty {
                    span { class: "dirty", {t(lang, "unsaved")} }
                }
            }
            span { class: "spacer" }
            span { "Local only" }
        }
    }
}

fn file_name(path: &Option<std::path::PathBuf>) -> String {
    path.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "—".into())
}
