//! External tool argument expansion tests (RFC-029 §"Acceptance criteria",
//! §"Security policy", §"Test strategy").
//!
//! Key property verified: no shell interpretation. Paths with shell-special
//! characters (spaces, semicolons, dollar signs, backticks) must arrive as
//! intact single arguments, not split or interpreted by a shell.

use std::path::PathBuf;

use crate::external_tool::{
    ExpandContext, ExternalToolArg, ExternalToolCommand, ExternalToolPlaceholder,
    ToolId, UnknownTokenError, expand_args, parse_arg,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn cmd(args: Vec<ExternalToolArg>) -> ExternalToolCommand {
    ExternalToolCommand {
        id:         ToolId("test".into()),
        name:       "Test Tool".into(),
        executable: PathBuf::from("tool"),
        args,
    }
}

fn ctx_path(p: &str) -> ExpandContext {
    ExpandContext { path: Some(PathBuf::from(p)), ..Default::default() }
}

fn ctx_lr(l: &str, r: &str) -> ExpandContext {
    ExpandContext {
        left_path:  Some(PathBuf::from(l)),
        right_path: Some(PathBuf::from(r)),
        ..Default::default()
    }
}

// ── Literal args ──────────────────────────────────────────────────────────────

#[test]
fn literal_args_pass_through_unchanged() {
    let c = cmd(vec![
        ExternalToolArg::Literal("--flag".into()),
        ExternalToolArg::Literal("value".into()),
    ]);
    assert_eq!(expand_args(&c, &ExpandContext::default()), vec!["--flag", "value"]);
}

#[test]
fn empty_arg_list_produces_empty_result() {
    let c = cmd(vec![]);
    assert!(expand_args(&c, &ExpandContext::default()).is_empty());
}

// ── Placeholder expansion ─────────────────────────────────────────────────────

#[test]
fn path_placeholder_expands_to_path() {
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)]);
    let result = expand_args(&c, &ctx_path("/project/src/main.rs"));
    assert_eq!(result, vec!["/project/src/main.rs"]);
}

#[test]
fn left_right_placeholders_expand_independently() {
    let c = cmd(vec![
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::LeftPath),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::RightPath),
    ]);
    let result = expand_args(&c, &ctx_lr("/old/main.rs", "/new/main.rs"));
    assert_eq!(result, vec!["/old/main.rs", "/new/main.rs"]);
}

#[test]
fn line_column_placeholders_expand_as_decimal_strings() {
    let c = cmd(vec![
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Line),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Column),
    ]);
    let ctx = ExpandContext { line: Some(42), column: Some(7), ..Default::default() };
    assert_eq!(expand_args(&c, &ctx), vec!["42", "7"]);
}

#[test]
fn mixed_literal_and_placeholder_in_order() {
    let c = cmd(vec![
        ExternalToolArg::Literal("--goto".into()),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path),
    ]);
    let result = expand_args(&c, &ctx_path("/a/b.rs"));
    assert_eq!(result, vec!["--goto", "/a/b.rs"]);
}

// ── Security: shell-special characters in paths ───────────────────────────────

#[test]
fn path_with_spaces_is_a_single_argument() {
    // Security: if this were shell-expanded, "/path/my file.rs" would become
    // two tokens. The argument array must keep it as one.
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)]);
    let result = expand_args(&c, &ctx_path("/path/my file.rs"));
    assert_eq!(result.len(), 1, "path with spaces must be a single argument");
    assert_eq!(result[0], "/path/my file.rs");
}

#[test]
fn path_with_semicolons_is_not_split() {
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)]);
    let result = expand_args(&c, &ctx_path("/weird;path/file.rs"));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "/weird;path/file.rs");
}

#[test]
fn path_with_dollar_sign_is_not_interpolated() {
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)]);
    let result = expand_args(&c, &ctx_path("/path/$HOME/file.rs"));
    assert_eq!(result[0], "/path/$HOME/file.rs",
        "$HOME must not be interpolated — no shell expansion");
}

