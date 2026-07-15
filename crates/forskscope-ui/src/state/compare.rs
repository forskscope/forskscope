//! Comparison lifecycle: open, reload, load_and_diff, and directory tabs.

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_core::spawn_forever;
use forskscope_core::diff::DiffDocument;
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};
use forskscope_ui_logic::{
    CompletionDecision, LoadGeneration, LoadIdentitySnapshot, LoadToken, completion_decision,
};

use crate::i18n::t;
use crate::state::tab::{CompareTab, TabState, tab_title};
use crate::state::{Store, settings::Lang};

struct LoadedComparison {
    left_doc: LoadedDocument,
    right_doc: LoadedDocument,
    diff: DiffDocument,
    merge: MergeSession,
    can_save: bool,
}

enum LoadResult {
    Ready(Box<LoadedComparison>),
    Error(String),
}

/// Install a prepared load only when its complete runtime token is still live.
fn commit_load_result(
    tabs: &mut [CompareTab],
    token: LoadToken,
    result: LoadResult,
) -> CompletionDecision {
    let Some(tab) = tabs.iter_mut().find(|tab| tab.id == token.tab_id) else {
        return completion_decision(token, None);
    };
    let snapshot = LoadIdentitySnapshot::new(
        LoadToken::new(tab.id, tab.load_generation),
        tab.state == TabState::Loading,
    );
    let decision = completion_decision(token, Some(snapshot));
    if decision != CompletionDecision::Accept {
        return decision;
    }

    match result {
        LoadResult::Ready(loaded) => {
            tab.state = TabState::Ready;
            tab.left_doc = loaded.left_doc;
            tab.right_doc = loaded.right_doc;
            tab.diff = loaded.diff;
            tab.merge = loaded.merge;
            tab.can_save = loaded.can_save;
            tab.char_mode = false;
            tab.focused_change = 0;
        }
        LoadResult::Error(message) => {
            tab.state = TabState::Error(message);
        }
    }
    decision
}

fn prepared_result(
    result: Result<
        (
            LoadedDocument,
            LoadedDocument,
            DiffDocument,
            MergeSession,
            bool,
        ),
        String,
    >,
) -> LoadResult {
    match result {
        Ok((left_doc, right_doc, diff, merge, can_save)) => {
            LoadResult::Ready(Box::new(LoadedComparison {
                left_doc,
                right_doc,
                diff,
                merge,
                can_save,
            }))
        }
        Err(message) => LoadResult::Error(message),
    }
}

pub fn reload_tab(store: &mut Store, index: usize) {
    let (lp, rp, opts, token) = {
        let mut tabs = store.tabs.write();
        let Some(tab) = tabs.get_mut(index) else {
            return;
        };
        let generation = match tab.load_generation.next() {
            Ok(generation) => generation,
            Err(error) => {
                tab.state = TabState::Error(error.to_string());
                return;
            }
        };
        tab.load_generation = generation;
        tab.state = TabState::Loading;
        (
            tab.left_path.clone(),
            tab.right_path.clone(),
            tab.diff_options,
            LoadToken::new(tab.id, generation),
        )
    };
    let enable_binary = store.settings.read().enable_binary_comparison;

    let lang = store.lang();
    let mut tabs_signal = store.tabs;

    // spawn_forever: reload must survive any component remounting during load.
    spawn_forever(async move {
        let left = lp.unwrap_or_default();
        let right = rp.unwrap_or_default();
        let result = tokio::task::spawn_blocking(move || {
            load_and_diff(left, right, opts, lang, enable_binary)
        })
        .await;

        let mut tabs = tabs_signal.write();
        let result = match result {
            Ok(result) => prepared_result(result),
            Err(_) => LoadResult::Error(t(lang, "Could not open")),
        };
        commit_load_result(&mut tabs, token, result);
    });
}

