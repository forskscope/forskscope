//! Diff/merge workspace: coordination, snapshot, and loading/error states.
//! Hunk rendering lives in [`crate::ui::view::hunk`].
//! Toolbar lives in [`diff::toolbar`].

pub mod toolbar;

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::i18n::t;
use crate::state::Store;
use crate::ui::component::notice::{Notice, NoticeKind};
use crate::ui::view::diff_actions::trunc;
pub use crate::ui::view::diff_actions::{
    SaveAsPrecheck, apply_focused_hunk, confirm_overwrite, move_focus, precheck_save_as_target,
    save_as, save_tab,
};
use crate::ui::view::hunk::{HunkBlock, HunkCol};
use crate::ui::view::search::{SearchBar, SearchCtx, scroll_to_focused};
use forskscope_ui_logic::MatchIndex;
use toolbar::Toolbar;

type HunkSearchRows<'a> = Vec<(u64, Vec<(Option<&'a str>, Option<&'a str>)>)>;

// ── Workspace component ───────────────────────────────────────────────────────

#[component]
pub fn DiffWorkspace(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let font_size = store.settings.read().diff_font_size;
    let font_family = store.settings.read().diff_font_family.css_value();
    let context_lines = store.settings.read().context_lines;

    // Loading / Error states (RFC-065).
    // Important: extract state and title *then drop the guard* before returning.
    // Holding the read guard across the return boundary prevents Dioxus from
    // registering this component as a subscriber, so the signal write from the
    // background task never triggers a re-render.
    {
        let (state, title) = {
            let tabs = store.tabs.read();
            let state = tabs.get(index).map(|t| t.state.clone());
            let title = tabs.get(index).map(|t| t.title.clone()).unwrap_or_default();
            (state, title)
        };
        match state {
            None => return rsx! { Notice { kind: NoticeKind::Info, {t(lang, "No comparison.")} } },
            Some(crate::state::TabState::Loading) => {
                return rsx! {
                    div { class: "diff-loading",
                        span { class: "diff-loading-spinner", "⟳" }
                        span { {t(lang, "Loading")} " " {title} "…" }
                    }
                };
            }
            Some(crate::state::TabState::Error(msg)) => {
                return rsx! { TabError { msg } };
            }
            Some(crate::state::TabState::Ready) => {}
        }
    }

    let snap = {
        let tabs = store.tabs.read();
        match tabs.get(index) {
            Some(tab) => TabSnapshot::from_tab(tab, font_size, font_family, context_lines, lang),
            None => return rsx! { Notice { kind: NoticeKind::Info, {t(lang, "No comparison.")} } },
        }
    };

    let mut search_ctx: Signal<SearchCtx> =
        use_context_provider(|| Signal::new(SearchCtx::default()));
    let mut expanded: Signal<HashSet<u64>> = use_signal(HashSet::new);

    // Rebuild match index on query change; auto-expand hunks containing matches.
    {
        let query = search_ctx.read().query.clone();
        let active = search_ctx.read().active;
        if active && !query.is_empty() {
            let hunk_rows: HunkSearchRows<'_> = snap
                .hunks
                .iter()
                .map(|h| {
                    let rows = h
                        .rows
                        .iter()
                        .map(|r| {
                            (
                                r.left.as_ref().map(|l| l.content.as_str()),
                                r.right.as_ref().map(|r| r.content.as_str()),
                            )
                        })
                        .collect();
                    (h.hunk_id, rows)
                })
                .collect();
            let new_index = MatchIndex::build(
                hunk_rows.iter().map(|(id, rows)| (*id, rows.as_slice())),
                &query,
            );
            for id in new_index.matching_hunk_ids() {
                expanded.write().insert(id);
            }
            let prev_len = search_ctx.read().index.len();
            if new_index.len() != prev_len || search_ctx.read().index.focused_number() == Some(1) {
                let ctx_snap = search_ctx.read();
                scroll_to_focused(&ctx_snap);
                drop(ctx_snap);
            }
            search_ctx.write().index = new_index;
        } else if !active {
            search_ctx.write().index = MatchIndex::default();
        }
    }

    let wrap_class = if snap.word_wrap {
        "diff-scroll wrap"
    } else {
        "diff-scroll"
    };

    rsx! {
        div {
            class: "diff-wrap",
            role: "region",
            aria_label: t(lang, "File comparison"),
            DiffHeader { index }
            Toolbar { index, snap: snap.clone(), lang }
            SearchBar {}
            for w in snap.warnings.iter() {
                Notice { kind: NoticeKind::Warning, "⚠ {w}" }
            }
            if !snap.can_save {
                Notice { kind: NoticeKind::Info, {snap.readonly_notice.clone()} }
            }
            if snap.identical {
                Notice { kind: NoticeKind::Ok, {t(lang, "Files are identical")} }
            }
            div { class: "diff-pane-labels", aria_hidden: "true",
                span { class: "pane-label-left",  {t(lang, "Left / Old")} }
                span { class: "pane-label-act" }
                span { class: "pane-label-right", {t(lang, "Right / New")} }
            }
            div {
                class: "{wrap_class}",
                style: "--diff-fs:{snap.font_size}px; --diff-ff:{snap.font_family};",
                // .diff-columns: inner grid that places the three parallel columns.
                // .diff-scroll owns vertical scrolling; .diff-columns sets column widths;
                // left and right columns each own their own horizontal scroll, kept
                // in sync by install_hscroll_sync (mirrors scrollLeft between panes).
                div {
                    class: "diff-columns",
                    onmounted: move |_| { install_hscroll_sync(index); },
                    div { class: "diff-col-left", id: "diff-col-left-{index}",
                    div { class: "diff-col-table",
                    for hunk in snap.hunks.iter() {
                        HunkBlock {
                            index, hunk: hunk.clone(), col: HunkCol::Left,
                            char_mode: snap.char_mode, context_lines: snap.context_lines,
                            focused: snap.focused_id == Some(hunk.hunk_id),
                            can_save: snap.can_save,
                            is_expanded: expanded.read().contains(&hunk.hunk_id),
                            on_expand: move |id: u64| { expanded.write().insert(id); },
                        }
                    }
                    } // .diff-col-table
                }
                div { class: "diff-col-act",
                    div { class: "diff-col-table",
                    for hunk in snap.hunks.iter() {
                        HunkBlock {
                            index, hunk: hunk.clone(), col: HunkCol::Act,
                            char_mode: snap.char_mode, context_lines: snap.context_lines,
                            focused: snap.focused_id == Some(hunk.hunk_id),
                            can_save: snap.can_save,
                            is_expanded: expanded.read().contains(&hunk.hunk_id),
                            on_expand: move |id: u64| { expanded.write().insert(id); },
                        }
                    }
                    } // .diff-col-table
                }
                div { class: "diff-col-right", id: "diff-col-right-{index}",
                    div { class: "diff-col-table",
                    for hunk in snap.hunks.iter() {
                        HunkBlock {
                            index, hunk: hunk.clone(), col: HunkCol::Right,
                            char_mode: snap.char_mode, context_lines: snap.context_lines,
                            focused: snap.focused_id == Some(hunk.hunk_id),
                            can_save: snap.can_save,
                            is_expanded: expanded.read().contains(&hunk.hunk_id),
                            on_expand: move |id: u64| { expanded.write().insert(id); },
                        }
                    }
                    } // .diff-col-table
                }
                } // .diff-columns
            }
        }
    }
}

