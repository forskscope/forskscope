use std::fs;

use super::*;
use forskscope_core::persist::schema::PersistenceLoad;
use forskscope_core::persist::schema::session::SessionRepository;
use forskscope_ui_logic::{CompareTabId, LoadGeneration};

fn id(value: u64) -> CompareTabId {
    CompareTabId::new(value).unwrap()
}

fn generation(value: u64) -> LoadGeneration {
    LoadGeneration::new(value).unwrap()
}

fn loading_tab(id_value: u64, generation_value: u64) -> CompareTab {
    CompareTab {
        id: id(id_value),
        load_generation: generation(generation_value),
        title: format!("tab-{id_value}"),
        left_path: Some(PathBuf::from(format!("left-{id_value}"))),
        right_path: Some(PathBuf::from(format!("right-{id_value}"))),
        state: TabState::Loading,
        left_doc: LoadedDocument::empty(),
        right_doc: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
        diff_options: DiffOptions::default(),
        can_save: false,
        save_capability: SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText),
        char_mode: true,
        word_wrap: false,
        focused_change: 9,
        save_target: None,
        launch_mode: CompareLaunchMode::Normal,
    }
}

fn token(tab: &CompareTab) -> LoadToken {
    LoadToken::new(tab.id, tab.load_generation)
}

fn ready_result(can_save: bool) -> LoadResult {
    let right = PathBuf::from("right");
    let save_target =
        forskscope_core::compare_prep::save_target_from_loaded(&right, &LoadedDocument::empty());
    let save_capability = if can_save {
        SaveCapability::Saveable
    } else {
        SaveCapability::Blocked(SaveCapabilityBlockReason::NotMergeableText)
    };
    LoadResult::Ready(Box::new(PreparedCompare {
        left: LoadedDocument::empty(),
        right: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
        save_target,
        save_capability,
    }))
}

#[test]
fn close_before_tab_reindexes_does_not_redirect_completion() {
    let first = loading_tab(1, 1);
    let second = loading_tab(2, 1);
    let second_token = token(&second);
    let mut tabs = vec![first, second];

    tabs.remove(0);
    let decision = commit_load_result(&mut tabs, second_token, ready_result(true));

    assert_eq!(decision, CompletionDecision::Accept);
    assert_eq!(tabs[0].id, id(2));
    assert_eq!(tabs[0].state, TabState::Ready);
    assert!(tabs[0].can_save);
}

#[test]
fn older_reload_is_rejected_before_current_generation_commits() {
    let mut tab = loading_tab(1, 1);
    let old_token = token(&tab);
    tab.load_generation = tab.load_generation.next().unwrap();
    let current_token = token(&tab);
    let mut tabs = vec![tab];

    let obsolete = commit_load_result(&mut tabs, old_token, ready_result(true));
    assert_eq!(obsolete, CompletionDecision::RejectGenerationMismatch);
    assert_eq!(tabs[0].state, TabState::Loading);
    assert!(!tabs[0].can_save);

    let current = commit_load_result(&mut tabs, current_token, ready_result(true));
    assert_eq!(current, CompletionDecision::Accept);
    assert_eq!(tabs[0].state, TabState::Ready);
    assert!(tabs[0].can_save);
}

#[test]
fn completion_for_closed_tab_cannot_mutate_reused_vector_position() {
    let closed = loading_tab(1, 1);
    let closed_token = token(&closed);
    let replacement = loading_tab(2, 1);
    let mut tabs = vec![closed];

    tabs.remove(0);
    tabs.push(replacement);
    let decision = commit_load_result(&mut tabs, closed_token, ready_result(true));

    assert_eq!(decision, CompletionDecision::RejectTabMissing);
    assert_eq!(tabs[0].id, id(2));
    assert_eq!(tabs[0].state, TabState::Loading);
    assert!(!tabs[0].can_save);
}

#[test]
fn current_failure_changes_only_its_own_tab() {
    let first = loading_tab(1, 1);
    let second = loading_tab(2, 1);
    let second_token = token(&second);
    let mut tabs = vec![first, second];

    let decision = commit_load_result(
        &mut tabs,
        second_token,
        LoadResult::Error("second failed".into()),
    );

    assert_eq!(decision, CompletionDecision::Accept);
    assert_eq!(tabs[0].state, TabState::Loading);
    assert_eq!(tabs[1].state, TabState::Error("second failed".into()));
}

