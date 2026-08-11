use super::{CompareLaunchMode, CompareTab, TabState, recompute_diff, tab_title};
use crate::state::settings::Lang;
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
    let left = text_doc("one\ntwo\nthree\n");
    let right = text_doc("one\nTWO\nthree\n");
    let diff_options = DiffOptions::default();
    let diff = compute_diff(left.diff_text(), right.diff_text(), diff_options);
    let hunk_id = diff
        .hunks
        .iter()
        .find(|h| h.kind.is_change())
        .expect("fixture must contain a change")
        .hunk_id;

    let mut tab = CompareTab {
        id: CompareTabId::new(1).unwrap(),
        load_generation: LoadGeneration::new(1).unwrap(),
        title: "t".into(),
        left_path: None,
        right_path: None,
        state: TabState::Ready,
        left_doc: left,
        right_doc: right,
        merge: MergeSession::from_diff(&diff),
        diff,
        diff_options,
        can_save: true,
        char_mode: false,
        word_wrap: false,
        focused_change: 0,
        save_target: None,
        launch_mode: CompareLaunchMode::Normal,
    };

    tab.merge.apply_left_to_right(hunk_id).unwrap();
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