// ── Horizontal scroll synchronisation ────────────────────────────────────────

/// Install a horizontal scroll-sync binding between the left and right diff
/// panes for the tab at `index`.
///
/// The two panes (`#diff-col-left-{index}` and `#diff-col-right-{index}`) are
/// independent horizontal scroll containers. This mirrors each pane's
/// `scrollLeft` onto the other so matched content stays horizontally aligned.
///
/// Implementation notes:
/// - A re-entrancy guard (`__fsLocked`) prevents the programmatic scroll of one
///   pane from triggering a feedback loop back through the other's listener.
/// - A `data-fs-hsync` marker makes the binding idempotent: if the component
///   re-mounts and re-runs this, the listeners are not attached twice.
/// - Only `scrollLeft` is synced; vertical scrolling is owned by the shared
///   `.diff-scroll` container and needs no per-pane sync.
fn install_hscroll_sync(index: usize) {
    let js = format!(
        r#"
        (function() {{
            var L = document.getElementById('diff-col-left-{index}');
            var R = document.getElementById('diff-col-right-{index}');
            if (!L || !R) return;
            if (L.dataset.fsHsync === '1') return;   // already bound
            L.dataset.fsHsync = '1';
            R.dataset.fsHsync = '1';
            var locked = false;
            function mirror(src, dst) {{
                if (locked) return;
                locked = true;
                dst.scrollLeft = src.scrollLeft;
                locked = false;
            }}
            L.addEventListener('scroll', function() {{ mirror(L, R); }}, {{ passive: true }});
            R.addEventListener('scroll', function() {{ mirror(R, L); }}, {{ passive: true }});
        }})();
        "#
    );
    spawn(async move {
        let _ = dioxus::document::eval(&js).await;
    });
}

