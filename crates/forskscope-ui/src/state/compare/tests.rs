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
    LoadResult::Ready(Box::new(PreparedCompare {
        left: LoadedDocument::empty(),
        right: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
        save_target,
        can_save,
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
        prepared.can_save,
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

#[test]
fn xlsx_target_error_message_is_unchanged() {
    let dir = temp_dir("xlsx");
    let left = dir.join("left.txt");
    let right = dir.join("right.xlsx");
    fs::write(&left, "text\n").unwrap();
    fs::write(&right, b"not a real workbook").unwrap();

    let result = load_and_diff(
        normal_request(left, right),
        DiffOptions::default(),
        Lang::En,
        false,
    );

    assert_eq!(
        result.unwrap_err(),
        "Spreadsheet comparison is temporarily disabled for security."
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

static XDG_CONFIG_HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn opening_a_tab_persists_the_session_without_any_further_render() {
    let _guard = XDG_CONFIG_HOME_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let dir = temp_dir("f61-open-compare-persists");
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "alpha\n").unwrap();
    fs::write(&right, "beta\n").unwrap();
    let config_home = dir.join("config");
    fs::create_dir_all(&config_home).unwrap();

    let previous_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    // SAFETY: serialized by XDG_CONFIG_HOME_LOCK; no other test in this
    // suite reads or writes this env var.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &config_home);
    }

    crate::state::with_test_store(|store| {
        open_compare_request(store, normal_request(left.clone(), right.clone()));
    });

    // SAFETY: still serialized by XDG_CONFIG_HOME_LOCK.
    unsafe {
        match &previous_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

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
