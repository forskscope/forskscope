//! F54/F75: `cargo xtask ui-logic-connectivity` — verifies every name
//! `forskscope-ui-logic`'s crate root re-exports (`lib.rs`'s `pub use`
//! list) is actually referenced by `forskscope-ui` (handoff 033).
//!
//! ## Why this exists
//!
//! Five defects (F52, F75, F84, F88a, RFC-085's restoration) shipped the
//! same shape: built, tested, documented in `forskscope-ui-logic`, never
//! wired into `forskscope-ui`. A `pub use` at the crate root is what makes
//! an unconnected module *look* like part of the product surface — this
//! check keeps that list honest.
//!
//! ## What counts as a consumer
//!
//! A crate-root export is "consumed" if its name appears as a whole word
//! anywhere in `forskscope-ui/src` — any `.rs` file, including that
//! crate's own tests (a UI-side test genuinely exercising a `ui-logic`
//! type is real integration, not a loophole). This is deliberately a text
//! search, not a type-flow analysis: a plain word search can *overcount*
//! (a name used only inside `ui-logic`'s own tests, or as an unrelated
//! identifier that happens to collide) and can *undercount* (a value
//! consumed only through field/method access on a call chain that never
//! spells the type's name, e.g. `store.settings.read().theme` never
//! writing `Theme`).
//!
//! The overcounting risk is why this only ever searches `forskscope-ui`,
//! never `forskscope-ui-logic` itself — a name used solely in `ui-logic`'s
//! own `#[cfg(test)]` modules is not consumed by the product.
//!
//! The undercounting risk is real and was found repeatedly while writing
//! this check (`RecoveryButton`, `MatchPosition`/`MatchSide`,
//! `guard_for_sizes_with_limits`, the two `MigrationNotice`/
//! `RecoveryDialogView` pairs — all genuinely used by `forskscope-ui`,
//! purely through field access, never spelled by name). The fix applied
//! throughout handoff 033 was **not** to special-case the check for them:
//! it was to stop re-exporting them at the crate root, since nothing
//! needs the root-level name. They remain `pub` within their own module
//! (`forskscope_ui_logic::compare::search_index::MatchPosition`, etc.),
//! reachable if a future caller ever needs to spell them explicitly. This
//! is the invariant a contributor must keep: **only re-export a name at
//! the crate root once something in `forskscope-ui` actually writes that
//! name.** Breaking that invariant is what would make this check start
//! producing false failures — not a bug in the check itself.
//!
//! ## Comments and string literals do not count (review 106 §3)
//!
//! The first version of this check searched raw file text, which meant a
//! name mentioned only in a comment or a string literal counted as
//! "consumed" — the opposite of what the check exists to catch: a layer
//! that is *talked about* but not *called*. `strip_comments_and_strings`
//! runs over both `lib.rs` and every `forskscope-ui` source file before
//! any of `extract_root_exports`, `contains_glob_import`, or
//! `contains_word` sees them. It removes line comments (`//`, `///`,
//! `//!`), block comments (nesting `/* /* */ */` correctly), and string
//! literals (`"..."` and raw `r#"..."#` with any number of `#`s),
//! replacing each with a single space so tokens on either side never
//! fuse into one word. A lifetime (`'a`) is not a char literal and is
//! left untouched — the two are told apart by what follows the closing
//! position: a char literal (`'x'`, `'\n'`) is closed by another `'`
//! within a few characters, and valid Rust never places a `'` right after
//! a lifetime name.
//!
//! ## The glob-import guard
//!
//! A `use forskscope_ui_logic::*;` (or `forskscope_ui_logic::some_module::*`)
//! anywhere in `forskscope-ui/src` fails this check outright, before the
//! consumer search runs at all. This check's own text search does not
//! actually depend on explicit `use` statements — it searches whole file
//! contents — so a glob import does not literally blind *this*
//! implementation. It is refused anyway, on principle: a glob import is
//! exactly the shape of change that could blind a *future*, differently
//! implemented version of this check without that regression ever being
//! visible (the F42 shape — a check that can go blind without failing).
//! Cheaper to refuse the import than to trust every future revision of
//! this file to preserve an invariant it does not need today.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

