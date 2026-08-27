//! Comparison lifecycle: open, reload, load_and_diff, and directory tabs.

use std::fs;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use dioxus_core::spawn_forever;
use forskscope_core::compare_prep::{
    PreparedCompare, inspect_save_target, save_target_from_loaded,
};
use forskscope_core::diff::{DiffDocument, InlineMode};
use forskscope_core::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_core::file_kind::FileKind;
use forskscope_core::{DiffOptions, MergeSession, compute_diff};
use forskscope_ui_logic::{
    CompareRequest, CompletionDecision, LoadGeneration, LoadGuard, LoadIdentitySnapshot, LoadToken,
    SaveDestination, completion_decision, guard_for_sizes,
};

use crate::i18n::t;
use crate::state::tab::{CompareLaunchMode, CompareTab, TabState, tab_title};
use crate::state::types::{LargeLoadPrompt, LargeLoadTarget};
use crate::state::{Modal, Store, save_session, settings::Lang};

// ── F84: pre-load size guard (RFC-013 §"Large file prompt") ────────────────────

/// Byte size of the file at `path`, or `0` if it cannot be read. A side may
/// legitimately be absent (`LoadOptions { allow_missing: true }` — the
/// existing missing-file handling in `load_and_diff` reports that on its
/// own), so a failed `metadata` call must not be turned into a guard
/// failure; treating it as `0` bytes always classifies as `Small` and lets
/// that entry alone decide the outcome.
fn size_or_zero(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// What a call site should do about one file pair, given their sizes and
/// the `DiffOptions` it would otherwise use. Pure — no I/O, no `Store` — so
/// both the guard's own reachability and `suppress_inline`'s effect on
/// `opts` are directly testable without a Dioxus runtime (handoff 011 §7:
/// the same reasoning `classify_entry`/`apply_epoch_result` already
/// established in this codebase).
enum LoadDecision {
    /// Proceed now with `opts` (inline-suppressed if the guard demanded
    /// it). `banner` is the non-blocking notice to show first, if any.
    Go {
        opts: DiffOptions,
        banner: Option<String>,
    },
    /// Block: nothing has been loaded. `opts` is the inline-suppressed
    /// options a confirmed resume must use (`ConfirmPrompt` always implies
    /// `suppress_inline()`).
    Confirm {
        opts: DiffOptions,
        title: String,
        body: String,
        confirm_label: String,
        too_large: bool,
    },
}

fn decide_load(left_bytes: u64, right_bytes: u64, opts: DiffOptions) -> LoadDecision {
    let guard = guard_for_sizes(left_bytes, right_bytes);
    let mut adjusted = opts;
    if guard.suppress_inline() {
        adjusted.inline_mode = InlineMode::None;
    }
    match guard {
        LoadGuard::Proceed => LoadDecision::Go {
            opts: adjusted,
            banner: None,
        },
        LoadGuard::WarnBanner { message, .. } => LoadDecision::Go {
            opts: adjusted,
            banner: Some(message),
        },
        LoadGuard::ConfirmPrompt {
            title,
            body,
            confirm_label,
            too_large,
            ..
        } => LoadDecision::Confirm {
            opts: adjusted,
            title,
            body,
            confirm_label,
            too_large,
        },
    }
}

enum LoadResult {
    Ready(Box<PreparedCompare>),
    Error(String),
}

/// Install a prepared load only when its complete runtime token is still live.
/// `PreparedCompare` commits `left_doc`/`right_doc`/`diff`/`merge`/`can_save`/
/// `save_target` together (RFC-077) — the same atomicity RFC-075 already gave
/// tab identity, extended so a save target is never installed from a
/// different load than the documents it was derived from.
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
        LoadResult::Ready(prepared) => {
            tab.state = TabState::Ready;
            tab.left_doc = prepared.left;
            tab.right_doc = prepared.right;
            tab.diff = prepared.diff;
            tab.merge = prepared.merge;
            tab.can_save = prepared.can_save;
            tab.save_target = Some(prepared.save_target);
            tab.char_mode = false;
            tab.focused_change = 0;
        }
        LoadResult::Error(message) => {
            tab.state = TabState::Error(message);
        }
    }
    decision
}

fn prepared_result(result: Result<PreparedCompare, String>) -> LoadResult {
    match result {
        Ok(prepared) => LoadResult::Ready(Box::new(prepared)),
        Err(message) => LoadResult::Error(message),
    }
}