#[test]
fn obsolete_failure_does_not_replace_newer_ready_result() {
    let mut tab = loading_tab(1, 1);
    let old_token = token(&tab);
    tab.load_generation = tab.load_generation.next().unwrap();
    let current_token = token(&tab);
    let mut tabs = vec![tab];

    assert_eq!(
        commit_load_result(&mut tabs, current_token, ready_result(true)),
        CompletionDecision::Accept
    );
    let decision = commit_load_result(
        &mut tabs,
        old_token,
        LoadResult::Error("obsolete failure".into()),
    );

    assert_eq!(decision, CompletionDecision::RejectGenerationMismatch);
    assert_eq!(tabs[0].state, TabState::Ready);
    assert!(tabs[0].can_save);
}

#[test]
fn accepted_ready_result_resets_transient_navigation_state() {
    let tab = loading_tab(1, 1);
    let current_token = token(&tab);
    let mut tabs = vec![tab];

    commit_load_result(&mut tabs, current_token, ready_result(false));

    assert!(!tabs[0].char_mode);
    assert_eq!(tabs[0].focused_change, 0);
}

// ── load_and_diff (RFC-077 patches 3-4a: PreparedCompare, CompareRequest) ──

use forskscope_core::compare_prep::{SaveTargetState, TargetExpectation};

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("fsk-ui-load-and-diff-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

fn normal_request(left: PathBuf, right: PathBuf) -> CompareRequest {
    CompareRequest {
        left_input: left,
        right_input: right,
        save_destination: SaveDestination::RightInput,
    }
}

fn mergetool_request(local: PathBuf, remote: PathBuf, merged: PathBuf) -> CompareRequest {
    CompareRequest {
        left_input: local,
        right_input: remote,
        save_destination: SaveDestination::Explicit(merged),
    }
}

#[test]
fn normal_compare_save_target_is_the_right_input_must_match_its_own_fingerprint() {
    let dir = temp_dir("existing-right");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "a\n").unwrap();
    fs::write(&right, "b\n").unwrap();

    let prepared = load_and_diff(
        normal_request(left, right.clone()),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert_eq!(prepared.save_target.path, right);
    let expected_fp = forskscope_core::document::FileFingerprint::capture(&right, None).unwrap();
    match prepared.save_target.state {
        SaveTargetState::Writable { expectation, .. } => match expectation {
            TargetExpectation::MustMatch(fp) => assert_eq!(fp.len, expected_fp.len),
            other => panic!("expected MustMatch, got {other:?}"),
        },
        other => panic!("expected Writable, got {other:?}"),
    }
}

#[test]
fn normal_compare_save_target_is_must_be_absent_when_right_is_missing() {
    let dir = temp_dir("missing-right");
    let left = dir.join("left.txt");
    let right = dir.join("does-not-exist.txt");
    fs::write(&left, "a\n").unwrap();
    let _ = fs::remove_file(&right);

    let prepared = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    match prepared.save_target.state {
        SaveTargetState::Writable { expectation, .. } => {
            assert_eq!(expectation, TargetExpectation::MustBeAbsent);
        }
        other => panic!("expected Writable(MustBeAbsent), got {other:?}"),
    }
}

#[test]
fn normal_compare_can_save_and_diff_are_unaffected_by_the_prepared_compare_refactor() {
    let dir = temp_dir("can-save");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "one\ntwo\n").unwrap();
    fs::write(&right, "one\nTWO\n").unwrap();

    let prepared = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert!(
        prepared.save_capability.is_saveable(),
        "both sides are plain text — must remain saveable"
    );
    assert!(
        !prepared.diff.hunks.is_empty(),
        "the actual content difference must still be computed"
    );
}