pub fn run(root: &Path) {
    let lib_path = root.join("crates/forskscope-ui-logic/src/lib.rs");
    let lib_src = fs::read_to_string(&lib_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", lib_path.display()));
    let exports = extract_root_exports(&strip_comments_and_strings(&lib_src));

    let ui_src_dir = root.join("crates/forskscope-ui/src");
    let ui_files = collect_rs_files(&ui_src_dir);
    let ui_contents: Vec<(PathBuf, String)> = ui_files
        .into_iter()
        .map(|p| {
            let content = fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
            (p, strip_comments_and_strings(&content))
        })
        .collect();

    // Checked first: if this fires, the consumer search below cannot be
    // trusted, so report only this and stop.
    let glob_offenders: Vec<&Path> = ui_contents
        .iter()
        .filter(|(_, c)| contains_glob_import(c))
        .map(|(p, _)| p.as_path())
        .collect();
    if !glob_offenders.is_empty() {
        eprintln!(
            "ui-logic-connectivity check failed: a glob import of forskscope_ui_logic \
             was found - refused on principle (F54), see this check's own doc comment:"
        );
        for p in &glob_offenders {
            eprintln!("  - {}", p.display());
        }
        eprintln!("Use explicit imports instead.");
        process::exit(1);
    }

    let mut unconsumed: Vec<&String> = exports
        .iter()
        .filter(|name| !ui_contents.iter().any(|(_, c)| contains_word(c, name)))
        .collect();
    unconsumed.sort();

    if !unconsumed.is_empty() {
        eprintln!(
            "ui-logic-connectivity check failed: these forskscope-ui-logic crate-root \
             exports have no consumer in forskscope-ui (F54/F75):"
        );
        for name in &unconsumed {
            eprintln!("  - {name}");
        }
        eprintln!(
            "Wire forskscope-ui to use them, or remove them from lib.rs's `pub use` list \
             (they can stay `pub` within their own module if ui-logic's own tests still need them)."
        );
        process::exit(1);
    }

    println!(
        "ui-logic connectivity check passed: {} crate-root exports all have a consumer in forskscope-ui.",
        exports.len()
    );
}

/// Removes line comments, block comments (nested), and string literals
/// (normal and raw) from Rust source, replacing each with a single space.
/// Leaves lifetimes (`'a`) untouched — see this module's doc comment for
/// how a char literal is told apart from one. Written by hand, on
/// purpose: review 106 §3 asked for no new parser dependency.
fn strip_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < n {
        let c = chars[i];

        if let Some(hashes) = raw_string_hashes(&chars, i) {
            let prefix_len = usize::from(chars[i] == 'b') + 1 + hashes + 1; // (b)? r #* "
            let mut j = i + prefix_len;
            loop {
                if j >= n {
                    break;
                }
                if chars[j] == '"' && chars[j + 1..].iter().take(hashes).all(|&h| h == '#') {
                    j += 1 + hashes;
                    break;
                }
                j += 1;
            }
            i = j;
            out.push(' ');
            continue;
        }

        if c == '/' && chars.get(i + 1) == Some(&'/') {
            let mut j = i + 2;
            while j < n && chars[j] != '\n' {
                j += 1;
            }
            i = j;
            out.push(' ');
            continue;
        }

        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let mut depth = 1usize;
            let mut j = i + 2;
            while j < n && depth > 0 {
                if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
                    depth += 1;
                    j += 2;
                } else if chars[j] == '*' && chars.get(j + 1) == Some(&'/') {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            i = j;
            out.push(' ');
            continue;
        }

        if c == '"' {
            let mut j = i + 1;
            while j < n {
                if chars[j] == '\\' && j + 1 < n {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            out.push(' ');
            continue;
        }

        if c == '\'' {
            // A one-character escape ('\n', '\'', '\\', ...): closed by
            // another `'` three positions later.
            if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') {
                i += 4;
                out.push(' ');
                continue;
            }
            // A plain single-character literal ('x', '0', ' '): closed by
            // another `'` two positions later.
            if chars.get(i + 1).is_some_and(|&next| next != '\\') && chars.get(i + 2) == Some(&'\'')
            {
                i += 3;
                out.push(' ');
                continue;
            }
            // Anything else starting with `'` - a lifetime, or a char
            // literal form this scanner does not special-case (`'\x41'`,
            // `'\u{1F600}'`) - is left as ordinary text rather than risk
            // treating a lifetime as a string delimiter.
        }

        out.push(c);
        i += 1;
    }
    out
}

/// `Some(hash_count)` if `chars[i..]` starts a raw (optionally byte)
/// string: `r"`, `r#"`, `r##"`, ..., or the `b`-prefixed forms.
fn raw_string_hashes(chars: &[char], i: usize) -> Option<usize> {
    let mut j = i;
    if chars.get(j) == Some(&'b') {
        j += 1;
    }
    if chars.get(j) != Some(&'r') {
        return None;
    }
    j += 1;
    let mut hashes = 0;
    while chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    if chars.get(j) == Some(&'"') {
        Some(hashes)
    } else {
        None
    }
}

fn extract_root_exports(lib_src: &str) -> Vec<String> {
    let mut exports = Vec::new();
    let mut rest = lib_src;
    while let Some(start) = rest.find("pub use ") {
        let after = &rest[start + "pub use ".len()..];
        let end = after
            .find(';')
            .unwrap_or_else(|| panic!("unterminated `pub use` in lib.rs"));
        exports.extend(parse_use_items(&after[..end]));
        rest = &after[end + 1..];
    }
    exports
}

fn parse_use_items(stmt: &str) -> Vec<String> {
    let stmt = stmt.trim();
    if let Some(brace_start) = stmt.find('{') {
        let brace_end = stmt
            .rfind('}')
            .unwrap_or_else(|| panic!("unmatched `{{` in `pub use {stmt}`"));
        stmt[brace_start + 1..brace_end]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(item_name)
            .collect()
    } else {
        vec![item_name(stmt)]
    }
}

/// The externally-visible name for one `use` item: the part after ` as `
/// when aliased, otherwise the last `::`-separated path segment.
fn item_name(item: &str) -> String {
    let item = item.trim();
    if let Some((_, alias)) = item.rsplit_once(" as ") {
        alias.trim().to_string()
    } else {
        item.rsplit("::").next().unwrap_or(item).trim().to_string()
    }
}

fn contains_glob_import(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.contains("forskscope_ui_logic") && line.contains("::*"))
}

