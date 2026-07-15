use super::*;
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
    }
}

fn token(tab: &CompareTab) -> LoadToken {
    LoadToken::new(tab.id, tab.load_generation)
}

fn ready_result(can_save: bool) -> LoadResult {
    LoadResult::Ready(Box::new(LoadedComparison {
        left_doc: LoadedDocument::empty(),
        right_doc: LoadedDocument::empty(),
        diff: DiffDocument::empty(),
        merge: MergeSession::empty(),
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