#[test]
fn binary_comparison_disabled_error_message_is_unchanged() {
    let dir = temp_dir("binary-disabled");
    let left = dir.join("left.bin");
    let right = dir.join("right.txt");
    fs::write(&left, [0u8, 1, 2, 3]).unwrap();
    fs::write(&right, "text\n").unwrap();

    let result = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    );

    assert_eq!(
        result.unwrap_err(),
        "Binary comparison is off. Enable it in Settings → Advanced."
    );
}

#[test]
fn binary_vs_text_mismatch_error_message_is_unchanged() {
    let dir = temp_dir("binary-text-mismatch");
    let left = dir.join("left.bin");
    let right = dir.join("right.txt");
    fs::write(&left, [0u8, 1, 2, 3]).unwrap();
    fs::write(&right, "text\n").unwrap();

    let result = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        true,
    );

    assert_eq!(
        result.unwrap_err(),
        "Cannot compare: one file is binary and the other is text. Compare text with text, or binary with binary."
    );
}

/// RFC-085: a mixed pair (one `.xlsx` side, one text side) is no longer
/// refused outright — only a pair where *both* sides are `.xlsx` gets the
/// derived-text projection (`load_and_diff`'s own `&&` gate, matching the
/// pre-suspension behavior this restores). The xlsx side's `diff_text()`
/// stays empty, same as before RFC-085, so this compares "text\n" against
/// "" rather than erroring.
#[test]
fn a_mixed_text_and_xlsx_pair_compares_the_xlsx_side_as_empty() {
    let dir = temp_dir("xlsx-mixed");
    let left = dir.join("left.txt");
    let right = dir.join("right.xlsx");
    fs::write(&left, "text\n").unwrap();
    fs::write(&right, b"not a real workbook").unwrap();

    let prepared = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert_eq!(prepared.left.diff_text(), "text\n");
    assert_eq!(prepared.right.diff_text(), "");
}

/// RFC-085: both sides `.xlsx` restores the real structural comparison —
/// `load_and_diff` derives per-side text from the actual workbook diff, not
/// an empty placeholder, so a genuine cell change is visible in the diff
/// text on both sides (`+`/`-`/`~` prefixes are `build_side_text`'s, out of
/// this handoff's scope — this only proves *some* real content reached the
/// diff engine, not that its formatting is a specific string).
#[test]
fn both_sides_xlsx_produces_a_real_structural_diff_not_an_empty_one() {
    // Shared with forskscope-core's own xlsx tests (handoff 022's review
    // request explains why these are real workbooks, not hand-built bytes:
    // built once with rust_xlsxwriter, committed as binary fixtures, not a
    // project dependency).
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../forskscope-core/src/tests/fixtures/xlsx/basic");
    let left = fixtures.join("old.xlsx");
    let right = fixtures.join("new.xlsx");

    let prepared = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert!(
        !prepared.left.diff_text().is_empty(),
        "the xlsx pair's left side must derive real diff text, not stay empty"
    );
    assert!(
        !prepared.right.diff_text().is_empty(),
        "the xlsx pair's right side must derive real diff text, not stay empty"
    );
    assert_ne!(
        prepared.left.diff_text(),
        prepared.right.diff_text(),
        "the fixture's A1 cell differs (\"hello\" vs \"world\") — the two \
         sides' derived text must not be identical"
    );
}

// ── load_and_diff: Git mergetool save destination (RFC-077 patch 4a) ──────