#[test]
fn path_with_backticks_is_not_executed() {
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path)]);
    let result = expand_args(&c, &ctx_path("/path/`whoami`/file.rs"));
    assert_eq!(result[0], "/path/`whoami`/file.rs",
        "backtick command substitution must not occur");
}

// ── Missing context values are omitted ───────────────────────────────────────

#[test]
fn missing_path_omits_argument() {
    let c = cmd(vec![
        ExternalToolArg::Literal("--file".into()),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path),
    ]);
    // No path in context — both args should be present but Path is omitted.
    let result = expand_args(&c, &ExpandContext::default());
    assert_eq!(result, vec!["--file"],
        "absent placeholder must be omitted, not produce a 'None' string");
}

#[test]
fn missing_line_omits_argument_not_produces_none_string() {
    let c = cmd(vec![ExternalToolArg::Placeholder(ExternalToolPlaceholder::Line)]);
    let result = expand_args(&c, &ExpandContext::default());
    assert!(result.is_empty(),
        "absent line must be omitted, not produce literal \"None\"");
}

#[test]
fn all_absent_context_produces_only_literals() {
    let c = cmd(vec![
        ExternalToolArg::Literal("open".into()),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Path),
        ExternalToolArg::Placeholder(ExternalToolPlaceholder::Line),
    ]);
    let result = expand_args(&c, &ExpandContext::default());
    assert_eq!(result, vec!["open"]);
}

// ── parse_arg ─────────────────────────────────────────────────────────────────

#[test]
fn parse_arg_recognises_all_placeholders() {
    for ph in ExternalToolPlaceholder::all() {
        let token = ph.token();
        let parsed = parse_arg(token).unwrap();
        assert_eq!(parsed, ExternalToolArg::Placeholder(*ph),
            "token {token} must parse as its placeholder variant");
    }
}

#[test]
fn parse_arg_treats_non_token_as_literal() {
    let arg = parse_arg("--verbose").unwrap();
    assert_eq!(arg, ExternalToolArg::Literal("--verbose".into()));
}

#[test]
fn parse_arg_rejects_unknown_token() {
    let result = parse_arg("{pat}"); // typo of {path}
    assert!(result.is_err(), "unknown token must return Err");
    let err = result.unwrap_err();
    assert_eq!(err, UnknownTokenError { token: "{pat}".into() });
    // Error message mentions the bad token and valid alternatives.
    let msg = err.to_string();
    assert!(msg.contains("{pat}"), "error must name the bad token");
    assert!(msg.contains("{path}"), "error must list a valid token");
}

#[test]
fn parse_arg_rejects_all_curly_brace_strings_not_in_set() {
    for bad in ["{unknown}", "{PATH}", "{LINE}", "{file}"] {
        assert!(parse_arg(bad).is_err(),
            "{bad} must be rejected as an unknown token");
    }
}

#[test]
fn parse_arg_accepts_plain_string_without_braces() {
    // Strings that don't look like tokens are always literals.
    assert_eq!(parse_arg("file.txt").unwrap(), ExternalToolArg::Literal("file.txt".into()));
    assert_eq!(parse_arg("/abs/path").unwrap(), ExternalToolArg::Literal("/abs/path".into()));
}

// ── Placeholder token() and from_token() ─────────────────────────────────────

#[test]
fn all_placeholder_tokens_round_trip() {
    for ph in ExternalToolPlaceholder::all() {
        let token = ph.token();
        let back = ExternalToolPlaceholder::from_token(token);
        assert_eq!(back, Some(*ph), "token {token} must round-trip through from_token");
    }
}

#[test]
fn from_token_returns_none_for_unknown() {
    assert_eq!(ExternalToolPlaceholder::from_token("{nope}"), None);
    assert_eq!(ExternalToolPlaceholder::from_token("path"), None); // no braces
}
