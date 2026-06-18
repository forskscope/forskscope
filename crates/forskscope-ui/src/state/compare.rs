//! Comparison lifecycle: open, reload, load_and_diff, and directory tabs.

use std::path::PathBuf;

use dioxus::prelude::*;
use forskscope_core::diff::DiffDocument;
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};

use crate::i18n::t;
use crate::state::{Store, settings::Lang};
use crate::state::tab::{CompareTab, TabState, tab_title};

pub fn reload_tab(store: &mut Store, index: usize) {
    let (lp, rp, opts) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return };
        (tab.left_path.clone(), tab.right_path.clone(), tab.diff_options)
    };
    let enable_binary = store.settings.read().enable_binary_comparison;

    if let Some(tab) = store.tabs.write().get_mut(index) {
        tab.state = TabState::Loading;
    }

    let lang           = store.lang();
    let mut tabs_signal = store.tabs;

    spawn(async move {
        let left  = lp.unwrap_or_default();
        let right = rp.unwrap_or_default();
        let result = tokio::task::spawn_blocking(move || {
            load_and_diff(left, right, opts, lang, enable_binary)
        }).await;

        let mut tabs = tabs_signal.write();
        let Some(tab) = tabs.get_mut(index) else { return; };
        if tab.state != TabState::Loading { return; }

        match result {
            Ok(Ok((ld, rd, diff, merge, can_save))) => {
                tab.state          = TabState::Ready;
                tab.left_doc       = ld;
                tab.right_doc      = rd;
                tab.diff           = diff;
                tab.merge          = merge;
                tab.can_save       = can_save;
                tab.char_mode      = false;
                tab.focused_change = 0;
            }
            Ok(Err(msg)) => { tab.state = TabState::Error(msg); }
            Err(_)       => { tab.state = TabState::Error(t(lang, "Could not open").into()); }
        }
    });
}

pub fn open_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    let (opts, enable_binary) = {
        let settings = store.settings.read();
        let opts = settings.profiles
            .get(settings.active_profile)
            .map(|p| p.to_diff_options())
            .unwrap_or_default();
        (opts, settings.enable_binary_comparison)
    };

    let title = tab_title(&left, &right, store.lang());
    let tab = CompareTab {
        title,
        left_path: Some(left.clone()), right_path: Some(right.clone()),
        state: TabState::Loading,
        left_doc: LoadedDocument::empty(), right_doc: LoadedDocument::empty(),
        diff: DiffDocument::empty(), merge: MergeSession::empty(),
        diff_options: opts, can_save: false,
        char_mode: false, word_wrap: false, focused_change: 0,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(idx));

    let mut tabs_signal = store.tabs;
    let lang            = store.lang();

    spawn(async move {
        let load_result = tokio::task::spawn_blocking(move || {
            load_and_diff(left, right, opts, lang, enable_binary)
        }).await;

        let mut tabs = tabs_signal.write();
        let Some(tab) = tabs.get_mut(idx) else { return; };
        if tab.state != TabState::Loading { return; }

        match load_result {
            Ok(Ok((ld, rd, diff, merge, can_save))) => {
                tab.state     = TabState::Ready;
                tab.left_doc  = ld;
                tab.right_doc = rd;
                tab.diff      = diff;
                tab.merge     = merge;
                tab.can_save  = can_save;
            }
            Ok(Err(msg))   => { tab.state = TabState::Error(msg); }
            Err(_join_err) => { tab.state = TabState::Error(t(lang, "Could not open").into()); }
        }
    });
}

/// Load, classify, and diff two files off the UI thread (RFC-065).
pub(super) fn load_and_diff(
    left: PathBuf, right: PathBuf, opts: DiffOptions, lang: Lang,
    enable_binary: bool,
) -> Result<(LoadedDocument, LoadedDocument, DiffDocument, MergeSession, bool), String> {
    let options = LoadOptions { allow_missing: true };

    let mut ld = load_path(&left, options).map_err(|e| format!(
        "{} \"{}\" — {e}. {}",
        t(lang, "Could not open"),
        left.file_name().map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| left.display().to_string()),
        t(lang, "Check that the file exists and you have read permission.")
    ))?;

    let mut rd = load_path(&right, options).map_err(|e| format!(
        "{} \"{}\" — {e}. {}",
        t(lang, "Could not open"),
        right.file_name().map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| right.display().to_string()),
        t(lang, "Check that the file exists and you have read permission.")
    ))?;

    let l_bin = matches!(ld.kind, FileKind::Binary);
    let r_bin = matches!(rd.kind, FileKind::Binary);
    if (l_bin || r_bin) && !enable_binary {
        return Err(t(lang, "Binary comparison is off. Enable it in Settings → Advanced.").into());
    }

    let l_text = matches!(ld.kind, FileKind::Text);
    let r_text = matches!(rd.kind, FileKind::Text);
    if (l_bin && r_text) || (l_text && r_bin) {
        return Err(t(lang, "Cannot compare: one file is binary and the other is text. Compare text with text, or binary with binary.").into());
    }

    if ld.kind == FileKind::ExcelXlsx && rd.kind == FileKind::ExcelXlsx {
        let (lt, rt) = forskscope_core::xlsx::derive_pair_text(&left, &right);
        ld.text = Some(lt); rd.text = Some(rt);
    }

    let diff     = compute_diff(ld.diff_text(), rd.diff_text(), opts);
    let merge    = MergeSession::from_diff(&diff);
    let can_save = ld.kind.is_mergeable_text() && rd.kind.is_mergeable_text();
    Ok((ld, rd, diff, merge, can_save))
}

pub fn open_dir_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    store.dir_tabs.write().push((left, right));
    let idx = store.dir_tabs.read().len() - 1;
    store.active.set(None);
    store.active_dir.set(Some(idx));
}

pub fn close_dir_tab(store: &mut Store, index: usize) {
    store.dir_tabs.write().remove(index);
    let len = store.dir_tabs.read().len();
    let cur = *store.active_dir.read();
    if len == 0 {
        store.active_dir.set(None);
    } else if cur == Some(index) {
        store.active_dir.set(Some(index.saturating_sub(1).min(len - 1)));
    } else if cur > Some(index) {
        store.active_dir.set(cur.map(|i| i - 1));
    }
}