#[test]
fn mergetool_save_target_is_the_merged_path_not_the_compared_remote() {
    // The exact defect RFC-077 closes: local/remote are what's compared,
    // merged is where a save goes — and they must never be conflated.
    let dir = temp_dir("mergetool-existing-merged");
    let local = dir.join("local.txt");
    let remote = dir.join("remote.txt");
    let merged = dir.join("merged.txt");
    fs::write(&local, "ours\n").unwrap();
    fs::write(&remote, "theirs\n").unwrap();
    fs::write(&merged, "already merged\n").unwrap();

    let prepared = load_and_diff(
        mergetool_request(local, remote.clone(), merged.clone()),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert_eq!(
        prepared.save_target.path, merged,
        "save target must be the merged output, not the compared remote"
    );
    assert_ne!(prepared.save_target.path, remote);
    let expected_fp = forskscope_core::document::FileFingerprint::capture(&merged, None).unwrap();
    match prepared.save_target.state {
        SaveTargetState::Writable { expectation, .. } => match expectation {
            TargetExpectation::MustMatch(fp) => assert_eq!(fp.len, expected_fp.len),
            other => panic!("expected MustMatch, got {other:?}"),
        },
        other => panic!("expected Writable, got {other:?}"),
    }
}

#[test]
fn mergetool_save_target_is_must_be_absent_when_merged_does_not_exist_yet() {
    let dir = temp_dir("mergetool-missing-merged");
    let local = dir.join("local.txt");
    let remote = dir.join("remote.txt");
    let merged = dir.join("does-not-exist-yet.txt");
    fs::write(&local, "ours\n").unwrap();
    fs::write(&remote, "theirs\n").unwrap();
    let _ = fs::remove_file(&merged);

    let prepared = load_and_diff(
        mergetool_request(local, remote, merged),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    match prepared.save_target.state {
        SaveTargetState::Writable { expectation, .. } => {
            assert_eq!(expectation, TargetExpectation::MustBeAbsent);
        }
        other => panic!("expected Writable(MustBeAbsent), got {other:?}"),
    }
}

#[test]
fn mergetool_compares_local_and_remote_not_local_and_merged() {
    // RFC-077 "Alternatives considered: compare local directly against
    // merged — Rejected". Prove the diff comes from local vs remote.
    let dir = temp_dir("mergetool-diff-source");
    let local = dir.join("local.txt");
    let remote = dir.join("remote.txt");
    let merged = dir.join("merged.txt");
    fs::write(&local, "same\n").unwrap();
    fs::write(&remote, "same\n").unwrap();
    fs::write(
        &merged,
        "wildly different content that would show as a diff\n",
    )
    .unwrap();

    let prepared = load_and_diff(
        mergetool_request(local, remote, merged),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();

    assert!(
        prepared.diff.hunks.iter().all(|h| !h.kind.is_change()),
        "local and remote are identical — the diff must show no changes, proving \
         merged's very different content was never fed into the comparison: {:?}",
        prepared.diff.hunks
    );
}

// ── F85 (RFC-082 §D2): save_target invariant on the load path ─────────────

/// The same invariant `swap_sides`'s tests assert (`tab::tests`), applied
/// here to the load path — `load_and_diff` is the function both
/// `open_compare_request` and `reload_tab` ultimately call, so exercising
/// it twice (once per distinct content, standing in for "load" then
/// "reload") covers both call sites without needing to drive the async load
/// task to completion through a full `Store`. Handoff 015 §6.3: "these
/// should already pass" — proven here with the identical assertion helper
/// the swap tests use, not an equivalent hand-rolled one.
#[test]
fn save_target_matches_right_input_after_load_and_reload() {
    let dir = temp_dir("f85-load-reload-invariant");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "left\n").unwrap();

    let build_tab = |right_content: &str| -> CompareTab {
        fs::write(&right, right_content).unwrap();
        let prepared = load_and_diff(
            normal_request(left.clone(), right.clone()),
            DiffOptions::default(),
            Lang::En,
            false,
        )
        .unwrap();
        CompareTab {
            id: id(1),
            load_generation: generation(1),
            title: "t".into(),
            left_path: Some(left.clone()),
            right_path: Some(right.clone()),
            state: TabState::Ready,
            left_doc: prepared.left,
            right_doc: prepared.right,
            diff: prepared.diff,
            merge: prepared.merge,
            diff_options: DiffOptions::default(),
            can_save: prepared.save_capability.is_saveable(),
            save_capability: prepared.save_capability,
            char_mode: false,
            word_wrap: false,
            focused_change: 0,
            save_target: Some(prepared.save_target),
            launch_mode: CompareLaunchMode::Normal,
        }
    };

    // "After a load."
    let loaded = build_tab("right\n");
    crate::state::tab::assert_save_target_matches_right_input(&loaded);

    // "After a reload" — right.txt changed on disk between the two loads,
    // so this is not vacuously true because nothing moved.
    let reloaded = build_tab("right changed on reload\n");
    assert_ne!(
        reloaded.right_doc.diff_text(),
        loaded.right_doc.diff_text(),
        "test setup: the reload must actually observe different content"
    );
    crate::state::tab::assert_save_target_matches_right_input(&reloaded);
}

// ── F61 regression: a CLI-opened tab must persist without waiting for any
// reactive effect ────────────────────────────────────────────────────────
//
// `app.rs` used to persist the session via a `use_effect` on `store.tabs`.
// Confirmed on a real desktop process (not a VirtualDom harness — that
// diverged from production, see ROADMAP.md's F61 entry for the full
// account) that this effect never ran for a signal write made outside a
// discrete Dioxus event dispatch: neither the synchronous push here nor
// the async load task's later write ever reached it, even after 30 real
// seconds idle, even though the same writes correctly drove a visual
// re-render. Only a write made from inside a real `onclick` handler
// (`close_tab`) reliably flushed it. Fixed by removing the effect and
// calling `save_session` explicitly at every site that changes what a
// session needs to remember (this function's push; `swap_sides`), the
// same way `close_tab` already did. This test exercises that explicit
// call, synchronously, through `with_test_store` (F36) — no VirtualDom
// rendering, no waiting on a scheduler, and so nothing here can diverge
// from production timing the way the deleted harness did.

#[test]
fn opening_a_tab_persists_the_session_without_any_further_render() {
    let dir = temp_dir("f61-open-compare-persists");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "alpha\n").unwrap();
    fs::write(&right, "beta\n").unwrap();
    let config_home = dir.join("config");
    fs::create_dir_all(&config_home).unwrap();

    // F95: a thread-local override, not a process-global `XDG_CONFIG_HOME`
    // behind a mutex — `open_compare_request`'s `save_session` call (the
    // thing this test proves runs synchronously) resolves `config_file_path`
    // on this calling thread, never from a spawned task, so the override
    // covers exactly the work this test needs it to.
    let _config_root = crate::state::ConfigRootOverrideGuard::set(config_home.clone());

    crate::state::with_test_store(|store| {
        open_compare_request(store, normal_request(left.clone(), right.clone()));
    });

    let session_path = config_home.join("forskscope").join("session.json");
    assert!(
        session_path.exists(),
        "F61: opening a tab must persist session.json synchronously, with \
         no further render or async completion needed, but no file was \
         written at {}",
        session_path.display()
    );

    let repo = SessionRepository::new(session_path);
    match repo.load() {
        PersistenceLoad::Current { value } => {
            assert_eq!(value.tabs.len(), 1, "expected exactly the one opened tab");
            assert_eq!(value.tabs[0].left, left);
            assert_eq!(value.tabs[0].right, right);
        }
        other => panic!("session.json was written but does not parse as current: {other:?}"),
    }
}

// ── F84: pre-load size guard (RFC-013 §"Large file prompt", handoff 011) ───

/// Comfortably past the default 4 MiB medium threshold and comfortably under
/// the 64 MiB large threshold — classifies as `Large`, producing
/// `LoadGuard::ConfirmPrompt { too_large: false, .. }` under
/// `PerformanceLimits::default()` (`forskscope-core/src/job/limits.rs`).
const LARGE_FILE_BYTES: usize = 5 * 1024 * 1024;

#[test]
fn decide_load_dispatches_through_the_real_guard_for_every_size_class() {
    let small = decide_load(1_024, 1_024, DiffOptions::default());
    assert!(
        matches!(small, LoadDecision::Go { banner: None, .. }),
        "a small pair must proceed silently"
    );

    let confirm = decide_load(LARGE_FILE_BYTES as u64, 1_024, DiffOptions::default());
    assert!(
        matches!(
            confirm,
            LoadDecision::Confirm {
                too_large: false,
                ..
            }
        ),
        "a 5 MiB file must block on confirmation, not proceed"
    );

    let very_large = decide_load(65 * 1024 * 1024, 1_024, DiffOptions::default());
    assert!(
        matches!(
            very_large,
            LoadDecision::Confirm {
                too_large: true,
                ..
            }
        ),
        "a 65 MiB file must be flagged too_large"
    );
}

#[test]
fn confirm_prompt_suppresses_inline_diff_on_the_resumed_options() {
    let opts = DiffOptions::default();
    assert_eq!(
        opts.inline_mode,
        InlineMode::Lazy,
        "test assumes a non-None starting point, or suppression would be unobservable"
    );

    let decision = decide_load(LARGE_FILE_BYTES as u64, 1_024, opts);
    match decision {
        LoadDecision::Confirm { opts, .. } => {
            assert_eq!(
                opts.inline_mode,
                InlineMode::None,
                "ConfirmPrompt always implies suppress_inline() — the resumed \
                 options must reflect it, not just the banner text"
            );
        }
        LoadDecision::Go { .. } => panic!("expected ConfirmPrompt for a 5 MiB file"),
    }
}

#[test]
fn both_load_call_sites_stop_at_the_guard_for_a_large_pair() {
    let dir = temp_dir("f84-large-load-guard");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, vec![b'a'; LARGE_FILE_BYTES]).unwrap();
    fs::write(&right, "small\n").unwrap();

    // open_compare_request: a ConfirmPrompt pair must not allocate a tab —
    // there is nothing to clean up if the user cancels.
    crate::state::with_test_store(|store| {
        open_compare_request(store, normal_request(left.clone(), right.clone()));

        assert!(
            store.tabs.read().is_empty(),
            "a ConfirmPrompt pair must not allocate a tab before confirmation"
        );
        match &*store.modal.read() {
            Modal::ConfirmLargeLoad(prompt) => {
                assert!(
                    matches!(prompt.target, LargeLoadTarget::Open(_)),
                    "expected a LargeLoadTarget::Open prompt"
                );
            }
            _ => panic!("expected Modal::ConfirmLargeLoad from open_compare_request"),
        }
    });

    // reload_tab: an existing tab must be left exactly as it was — still
    // Loading, not re-entered into a fresh load — until confirmed.
    crate::state::with_test_store(|store| {
        store.tabs.write().push(loading_tab(1, 1));
        {
            let mut tabs = store.tabs.write();
            tabs[0].left_path = Some(left.clone());
            tabs[0].right_path = Some(right.clone());
        }

        reload_tab(store, 0);

        assert_eq!(
            store.tabs.read()[0].state,
            TabState::Loading,
            "reload_tab must not touch the tab's state before confirmation"
        );
        match &*store.modal.read() {
            Modal::ConfirmLargeLoad(prompt) => {
                assert!(
                    matches!(prompt.target, LargeLoadTarget::Reload(0)),
                    "expected a LargeLoadTarget::Reload(0) prompt"
                );
            }
            _ => panic!("expected Modal::ConfirmLargeLoad from reload_tab"),
        }
    });

    let _ = fs::remove_dir_all(&dir);
}