// ── Diff file header ──────────────────────────────────────────────────────────

#[component]
fn DiffHeader(index: usize) -> Element {
    let store = use_context::<Store>();
    let lang = store.lang();
    let (left, right, merged) = {
        let tabs = store.tabs.read();
        let tab = match tabs.get(index) {
            Some(t) => t,
            None => return rsx! {},
        };
        let merged = match &tab.launch_mode {
            crate::state::tab::CompareLaunchMode::MergeTool { merged } => {
                Some(merged.display().to_string())
            }
            crate::state::tab::CompareLaunchMode::Normal => None,
        };
        (
            tab.left_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "—".into()),
            tab.right_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "—".into()),
            merged,
        )
    };
    rsx! {
        div { class: "diff-file-header",
            span { class: "path-old", title: "{left}",  {trunc(&left)} }
            span { class: "arrow", "↔" }
            span { class: "path-new", title: "{right}", {trunc(&right)} }
        }
        // RFC-077: a quiet, non-interactive line naming the save
        // destination for a Git mergetool tab. Never a button/link — the
        // normal save guards (conflict/backup/no-clobber) are the only path
        // to writing here.
        if let Some(merged) = merged {
            div { class: "diff-mergetool-result", title: "{merged}",
                {t(lang, "Result:")}
                " "
                span { class: "result-path", {trunc(&merged)} }
            }
        }
    }
}

/// The error tab (F122): the message, and nothing else. It used to append
/// "Check that the file exists and you have read permission." to every error —
/// under a size refusal, a binary-versus-text error, and a workbook that exists
/// and is readable. Guidance belongs to the error that earns it and is part of
/// its message (`compare.rs`'s `open_error`), so a bare message stays bare.
#[component]
pub(crate) fn TabError(msg: String) -> Element {
    rsx! {
        div { class: "diff-error",
            Notice { kind: NoticeKind::Error, "⚠ " {msg} }
        }
    }
}

// ── Tab snapshot ──────────────────────────────────────────────────────────────

/// Whether the toolbar's Inline diff toggle may be used on a tab with these
/// options. The load guard sets `inline_mode = None` for a file over the size
/// it warns about (`decide_load`), and every profile the app builds is `Lazy`,
/// so `None` here means exactly "the guard turned inline diff off" (F120).
pub(crate) fn inline_available(opts: &forskscope_core::DiffOptions) -> bool {
    opts.inline_mode != forskscope_core::diff::InlineMode::None
}