/// Checked entry point — every existing caller (the `ReloadModal` unsaved-
/// work confirmation included) goes through this, so the size guard is
/// consulted on every reload, not just the first load (handoff 011 §5: "a
/// user re-opens the file they were warned about" is exactly the hole a
/// one-sided check would leave). Reads the tab without mutating anything;
/// a `ConfirmPrompt` outcome leaves the tab exactly as it was.
pub fn reload_tab(store: &mut Store, index: usize) {
    let (left_path, right_path, opts) = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else {
            return;
        };
        (
            tab.left_path.clone().unwrap_or_default(),
            tab.right_path.clone().unwrap_or_default(),
            tab.diff_options,
        )
    };
    let decision = decide_load(size_or_zero(&left_path), size_or_zero(&right_path), opts);
    match decision {
        LoadDecision::Go { opts, banner } => {
            if let Some(message) = banner {
                store.notify_warning(message);
            }
            reload_tab_with_options(store, index, opts);
        }
        LoadDecision::Confirm {
            opts,
            title,
            body,
            confirm_label,
            too_large,
        } => {
            store.modal.set(Modal::ConfirmLargeLoad(LargeLoadPrompt {
                target: LargeLoadTarget::Reload(index),
                opts,
                title,
                body,
                confirm_label,
                too_large,
            }));
        }
    }
}

/// The actual reload, unconditional — used by [`reload_tab`] once the guard
/// says `Go`, and by `LargeLoadModal`'s confirm handler directly (the guard
/// already ran once to produce `opts`; running it again here would either
/// repeat the same prompt the user just dismissed, or silently re-derive a
/// fresh "current" `opts` that discards the suppression they already
/// accepted).
pub(crate) fn reload_tab_with_options(store: &mut Store, index: usize, opts: DiffOptions) {
    let (request, token) = {
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
        // Reload re-derives its request from `launch_mode` rather than
        // re-deciding launch mode — the tab's save destination must not
        // silently change identity on reload (RFC-077).
        let save_destination = match &tab.launch_mode {
            CompareLaunchMode::Normal => SaveDestination::RightInput,
            CompareLaunchMode::MergeTool { merged } => SaveDestination::Explicit(merged.clone()),
        };
        let request = CompareRequest {
            left_input: tab.left_path.clone().unwrap_or_default(),
            right_input: tab.right_path.clone().unwrap_or_default(),
            save_destination,
        };
        (request, LoadToken::new(tab.id, generation))
    };
    let enable_binary = store.settings.read().enable_binary_comparison;

    let lang = store.lang();
    let mut tabs_signal = store.tabs;

    // spawn_forever: reload must survive any component remounting during load.
    spawn_forever(async move {
        let result =
            tokio::task::spawn_blocking(move || load_and_diff(request, opts, lang, enable_binary))
                .await;

        let mut tabs = tabs_signal.write();
        let result = match result {
            Ok(result) => prepared_result(result),
            Err(_) => LoadResult::Error(t(lang, "Could not open")),
        };
        commit_load_result(&mut tabs, token, result);
    });
}

/// Open a normal two-file comparison — `git difftool`-style, Explorer
/// clicks, deep-compare, and session restore all use this. Thin wrapper over
/// [`open_compare_request`] kept as its own function so these many in-app
/// call sites never need to construct a `CompareRequest` themselves for what
/// is always the same `SaveDestination::RightInput` case.
pub fn open_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    open_compare_request(
        store,
        CompareRequest {
            left_input: left,
            right_input: right,
            save_destination: SaveDestination::RightInput,
        },
    );
}

/// Open a comparison from a fully-formed request — the one entry point that
/// understands both normal compare and Git mergetool mode (RFC-077).
/// `app.rs`'s startup wiring is the only caller that constructs a
/// `MergeTool`-derived request directly; everything else goes through
/// [`open_compare`]. Checked: the size guard is consulted before anything
/// is allocated — a `ConfirmPrompt` outcome creates no tab at all, so
/// cancelling leaves nothing to clean up (handoff 011 §5).
pub fn open_compare_request(store: &mut Store, request: CompareRequest) {
    let opts = {
        let settings = store.settings.read();
        settings
            .profiles
            .get(settings.active_profile)
            .map(|p| p.to_diff_options())
            .unwrap_or_default()
    };
    let decision = decide_load(
        size_or_zero(&request.left_input),
        size_or_zero(&request.right_input),
        opts,
    );
    match decision {
        LoadDecision::Go { opts, banner } => {
            if let Some(message) = banner {
                store.notify_warning(message);
            }
            open_compare_request_with_options(store, request, opts);
        }
        LoadDecision::Confirm {
            opts,
            title,
            body,
            confirm_label,
            too_large,
        } => {
            store.modal.set(Modal::ConfirmLargeLoad(LargeLoadPrompt {
                target: LargeLoadTarget::Open(request),
                opts,
                title,
                body,
                confirm_label,
                too_large,
            }));
        }
    }
}