pub fn open_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    let id = match store.allocate_compare_tab_id() {
        Ok(id) => id,
        Err(error) => {
            store.notify(error.to_string());
            return;
        }
    };
    let (opts, enable_binary) = {
        let settings = store.settings.read();
        let opts = settings
            .profiles
            .get(settings.active_profile)
            .map(|p| p.to_diff_options())
            .unwrap_or_default();
        (opts, settings.enable_binary_comparison)
    };

    let title = tab_title(&left, &right, store.lang());
    let generation = LoadGeneration::INITIAL;
    let tab = CompareTab {
        id,
        load_generation: generation,
        title,
        left_path: Some(left.clone()),
        right_path: Some(right.clone()),
        state: TabState::Loading,
        left_doc: LoadedDocument::empty(),
        right_doc: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
        diff_options: opts,
        can_save: false,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(idx));

    let mut tabs_signal = store.tabs;
    let lang = store.lang();
    let token = LoadToken::new(id, generation);

    // spawn_forever: the task must survive the Explorer unmounting when the
    // new tab opens and replaces it with DiffWorkspace (RFC-065).
    spawn_forever(async move {
        let load_result = tokio::task::spawn_blocking(move || {
            load_and_diff(left, right, opts, lang, enable_binary)
        })
        .await;

        let mut tabs = tabs_signal.write();
        let result = match load_result {
            Ok(result) => prepared_result(result),
            Err(_join_err) => LoadResult::Error(t(lang, "Could not open")),
        };
        commit_load_result(&mut tabs, token, result);
    });
}

/// Load, classify, and diff two files off the UI thread (RFC-065).
pub(super) fn load_and_diff(
    left: PathBuf,
    right: PathBuf,
    opts: DiffOptions,
    lang: Lang,
    enable_binary: bool,
) -> Result<
    (
        LoadedDocument,
        LoadedDocument,
        DiffDocument,
        MergeSession,
        bool,
    ),
    String,
> {
    let options = LoadOptions {
        allow_missing: true,
    };

    let ld = load_path(&left, options).map_err(|e| {
        format!(
            "{} \"{}\" — {e}. {}",
            t(lang, "Could not open"),
            left.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| left.display().to_string()),
            t(
                lang,
                "Check that the file exists and you have read permission."
            )
        )
    })?;

    let rd = load_path(&right, options).map_err(|e| {
        format!(
            "{} \"{}\" — {e}. {}",
            t(lang, "Could not open"),
            right
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| right.display().to_string()),
            t(
                lang,
                "Check that the file exists and you have read permission."
            )
        )
    })?;

    let l_bin = matches!(ld.kind, FileKind::Binary);
    let r_bin = matches!(rd.kind, FileKind::Binary);
    if (l_bin || r_bin) && !enable_binary {
        return Err(t(
            lang,
            "Binary comparison is off. Enable it in Settings → Advanced.",
        ));
    }

    let l_text = matches!(ld.kind, FileKind::Text);
    let r_text = matches!(rd.kind, FileKind::Text);
    if (l_bin && r_text) || (l_text && r_bin) {
        return Err(t(
            lang,
            "Cannot compare: one file is binary and the other is text. Compare text with text, or binary with binary.",
        ));
    }

    if ld.kind == FileKind::ExcelXlsx || rd.kind == FileKind::ExcelXlsx {
        return Err(t(
            lang,
            "Spreadsheet comparison is temporarily disabled for security.",
        ));
    }

    let diff = compute_diff(ld.diff_text(), rd.diff_text(), opts);
    let merge = MergeSession::from_diff(&diff);
    let can_save = ld.kind.is_mergeable_text() && rd.kind.is_mergeable_text();
    Ok((ld, rd, diff, merge, can_save))
}

#[cfg(test)]
mod tests;

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
        store
            .active_dir
            .set(Some(index.saturating_sub(1).min(len - 1)));
    } else if cur > Some(index) {
        store.active_dir.set(cur.map(|i| i - 1));
    }
}
