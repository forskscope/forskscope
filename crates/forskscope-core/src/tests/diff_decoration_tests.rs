//! DiffDecorationSet tests (RFC-024 §"Visual contract", §"CSS class contract").

use crate::diff::{DiffOptions, compute_diff};
use crate::diff_decoration::{
    DiffDecorationSet, DiffSide, InlineDecorationKind, LineDecorationKind,
};

fn decoration(old: &str, new: &str) -> DiffDecorationSet {
    let doc = compute_diff(old, new, DiffOptions::default());
    DiffDecorationSet::from_diff(&doc, None)
}

// ── CSS class tokens ──────────────────────────────────────────────────────────

#[test]
fn all_line_decoration_kinds_have_non_empty_css_class() {
    for kind in [
        LineDecorationKind::Unchanged, LineDecorationKind::Added,
        LineDecorationKind::Deleted,  LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart, LineDecorationKind::Conflict,
        LineDecorationKind::MergeApplied,
    ] {
        assert!(!kind.css_class().is_empty(), "{kind:?} must have a CSS class");
        assert!(kind.css_class().starts_with("fs-"),
            "{kind:?} CSS class must start with fs-");
    }
}

#[test]
fn all_line_decoration_kinds_have_unique_css_classes() {
    let kinds = [
        LineDecorationKind::Unchanged, LineDecorationKind::Added,
        LineDecorationKind::Deleted,  LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart, LineDecorationKind::Conflict,
        LineDecorationKind::MergeApplied,
    ];
    let classes: std::collections::HashSet<_> = kinds.iter().map(|k| k.css_class()).collect();
    assert_eq!(classes.len(), kinds.len(), "CSS classes must be unique");
}

#[test]
fn all_inline_decoration_kinds_have_fs_prefix_css_class() {
    for kind in [
        InlineDecorationKind::InsertedChars,
        InlineDecorationKind::DeletedChars,
        InlineDecorationKind::ReplacedChars,
        InlineDecorationKind::WhitespaceOnly,
    ] {
        assert!(kind.css_class().starts_with("fs-"));
    }
}

// ── Gutter symbols ────────────────────────────────────────────────────────────

#[test]
fn added_line_has_plus_gutter_symbol() {
    assert_eq!(LineDecorationKind::Added.gutter_symbol(), '+');
}

#[test]
fn deleted_line_has_minus_gutter_symbol() {
    assert_eq!(LineDecorationKind::Deleted.gutter_symbol(), '-');
}

#[test]
fn conflict_line_has_exclamation_gutter_symbol() {
    assert_eq!(LineDecorationKind::Conflict.gutter_symbol(), '!');
}

// ── from_diff for identical documents ────────────────────────────────────────

#[test]
fn identical_docs_produce_empty_decoration_set() {
    let d = decoration("hello\nworld\n", "hello\nworld\n");
    assert!(d.is_empty(), "identical docs must produce no changed decorations");
    assert_eq!(d.changed_hunk_count(), 0);
}

// ── from_diff for pure insert ─────────────────────────────────────────────────

#[test]
fn inserted_lines_get_added_decoration_on_right() {
    let d = decoration("a\n", "a\nb\n");
    let right_added = d.right.iter()
        .any(|ld| ld.kind == LineDecorationKind::Added && ld.side == DiffSide::Right);
    assert!(right_added, "inserted line must have Added decoration on right side");
}

#[test]
fn insert_hunk_appears_in_hunk_list() {
    let d = decoration("a\n", "a\nb\n");
    assert_eq!(d.changed_hunk_count(), 1, "one inserted hunk must appear");
}

// ── from_diff for pure delete ─────────────────────────────────────────────────

#[test]
fn deleted_lines_get_deleted_decoration_on_left() {
    let d = decoration("a\nb\n", "a\n");
    let left_deleted = d.left.iter()
        .any(|ld| ld.kind == LineDecorationKind::Deleted && ld.side == DiffSide::Left);
    assert!(left_deleted, "deleted line must have Deleted decoration on left side");
}

// ── from_diff for replace hunk ────────────────────────────────────────────────

#[test]
fn replaced_lines_get_modified_decoration_on_both_sides() {
    let d = decoration("hello\n", "world\n");
    let left_mod  = d.left.iter().any(|ld| ld.kind == LineDecorationKind::Modified);
    let right_mod = d.right.iter().any(|ld| ld.kind == LineDecorationKind::Modified);
    assert!(left_mod,  "replaced old line must be Modified on left");
    assert!(right_mod, "replaced new line must be Modified on right");
}

// ── focused hunk ──────────────────────────────────────────────────────────────

#[test]
fn focused_hunk_id_marks_the_correct_hunk_decoration() {
    let doc = compute_diff("a\n", "b\n", DiffOptions::default());
    let hunk_id = doc.hunks.iter()
        .find(|h| h.kind.is_change())
        .map(|h| h.hunk_id)
        .expect("must have a changed hunk");
    let d = DiffDecorationSet::from_diff(&doc, Some(hunk_id));
    let focused = d.hunks.iter().find(|hd| hd.hunk_id == hunk_id);
    assert!(focused.is_some(), "focused hunk must appear in hunk list");
    assert!(focused.unwrap().is_focused, "hunk must be marked focused");
}

#[test]
fn no_focused_hunk_means_all_hunks_are_unfocused() {
    let d = decoration("a\nb\n", "a\nc\n");
    assert!(d.hunks.iter().all(|hd| !hd.is_focused));
}

// ── aria labels ───────────────────────────────────────────────────────────────

#[test]
fn all_line_decoration_kinds_have_non_empty_aria_labels() {
    for kind in [
        LineDecorationKind::Unchanged, LineDecorationKind::Added,
        LineDecorationKind::Deleted,  LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart, LineDecorationKind::Conflict,
        LineDecorationKind::MergeApplied,
    ] {
        assert!(!kind.aria_label().is_empty(), "{kind:?} must have an aria label");
    }
}
