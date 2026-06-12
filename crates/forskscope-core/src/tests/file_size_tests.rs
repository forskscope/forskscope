//! FileSizeClass and PerformanceLimits tests (RFC-013 §5 "Threshold Policy").

use crate::job::{FileSizeClass, PerformanceLimits};

// ── Default limits sanity ─────────────────────────────────────────────────────

#[test]
fn default_limits_are_ordered_ascending() {
    let l = PerformanceLimits::default();
    assert!(l.max_eager_text_bytes < l.medium_text_threshold_bytes,
        "small < medium");
    assert!(l.medium_text_threshold_bytes < l.large_text_threshold_bytes,
        "medium < large");
}

// ── FileSizeClass::classify ───────────────────────────────────────────────────

#[test]
fn zero_bytes_is_small() {
    assert_eq!(FileSizeClass::classify(0, &PerformanceLimits::default()), FileSizeClass::Small);
}

#[test]
fn at_max_eager_boundary_is_small() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.max_eager_text_bytes, &l), FileSizeClass::Small);
}

#[test]
fn one_byte_above_eager_boundary_is_medium() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.max_eager_text_bytes + 1, &l), FileSizeClass::Medium);
}

#[test]
fn at_medium_boundary_is_medium() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.medium_text_threshold_bytes, &l), FileSizeClass::Medium);
}

#[test]
fn one_byte_above_medium_is_large() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.medium_text_threshold_bytes + 1, &l), FileSizeClass::Large);
}

#[test]
fn at_large_boundary_is_large() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.large_text_threshold_bytes, &l), FileSizeClass::Large);
}

#[test]
fn above_large_boundary_is_very_large() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(l.large_text_threshold_bytes + 1, &l), FileSizeClass::VeryLarge);
}

#[test]
fn huge_file_is_very_large() {
    let l = PerformanceLimits::default();
    assert_eq!(FileSizeClass::classify(1024 * 1024 * 1024, &l), FileSizeClass::VeryLarge);
}

// ── FileSizeClass predicates ──────────────────────────────────────────────────

#[test]
fn small_has_eager_inline_diff() {
    assert!(FileSizeClass::Small.inline_diff_eager());
    assert!(!FileSizeClass::Medium.inline_diff_eager());
    assert!(!FileSizeClass::Large.inline_diff_eager());
    assert!(!FileSizeClass::VeryLarge.inline_diff_eager());
}

#[test]
fn large_and_very_large_require_user_prompt() {
    assert!(!FileSizeClass::Small.requires_user_prompt());
    assert!(!FileSizeClass::Medium.requires_user_prompt());
    assert!(FileSizeClass::Large.requires_user_prompt());
    assert!(FileSizeClass::VeryLarge.requires_user_prompt());
}

#[test]
fn only_very_large_is_too_large_for_diff() {
    assert!(!FileSizeClass::Small.too_large_for_diff());
    assert!(!FileSizeClass::Medium.too_large_for_diff());
    assert!(!FileSizeClass::Large.too_large_for_diff());
    assert!(FileSizeClass::VeryLarge.too_large_for_diff());
}

// ── FileSizeClass ordering ────────────────────────────────────────────────────

#[test]
fn file_size_class_ordering_is_ascending_by_severity() {
    assert!(FileSizeClass::Small < FileSizeClass::Medium);
    assert!(FileSizeClass::Medium < FileSizeClass::Large);
    assert!(FileSizeClass::Large < FileSizeClass::VeryLarge);
}

// ── Custom limits ─────────────────────────────────────────────────────────────

#[test]
fn custom_limits_classify_correctly() {
    let limits = PerformanceLimits {
        max_eager_text_bytes:           1_000,
        medium_text_threshold_bytes:    10_000,
        large_text_threshold_bytes:     100_000,
        max_inline_diff_chars_per_hunk: 500,
        max_directory_entries_eager:    100,
        max_eager_lines:                10_000,
    };
    assert_eq!(FileSizeClass::classify(999,    &limits), FileSizeClass::Small);
    assert_eq!(FileSizeClass::classify(1_000,  &limits), FileSizeClass::Small);
    assert_eq!(FileSizeClass::classify(1_001,  &limits), FileSizeClass::Medium);
    assert_eq!(FileSizeClass::classify(10_001, &limits), FileSizeClass::Large);
    assert_eq!(FileSizeClass::classify(100_001,&limits), FileSizeClass::VeryLarge);
}
