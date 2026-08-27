//! Diff workspace action functions (pure state mutations, RFC-003 §state ownership).
//! These are free functions used by `diff.rs` components and `app.rs` keyboard handlers.

use std::path::PathBuf;

use dioxus::prelude::*;

use forskscope_core::CoreError;
use forskscope_core::compare_prep::{
    SaveTargetBlockReason, SaveTargetSnapshot, SaveTargetState, TargetExpectation,
    inspect_save_target,
};
use forskscope_core::error::{AppError, RecoveryAction};
use forskscope_core::save::{
    BackupPolicy, SaveOutcome, SaveRequest, TargetPrecondition, save_text,
};
use forskscope_ui_logic::SaveErrorView;

use crate::i18n::t;
use crate::state::tab::CompareTab;
use crate::state::{Modal, Store};

// ─── Public action functions ──────────────────────────────────────────────────

/// Apply the focused changed hunk and auto-advance to the next one.
pub fn apply_focused_hunk(store: &mut Store, index: usize) {
    let hunk_id = {
        let tabs = store.tabs.read();
        let Some(tab) = tabs.get(index) else { return };
        if !tab.can_save {
            return;
        }
        let ids: Vec<u64> = tab
            .merge
            .hunks()
            .iter()
            .filter(|h| h.is_pending_change())
            .map(|h| h.hunk_id)
            .collect();
        ids.get(tab.focused_change).copied()
    };
    if let Some(id) = hunk_id {
        let _ = store
            .tabs
            .write()
            .get_mut(index)
            .map(|t| t.merge.apply_left_to_right(id));
        // Advance to the next pending change so the user can keep pressing Enter.
        move_focus(store, index, 1);
    }
}

pub fn move_focus(store: &mut Store, index: usize, delta: i32) {
    let hunk_id = {
        let mut tabs = store.tabs.write();
        let Some(tab) = tabs.get_mut(index) else {
            return;
        };
        let ids: Vec<u64> = tab
            .merge
            .hunks()
            .iter()
            .filter(|h| h.kind.is_change())
            .map(|h| h.hunk_id)
            .collect();
        if ids.is_empty() {
            return;
        }
        let next = ((tab.focused_change as i32 + delta).rem_euclid(ids.len() as i32)) as usize;
        tab.focused_change = next;
        ids[next]
    };
    spawn(async move {
        let _ = dioxus::document::eval(
            &format!("document.getElementById('h-{hunk_id}')?.scrollIntoView({{block:'nearest',behavior:'smooth'}});")
        ).await;
    });
}

pub fn save_tab(store: &mut Store, index: usize) {
    dispatch(store, index, build_request(store, index, false, None));
}

/// Save As: the destination is a path the tab's `save_target` doesn't
/// describe, so it's inspected fresh rather than reused — RFC-077:
/// "validate/inspect the selected destination and derive `MustBeAbsent` or
/// `MustMatch`; selecting an existing file does not imply force." `force`
/// is always `false` here; only [`confirm_overwrite`] ever constructs
/// [`TargetPrecondition::Force`], and only for the exact path the user just
/// confirmed (review 048 C1 — never a path a stale `Store` read reintroduces).
pub fn save_as(store: &mut Store, index: usize, path: String) {
    let target = PathBuf::from(&path);
    dispatch(
        store,
        index,
        build_request(store, index, false, Some(target)),
    );
}

/// What choosing `target` as a Save As destination requires, before any
/// write is attempted — `SaveAsModal`'s pre-check (review 050 §3.2). Reuses
/// [`inspect_save_target`]'s classification rather than a plain
/// `Path::exists()`, which could not distinguish "exists and is
/// overwritable" from "exists and can never be written to" (a directory, a
/// binary file, ...) — the latter used to show a confirmation dialog asking
/// to overwrite something that would then be refused as unwritable one step
/// later.
pub enum SaveAsPrecheck {
    /// Nothing exists at this path yet — proceed straight to `save_as`.
    New,
    /// A plain, writable file exists — show a pre-write confirmation.
    Overwrite,
    /// This path can never be written to — report immediately, no
    /// confirmation dialog to dismiss first.
    Blocked(String),
}

