//! DiffDecorationSet tests (RFC-024 §"Visual contract", §"CSS class contract").

use crate::diff_decoration::{
    BidiMarkedPart, DiffDecorationSet, DiffSide, InlineDecorationKind, LineDecorationKind,
    split_bidi_marks,
};
use forskscope_compare::diff::{DiffOptions, compute_diff};

fn decoration(old: &str, new: &str) -> DiffDecorationSet {
    let doc = compute_diff(old, new, DiffOptions::default());
    DiffDecorationSet::from_diff(&doc, None)
}

// ── CSS class tokens ──────────────────────────────────────────────────────────

#[test]
fn all_line_decoration_kinds_have_non_empty_css_class() {
    for kind in [
        LineDecorationKind::Unchanged,
        LineDecorationKind::Added,
        LineDecorationKind::Deleted,
        LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart,
        LineDecorationKind::Conflict,
        LineDecorationKind::MergeApplied,
    ] {
        assert!(
            !kind.css_class().is_empty(),
            "{kind:?} must have a CSS class"
        );
        assert!(
            kind.css_class().starts_with("fs-"),
            "{kind:?} CSS class must start with fs-"
        );
    }
}

#[test]
fn all_line_decoration_kinds_have_unique_css_classes() {
    let kinds = [
        LineDecorationKind::Unchanged,
        LineDecorationKind::Added,
        LineDecorationKind::Deleted,
        LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart,
        LineDecorationKind::Conflict,
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
    assert!(
        d.is_empty(),
        "identical docs must produce no changed decorations"
    );
    assert_eq!(d.changed_hunk_count(), 0);
}

// ── from_diff for pure insert ─────────────────────────────────────────────────

#[test]
fn inserted_lines_get_added_decoration_on_right() {
    let d = decoration("a\n", "a\nb\n");
    let right_added = d
        .right
        .iter()
        .any(|ld| ld.kind == LineDecorationKind::Added && ld.side == DiffSide::Right);
    assert!(
        right_added,
        "inserted line must have Added decoration on right side"
    );
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
    let left_deleted = d
        .left
        .iter()
        .any(|ld| ld.kind == LineDecorationKind::Deleted && ld.side == DiffSide::Left);
    assert!(
        left_deleted,
        "deleted line must have Deleted decoration on left side"
    );
}

// ── from_diff for replace hunk ────────────────────────────────────────────────

#[test]
fn replaced_lines_get_modified_decoration_on_both_sides() {
    let d = decoration("hello\n", "world\n");
    let left_mod = d
        .left
        .iter()
        .any(|ld| ld.kind == LineDecorationKind::Modified);
    let right_mod = d
        .right
        .iter()
        .any(|ld| ld.kind == LineDecorationKind::Modified);
    assert!(left_mod, "replaced old line must be Modified on left");
    assert!(right_mod, "replaced new line must be Modified on right");
}

// ── focused hunk ──────────────────────────────────────────────────────────────

#[test]
fn focused_hunk_id_marks_the_correct_hunk_decoration() {
    let doc = compute_diff("a\n", "b\n", DiffOptions::default());
    let hunk_id = doc
        .hunks
        .iter()
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
        LineDecorationKind::Unchanged,
        LineDecorationKind::Added,
        LineDecorationKind::Deleted,
        LineDecorationKind::Modified,
        LineDecorationKind::EmptyCounterpart,
        LineDecorationKind::Conflict,
        LineDecorationKind::MergeApplied,
    ] {
        assert!(
            !kind.aria_label().is_empty(),
            "{kind:?} must have an aria label"
        );
    }
}

// ── split_bidi_marks (RFC-100 §2) ──────────────────────────────────────────────

#[test]
fn plain_text_with_no_controls_is_one_unsplit_part() {
    let parts = split_bidi_marks("ordinary text, nothing special");
    assert_eq!(
        parts,
        vec![BidiMarkedPart::Plain(
            "ordinary text, nothing special".to_string()
        )]
    );
}

#[test]
fn a_single_control_in_the_middle_splits_into_three_parts() {
    let text = "before\u{202E}after";
    let parts = split_bidi_marks(text);
    assert_eq!(
        parts,
        vec![
            BidiMarkedPart::Plain("before".to_string()),
            BidiMarkedPart::Marker {
                token: "RLO",
                name: "bidi-rlo",
            },
            BidiMarkedPart::Plain("after".to_string()),
        ]
    );
}

#[test]
fn a_control_at_the_very_start_or_end_has_no_empty_plain_part_either_side() {
    let leading = split_bidi_marks("\u{2066}text");
    assert_eq!(
        leading,
        vec![
            BidiMarkedPart::Marker {
                token: "LRI",
                name: "bidi-lri",
            },
            BidiMarkedPart::Plain("text".to_string()),
        ]
    );
    let trailing = split_bidi_marks("text\u{2069}");
    assert_eq!(
        trailing,
        vec![
            BidiMarkedPart::Plain("text".to_string()),
            BidiMarkedPart::Marker {
                token: "PDI",
                name: "bidi-pdi",
            },
        ]
    );
}

#[test]
fn adjacent_controls_produce_no_plain_part_between_them() {
    let parts = split_bidi_marks("a\u{202E}\u{2066}b");
    assert_eq!(
        parts,
        vec![
            BidiMarkedPart::Plain("a".to_string()),
            BidiMarkedPart::Marker {
                token: "RLO",
                name: "bidi-rlo",
            },
            BidiMarkedPart::Marker {
                token: "LRI",
                name: "bidi-lri",
            },
            BidiMarkedPart::Plain("b".to_string()),
        ]
    );
}

#[test]
fn no_part_ever_holds_the_control_character_itself() {
    let text = "a\u{202E}b\u{2066}c\u{200E}d";
    for part in split_bidi_marks(text) {
        if let BidiMarkedPart::Plain(s) = part {
            assert!(
                !s.chars().any(forskscope_compare::is_bidi_control),
                "a Plain part must never contain a bidi control character: {s:?}"
            );
        }
    }
}

#[test]
fn every_rfc_100_control_splits_with_its_own_token_and_name() {
    for c in forskscope_compare::BIDI_CONTROLS {
        let text = format!("x{}y", c.ch);
        let parts = split_bidi_marks(&text);
        assert_eq!(
            parts,
            vec![
                BidiMarkedPart::Plain("x".to_string()),
                BidiMarkedPart::Marker {
                    token: c.marker,
                    name: c.name,
                },
                BidiMarkedPart::Plain("y".to_string()),
            ],
            "{:?} did not split as expected",
            c.ch
        );
    }
}
