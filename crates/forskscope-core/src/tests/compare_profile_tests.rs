//! CompareProfile, WhitespaceMode, NewlineCompareMode, CaseSensitivity tests
//! (RFC-028 §"Default profiles", §"Compare option types").

use crate::diff::{
    CaseSensitivity, CompareProfile, DiffAlgorithm, InlineMode,
    NewlineCompareMode, WhitespaceMode,
};

// ── Default profile ───────────────────────────────────────────────────────────

#[test]
fn default_profile_has_expected_settings() {
    let p = CompareProfile::default_profile();
    assert_eq!(p.name, "Default");
    assert_eq!(p.whitespace,  WhitespaceMode::Significant);
    assert_eq!(p.newlines,    NewlineCompareMode::Significant);
    assert_eq!(p.case,        CaseSensitivity::Sensitive);
    assert_eq!(p.inline_mode, InlineMode::Lazy);
    assert_eq!(p.algorithm,   DiffAlgorithm::Myers);
}

// ── Code Review profile ───────────────────────────────────────────────────────

#[test]
fn code_review_profile_uses_histogram_algorithm() {
    let p = CompareProfile::code_review();
    assert_eq!(p.algorithm, DiffAlgorithm::Histogram,
        "Code Review should use histogram diff for better hunk alignment");
}

// ── Loose Text profile ────────────────────────────────────────────────────────

#[test]
fn loose_text_ignores_trailing_whitespace_and_newlines() {
    let p = CompareProfile::loose_text();
    assert_eq!(p.whitespace, WhitespaceMode::IgnoreTrailing);
    assert_eq!(p.newlines,   NewlineCompareMode::IgnoreDifference);
}

// ── Large File Safe profile ───────────────────────────────────────────────────

#[test]
fn large_file_safe_disables_inline_diff() {
    let p = CompareProfile::large_file_safe();
    assert_eq!(p.inline_mode, InlineMode::None,
        "Large file safe must disable inline diff");
}

// ── All presets ───────────────────────────────────────────────────────────────

#[test]
fn all_presets_returns_four_profiles() {
    assert_eq!(CompareProfile::all_presets().len(), 4);
}

#[test]
fn all_preset_names_are_non_empty_and_unique() {
    let presets = CompareProfile::all_presets();
    let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
    assert!(names.iter().all(|n| !n.is_empty()), "all names must be non-empty");
    let unique: std::collections::HashSet<_> = names.iter().copied().collect();
    assert_eq!(unique.len(), names.len(), "all preset names must be unique");
}

// ── Default implementation ────────────────────────────────────────────────────

#[test]
fn default_compare_profile_is_default_profile() {
    assert_eq!(CompareProfile::default(), CompareProfile::default_profile());
}

// ── to_diff_options conversion ────────────────────────────────────────────────

#[test]
fn default_profile_to_diff_options_has_expected_flags() {
    let opts = CompareProfile::default_profile().to_diff_options();
    assert!(!opts.ignore_whitespace, "default profile: whitespace significant");
    assert!(!opts.ignore_case,       "default profile: case sensitive");
    assert_eq!(opts.algorithm, DiffAlgorithm::Myers);
}

#[test]
fn loose_text_to_diff_options_enables_ignore_whitespace() {
    let opts = CompareProfile::loose_text().to_diff_options();
    assert!(opts.ignore_whitespace, "loose text: whitespace ignored");
}

#[test]
fn case_insensitive_profile_maps_to_ignore_case() {
    let p = CompareProfile {
        name:        "Custom".into(),
        whitespace:  WhitespaceMode::Significant,
        newlines:    NewlineCompareMode::Significant,
        case:        CaseSensitivity::Insensitive,
        inline_mode: InlineMode::Lazy,
        algorithm:   DiffAlgorithm::Myers,
    };
    assert!(p.to_diff_options().ignore_case);
}

#[test]
fn large_file_safe_to_diff_options_has_none_inline_mode() {
    let opts = CompareProfile::large_file_safe().to_diff_options();
    assert_eq!(opts.inline_mode, InlineMode::None);
}

// ── Type defaults ─────────────────────────────────────────────────────────────

#[test]
fn whitespace_mode_default_is_significant() {
    assert_eq!(WhitespaceMode::default(), WhitespaceMode::Significant);
}

#[test]
fn newline_compare_mode_default_is_significant() {
    assert_eq!(NewlineCompareMode::default(), NewlineCompareMode::Significant);
}

#[test]
fn case_sensitivity_default_is_sensitive() {
    assert_eq!(CaseSensitivity::default(), CaseSensitivity::Sensitive);
}