pub fn precheck_save_as_target(
    store: &Store,
    index: usize,
    target: &std::path::Path,
) -> SaveAsPrecheck {
    let tabs = store.tabs.read();
    let Some(tab) = tabs.get(index) else {
        return SaveAsPrecheck::New;
    };
    let fallback_encoding = current_encoding_label(tab);
    match inspect_save_target(target, &fallback_encoding).state {
        SaveTargetState::Writable {
            expectation: TargetExpectation::MustBeAbsent,
            ..
        } => SaveAsPrecheck::New,
        SaveTargetState::Writable {
            expectation: TargetExpectation::MustMatch(_),
            ..
        } => SaveAsPrecheck::Overwrite,
        SaveTargetState::Blocked { reason } => SaveAsPrecheck::Blocked(describe_block(&reason)),
    }
}

/// The confirmed-overwrite flow, reached only from `OverwriteModal`'s
/// `Modal::ConfirmOverwrite(index, target)` — `target` is `request.target`
/// from whichever save produced the conflict (review 048 C1: the tab's own
/// save target for a plain `save_tab` conflict, or the exact Save As
/// destination for a Save As conflict — never one confused for the other).
pub fn confirm_overwrite(store: &mut Store, index: usize, target: PathBuf) {
    dispatch(
        store,
        index,
        build_request(store, index, true, Some(target)),
    );
}

/// Common tail for every save entry point: run the request (if any) through
/// `save_text` and `handle_result`, or report a blocked destination
/// (review 048 C2 — a blocked target used to fail silently).
fn dispatch(store: &mut Store, index: usize, outcome: RequestOutcome) {
    match outcome {
        RequestOutcome::Ready(request) => {
            let result = save_text(&request);
            handle_result(store, index, &request, result);
        }
        RequestOutcome::NotSaveable => {}
        RequestOutcome::Blocked(reason) => store.notify(describe_block(&reason)),
    }
}

/// What [`build_request`] found, distinguishing an ordinary "nothing to do"
/// (no tab, tab isn't saveable) from a destination that exists but cannot be
/// written to — the two used to be conflated into one silent `None`
/// (review 048 C2).
enum RequestOutcome {
    Ready(SaveRequest),
    NotSaveable,
    Blocked(SaveTargetBlockReason),
}

/// Builds the exact write that will be attempted, using only `tab.save_target`
/// for target path, precondition, and encoding (RFC-077: `build_request`
/// "never reads `right_doc.fingerprint_at_load` as an implicit save
/// target"). `target: Some(_)` is Save As (or, with `force: true`, a
/// confirmed overwrite of that same explicit destination — see
/// [`confirm_overwrite`]); `target: None` is a normal, unforced save.
fn build_request(
    store: &Store,
    index: usize,
    force: bool,
    target: Option<PathBuf>,
) -> RequestOutcome {
    let tabs = store.tabs.read();
    let Some(tab) = tabs.get(index) else {
        return RequestOutcome::NotSaveable;
    };
    if !tab.can_save {
        return RequestOutcome::NotSaveable;
    }

    let (tgt, precondition, encoding_label) = match target {
        Some(explicit) => {
            let fallback_encoding = current_encoding_label(tab);
            let snapshot = inspect_save_target(&explicit, &fallback_encoding);
            match snapshot.state {
                SaveTargetState::Writable {
                    expectation,
                    encoding_label,
                } => {
                    let precondition = if force {
                        TargetPrecondition::Force
                    } else {
                        to_precondition(expectation)
                    };
                    (explicit, precondition, encoding_label)
                }
                // Force still rejects an unsupported target kind (RFC-077:
                // "Force... still rejects unsupported target kinds unless
                // the user chose a different valid path") — it bypasses the
                // conflict check, not the classification.
                SaveTargetState::Blocked { reason } => return RequestOutcome::Blocked(reason),
            }
        }
        None => {
            let Some(save_target) = tab.save_target.as_ref() else {
                return RequestOutcome::NotSaveable;
            };
            match &save_target.state {
                SaveTargetState::Writable {
                    expectation,
                    encoding_label,
                } => {
                    let precondition = if force {
                        TargetPrecondition::Force
                    } else {
                        to_precondition(*expectation)
                    };
                    (
                        save_target.path.clone(),
                        precondition,
                        encoding_label.clone(),
                    )
                }
                SaveTargetState::Blocked { reason } => {
                    return RequestOutcome::Blocked(reason.clone());
                }
            }
        }
    };

    RequestOutcome::Ready(SaveRequest {
        target: tgt,
        content: tab.merge.result_text(),
        encoding_label,
        precondition,
        backup: BackupPolicy::SiblingBak,
    })
}

