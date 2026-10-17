//! Status bar: passive context — file names, encoding, diff stats, local-only marker.

use dioxus::prelude::*;

use crate::i18n::t;
use crate::startup_mode::{self, ModeDisplay};
use crate::state::Store;

/// RFC-091 §2/§3, acceptance criterion 7: the exact text for each
/// non-installed mode. `None` for `Installed` — today's behaviour is to
/// show nothing here, the RFC's texts cover every *other* case.
fn mode_text(lang: crate::state::Lang, mode: &ModeDisplay) -> Option<String> {
    match mode {
        ModeDisplay::Installed => None,
        ModeDisplay::Portable(_) => Some(t(lang, "Portable")),
        ModeDisplay::PortableReadOnly(_) => {
            Some(t(lang, "Portable (read-only): changes are not saved"))
        }
        ModeDisplay::Private => Some(t(lang, "Private: nothing is saved")),
    }
}

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
            let enc = tab.right_label();
            let dirty = tab.can_save && tab.merge.is_dirty();
            let stats = &tab.diff.stats;
            let stat_str = if stats.lines_inserted > 0 || stats.lines_deleted > 0 {
                format!("+{} / -{}", stats.lines_inserted, stats.lines_deleted)
            } else {
                String::new()
            };
            // F85/RFC-082 §D6: shown unconditionally, not only when it
            // differs from the right pane — an absence would carry meaning
            // the user has to know to read, and this is the only place
            // $MERGED (mergetool mode) is visible at all.
            let save_target = tab.save_target.as_ref().map(|st| {
                let short = st
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "—".into());
                (short, st.path.display().to_string())
            });
            (
                format!("{left} ↔ {right}"),
                enc,
                stat_str,
                dirty,
                save_target,
            )
        })
    });

    rsx! {
        div { class: "statusbar", role: "status",
            if let Some((pair, enc, stats, dirty, save_target)) = context {
                span { "{pair}" }
                if !enc.is_empty() { span { "{enc}" } }
                if !stats.is_empty() { span { class: "stats", "{stats}" } }
                if dirty { span { class: "dirty", {t(lang, "unsaved")} } }
                if let Some((short, full)) = save_target {
                    span { class: "save-target", title: "{full}",
                        {format!("{}: {short}", t(lang, "Save target"))}
                    }
                }
            }
            span { class: "spacer" }
            {
                let mode = startup_mode::mode_display();
                let text = mode_text(lang, &mode);
                let tooltip = match &mode {
                    ModeDisplay::Portable(dir) | ModeDisplay::PortableReadOnly(dir) => {
                        Some(dir.display().to_string())
                    }
                    ModeDisplay::Installed | ModeDisplay::Private => None,
                };
                rsx! {
                    if let Some(text) = text {
                        span { class: "mode-indicator", title: tooltip.unwrap_or_default(), "{text}" }
                    }
                }
            }
            span { class: "local-only", title: t(lang, "Files stay on this computer. ForskScope does not upload them."),
                "🔒 " {t(lang, "Local only")} }
        }
    }
}

fn file_name(path: &Option<std::path::PathBuf>) -> String {
    path.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "—".into())
}
