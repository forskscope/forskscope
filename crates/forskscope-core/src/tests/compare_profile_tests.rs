//! CompareProfile, WhitespaceMode, NewlineCompareMode, CaseSensitivity tests
//! (RFC-028 §"Default profiles", §"Compare option types").

use crate::diff::{
    CaseSensitivity, CompareProfile, DiffAlgorithm, DiffOptions, InlineMode,
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

// ── NewlineCompareMode::IgnoreDifference wired into engine (RFC-028) ──────────

#[test]
fn ignore_newlines_option_false_by_default() {
    assert!(!DiffOptions::default().ignore_newlines);
}

#[test]
fn profile_with_ignore_newlines_sets_option() {
    let mut profile = CompareProfile::default_profile();
    profile.newlines = NewlineCompareMode::IgnoreDifference;
    let opts = profile.to_diff_options();
    assert!(opts.ignore_newlines,
        "IgnoreDifference must set ignore_newlines on DiffOptions");
}

#[test]
fn profile_with_significant_newlines_leaves_option_false() {
    let profile = CompareProfile::default_profile();
    // Default is Significant.
    let opts = profile.to_diff_options();
    assert!(!opts.ignore_newlines,
        "Significant newlines must not set ignore_newlines");
}

#[test]
fn lf_and_crlf_lines_are_equal_when_ignore_newlines_set() {
    use crate::diff::compute_diff;
    // Left uses LF, right uses CRLF for the same content.
    let left  = "hello\nworld\n";
    let right = "hello\r\nworld\r\n";

    let opts_ignore = DiffOptions { ignore_newlines: true, ..DiffOptions::default() };
    let doc = compute_diff(left, right, opts_ignore);
    assert!(doc.is_identical(),
        "LF vs CRLF must be treated as equal when ignore_newlines is set");
}

#[test]
fn lf_and_crlf_lines_are_different_when_newlines_significant() {
    use crate::diff::compute_diff;
    let left  = "hello\nworld\n";
    let right = "hello\r\nworld\r\n";

    let opts = DiffOptions::default(); // ignore_newlines = false
    let doc  = compute_diff(left, right, opts);
    // With newlines significant, CRLF ≠ LF, so the lines differ.
    assert!(!doc.is_identical(),
        "LF vs CRLF must differ when newlines are significant");
}

#[test]
fn ignore_newlines_does_not_suppress_genuine_content_diff() {
    use crate::diff::compute_diff;
    // Even with ignore_newlines, different content must still show as changed.
    let left  = "hello\n";
    let right = "world\r\n";

    let opts = DiffOptions { ignore_newlines: true, ..DiffOptions::default() };
    let doc  = compute_diff(left, right, opts);
    assert!(!doc.is_identical(),
        "Content differences must still be reported even when newlines are ignored");
}

#[test]
fn code_review_profile_newlines_are_significant() {
    // Code Review profile does NOT ignore newlines by default.
    let opts = CompareProfile::code_review().to_diff_options();
    assert!(!opts.ignore_newlines);
}