/// Whole-word substring search - `word` must not be immediately preceded
/// or followed by an identifier character (alphanumeric or `_`).
fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let wbytes = word.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(word) {
        let abs = start + pos;
        let before_ok = abs == 0 || !is_ident_char(bytes[abs - 1]);
        let after_idx = abs + wbytes.len();
        let after_ok = after_idx >= bytes.len() || !is_ident_char(bytes[after_idx]);
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_rs_files(&path));
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_item_pub_use() {
        let src = "pub use compare::save_error::SaveErrorView;\n";
        assert_eq!(extract_root_exports(src), vec!["SaveErrorView"]);
    }

    #[test]
    fn extracts_multi_item_braced_pub_use() {
        let src = "pub use compare::load_guard::{LoadGuard, guard_for_sizes};\n";
        assert_eq!(
            extract_root_exports(src),
            vec!["LoadGuard", "guard_for_sizes"]
        );
    }

    #[test]
    fn extracts_aliased_items() {
        let src = "pub use a::{Foo as Bar, Baz};\n";
        assert_eq!(extract_root_exports(src), vec!["Bar", "Baz"]);
    }

    #[test]
    fn extracts_across_multiple_lines() {
        let src = "pub use a::{\n    One,\n    Two,\n};\n";
        assert_eq!(extract_root_exports(src), vec!["One", "Two"]);
    }

    #[test]
    fn ignores_pub_mod() {
        let src = "pub mod compare;\npub use compare::save_error::SaveErrorView;\n";
        assert_eq!(extract_root_exports(src), vec!["SaveErrorView"]);
    }

    #[test]
    fn word_boundary_does_not_match_substring() {
        // "Match" must not match inside "MatchIndex".
        assert!(!contains_word(
            "use forskscope_ui_logic::MatchIndex;",
            "Match"
        ));
        assert!(contains_word(
            "use forskscope_ui_logic::MatchIndex;",
            "MatchIndex"
        ));
    }

    #[test]
    fn word_boundary_matches_field_access() {
        assert!(contains_word("let x = pos.hunk_elem_id;", "pos"));
    }

    #[test]
    fn detects_glob_import() {
        assert!(contains_glob_import("use forskscope_ui_logic::*;"));
        assert!(contains_glob_import("use forskscope_ui_logic::compare::*;"));
        assert!(!contains_glob_import(
            "use forskscope_ui_logic::{DeepFilter, apply_filter};"
        ));
    }

    // ── strip_comments_and_strings (review 106 §3) ─────────────────────────────

    #[test]
    fn strips_line_comments() {
        let stripped = strip_comments_and_strings("let x = 1; // Foo lives here\n");
        assert!(!contains_word(&stripped, "Foo"));
        assert!(contains_word(&stripped, "x"));
    }

    #[test]
    fn strips_doc_comments() {
        let stripped = strip_comments_and_strings("/// Foo does a thing.\nfn f() {}");
        assert!(!contains_word(&stripped, "Foo"));
        assert!(contains_word(&stripped, "f"));

        let stripped2 = strip_comments_and_strings("//! Crate-level mention of Foo.\n");
        assert!(!contains_word(&stripped2, "Foo"));
    }

    #[test]
    fn strips_block_comments_including_nested() {
        let stripped = strip_comments_and_strings("a /* Foo /* nested Bar */ still comment */ b");
        assert!(!contains_word(&stripped, "Foo"));
        assert!(!contains_word(&stripped, "Bar"));
        // The comment closes with the *outer* `*/`, not the inner one -
        // "still" and "comment" (between the nested close and the outer
        // one) must also have been stripped, and code on both sides of
        // the whole comment must survive.
        assert!(!contains_word(&stripped, "still"));
        assert!(!contains_word(&stripped, "comment"));
        assert!(contains_word(&stripped, "a"));
        assert!(contains_word(&stripped, "b"));
    }

    #[test]
    fn strips_normal_strings() {
        let stripped = strip_comments_and_strings(r#"let s = "Foo lives here";"#);
        assert!(!contains_word(&stripped, "Foo"));
        assert!(contains_word(&stripped, "s"));
    }

    #[test]
    fn strips_normal_strings_with_escaped_quote() {
        // The escaped `"` inside the string must not end it early.
        let stripped = strip_comments_and_strings(r#"let s = "a \" Foo \" b"; c"#);
        assert!(!contains_word(&stripped, "Foo"));
        assert!(contains_word(&stripped, "c"));
    }

    #[test]
    fn strips_raw_strings_with_hashes() {
        let stripped = strip_comments_and_strings(r####"let s = r###"Foo "# still inside"###;"####);
        assert!(!contains_word(&stripped, "Foo"));
        assert!(!contains_word(&stripped, "inside"));
        assert!(contains_word(&stripped, "s"));
        assert!(stripped.trim_end().ends_with(';'));
    }

    #[test]
    fn lifetime_next_to_identifier_is_not_stripped() {
        let src = "fn f<'a>(x: &'a Foo) -> &'a Foo { x }";
        let stripped = strip_comments_and_strings(src);
        // Nothing should be removed at all - no comment or string exists.
        assert_eq!(stripped, src);
        assert!(contains_word(&stripped, "Foo"));
    }

    #[test]
    fn char_literal_is_stripped_without_eating_following_lifetime() {
        let src = "let c = 'x'; fn f<'a>(v: &'a Foo) {}";
        let stripped = strip_comments_and_strings(src);
        assert!(!contains_word(&stripped, "x"));
        assert!(contains_word(&stripped, "Foo"));
        assert!(contains_word(&stripped, "a") || stripped.contains("'a"));
    }

    #[test]
    fn a_name_that_appears_only_in_a_comment_does_not_count_as_consumed() {
        // The exact shape review 106's falsification planted: a symbol
        // mentioned only in a `//` comment must not satisfy the search.
        let ui_file = "// f54_probe_symbol (architect probe: comment-only mention)\n";
        let stripped = strip_comments_and_strings(ui_file);
        assert!(!contains_word(&stripped, "f54_probe_symbol"));
    }

    #[test]
    fn a_name_in_real_code_still_counts_once_stripped() {
        let ui_file = "// f54_probe_symbol is unrelated\nlet _ = f54_probe_symbol();\n";
        let stripped = strip_comments_and_strings(ui_file);
        assert!(contains_word(&stripped, "f54_probe_symbol"));
    }

    #[test]
    fn extract_root_exports_ignores_pub_use_mentioned_in_a_doc_comment() {
        let src = "//! Example: `pub use foo::Bar;`\npub use compare::save_error::SaveErrorView;\n";
        let stripped = strip_comments_and_strings(src);
        assert_eq!(extract_root_exports(&stripped), vec!["SaveErrorView"]);
    }
}