#[derive(Clone, PartialEq)]
pub struct TabSnapshot {
    pub hunks: Vec<forskscope_core::merge::MergeHunk>,
    pub identical: bool,
    pub char_mode: bool,
    /// `false` when the load guard turned inline diff off for this tab (a
    /// file over the size the guard warns about): the toolbar toggle is then
    /// disabled, so the "inline diff disabled" wording is true (F120).
    pub inline_available: bool,
    pub word_wrap: bool,
    pub can_save: bool,
    pub is_dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub font_size: u32,
    pub font_family: &'static str,
    pub focused_id: Option<u64>,
    pub focused_change: usize,
    pub changes: usize,
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub context_lines: usize,
    pub algorithm: forskscope_core::DiffAlgorithm,
    pub warnings: Vec<String>,
    pub readonly_notice: String,
    /// RFC-083 §3: whether the right side is `FileKind::Text` — the
    /// encoding override control only makes sense where there's a decoded
    /// text with a possibly-wrong label. Independent of `can_save`: a
    /// target-writability problem shouldn't hide the ability to fix a
    /// misdetected encoding on a side that is genuinely text.
    pub right_is_text: bool,
    pub right_encoding_label: String,
}

/// At most this many sheet names are listed in one warning; the rest are a count.
const MAX_SHEETS_NAMED: usize = 5;

/// One localised line for a spreadsheet warning: what may be wrong, then the
/// sheets it concerns (F131).
fn spreadsheet_warning_text(
    lang: crate::state::Lang,
    warning: &forskscope_core::xlsx::SpreadsheetWarning,
) -> String {
    use crate::i18n::t;
    use forskscope_core::xlsx::SpreadsheetWarningKind as K;

    let sentence = match warning.kind {
        K::DuplicateAlignmentKey => t(
            lang,
            "Some rows share an alignment key, so rows may be paired wrongly.",
        ),
        K::AlignmentFellBack => t(
            lang,
            "A sheet was too large to align rows and was compared position by position, so an inserted row may appear as many changes.",
        ),
        K::AmbiguousSheetMatch => t(
            lang,
            "Several sheets could have been renamed, so none was matched; they appear as added and removed.",
        ),
        K::SheetNotCompared => t(lang, "Not compared (not a worksheet):"),
        K::Other => t(lang, "The comparison reported a warning:"),
    };
    let mut names: Vec<String> = warning
        .sheets
        .iter()
        .take(MAX_SHEETS_NAMED)
        .map(|s| format!("‘{s}’"))
        .collect();
    if warning.sheets.len() > MAX_SHEETS_NAMED {
        names.push(format!("(+{})", warning.sheets.len() - MAX_SHEETS_NAMED));
    }
    let mut out = sentence;
    if let Some(detail) = &warning.detail {
        out.push(' ');
        out.push_str(detail);
    }
    if !names.is_empty() {
        if !matches!(warning.kind, K::SheetNotCompared) {
            out.push(' ');
            out.push_str(&t(lang, "Sheets:"));
        }
        out.push(' ');
        out.push_str(&names.join(", "));
    }
    out
}