fn to_precondition(expectation: TargetExpectation) -> TargetPrecondition {
    match expectation {
        TargetExpectation::MustMatch(fp) => TargetPrecondition::MustMatch(fp),
        TargetExpectation::MustBeAbsent => TargetPrecondition::MustBeAbsent,
    }
}

fn current_encoding_label(tab: &CompareTab) -> String {
    tab.save_target
        .as_ref()
        .and_then(|st| match &st.state {
            SaveTargetState::Writable { encoding_label, .. } => Some(encoding_label.clone()),
            SaveTargetState::Blocked { .. } => None,
        })
        .unwrap_or_else(|| "UTF-8".into())
}

/// User-facing message for a save destination `build_request` refused
/// (review 048 C2). Not run through `t()`: `handle_result`'s own
/// `Err(e) => store.notify(e.to_string())` arm below is the established
/// precedent for this function's error messages staying English-only.
fn describe_block(reason: &SaveTargetBlockReason) -> String {
    match reason {
        SaveTargetBlockReason::Binary => "Cannot save here: the target is a binary file.".into(),
        SaveTargetBlockReason::Spreadsheet => {
            "Cannot save here: the target is a spreadsheet file.".into()
        }
        SaveTargetBlockReason::NotAPlainFile { reason } => {
            format!("Cannot save here: {reason}.")
        }
        SaveTargetBlockReason::Unreadable { message } => {
            format!("Cannot save here: {message}")
        }
    }
}

/// On success, replaces `tab.save_target` with `MustMatch(outcome.new_fingerprint)`
/// at the path just written — never `tab.right_doc.fingerprint_at_load`, and
/// never `tab.right_path` (RFC-077: "do not change the compared right input
/// path"). `request` is the exact request that produced `result`, so the
/// updated snapshot's path/encoding always match what was actually written,
/// and a conflict's `ConfirmOverwrite` always names that same exact path
/// (review 048 C1).
fn handle_result(
    store: &mut Store,
    index: usize,
    request: &SaveRequest,
    result: Result<SaveOutcome, CoreError>,
) {
    match result {
        Ok(outcome) => {
            let mut tabs = store.tabs.write();
            if let Some(tab) = tabs.get_mut(index) {
                tab.merge.mark_saved();
                tab.save_target = Some(SaveTargetSnapshot {
                    path: request.target.clone(),
                    state: SaveTargetState::Writable {
                        expectation: TargetExpectation::MustMatch(outcome.new_fingerprint),
                        encoding_label: request.encoding_label.clone(),
                    },
                });
            }
            drop(tabs);
            store.modal.set(Modal::None);
            store.notify_success(t(store.lang(), "Saved."));
        }
        Err(CoreError::Conflict { .. }) => {
            store
                .modal
                .set(Modal::ConfirmOverwrite(index, request.target.clone()));
        }
        Err(e) => {
            let app_err = AppError::from_core(&e);
            let view =
                SaveErrorView::from_error(&app_err, Some(request.target.display().to_string()));
            store
                .modal
                .set(Modal::SaveError(index, request.target.clone(), view));
        }
    }
}

