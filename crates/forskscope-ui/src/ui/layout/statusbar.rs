//! Status bar: passive context — file names, encoding, diff stats, local-only marker.

use dioxus::prelude::*;

use crate::i18n::{t, tf};
use crate::startup_mode::{self, ModeDisplay};
use crate::state::Store;
use crate::ui::view::BidiText;

/// RFC-091 §2/§3, acceptance criterion 7: the exact text for each
/// non-installed mode. `None` for `Installed` — today's behaviour is to
/// show nothing here, the RFC's texts cover every *other* case.
/// RFC-100 §4: the status bar's own words for a file containing one or
/// more of RFC-100 §1's bidirectional control characters — `None` when
/// `n` is `0` (nothing to say). Counts both sides together: the status
/// bar has room for one compact line, not a per-side breakdown (the CLI
/// and JSON output carry that, §4).
fn bidi_notice_text(lang: crate::state::Lang, n: usize) -> Option<String> {
    if n == 0 {
        return None;
    }
    Some(tf(
        lang,
        "contains-direction-control-characters",
        &[],
        Some(n as i64),
    ))
}

fn mode_text(lang: crate::state::Lang, mode: &ModeDisplay) -> Option<String> {
    match mode {
        ModeDisplay::Installed => None,
        ModeDisplay::Portable(_) => Some(t(lang, "portable")),
        ModeDisplay::PortableReadOnly(_) => {
            Some(t(lang, "portable-read-only-changes-are-not-saved"))
        }
        ModeDisplay::Private => Some(t(lang, "private-nothing-is-saved")),
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
                // RFC-095 §4.5: beside the save-target line, one short
                // sentence when this file's backup will be skipped -
                // never silently, since the backup would otherwise look
                // like it succeeded.
                let flatpak = forskscope_desktop::platform::flatpak();
                let no_backup =
                    forskscope_desktop::platform::is_document_portal_file(&flatpak, &st.path);
                (
                    short,
                    forskscope_desktop::platform::display_path(&st.path)
                        .display()
                        .to_string(),
                    no_backup,
                )
            });
            let bidi_notice = bidi_notice_text(
                lang,
                tab.left_doc.bidi_controls() + tab.right_doc.bidi_controls(),
            );
            (
                format!("{left} ↔ {right}"),
                enc,
                stat_str,
                dirty,
                save_target,
                bidi_notice,
            )
        })
    });

    rsx! {
        div { class: "statusbar", role: "status",
            if let Some((pair, enc, stats, dirty, save_target, bidi_notice)) = context {
                // Review 204 §2.2 (F218): only `.pair` and `.save-target`
                // ever ellipsize (02-layout-shell.css) — `title` keeps each
                // one's full text as an accessible name once it does. `enc`/
                // `stats`/`dirty` never truncate at all (short, fixed-format,
                // and the information itself — a fragment of "UTF-8" or
                // "+2 / -1" is worse than not showing it), so they need no
                // `title`: the visible text is always the whole text.
                // F235: `title` is a plain tooltip string (no markup), so
                // it gets the `<U+XXXX>` escape; the visible text marks
                // any bidi-control character with `BidiText` instead
                // (passing the whole "left ↔ right" string through it is
                // fine — the " ↔ " separator has no control character of
                // its own to mark, and renders as an ordinary plain run).
                span { class: "pair",
                    title: "{forskscope_compare::escape_to_plain_marker(&pair)}",
                    BidiText { text: pair.clone(), lang } }
                if !enc.is_empty() { span { class: "enc", "{enc}" } }
                if !stats.is_empty() { span { class: "stats", "{stats}" } }
                if let Some(ref notice) = bidi_notice {
                    span { class: "bidi-notice", "{notice}" }
                }
                if dirty { span { class: "dirty", {t(lang, "unsaved")} } }
                if let Some((short, full, no_backup)) = save_target {
                    // F235: `title` escapes the full path's name; the
                    // visible text keeps the translated label as plain
                    // text and marks only the name with `BidiText`.
                    span { class: "save-target",
                        title: "{forskscope_compare::escape_to_plain_marker(&full)}",
                        {t(lang, "save-target")}": "
                        BidiText { text: short, lang }
                    }
                    if no_backup {
                        span { class: "save-target-no-backup",
                            title: t(lang, "no-backup-can-be-kept-for-this-file"),
                            "⚠ " {t(lang, "no-backup")}
                        }
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
            span { class: "local-only", title: t(lang, "files-stay-on-this-computer-forskscope-does-not-upload-them"),
                "🔒 " {t(lang, "local-only")} }
        }
    }
}

fn file_name(path: &Option<std::path::PathBuf>) -> String {
    path.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::*;
    use forskscope_compare::{
        BomPresence, DiffOptions, FileKind, LoadedDocument, NewlineStyle, TextDocument,
        TextEncoding, compute_diff,
    };
    use forskscope_edit::MergeSession;
    use forskscope_edit::compare_prep::{
        SaveCapability, SaveTargetSnapshot, SaveTargetState, TargetExpectation,
    };
    use forskscope_ui_logic::{CompareTabId, LoadGeneration};

    use super::StatusBar;
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

    /// A tab whose own left/right paths and save target all carry a bidi
    /// control character in their file name (the classic spoofing shape).
    fn bidi_named_tab() -> CompareTab {
        let left = text_doc("one\ntwo\n");
        let right = text_doc("one\nTWO\n");
        let diff_options = DiffOptions::default();
        let diff = compute_diff(left.diff_text(), right.diff_text(), diff_options);
        let merge = MergeSession::from_diff(&diff);

        CompareTab {
            id: CompareTabId::new(1).unwrap(),
            load_generation: LoadGeneration::new(1).unwrap(),
            title: "t".into(),
            left_path: Some(std::path::PathBuf::from("/tmp/invoice\u{202E}fdp.exe")),
            right_path: Some(std::path::PathBuf::from("/tmp/invoice.pdf")),
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
            save_target: Some(SaveTargetSnapshot {
                path: std::path::PathBuf::from("/tmp/invoice\u{202E}fdp.exe"),
                state: SaveTargetState::Writable {
                    expectation: TargetExpectation::MustBeAbsent,
                    encoding_label: "UTF-8".to_string(),
                    bom: BomPresence::Absent,
                },
            }),
            launch_mode: CompareLaunchMode::Normal,
        }
    }

    /// F235: a bidi control character in a tab's own left/right path, and
    /// in its save target's own path, never reaches the status bar raw —
    /// `.pair`/`.save-target`'s visible text shows `BidiText`'s own
    /// marker, and both `title` tooltips show the `<U+XXXX>` escape
    /// instead. Falsify by reverting either `BidiText` call site in
    /// `StatusBar` back to a bare interpolation, or either
    /// `escape_to_plain_marker` call back to the bare value: the
    /// corresponding assertion below then fails.
    #[test]
    fn a_bidi_control_character_in_the_status_bars_pair_and_save_target_never_reaches_the_rendered_page_raw()
     {
        fn root() -> Element {
            let store = use_context_provider(|| {
                let mut store = Store::new(AppSettings::default(), Default::default(), false);
                store.tabs.write().push(bidi_named_tab());
                store.active.set(Some(0));
                store
            });
            let _ = store;
            rsx! { StatusBar {} }
        }

        let mut vdom = VirtualDom::new(root);
        let mutations = vdom.rebuild_to_vec();

        let texts: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect();
        for text in &texts {
            assert!(
                !text.chars().any(forskscope_compare::is_bidi_control),
                "a bidi control character reached a rendered text node: {text:?}"
            );
        }
        assert!(
            texts.iter().filter(|t| t.contains("RLO")).count() >= 2,
            "expected the RLO marker token for both .pair and the save \
             target among the rendered text nodes: {texts:?}"
        );

        let title_attrs: Vec<String> = mutations
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::SetAttribute {
                    name: "title",
                    value: dioxus_core::AttributeValue::Text(s),
                    ..
                } => Some(s.clone()),
                _ => None,
            })
            .collect();
        for title in &title_attrs {
            assert!(
                !title.chars().any(forskscope_compare::is_bidi_control),
                "the raw control character must never reach a title attribute either: {title:?}"
            );
        }
        assert!(
            title_attrs
                .iter()
                .filter(|t| t.contains("<U+202E>"))
                .count()
                >= 2,
            "expected the <U+202E> escape for both .pair's and the save \
             target's title among the title attributes: {title_attrs:?}"
        );
    }
}