impl TabSnapshot {
    pub fn from_tab(
        tab: &crate::state::CompareTab,
        font_size: u32,
        font_family: &'static str,
        context_lines: usize,
        lang: crate::state::Lang,
    ) -> Self {
        use crate::i18n::t;
        use forskscope_core::diff::DiffWarning;
        use forskscope_core::file_kind::FileKind;

        let hunks = tab.merge.hunks().to_vec();
        let ids: Vec<u64> = hunks
            .iter()
            .filter(|h| h.kind.is_change())
            .map(|h| h.hunk_id)
            .collect();
        let mut warnings: Vec<String> = tab
            .diff
            .warnings
            .iter()
            .map(|w| match w {
                DiffWarning::LargeFilePolicyApplied => t(
                    lang,
                    "Large file — inline diff disabled and deadline shortened.",
                ),
                DiffWarning::DeadlineExpired => {
                    t(lang, "Diff timed out — result may be approximate.")
                }
                DiffWarning::InlineSkippedHunkTooLarge => {
                    t(lang, "Some hunks were too large for character-level diff.")
                }
            })
            .collect();
        // F120: character mode skips a pair over the length limit. Say so
        // here as well as on the row, or a skipped pair reads as "no
        // character-level differences".
        if tab.char_mode && forskscope_core::diff::skipped_inline_pairs(&tab.diff) > 0 {
            warnings.push(t(
                lang,
                "Some hunks were too large for character-level diff.",
            ));
        }
        // F131: what the spreadsheet parser doubted, in the strip that already
        // sits above the panes and reads as "this result may be wrong", not as
        // a row of the diff. One line per kind, naming its sheets.
        warnings.extend(
            tab.spreadsheet_warnings
                .iter()
                .map(|w| spreadsheet_warning_text(lang, w)),
        );
        let both_missing = matches!(tab.left_doc.kind, FileKind::Missing)
            && matches!(tab.right_doc.kind, FileKind::Missing);
        let readonly_notice = if tab.can_save {
            String::new()
        } else {
            match (&tab.left_doc.kind, &tab.right_doc.kind) {
                (FileKind::Missing, FileKind::Missing) => {
                    t(lang, "Both files not found — read-only.")
                }
                (FileKind::Binary, _) | (_, FileKind::Binary) => {
                    t(lang, "Binary file — read-only comparison (hex preview).")
                }
                (FileKind::ExcelXlsx, _) | (_, FileKind::ExcelXlsx) => {
                    t(lang, "Spreadsheet comparison is read-only.")
                }
                (FileKind::Missing, _) | (_, FileKind::Missing) => {
                    t(lang, "One side is missing — read-only.")
                }
                (FileKind::Unsupported { .. }, _) | (_, FileKind::Unsupported { .. }) => {
                    t(lang, "File type not supported for merge — read-only.")
                }
                _ => t(lang, "Merge/save unavailable for this file type."),
            }
        };
        Self {
            // A green "Files are identical" beside "a sheet was not compared" is
            // a claim the comparison cannot make, so a warned pair never says it.
            identical: tab.diff.is_identical()
                && !both_missing
                && tab.spreadsheet_warnings.is_empty(),
            char_mode: tab.char_mode,
            inline_available: inline_available(&tab.diff_options),
            word_wrap: tab.word_wrap,
            can_save: tab.can_save,
            is_dirty: tab.merge.is_dirty(),
            can_undo: tab.merge.can_undo(),
            can_redo: tab.merge.can_redo(),
            font_size,
            font_family,
            focused_id: ids.get(tab.focused_change).copied(),
            focused_change: tab.focused_change,
            changes: ids.len(),
            ignore_whitespace: tab.diff_options.ignore_whitespace,
            ignore_case: tab.diff_options.ignore_case,
            algorithm: tab.diff_options.algorithm,
            context_lines,
            hunks,
            warnings,
            readonly_notice,
            right_is_text: matches!(tab.right_doc.kind, FileKind::Text),
            right_encoding_label: tab.right_label(),
        }
    }
}

#[cfg(test)]
mod tab_error_tests {
    use super::*;

    fn rendered_text(root: fn() -> Element) -> Vec<String> {
        let mut vdom = VirtualDom::new(root);
        vdom.rebuild_to_vec()
            .edits
            .iter()
            .filter_map(|m| match m {
                dioxus_core::Mutation::CreateTextNode { value, .. } => Some(value.clone()),
                _ => None,
            })
            .collect()
    }

    /// F122: the error tab shows the message and nothing else. Falsify by
    /// putting the old unconditional advice `Notice` back into `TabError`
    /// (the one that rendered the "check that the file exists" sentence
    /// through `t`). The text nodes collected here are the *dynamic* ones — the
    /// message and anything passed through `t` — which is how the old advice
    /// was written; a bare string literal would sit in the static template
    /// and not be seen.
    #[test]
    fn the_error_tab_shows_the_message_and_no_advice_of_its_own() {
        fn root() -> Element {
            rsx! { TabError { msg: "Could not compare the spreadsheets — too large".to_string() } }
        }
        let text = rendered_text(root).join("|");
        assert!(text.contains("too large"), "{text}");
        assert!(
            !text.contains("Check that the file exists"),
            "advice belongs to the error, not the tab: {text}"
        );
    }
}

#[cfg(test)]
mod spreadsheet_warning_tests {
    use super::*;
    use crate::state::Lang;
    use forskscope_core::xlsx::{SpreadsheetWarning, SpreadsheetWarningKind as K};

    fn warning(kind: K, sheets: &[&str]) -> SpreadsheetWarning {
        SpreadsheetWarning {
            kind,
            sheets: sheets.iter().map(|s| s.to_string()).collect(),
            detail: None,
        }
    }