/// Exhaustive handler for every button [`SaveErrorModal`](crate::ui::overlay::modals::SaveErrorModal)
/// can show. Only [`RecoveryAction::ChooseAnotherFile`], [`RecoveryAction::Dismiss`],
/// and [`RecoveryAction::SaveAs`] are reachable here — the only `CoreError`
/// variants `save_text` can produce (`Conflict` aside, which never reaches
/// this dialog — see [`handle_result`]) are `Io { Metadata | Write | Rename
/// | CreateBackup }`, which map through `AppErrorKind::from_core` to
/// `FileReadFailed | FileWriteFailed | BackupFailed`, and through
/// `default_recovery_actions()` to exactly this set (F52 review request has
/// the full trace). The other nine variants are named explicitly rather than
/// matched with `_` so that a future thirteenth `RecoveryAction` variant is a
/// compile error here, not a silently swallowed button — the same principle
/// `file_digest_equal`'s `unreachable!()` established for F77 (review 074 §5).
pub fn handle_save_recovery_action(
    store: &mut Store,
    index: usize,
    target: PathBuf,
    action: RecoveryAction,
) {
    match action {
        RecoveryAction::Dismiss => store.modal.set(Modal::None),
        RecoveryAction::ChooseAnotherFile | RecoveryAction::SaveAs => {
            store
                .modal
                .set(Modal::SaveAs(index, target.display().to_string()));
        }
        RecoveryAction::Reload
        | RecoveryAction::OverwriteAnyway
        | RecoveryAction::OpenLimitedDiff
        | RecoveryAction::OpenAsBinary
        | RecoveryAction::Retry
        | RecoveryAction::RetryWithoutInline
        | RecoveryAction::Cancel
        | RecoveryAction::StartFresh
        | RecoveryAction::ReportBug => unreachable!(
            "{action:?} is not in save_text's reachable CoreError set — see F52 review request"
        ),
    }
}

pub(crate) fn trunc(s: &str) -> String {
    if let Some(i) = s.rfind('/').or_else(|| s.rfind('\\')) {
        let (parent, name) = s.split_at(i + 1);
        if parent.len() > 24 {
            return format!("…/{name}");
        }
    }
    s.to_string()
}

pub(crate) fn algo_val(a: forskscope_core::DiffAlgorithm) -> &'static str {
    use forskscope_core::DiffAlgorithm;
    match a {
        DiffAlgorithm::Patience => "patience",
        DiffAlgorithm::Histogram => "histogram",
        _ => "myers",
    }
}

