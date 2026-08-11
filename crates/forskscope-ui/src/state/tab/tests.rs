use super::{CompareLaunchMode, CompareTab, TabState, recompute_diff, tab_title};
use crate::state::settings::Lang;
use dioxus::prelude::{ReadableExt, WritableExt};
use forskscope_core::{
    DiffOptions, FileKind, LoadedDocument, MergeSession, NewlineStyle, TextDocument, TextEncoding,
    compute_diff,
};
use forskscope_ui_logic::{CompareTabId, LoadGeneration};
use std::path::Path;

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
        }),
        warnings: Vec::new(),
    }
}

/// Documents `recompute_diff`'s destructive contract (F40): it always
/// rebuilds `MergeSession::from_diff` from scratch, discarding any applied
/// merge work and the entire undo/redo stack, regardless of whether the
/// underlying content or diff options actually changed. This is why every
/// call site — `swap_sides`, `change_diff_options` — must check
/// `merge.is_dirty()` and get confirmation *before* ever calling this,
/// rather than this function being safe to call unconditionally.
#[test]
fn recompute_diff_discards_applied_merge_work_and_undo_history() {
    let mut tab = dirty_tab();
    assert!(tab.merge.is_dirty());
    assert!(tab.merge.can_undo());

    recompute_diff(&mut tab);

    assert!(
        !tab.merge.is_dirty(),
        "recompute_diff produces a genuinely fresh, non-dirty session"
    );
    assert!(
        !tab.merge.can_undo(),
        "recompute_diff discards the undo stack -- every call site must \
         guard on is_dirty() before reaching it (F40)"
    );
}

/// A tab with one applied hunk (`is_dirty() == true`), for guard tests that
/// need a real dirty tab rather than a fresh one.
fn dirty_tab() -> CompareTab {
    let left = text_doc("one\ntwo\n");
    let right = text_doc("one\nTWO\n");
    let diff_options = DiffOptions::default();
    let diff = compute_diff(left.diff_text(), right.diff_text(), diff_options);
    let hunk_id = diff
        .hunks
        .iter()
        .find(|h| h.kind.is_change())
        .expect("fixture must contain a change")
        .hunk_id;
    let mut merge = MergeSession::from_diff(&diff);
    merge.apply_left_to_right(hunk_id).unwrap();

    CompareTab {
        id: CompareTabId::new(1).unwrap(),
        load_generation: LoadGeneration::new(1).unwrap(),
        title: "t".into(),
        left_path: None,
        right_path: None,
        state: TabState::Ready,
        left_doc: left,
        right_doc: right,
        merge,
        diff,
        diff_options,
        can_save: true,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: None,
        launch_mode: CompareLaunchMode::Normal,
    }
}

/// F40's guard, tested directly against a real `Store` (F36) rather than
/// only through AT-SPI runtime evidence: a dirty tab must defer a diff-
/// option change to the confirm dialog instead of applying it and
/// discarding the applied hunk immediately.
#[test]
fn change_diff_options_defers_to_confirmation_when_the_tab_is_dirty() {
    use crate::state::Modal;
    use crate::state::tab::change_diff_options;
    use crate::state::with_test_store;

    with_test_store(|store| {
        let tab = dirty_tab();
        let diff_options = tab.diff_options;
        store.tabs.write().push(tab);

        let mut next = diff_options;
        next.ignore_whitespace = true;
        change_diff_options(store, 0, next);

        assert!(
            matches!(*store.modal.read(), Modal::ConfirmDiffOptionChange(0, _)),
            "a dirty tab must show the confirm dialog, not apply immediately"
        );
        assert!(
            store.tabs.read()[0].merge.is_dirty(),
            "the guard must not touch merge state before confirmation"
        );
    });
}

/// The other half of F40's guard: a clean tab applies immediately, no
/// confirmation needed.
#[test]
fn change_diff_options_applies_immediately_when_the_tab_is_clean() {
    use crate::state::Modal;
    use crate::state::tab::change_diff_options;
    use crate::state::with_test_store;

    with_test_store(|store| {
        let mut tab = dirty_tab();
        tab.merge.mark_saved();
        assert!(!tab.merge.is_dirty(), "fixture must start clean");
        let diff_options = tab.diff_options;
        store.tabs.write().push(tab);

        let mut next = diff_options;
        next.ignore_whitespace = true;
        change_diff_options(store, 0, next);

        assert!(
            matches!(*store.modal.read(), Modal::None),
            "a clean tab must not show any confirmation dialog"
        );
        assert_eq!(store.tabs.read()[0].diff_options, next);
    });
}

#[test]
fn same_filename_both_sides_shows_single_name() {
    assert_eq!(
        tab_title(
            Path::new("/old/src/main.rs"),
            Path::new("/new/src/main.rs"),
            Lang::En
        ),
        "main.rs"
    );
}

#[test]
fn different_filenames_shows_both_with_arrow() {
    assert_eq!(
        tab_title(
            Path::new("/old/foo.txt"),
            Path::new("/new/bar.txt"),
            Lang::En
        ),
        "foo.txt ↔ bar.txt"
    );
}

#[test]
fn left_only_filename_shows_left() {
    assert_eq!(
        tab_title(Path::new("/project/README.md"), Path::new("/"), Lang::En),
        "README.md"
    );
}

#[test]
fn both_missing_filenames_shows_fallback() {
    assert_eq!(
        tab_title(Path::new("/"), Path::new("/"), Lang::En),
        "comparison"
    );
}

#[test]
fn hidden_dotfile_names_match_correctly() {
    assert_eq!(
        tab_title(
            Path::new("/a/.gitignore"),
            Path::new("/b/.gitignore"),
            Lang::En
        ),
        ".gitignore"
    );
}

#[test]
fn deeply_nested_same_filename_shows_single_name() {
    assert_eq!(
        tab_title(
            Path::new("/home/alice/projectA/src/lib/core/mod.rs"),
            Path::new("/home/bob/projectB/src/lib/core/mod.rs"),
            Lang::En,
        ),
        "mod.rs"
    );
}