/// The actual open, unconditional — used by [`open_compare_request`] once
/// the guard says `Go`, and by `LargeLoadModal`'s confirm handler directly,
/// for the same reason [`reload_tab_with_options`] exists.
pub(crate) fn open_compare_request_with_options(
    store: &mut Store,
    request: CompareRequest,
    opts: DiffOptions,
) {
    let id = match store.allocate_compare_tab_id() {
        Ok(id) => id,
        Err(error) => {
            store.notify(error.to_string());
            return;
        }
    };
    let enable_binary = store.settings.read().enable_binary_comparison;

    let left = request.left_input.clone();
    let right = request.right_input.clone();
    let launch_mode = match &request.save_destination {
        SaveDestination::RightInput => CompareLaunchMode::Normal,
        SaveDestination::Explicit(merged) => CompareLaunchMode::MergeTool {
            merged: merged.clone(),
        },
    };

    let mut title = tab_title(&left, &right, store.lang());
    if matches!(launch_mode, CompareLaunchMode::MergeTool { .. }) {
        title = format!("{title} ({})", t(store.lang(), "merge"));
    }

    let generation = LoadGeneration::INITIAL;
    let tab = CompareTab {
        id,
        load_generation: generation,
        title,
        left_path: Some(left),
        right_path: Some(right),
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
        save_target: None,
        launch_mode,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    store.active.set(Some(idx));
    // F61: session persistence is an explicit call, not the reactive
    // `use_effect` app.rs used to have on `store.tabs` - confirmed on a
    // real desktop process that the effect never runs for a signal write
    // made here (synchronously, during startup hook execution, outside any
    // discrete UI-event dispatch), even though the exact same write
    // correctly triggers a visual re-render. Only a write made from inside
    // a real Dioxus event handler (e.g. `close_tab`'s `onclick`) reliably
    // flushed the effect queue. See ROADMAP.md's F61 entry for the full
    // account.
    save_session(store);

    let mut tabs_signal = store.tabs;
    let lang = store.lang();
    let token = LoadToken::new(id, generation);

    // spawn_forever: the task must survive the Explorer unmounting when the
    // new tab opens and replaces it with DiffWorkspace (RFC-065).
    spawn_forever(async move {
        let load_result =
            tokio::task::spawn_blocking(move || load_and_diff(request, opts, lang, enable_binary))
                .await;

        let mut tabs = tabs_signal.write();
        let result = match load_result {
            Ok(result) => prepared_result(result),
            Err(_join_err) => LoadResult::Error(t(lang, "Could not open")),
        };
        commit_load_result(&mut tabs, token, result);
        // No save_session call needed here: the session payload only ever
        // holds left_path/right_path (build_save_payload), both already
        // final at the synchronous push above - the diff/load outcome
        // this task commits doesn't change what gets persisted.
    });
}

/// Load, classify, diff, and derive the save target for one comparison off
/// the UI thread (RFC-065, RFC-077). Normal compare's save target *is* the
/// already-loaded right document (`compare_prep::save_target_from_loaded`,
/// no extra I/O); Git mergetool mode independently inspects the merged
/// output path (`compare_prep::inspect_save_target`) — its content is never
/// fed into `left`/`right`.
pub(super) fn load_and_diff(
    request: CompareRequest,
    opts: DiffOptions,
    lang: Lang,
    enable_binary: bool,
) -> Result<PreparedCompare, String> {
    let CompareRequest {
        left_input: left,
        right_input: right,
        save_destination,
    } = request;
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
    let save_target = match &save_destination {
        SaveDestination::RightInput => save_target_from_loaded(&right, &rd),
        SaveDestination::Explicit(merged) => {
            let fallback_encoding = rd
                .text
                .as_ref()
                .map(|t| t.encoding.label.clone())
                .unwrap_or_else(|| "UTF-8".into());
            inspect_save_target(merged, &fallback_encoding)
        }
    };
    Ok(PreparedCompare {
        left: ld,
        right: rd,
        diff,
        merge,
        save_target,
        can_save,
    })
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