// ── F88a (RFC-082 §D3): a decode-substituted file must not be saved
// without the guard firing first ────────────────────────────────────────────

/// §2a's exact fixture: a UTF-8 BOM followed by an invalid byte. The BOM
/// forces the UTF-8 interpretation, so detection cannot fall back to a
/// lossless single-byte encoding the way it does for a bare invalid byte
/// with no BOM.
const F88A_FIXTURE_BYTES: &[u8] = &[0xEF, 0xBB, 0xBF, 0xFF, b'a', b'\n'];

#[test]
fn a_file_that_decoded_with_replacement_characters_cannot_be_saved_without_the_guard() {
    let dir = temp_dir("f88a-decode-guard");
    let left = dir.join("left.txt");
    let right = dir.join("legacy.txt");
    fs::write(&left, "left\n").unwrap();
    fs::write(&right, F88A_FIXTURE_BYTES).unwrap();

    let prepared = load_and_diff(
        normal_request(left.clone(), right.clone()),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();
    assert_eq!(
        prepared.save_capability,
        forskscope_core::compare_prep::SaveCapability::SaveableWithGuard,
        "test setup: the right side must have decoded with replacement characters"
    );

    let tab = CompareTab {
        id: id(1),
        load_generation: generation(1),
        title: "t".into(),
        left_path: Some(left),
        right_path: Some(right.clone()),
        state: TabState::Ready,
        left_doc: prepared.left,
        right_doc: prepared.right,
        diff: prepared.diff,
        merge: prepared.merge,
        diff_options: DiffOptions::default(),
        can_save: prepared.save_capability.is_saveable(),
        save_capability: prepared.save_capability,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: Some(prepared.save_target),
        launch_mode: CompareLaunchMode::Normal,
    };

    crate::state::with_test_store(|store| {
        store.tabs.write().push(tab);
        crate::ui::view::diff_actions::save_tab(store, 0);

        match &*store.modal.read() {
            Modal::SaveError(index, path, view) => {
                assert_eq!(*index, 0);
                assert_eq!(path, &right);
                assert!(!view.buttons.is_empty());
            }
            other_modal => panic!(
                "expected Modal::SaveError, got a different modal: {:?}",
                std::mem::discriminant(other_modal)
            ),
        }
    });

    assert_eq!(
        fs::read(&right).unwrap(),
        F88A_FIXTURE_BYTES,
        "a save that would not reproduce the original bytes must write nothing at all"
    );
}

/// F88b (RFC-082 §D3 §3): `Missing` is empty text, not an unsupported kind
/// — a deleted right side must not disappear the merge/save toolbar, and
/// saving must be able to *create* it.
#[test]
fn a_missing_right_side_can_be_created_by_saving() {
    let dir = temp_dir("f88b-restore-deleted");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "restored content\n").unwrap();
    let _ = fs::remove_file(&right);

    let prepared = load_and_diff(
        normal_request(left.clone(), right.clone()),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();
    assert_eq!(
        prepared.save_capability,
        forskscope_core::compare_prep::SaveCapability::Saveable,
        "a missing side must not block saving, and must not require a guard"
    );

    // The right side is empty/missing, so result_text() starts as "" —
    // apply the pending hunk to bring the left content into the merge
    // result, simulating the user restoring the deleted file's content.
    let mut merge = prepared.merge;
    let hunk_id = merge
        .hunks()
        .iter()
        .find(|h| h.is_pending_change())
        .expect("fixture must contain a pending change")
        .hunk_id;
    merge.apply_left_to_right(hunk_id).unwrap();
    assert_eq!(merge.result_text(), "restored content\n");

    let tab = CompareTab {
        id: id(1),
        load_generation: generation(1),
        title: "t".into(),
        left_path: Some(left),
        right_path: Some(right.clone()),
        state: TabState::Ready,
        left_doc: prepared.left,
        right_doc: prepared.right,
        diff: prepared.diff,
        merge,
        diff_options: DiffOptions::default(),
        can_save: prepared.save_capability.is_saveable(),
        save_capability: prepared.save_capability,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: Some(prepared.save_target),
        launch_mode: CompareLaunchMode::Normal,
    };
    assert!(
        tab.can_save,
        "the merge/save toolbar must be available for a missing side"
    );

    crate::state::with_test_store(|store| {
        store.tabs.write().push(tab);
        crate::ui::view::diff_actions::save_tab(store, 0);
    });

    assert_eq!(
        fs::read_to_string(&right).unwrap(),
        "restored content\n",
        "saving must be able to create a side that was missing"
    );
}

/// F88a/§4 acceptance criterion: the capability composition must not have
/// widened into permitting what `is_mergeable_text` already correctly
/// refused — a binary or spreadsheet side stays unsaveable regardless of
/// the other side's editability.
#[test]
fn a_binary_or_spreadsheet_side_is_still_not_saveable() {
    let dir = temp_dir("f88-binary-still-blocked");

    // Both sides binary: load_and_diff's own binary-vs-text mismatch check
    // (a separate, earlier refusal) only fires for a *mixed* pair, so this
    // is what actually reaches save_capability's own classification.
    let left_bin = dir.join("left.bin");
    let right_bin = dir.join("right.bin");
    fs::write(&left_bin, [0u8, 4, 5, 6]).unwrap();
    fs::write(&right_bin, [0u8, 1, 2, 3]).unwrap();
    let prepared = load_and_diff(
        normal_request(left_bin, right_bin),
        DiffOptions::default(),
        Lang::En,
        true, // enable_binary — otherwise load_and_diff refuses earlier for a different reason
    )
    .unwrap();
    assert!(
        !prepared.save_capability.is_saveable(),
        "a binary side must still block saving: {:?}",
        prepared.save_capability
    );

    // RFC-085: a text/xlsx mixed pair is no longer refused outright (see
    // `a_mixed_text_and_xlsx_pair_compares_the_xlsx_side_as_empty`) — the
    // load now succeeds, so this asserts what F88a actually needs: even
    // though it loads, `FileKind::ExcelXlsx` still isn't mergeable text
    // (`is_mergeable_text()` is unchanged by RFC-085 — merge/save for
    // `.xlsx` stays a separate, out-of-scope RFC), so save_capability still
    // blocks it.
    let left = dir.join("left.txt");
    fs::write(&left, "left\n").unwrap();

    let xlsx = dir.join("right.xlsx");
    fs::write(&xlsx, b"not a real workbook").unwrap();
    let prepared = load_and_diff(
        normal_request(left, xlsx),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();
    assert!(
        !prepared.save_capability.is_saveable(),
        "a spreadsheet side must still block saving: {:?}",
        prepared.save_capability
    );
}

/// F88a's own review request states a deliberate narrowing, and this pins
/// it as a falsifiable claim rather than just an assertion in prose:
/// `EditabilityClass::requires_save_guard()` is `true` both for decode
/// substitution *and* for a non-UTF-8 encoding that decoded cleanly — RFC-012's
/// original table treats these differently ("Save guarded / must show
/// warning" vs. "Warn on lossy save"), and F87's own save-time check
/// already handles the second precisely (no loss, no block, real
/// `SaveAsUtf8` escape when it *is* lossy). A file that merely uses a
/// non-UTF-8 encoding, with zero decode errors, must not be swept into
/// F88a's Dismiss-only, no-escape guard — that would make ordinary
/// legacy-encoded editing permanently unsaveable. `save_capability` blocks
/// only on `had_decode_errors`, not on `requires_save_guard()` alone.
#[test]
fn a_cleanly_decoded_non_utf8_file_is_not_swept_into_the_new_guard() {
    let dir = temp_dir("f88a-clean-non-utf8-not-guarded");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "あいう\n").unwrap();
    // Shift_JIS bytes for "あいう" (no trailing newline in this fixture,
    // matching legacy_bytes_are_decoded_via_detection) — decodes cleanly,
    // confirmed empirically: had_decode_errors == false.
    let sjis: &[u8] = &[0x82, 0xA0, 0x82, 0xA2, 0x82, 0xA4];
    fs::write(&right, sjis).unwrap();

    let prepared = load_and_diff(
        normal_request(left.clone(), right.clone()),
        DiffOptions::default(),
        Lang::En,
        false,
    )
    .unwrap();
    assert!(
        !prepared.right.had_decode_errors(),
        "test setup: this fixture must decode cleanly"
    );
    assert_eq!(
        prepared.save_capability,
        forskscope_core::compare_prep::SaveCapability::Saveable,
        "a clean non-UTF-8 decode must not require F88a's guard — only \
         had_decode_errors does"
    );

    let tab = CompareTab {
        id: id(1),
        load_generation: generation(1),
        title: "t".into(),
        left_path: Some(left),
        right_path: Some(right.clone()),
        state: TabState::Ready,
        left_doc: prepared.left,
        right_doc: prepared.right,
        diff: prepared.diff,
        merge: prepared.merge,
        diff_options: DiffOptions::default(),
        can_save: prepared.save_capability.is_saveable(),
        save_capability: prepared.save_capability,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: Some(prepared.save_target),
        launch_mode: CompareLaunchMode::Normal,
    };

    crate::state::with_test_store(|store| {
        store.tabs.write().push(tab);
        crate::ui::view::diff_actions::save_tab(store, 0);

        assert!(
            matches!(&*store.modal.read(), Modal::None),
            "a save that round-trips cleanly must not show any dialog at all"
        );
    });

    assert_eq!(
        fs::read(&right).unwrap(),
        sjis,
        "the file must actually be saveable — identical content re-saved \
         must round-trip byte for byte"
    );
}