    /// A present (not `Missing`) side, so "identical" is decided by the diff.
    fn text_doc() -> forskscope_core::document::LoadedDocument {
        let mut d = forskscope_core::document::LoadedDocument::empty();
        d.kind = forskscope_core::file_kind::FileKind::Text;
        d
    }

    fn snapshot_with(warnings: Vec<SpreadsheetWarning>, lang: Lang) -> TabSnapshot {
        use crate::state::tab::{CompareLaunchMode, TabState};
        use forskscope_core::compare_prep::{SaveCapability, SaveCapabilityBlockReason};
        use forskscope_core::{DiffDocument, DiffOptions, MergeSession};
        use forskscope_ui_logic::{CompareTabId, LoadGeneration};

        let tab = crate::state::CompareTab {
            id: CompareTabId::new(1).unwrap(),
            load_generation: LoadGeneration::new(1).unwrap(),
            title: "t".into(),
            left_path: None,
            right_path: None,
            state: TabState::Ready,
            left_doc: text_doc(),
            right_doc: text_doc(),
            diff: DiffDocument::empty(),
            merge: MergeSession::empty(),
            diff_options: DiffOptions::default(),
            can_save: false,
            save_capability: SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText),
            spreadsheet_warnings: warnings,
            char_mode: false,
            word_wrap: false,
            focused_change: 0,
            save_target: None,
            launch_mode: CompareLaunchMode::Normal,
        };
        TabSnapshot::from_tab(&tab, 14, "monospace", 3, lang)
    }

    /// The warning is in the strip that renders above the panes (the same one
    /// as "Diff timed out"), one line per kind, with the sheets named; and a
    /// warned pair never claims to be identical. Falsify by dropping the
    /// `warnings.extend` in `from_tab`: the first assertion fails; by dropping
    /// the `is_empty()` test on `identical`: the second fails.
    #[test]
    fn a_spreadsheet_warning_is_in_the_warning_strip_and_a_warned_pair_is_not_identical() {
        let snap = snapshot_with(
            vec![
                warning(K::AmbiguousSheetMatch, &["Gamma", "Delta"]),
                warning(K::SheetNotCompared, &["Chart1"]),
            ],
            Lang::En,
        );
        assert_eq!(
            snap.warnings,
            vec![
                "Several sheets could have been renamed, so none was matched; they appear as added and removed. Sheets: ‘Gamma’, ‘Delta’".to_string(),
                "Not compared (not a worksheet): ‘Chart1’".to_string(),
            ]
        );
        // An empty DiffDocument is identical; the warning is what stops the
        // green "Files are identical".
        assert!(!snap.identical);
        assert!(snapshot_with(vec![], Lang::En).identical);
    }

    #[test]
    fn the_warning_is_localised() {
        let snap = snapshot_with(vec![warning(K::SheetNotCompared, &["Chart1"])], Lang::Ja);
        assert_eq!(
            snap.warnings,
            vec!["比較されていません（ワークシートではありません）: ‘Chart1’".to_string()]
        );
    }

    /// Fifty sheets is one line naming five and counting the rest, not fifty.
    #[test]
    fn many_sheets_are_one_line_with_a_count() {
        let names: Vec<String> = (1..=50).map(|n| format!("S{n}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let snap = snapshot_with(vec![warning(K::AlignmentFellBack, &refs)], Lang::En);
        assert_eq!(snap.warnings.len(), 1);
        let line = &snap.warnings[0];
        assert!(
            line.ends_with("‘S1’, ‘S2’, ‘S3’, ‘S4’, ‘S5’, (+45)"),
            "{line}"
        );
    }

    #[test]
    fn an_unknown_warning_shows_the_parsers_own_message() {
        let mut w = warning(K::Other, &[]);
        w.detail = Some("something new upstream".into());
        let snap = snapshot_with(vec![w], Lang::En);
        assert_eq!(
            snap.warnings,
            vec!["The comparison reported a warning: something new upstream".to_string()]
        );
    }
}