/// Export the current comparison as a unified-diff patch file.
/// Opens a native save dialog, then writes the patch text to the chosen path.
/// Does nothing if the diff is identical (no changes to export).
pub fn export_patch(store: &Store, index: usize) {
    use forskscope_core::patch::{PatchOptions, patch_from_file_diff, to_unified};

    // Collect what we need from the tab before spawning.
    let tab = store.tabs.read();
    let Some(tab) = tab.get(index) else { return };

    let patch_doc = {
        // Use the relative filename as the patch path, falling back to "file".
        let rel = tab
            .right_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("file"));

        patch_from_file_diff(rel, &tab.diff, PatchOptions::default())
    };

    let Some(patch) = patch_doc else {
        // Identical files — nothing to export. Notify but don't error.
        let _ = tab;
        return;
    };

    let patch_text = to_unified(&patch);

    // Use a default filename based on the right-side file, e.g. "main.rs.patch".
    let default_name = tab
        .right_path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| format!("{}.patch", n.to_string_lossy()))
        .unwrap_or_else(|| "changes.patch".into());

    let _ = tab;

    // Spawn an async task to open the save dialog and write the file.
    spawn(async move {
        let handle = rfd::AsyncFileDialog::new()
            .set_title("Export patch")
            .set_file_name(&default_name)
            .add_filter("Patch files", &["patch", "diff"])
            .add_filter("All files", &["*"])
            .save_file()
            .await;

        if let Some(file) = handle {
            let path = file.path();
            if let Err(e) = std::fs::write(path, &patch_text) {
                eprintln!("export_patch: write error: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::document::FileFingerprint;

    // `build_request`/`save_tab`/`save_as`/`handle_result` all need a live
    // `Store` (Signal::new_in_scope panics outside a Dioxus runtime — F36,
    // the same limitation review 046 named for RFC-076's Store-dependent
    // code). `to_precondition` is the one piece of this migration's logic
    // that's genuinely pure; the rest is covered by runtime evidence
    // (see the RFC-077 patch 4b review request).

    #[test]
    fn to_precondition_maps_must_match_one_to_one() {
        let dir = std::env::temp_dir().join(format!("fsk-diff-actions-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("f.txt");
        std::fs::write(&path, "x").unwrap();
        let fp = FileFingerprint::capture(&path, None).unwrap();

        assert_eq!(
            to_precondition(TargetExpectation::MustMatch(fp)),
            TargetPrecondition::MustMatch(fp)
        );
    }

    #[test]
    fn to_precondition_maps_must_be_absent_one_to_one() {
        assert_eq!(
            to_precondition(TargetExpectation::MustBeAbsent),
            TargetPrecondition::MustBeAbsent
        );
    }

    // ── describe_block (review 048 C2) ──────────────────────────────────

    #[test]
    fn describe_block_is_non_empty_for_every_reason() {
        let reasons = [
            SaveTargetBlockReason::Binary,
            SaveTargetBlockReason::Spreadsheet,
            SaveTargetBlockReason::NotAPlainFile {
                reason: "not a regular file".into(),
            },
            SaveTargetBlockReason::Unreadable {
                message: "permission denied".into(),
            },
        ];
        for reason in reasons {
            assert!(!describe_block(&reason).is_empty());
        }
    }

    #[test]
    fn describe_block_includes_the_underlying_detail() {
        let reason = SaveTargetBlockReason::NotAPlainFile {
            reason: "not a regular file".into(),
        };
        assert!(describe_block(&reason).contains("not a regular file"));

        let reason = SaveTargetBlockReason::Unreadable {
            message: "permission denied".into(),
        };
        assert!(describe_block(&reason).contains("permission denied"));
    }

    // ── F52: save-error recovery dialog (handoff 012) ──────────────────────

    fn save_request(target: PathBuf, precondition: TargetPrecondition) -> SaveRequest {
        SaveRequest {
            target,
            content: "content\n".into(),
            encoding_label: "UTF-8".into(),
            precondition,
            backup: BackupPolicy::None,
        }
    }

    /// Drives the real `save_text` against a target whose parent path is a
    /// plain file, not a directory — a genuine `CoreError::Io` from the
    /// exact function `handle_result` is wired to, not a hand-built error
    /// (the "review 077 fed `classify_digest_outcome` a real `Err`"
    /// precedent).
    #[test]
    fn a_real_non_conflict_save_failure_opens_the_save_error_dialog_not_a_toast() {
        let dir = std::env::temp_dir().join(format!(
            "fsk-diff-actions-save-error-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, "not a directory").unwrap();
        let target = blocker.join("output.txt");

        let request = save_request(target, TargetPrecondition::MustBeAbsent);
        let result = save_text(&request);
        assert!(
            matches!(result, Err(CoreError::Io { .. })),
            "test setup must produce a real Io failure, got {result:?}"
        );

        crate::state::with_test_store(|store| {
            handle_result(store, 0, &request, result);
            match &*store.modal.read() {
                Modal::SaveError(index, path, view) => {
                    assert_eq!(*index, 0);
                    assert_eq!(path, &request.target);
                    assert!(!view.buttons.is_empty());
                }
                Modal::None => panic!(
                    "expected Modal::SaveError — the old notify(...) toast path must be gone"
                ),
                other_modal => panic!(
                    "expected Modal::SaveError, got a different modal: {:?}",
                    std::mem::discriminant(other_modal)
                ),
            }
        });
    }

    /// The exact case that matters more than the one above: a real
    /// `CoreError::Conflict` (target already exists under `MustBeAbsent`)
    /// must still raise `Modal::ConfirmOverwrite`, never the new F52 dialog
    /// — RFC-077/review 048 C1 machinery is untouched by this handoff.
    #[test]
    fn a_real_conflict_still_opens_confirm_overwrite_not_the_save_error_dialog() {
        let dir =
            std::env::temp_dir().join(format!("fsk-diff-actions-conflict-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("existing.txt");
        std::fs::write(&target, "already here\n").unwrap();

        let request = save_request(target.clone(), TargetPrecondition::MustBeAbsent);
        let result = save_text(&request);
        assert!(
            matches!(result, Err(CoreError::Conflict { .. })),
            "test setup must produce a real Conflict, got {result:?}"
        );

        crate::state::with_test_store(|store| {
            handle_result(store, 3, &request, result);
            match &*store.modal.read() {
                Modal::ConfirmOverwrite(index, path) => {
                    assert_eq!(*index, 3);
                    assert_eq!(path, &target);
                }
                _ => panic!(
                    "a save conflict must still raise Modal::ConfirmOverwrite, not the F52 dialog"
                ),
            }
        });
    }

    /// Every button `SaveErrorView` can actually produce for a save error
    /// (`{ChooseAnotherFile, Dismiss, SaveAs}` — see F52 review request for
    /// the full reachability trace) must have a real, non-panicking handler.
    #[test]
    fn every_reachable_recovery_action_has_a_working_non_panicking_handler() {
        let target = PathBuf::from("/some/target.txt");

        crate::state::with_test_store(|store| {
            handle_save_recovery_action(store, 2, target.clone(), RecoveryAction::Dismiss);
            assert!(matches!(&*store.modal.read(), Modal::None));
        });

        crate::state::with_test_store(|store| {
            handle_save_recovery_action(store, 2, target.clone(), RecoveryAction::SaveAs);
            match &*store.modal.read() {
                Modal::SaveAs(index, path) => {
                    assert_eq!(*index, 2);
                    assert_eq!(path, &target.display().to_string());
                }
                _ => panic!("SaveAs must open Modal::SaveAs"),
            }
        });

        crate::state::with_test_store(|store| {
            handle_save_recovery_action(
                store,
                2,
                target.clone(),
                RecoveryAction::ChooseAnotherFile,
            );
            match &*store.modal.read() {
                Modal::SaveAs(index, path) => {
                    assert_eq!(*index, 2);
                    assert_eq!(path, &target.display().to_string());
                }
                _ => panic!("ChooseAnotherFile must open Modal::SaveAs"),
            }
        });
    }

    /// Review 083 §4: naming all twelve `RecoveryAction` variants in
    /// `handle_save_recovery_action` means a *thirteenth* variant is a
    /// compile error — but it does not catch `forskscope-core` changing
    /// which actions an *existing* `AppErrorKind` emits. If
    /// `default_recovery_actions()` ever grows to include, say, `Retry` for
    /// `FileWriteFailed`, the button would render and panic on click, with
    /// no compile error and no failing test in this crate. This closes that
    /// gap from this crate's side: for every `AppErrorKind` a save error can
    /// actually produce (§3 of the F52 review request's reachability
    /// trace), `default_recovery_actions()` must stay a subset of what
    /// `handle_save_recovery_action` handles. A cross-crate change that
    /// violates this now fails here instead of panicking in the GUI.
    #[test]
    fn every_save_reachable_kind_only_emits_handled_recovery_actions() {
        use forskscope_core::error::AppErrorKind;

        const HANDLED: [RecoveryAction; 3] = [
            RecoveryAction::ChooseAnotherFile,
            RecoveryAction::Dismiss,
            RecoveryAction::SaveAs,
        ];
        for kind in [
            AppErrorKind::FileReadFailed,
            AppErrorKind::FileWriteFailed,
            AppErrorKind::BackupFailed,
        ] {
            for action in kind.default_recovery_actions() {
                assert!(
                    HANDLED.contains(action),
                    "{kind:?} can emit {action:?}, which handle_save_recovery_action \
                     does not handle — it would hit unreachable!() if a save ever \
                     produced this kind"
                );
            }
        }
    }
}
