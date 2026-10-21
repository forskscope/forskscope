//! Comparison lifecycle: open, reload, load_and_diff, and directory tabs.

use std::fs;
use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use dioxus_core::spawn_forever;
use forskscope_compare::diff::{DiffDocument, InlineMode};
use forskscope_compare::document::{LoadOptions, LoadedDocument, load_path};
use forskscope_compare::file_kind::FileKind;
use forskscope_compare::{DiffOptions, compute_diff};
use forskscope_edit::MergeSession;
use forskscope_edit::compare_prep::{
    PreparedCompare, SaveCapability, SaveCapabilityBlockReason, inspect_save_target,
    save_capability, save_target_from_loaded,
};
use forskscope_ui_logic::{
    CompareRequest, CompletionDecision, LoadGeneration, LoadGuard, LoadIdentitySnapshot, LoadToken,
    SaveDestination, completion_decision, guard_for_sizes,
};

use crate::i18n::t;
use crate::state::tab::{CompareLaunchMode, CompareTab, TabState, tab_title};
use crate::state::types::{LargeLoadPrompt, LargeLoadTarget};
use crate::state::{Modal, Store, settings::Lang};

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

/// Whether both sides are `.xlsx` workbooks, so the large-file prompt can say
/// what a spreadsheet comparison does instead of what a text one does.
fn is_spreadsheet_pair(left: &Path, right: &Path) -> bool {
    forskscope_compare::path::has_extension(left, "xlsx")
        && forskscope_compare::path::has_extension(right, "xlsx")
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
/// `PreparedCompare` commits `left_doc`/`right_doc`/`diff`/`merge`/
/// `save_capability`/`save_target` together (RFC-077) — the same atomicity
/// RFC-075 already gave tab identity, extended so a save target is never
/// installed from a different load than the documents it was derived from.
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
            tab.can_save = prepared.save_capability.is_saveable();
            tab.save_capability = prepared.save_capability;
            tab.spreadsheet_warnings = prepared.spreadsheet_warnings;
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
                spreadsheet: is_spreadsheet_pair(&left_path, &right_path),
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
            let spreadsheet = is_spreadsheet_pair(&request.left_input, &request.right_input);
            store.modal.set(Modal::ConfirmLargeLoad(LargeLoadPrompt {
                target: LargeLoadTarget::Open(request),
                opts,
                title,
                body,
                confirm_label,
                too_large,
                spreadsheet,
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
        save_capability: SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText),
        spreadsheet_warnings: Vec::new(),
        // F168 §2.2: a profile with character highlighting set to
        // "Automatically for small changes" starts a new tab with it
        // already on, rather than requiring the toolbar toggle first.
        char_mode: opts.inline_mode == InlineMode::EagerForSmallHunks,
        word_wrap: false,
        focused_change: 0,
        save_target: None,
        launch_mode,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    // F61: session persistence is an explicit call, not the reactive
    // `use_effect` app.rs used to have on `store.tabs` - confirmed on a
    // real desktop process that the effect never runs for a signal write
    // made here (synchronously, during startup hook execution, outside any
    // discrete UI-event dispatch), even though the exact same write
    // correctly triggers a visual re-render. Only a write made from inside
    // a real Dioxus event handler (e.g. `close_tab`'s `onclick`) reliably
    // flushed the effect queue. See ROADMAP.md's F61 entry for the full
    // account. `set_active_tab` (F194) is now that explicit call, since it
    // always saves when it sets `store.active` — kept here rather than
    // relying on a caller elsewhere to do it.
    super::session::set_active_tab(store, Some(idx));

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

/// Whether advice about the file's existence or permissions fits `error`
/// (F122): only a failed read or stat does. A directory given as a file, an
/// unsupported kind or a decode failure is not cured by checking that the file
/// exists, so it gets none — the default is no guidance, not wrong guidance.
fn wants_access_guidance(error: &forskscope_compare::CoreError) -> bool {
    use forskscope_compare::{CoreError, IoOperation};
    matches!(
        error,
        CoreError::Io {
            operation: IoOperation::Read | IoOperation::Metadata | IoOperation::ListDir,
            ..
        } | CoreError::InvalidPath { .. }
    )
}

/// A path's own file/folder name, falling back to the full path only when
/// it has none (e.g. `/`) — shared by every message that names a path
/// without claiming to show a full path (RFC-027's own display-name
/// default).
fn display_name_only(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The text for a side that could not be opened. Guidance is appended here,
/// by the error's class, and nowhere else: the error tab shows this message
/// as it is.
fn open_error(lang: Lang, path: &Path, error: &forskscope_compare::CoreError) -> String {
    let name = display_name_only(path);
    let mut message = format!("{} \"{name}\" — {error}.", t(lang, "Could not open"));
    if wants_access_guidance(error) {
        message.push(' ');
        message.push_str(&t(
            lang,
            "Check that the file exists and you have read permission.",
        ));
    }
    message
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

    let mut ld = load_path(&left, options).map_err(|e| open_error(lang, &left, &e))?;

    let mut rd = load_path(&right, options).map_err(|e| open_error(lang, &right, &e))?;

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

    // RFC-085: only a pair where *both* sides are spreadsheets gets the
    // derived text projection — one .xlsx side against a text/missing side
    // falls through with that side's `diff_text()` staying empty, same as
    // the pre-suspension behavior this restores. Comparison stays read-only
    // either way: `FileKind::ExcelXlsx -> EditabilityClass::ReadOnly` and
    // `save_capability`'s NotMergeableText refusal are unchanged (RFC-085
    // scope boundary — merge/save for .xlsx is a separate RFC).
    let mut spreadsheet_warnings = Vec::new();
    if ld.kind == FileKind::ExcelXlsx && rd.kind == FileKind::ExcelXlsx {
        // F117: an error here (corrupt workbook, or the size bound) must
        // reach the user as an error. Falling through with two empty sides
        // would diff as identical and report "these workbooks match" for a
        // pair that was never fully compared.
        let pair = forskscope_compare::xlsx::compare_pair(&left, &right).map_err(|e| {
            let detail = match e {
                forskscope_compare::CoreError::Unsupported { message } => message,
                other => other.to_string(),
            };
            format!(
                "{} — {detail}",
                t(lang, "Could not compare the spreadsheets")
            )
        })?;
        ld.text = Some(pair.left);
        rd.text = Some(pair.right);
        spreadsheet_warnings = pair.warnings;
    }

    let diff = compute_diff(ld.diff_text(), rd.diff_text(), opts);
    let merge = MergeSession::from_diff(&diff);
    let save_target = match &save_destination {
        SaveDestination::RightInput => save_target_from_loaded(&right, &rd),
        SaveDestination::Explicit(merged) => {
            let fallback_encoding = rd
                .text
                .as_ref()
                .map(|t| t.encoding.label.clone())
                .unwrap_or_else(|| "UTF-8".into());
            let fallback_bom = rd.text.as_ref().map(|t| t.bom).unwrap_or_default();
            inspect_save_target(merged, &fallback_encoding, fallback_bom)
        }
    };
    // F88/RFC-082 §D3: one source of truth for whether a save is possible —
    // composed from both sides' FileKind/EditabilityClass and the resolved
    // target's state, never a pair-wide `is_mergeable_text()` expression.
    let save_capability = save_capability(
        &ld.kind,
        &rd.kind,
        ld.editability(),
        rd.editability(),
        ld.had_decode_errors(),
        rd.had_decode_errors(),
        &save_target.state,
    );
    Ok(PreparedCompare {
        left: ld,
        right: rd,
        diff,
        merge,
        save_target,
        save_capability,
        spreadsheet_warnings,
    })
}

#[cfg(test)]
mod tests;

/// Derive a human-readable title for a directory comparison from its two
/// roots. The one function the tab bar's `DirTabItem` and the window-title
/// effect (RFC-089 §2) both call, so the two cannot disagree — mirrors
/// `state::tab::tab_title`'s file-tab equivalent.
pub(crate) fn dir_tab_title(l: &std::path::Path, r: &std::path::Path) -> String {
    let ln = l
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let rn = r
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if ln == rn {
        ln
    } else {
        format!("{ln} ↔ {rn}")
    }
}

pub fn open_dir_compare(store: &mut Store, left: PathBuf, right: PathBuf) {
    store.dir_tabs.write().push((left, right));
    let idx = store.dir_tabs.read().len() - 1;
    super::session::set_active_tab(store, None);
    store.active_dir.set(Some(idx));
}

// ── F209: classifying a startup request's paths (handoff 088) ──────────────
//
// `forskscope-ui-logic::parse_startup_args` stays pure and reads no file
// system — it only ever sees argument *counts* (0/2/3), never what a path
// actually names on disk. Two folders on the command line used to reach
// the ordinary two-file `open_compare_request` path anyway, which then
// failed with a raw "not a regular file" I/O error the moment it tried to
// read a folder as a file. This is the one place that fs-dependent
// decision — file, folder, or neither — actually lives, and it lives
// here (in the GUI, not in ui-logic) for exactly that reason.

/// What a startup path names on disk. `Other` covers "does not exist" and
/// "exists but its kind could not be read" alike — either way, today's
/// ordinary open error already handles it; F209's own scope is telling
/// files and folders apart, not re-deciding what a missing path means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    File,
    Folder,
    Other,
}

/// The real probe — `fs::metadata`, the same technique
/// `forskscope-cli compare`'s own file-vs-folder decision already uses
/// (see that crate's `compare.rs` module doc comment), never
/// `forskscope_compare::classify`, which reports a directory as "not a
/// regular file" rather than as a folder.
pub fn fs_probe(path: &Path) -> PathKind {
    match fs::metadata(path) {
        Ok(meta) if meta.is_dir() => PathKind::Folder,
        Ok(_) => PathKind::File,
        Err(_) => PathKind::Other,
    }
}

/// What a classified startup request should actually do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupAction {
    Explorer,
    /// Proceed exactly as today — covers file+file, a missing path on
    /// either side, and Git mergetool's own ordinary three-file case.
    /// Unchanged behavior; today's own open error still applies.
    AsRequested(forskscope_ui_logic::StartupRequest),
    /// Both sides of a two-argument launch are folders: open Deep Compare
    /// instead of a file tab.
    CompareFolders {
        left: PathBuf,
        right: PathBuf,
    },
    /// Refused in-window, in the app's own words — never a raw I/O error.
    /// `left`/`right` are what the resulting error tab shows (mergetool's
    /// own local/remote, when the refusal came from the third argument).
    Refused {
        left: PathBuf,
        right: PathBuf,
        message: String,
    },
}

/// Classifies a parsed `StartupRequest`'s paths with `probe`, never
/// touching the filesystem itself — a test passes a fake probe, covering
/// exactly the five shapes handoff 088 asks for: file+file, folder+folder,
/// file+folder, folder+file, and a missing path.
pub fn classify_startup_request(
    request: forskscope_ui_logic::StartupRequest,
    lang: Lang,
    probe: impl Fn(&Path) -> PathKind,
) -> StartupAction {
    use forskscope_ui_logic::StartupRequest;
    match request {
        StartupRequest::Explorer => StartupAction::Explorer,
        StartupRequest::Compare { left, right } => match (probe(&left), probe(&right)) {
            (PathKind::Folder, PathKind::Folder) => StartupAction::CompareFolders { left, right },
            (PathKind::File, PathKind::Folder) => StartupAction::Refused {
                message: file_folder_mismatch_message(lang, &left, &right, true),
                left,
                right,
            },
            (PathKind::Folder, PathKind::File) => StartupAction::Refused {
                message: file_folder_mismatch_message(lang, &left, &right, false),
                left,
                right,
            },
            _ => StartupAction::AsRequested(StartupRequest::Compare { left, right }),
        },
        StartupRequest::MergeTool {
            local,
            remote,
            merged,
        } => {
            // Git never passes a folder here — three arguments stay files
            // only. Checked against all three, not just local/remote: a
            // folder at the merged-output position is just as wrong.
            let folder = [&local, &remote, &merged]
                .into_iter()
                .find(|path| probe(path) == PathKind::Folder)
                .cloned();
            match folder {
                Some(path) => StartupAction::Refused {
                    message: mergetool_folder_message(lang, &path),
                    left: local,
                    right: remote,
                },
                None => StartupAction::AsRequested(StartupRequest::MergeTool {
                    local,
                    remote,
                    merged,
                }),
            }
        }
    }
}

/// "A file can only be compared with a file, and a folder with a folder.",
/// naming which side is which by its own display name — two whole-sentence
/// template shapes (one per which side is the file), matching this
/// codebase's established template-then-substitute pattern for a sentence
/// whose word order a translation cannot simply mirror.
fn file_folder_mismatch_message(
    lang: Lang,
    left: &Path,
    right: &Path,
    file_is_left: bool,
) -> String {
    // The quotes go in the *substituted* value, not the key - `xtask
    // i18n`'s own key extraction is line-based and not escape-aware, so a
    // key containing an escaped quote truncates there and can never
    // match between "used" and "translated" (see `docs/src/maintainers/
    // testing.md`'s i18n section). Each branch below passes its own
    // literal straight to the translator function, not through a
    // variable, so the audit can see both shapes as used at all.
    let left_name = format!("\"{}\"", display_name_only(left));
    let right_name = format!("\"{}\"", display_name_only(right));
    let template = if file_is_left {
        t(
            lang,
            "A file can only be compared with a file, and a folder with a folder. {left} is a file; {right} is a folder.",
        )
    } else {
        t(
            lang,
            "A file can only be compared with a file, and a folder with a folder. {left} is a folder; {right} is a file.",
        )
    };
    template
        .replace("{left}", &left_name)
        .replace("{right}", &right_name)
}

/// The three-argument (mergetool) shape of the same refusal — Git never
/// passes a folder there, so this names which of the three paths was one.
fn mergetool_folder_message(lang: Lang, path: &Path) -> String {
    let name = format!("\"{}\"", display_name_only(path));
    t(
        lang,
        "Git's merge tool compares files only. {name} is a folder.",
    )
    .replace("{name}", &name)
}

/// Opens a file tab exactly as a normal compare would, but already in
/// `TabState::Error(message)` — no load is ever attempted. Used only for
/// a startup file/folder mismatch (F209): the tab UI's own error surface
/// ("where an open error is shown today") shows a message this app wrote,
/// never the raw "not a regular file" I/O error `load_and_diff` would
/// otherwise reach.
pub fn open_startup_mismatch_error(
    store: &mut Store,
    left: PathBuf,
    right: PathBuf,
    message: String,
) {
    let id = match store.allocate_compare_tab_id() {
        Ok(id) => id,
        Err(error) => {
            store.notify(error.to_string());
            return;
        }
    };
    let title = tab_title(&left, &right, store.lang());
    let tab = CompareTab {
        id,
        load_generation: LoadGeneration::INITIAL,
        title,
        left_path: Some(left),
        right_path: Some(right),
        state: TabState::Error(message),
        left_doc: LoadedDocument::empty(),
        right_doc: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
        diff_options: DiffOptions::default(),
        can_save: false,
        save_capability: SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText),
        spreadsheet_warnings: Vec::new(),
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: None,
        launch_mode: CompareLaunchMode::Normal,
    };
    let idx = store.tabs.read().len();
    store.tabs.write().push(tab);
    super::session::set_active_tab(store, Some(idx));
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

/// Make the tab at flat `position` (RFC-087; see `state::tab_nav`) the
/// active one — what every navigation route (a tab-bar click, `Ctrl+Tab`,
/// the tab list's arrow keys) ultimately does. Goes through
/// [`super::session::set_active_tab`] for the file-tab case so the session
/// is saved on every one of those routes, not just an open or close
/// (F194).
pub(crate) fn activate_tab_at(
    store: &mut Store,
    position: usize,
    file_count: usize,
    dir_count: usize,
) {
    use super::tab_nav::TabKind;
    match super::tab_nav::kind_at(position, file_count, dir_count) {
        TabKind::Explorer => {
            super::session::set_active_tab(store, None);
            store.active_dir.set(None);
        }
        TabKind::File(i) => {
            super::session::set_active_tab(store, Some(i));
            store.active_dir.set(None);
        }
        TabKind::Dir(j) => {
            super::session::set_active_tab(store, None);
            store.active_dir.set(Some(j));
        }
    }
}

/// `Ctrl+Tab` / `Ctrl+PageDown` (`forward`) and `Ctrl+Shift+Tab` /
/// `Ctrl+PageUp` (`!forward`) (RFC-087 §2): the current flat position,
/// wrapped one step. Reads `store` for the current tab counts and active
/// state rather than taking them as parameters — its callers (the global
/// keydown handler) have nothing else to pass.
pub(crate) fn activate_adjacent_tab(store: &mut Store, forward: bool) {
    let file_count = store.tabs.read().len();
    let dir_count = store.dir_tabs.read().len();
    let total = super::tab_nav::tab_count(file_count, dir_count);
    let current =
        super::tab_nav::position_of(*store.active.read(), *store.active_dir.read(), file_count);
    let next = if forward {
        super::tab_nav::next_position(current, total)
    } else {
        super::tab_nav::prev_position(current, total)
    };
    activate_tab_at(store, next, file_count, dir_count);
}
