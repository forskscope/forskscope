use crate::IgnoreRules;
use crate::ignore::{normalize_dir_token, normalize_extension_token};

// ── RFC-099 §3: the per-token functions `from_settings` is now built on,
// and the Settings dialog's own ignore-list badges share directly. ──

#[test]
fn normalize_extension_token_trims_drops_the_dot_and_lowercases() {
    assert_eq!(
        normalize_extension_token("  .TMP  "),
        Some("tmp".to_string())
    );
    assert_eq!(
        normalize_extension_token("class"),
        Some("class".to_string())
    );
    assert_eq!(normalize_extension_token("..o"), Some("o".to_string()));
}

#[test]
fn normalize_extension_token_is_none_for_an_empty_result() {
    assert_eq!(normalize_extension_token(""), None);
    assert_eq!(normalize_extension_token("   "), None);
    assert_eq!(normalize_extension_token("."), None);
}

#[test]
fn normalize_dir_token_trims_but_keeps_case() {
    assert_eq!(
        normalize_dir_token("  target  "),
        Some("target".to_string())
    );
    assert_eq!(
        normalize_dir_token("Node_Modules"),
        Some("Node_Modules".to_string())
    );
}

#[test]
fn normalize_dir_token_is_none_for_an_empty_result() {
    assert_eq!(normalize_dir_token(""), None);
    assert_eq!(normalize_dir_token("   "), None);
}

#[test]
fn empty_rules_ignore_nothing() {
    let r = IgnoreRules::default();
    assert!(!r.is_file_ignored("main.rs"));
    assert!(!r.is_dir_ignored("target"));
}

#[test]
fn extension_match_is_case_insensitive() {
    let r = IgnoreRules::from_settings("o, class", "");
    assert!(r.is_file_ignored("main.o"), "bare extension");
    assert!(r.is_file_ignored("Main.O"), "uppercase extension");
    assert!(r.is_file_ignored("Foo.CLASS"), "uppercase class");
    assert!(!r.is_file_ignored("main.rs"), "non-ignored extension");
}

#[test]
fn extension_input_with_leading_dot_normalizes() {
    let r = IgnoreRules::from_settings(".tmp, .log", "");
    assert!(r.is_file_ignored("file.tmp"));
    assert!(r.is_file_ignored("server.log"));
    assert!(!r.is_file_ignored("tmp")); // no extension = no match
}

#[test]
fn files_without_extensions_are_not_matched_by_extension_rules() {
    let r = IgnoreRules::from_settings("rs", "");
    assert!(!r.is_file_ignored("Makefile"), "no extension");
}

#[test]
fn dir_exact_match() {
    let r = IgnoreRules::from_settings("", "target, node_modules");
    assert!(r.is_dir_ignored("target"));
    assert!(r.is_dir_ignored("node_modules"));
    assert!(!r.is_dir_ignored("src"));
}

#[test]
fn dir_star_suffix_pattern() {
    let r = IgnoreRules::from_settings("", "*.cache");
    assert!(r.is_dir_ignored("build.cache"));
    assert!(r.is_dir_ignored(".cache"));
    assert!(!r.is_dir_ignored("cache.build"), "suffix mismatch");
}

#[test]
fn dir_star_prefix_pattern() {
    let r = IgnoreRules::from_settings("", "tmp*");
    assert!(r.is_dir_ignored("tmp"));
    assert!(r.is_dir_ignored("tmpfiles"));
    assert!(!r.is_dir_ignored("atmp"), "prefix mismatch");
}

#[test]
fn dir_star_infix_pattern() {
    let r = IgnoreRules::from_settings("", "*backup*");
    assert!(r.is_dir_ignored("mybackup1"));
    assert!(r.is_dir_ignored("backup"));
    assert!(!r.is_dir_ignored("saved"), "no keyword");
}

#[test]
fn is_empty_reflects_rules_state() {
    assert!(IgnoreRules::default().is_empty());
    assert!(!IgnoreRules::from_settings("o", "").is_empty());
    assert!(!IgnoreRules::from_settings("", "target").is_empty());
}

#[test]
fn whitespace_in_csv_is_trimmed() {
    let r = IgnoreRules::from_settings("  o  ,  class  ", "  target  ,  node_modules  ");
    assert!(r.is_file_ignored("main.o"));
    assert!(r.is_dir_ignored("target"));
}
