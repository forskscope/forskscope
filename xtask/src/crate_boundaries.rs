//! RFC-093 §3: `cargo xtask crate-boundaries` — verifies the structural
//! promises RFC-093 §1's table makes about the crates it names, on every
//! push, the way a code review alone cannot keep catching by hand:
//!
//! - **Rule 1.** `forskscope-compare` and `forskscope-report` never write a
//!   file outside `#[cfg(test)]`/`tests/`.
//! - **Rule 2.** Neither crate ever spawns a process, anywhere.
//! - **Rule 3.** Every workspace crate's direct `forskscope-*` dependencies
//!   are a subset of what RFC-093 §1's table allows it to depend on - the
//!   table is a ceiling, not a floor a crate must exercise every cell of,
//!   so a table entry a crate does not currently use (`forskscope-ui-logic`
//!   and `forskscope-report`, today) is not itself a violation. A
//!   dependency the table does not list at all fails it, and so does a
//!   workspace crate RFC-093 §1 does not name at all (a future
//!   `forskscope-cli`, say, added here without ever being added to the
//!   table, or a crate renamed on one side and not the other).
//!
//! ## What counts as test code, and how the skip is done
//!
//! A file is skipped wholesale when its path has a `tests` directory
//! component, or when its own file name is exactly `tests.rs` (mirrors
//! `ui_logic_docs`'s leaf-module skip — see its own doc comment). Within
//! every other file, comments and string literals are stripped first via
//! `ui_logic_connectivity::strip_comments_and_strings` (so a forbidden call
//! name mentioned only in a doc comment or a string never counts — the
//! same reasoning that function's own doc comment gives), then an inline
//! `#[cfg(test)] mod ... { ... }` block — several production files in both
//! crates keep their own small test module this way, alongside the
//! `tests/` directory's larger files — is blanked out from the attribute
//! through its block's matching closing brace, counted the same way that
//! shared function counts nested `/* */` comments. A `#[cfg(test)]` that
//! governs a single item with no braces (a bare `use` or `const`) is
//! blanked up to its terminating `;` instead; neither shape can contain a
//! forbidden call that would otherwise be missed, since the whole point is
//! that nothing of substance follows a brace-less `#[cfg(test)]` item here.
//!
//! ## Rule 1's call list
//!
//! Textual, not a parse of the call graph, matching this project's other
//! xtask gates (review 106 §3: no new parser dependency) - and like those
//! gates, this can overcount (a name that happens to collide) far more
//! easily than it can undercount a real write disguised under a different
//! name, which is the risk that matters for a boundary gate. The exact
//! list, each anchored so it cannot collide with an unrelated method of
//! the same short name (`io::Write::write`, `str::rename`-shaped code that
//! does not exist here, etc.): `fs::write(`, `fs::create_dir(`,
//! `fs::create_dir_all(`, `fs::remove_file(`, `fs::remove_dir(`,
//! `fs::remove_dir_all(`, `fs::rename(`, `fs::copy(`,
//! `fs::set_permissions(`, `File::create(`, `.write(true)`, `.append(true)`,
//! `.create(true)` (the three `OpenOptions` builder calls RFC-093 §3 names),
//! and `tempfile::`/`NamedTempFile`/`TempDir` (any use of the crate at all,
//! since every one of its real entry points ends up creating a file).
//!
//! A qualified call (`fs::write(`) is not the only way to reach these
//! functions: `use std::fs::write;` followed by a bare `write(p, b)` call
//! site carries none of the text patterns above (review 181 §3's own
//! falsification). [`find_fs_write_fn_imports`] catches the import itself
//! — `use std::fs::<name>` or `use std::fs::{.., <name>, ..}` for any of
//! rule 1's nine write functions — rather than trying to track every later
//! bare-name call site, which risks missing one disguised behind a local
//! alias or a re-export. `File`/`OpenOptions` reached the same way still
//! trips rule 1 through the `.write(true)`/`.create(true)`/`.append(true)`
//! builder-method patterns above, which do not care how `File`/
//! `OpenOptions` itself was imported.
//!
//! ## Rule 2's call
//!
//! `process::Command` (covers both `std::process::Command` and an
//! imported bare `Command`’s qualified form `process::Command`, and that
//! qualifier is specific enough not to collide with anything else in
//! these two crates' own vocabulary).
//!
//! ## Rule 3's method
//!
//! `cargo tree -p <crate> -e normal --depth 1 --prefix none`, not a hand
//! parse of `cargo metadata`'s JSON (xtask carries no JSON parser - DEC-005
//! keeps xtask's own dependency footprint minimal - and `cargo tree` reads
//! the same resolved graph `cargo metadata` would; every other dependency
//! shape check in this file already reads that graph through `cargo tree`,
//! e.g. `assert_immediate_dependents`). The first output line is the crate
//! itself; every following line naming a `forskscope-*` package, with its
//! version and path stripped, is one direct dependency edge.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use crate::ui_logic_connectivity::strip_comments_and_strings;

/// RFC-093 §1's table, direct `forskscope-*` dependencies only.
fn expected_dependency_table() -> BTreeMap<&'static str, &'static [&'static str]> {
    BTreeMap::from([
        ("forskscope-compare", &[] as &[&str]),
        ("forskscope-report", &["forskscope-compare"] as &[&str]),
        (
            // RFC-094 §1: the headless program - depends only on the
            // read-only engine and report crates, nothing from edit,
            // desktop, ui-logic or ui.
            "forskscope-cli",
            &["forskscope-compare", "forskscope-report"] as &[&str],
        ),
        ("forskscope-edit", &["forskscope-compare"] as &[&str]),
        (
            "forskscope-desktop",
            &["forskscope-compare", "forskscope-edit"] as &[&str],
        ),
        (
            "forskscope-ui-logic",
            &[
                "forskscope-compare",
                "forskscope-report",
                "forskscope-edit",
                "forskscope-desktop",
            ] as &[&str],
        ),
        (
            // RFC-093 amendment (36eb2a4): the forskscope-ui package is
            // named forskscope, matching the program it builds.
            "forskscope",
            &[
                "forskscope-compare",
                "forskscope-report",
                "forskscope-edit",
                "forskscope-desktop",
                "forskscope-ui-logic",
            ] as &[&str],
        ),
    ])
}

const WRITE_CALL_PATTERNS: &[&str] = &[
    "fs::write(",
    "fs::create_dir(",
    "fs::create_dir_all(",
    "fs::remove_file(",
    "fs::remove_dir(",
    "fs::remove_dir_all(",
    "fs::rename(",
    "fs::copy(",
    "fs::set_permissions(",
    "File::create(",
    ".write(true)",
    ".append(true)",
    ".create(true)",
    "tempfile::",
    "NamedTempFile",
    "TempDir",
];

const PROCESS_SPAWN_PATTERN: &str = "process::Command";

/// Rule 1's nine write functions, by bare name - the identifiers
/// [`find_fs_write_fn_imports`] looks for inside a `use std::fs::...`
/// import.
const WRITE_FN_NAMES: &[&str] = &[
    "write",
    "create_dir_all",
    "create_dir",
    "remove_file",
    "remove_dir_all",
    "remove_dir",
    "rename",
    "copy",
    "set_permissions",
];

pub fn run(root: &Path) {
    let mut problems = Vec::new();

    for crate_name in ["forskscope-compare", "forskscope-report"] {
        let src = root.join("crates").join(crate_name).join("src");
        check_no_writes_or_spawns(&src, crate_name, &mut problems);
    }

    check_dependency_graph(root, &mut problems);

    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("{problem}");
        }
        eprintln!(
            "crate-boundaries check failed: {} problem(s) (RFC-093 §1/§3)",
            problems.len()
        );
        process::exit(1);
    }

    println!(
        "crate-boundaries check passed: forskscope-compare and forskscope-report write no file \
         and spawn no process outside #[cfg(test)]/tests/, and every workspace crate's direct \
         forskscope-* dependencies stay within RFC-093 §1's table."
    );
}

fn check_no_writes_or_spawns(src_root: &Path, crate_name: &str, problems: &mut Vec<String>) {
    for path in collect_production_rs_files(src_root) {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let stripped = strip_comments_and_strings(&raw);
        let stripped = blank_inline_cfg_test_blocks(&stripped);
        let rel = path
            .strip_prefix(src_root)
            .unwrap_or(&path)
            .display()
            .to_string();

        for pattern in WRITE_CALL_PATTERNS {
            if stripped.contains(pattern) {
                problems.push(format!(
                    "{crate_name}/src/{rel}: found `{pattern}` outside #[cfg(test)]/tests/ \
                     (rule 1: {crate_name} never writes a file)"
                ));
            }
        }
        for name in find_fs_write_fn_imports(&stripped) {
            problems.push(format!(
                "{crate_name}/src/{rel}: found `use std::fs::{name}` (or braced) outside \
                 #[cfg(test)]/tests/ - a bare `{name}(...)` call site would evade rule 1's \
                 qualified-call patterns (rule 1: {crate_name} never writes a file)"
            ));
        }
        if stripped.contains(PROCESS_SPAWN_PATTERN) {
            problems.push(format!(
                "{crate_name}/src/{rel}: found `{PROCESS_SPAWN_PATTERN}` (rule 2: {crate_name} \
                 never spawns a process)"
            ));
        }
    }
}

/// Finds `use std::fs::<name>` or `use std::fs::{.., <name>, ..}` imports
/// of any of [`WRITE_FN_NAMES`] in already comment/string/cfg-test-stripped
/// source — the bare-name import that would let a later unqualified
/// `<name>(...)` call evade `WRITE_CALL_PATTERNS`'s `fs::<name>(`-qualified
/// matches (review 181 §3). Returns each matched name once, even if
/// imported more than once.
fn find_fs_write_fn_imports(stripped: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = stripped[search_from..].find("fs::") {
        let after_prefix = search_from + rel + "fs::".len();
        // Only a `use` statement's path counts - look backward, within the
        // current statement (bounded by the previous `;` or `\n`), for a
        // line that starts with `use `.
        let scan_start = search_from.max(after_prefix.saturating_sub(200));
        let prefix = &stripped[scan_start..after_prefix - "fs::".len()];
        let is_use = prefix
            .rsplit(['\n', ';'])
            .next()
            .is_some_and(|segment| segment.trim_start().starts_with("use "));
        if is_use {
            let rest = &stripped[after_prefix..];
            if let Some(brace_rest) = rest.strip_prefix('{') {
                if let Some(close) = brace_rest.find('}') {
                    let list = &brace_rest[..close];
                    for &name in WRITE_FN_NAMES {
                        if contains_word(list, name) {
                            found.push(name);
                        }
                    }
                }
            } else {
                for &name in WRITE_FN_NAMES {
                    if let Some(tail) = rest.strip_prefix(name) {
                        let next_is_ident =
                            tail.as_bytes().first().is_some_and(|&b| is_ident_byte(b));
                        if !next_is_ident {
                            found.push(name);
                        }
                    }
                }
            }
        }
        search_from = after_prefix;
    }
    found.sort_unstable();
    found.dedup();
    found
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `true` if `word` appears in `haystack` with no identifier character
/// immediately before or after it - a whole-word match, not a substring
/// that merely contains `word` (so `write` does not match inside
/// `rewrite_count`).
fn contains_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut i = 0usize;
    while let Some(pos) = haystack[i..].find(word) {
        let start = i + pos;
        let end = start + word.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        i = start + 1;
    }
    false
}

/// Every `.rs` file under `src_root`, excluding a `tests` directory
/// component anywhere in the path and a file named exactly `tests.rs`
/// (mirrors `ui_logic_docs::leaf_modules`'s skip).
fn collect_production_rs_files(src_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_production_rs_files_into(src_root, &mut out);
    out
}

fn collect_production_rs_files_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                continue;
            }
            collect_production_rs_files_into(&path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if path.file_stem().and_then(|s| s.to_str()) == Some("tests") {
            continue;
        }
        out.push(path);
    }
}

/// Blanks every inline `#[cfg(test)] ...` item in already comment/string
/// -stripped source: a braced item (`mod`/`fn`/`impl`/…) is blanked from
/// the attribute through its block's matching `}`, counted the same way
/// `strip_comments_and_strings` counts nested block comments; a
/// brace-less item (a bare `use`/`const`/…) is blanked up to its `;`.
fn blank_inline_cfg_test_blocks(src: &str) -> String {
    const ATTR: &str = "#[cfg(test)]";
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    loop {
        let Some(attr_start) = rest.find(ATTR) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..attr_start]);
        let after_attr = &rest[attr_start + ATTR.len()..];
        let brace_pos = after_attr.find('{');
        let semi_pos = after_attr.find(';');
        let end = match (brace_pos, semi_pos) {
            (Some(b), Some(s)) if s < b => semi_pos.map(|p| p + 1),
            (Some(_), _) => {
                let bytes = after_attr.as_bytes();
                let open = brace_pos.unwrap();
                let mut depth: i64 = 0;
                let mut i = open;
                let mut close = None;
                while i < bytes.len() {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                close = Some(i + 1);
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                close
            }
            (None, Some(s)) => Some(s + 1),
            (None, None) => None,
        };
        let Some(end) = end else {
            // No terminator found (malformed or EOF right after the
            // attribute) - blank just the attribute itself and move on;
            // whatever follows gets scanned normally.
            out.push(' ');
            rest = after_attr;
            continue;
        };
        let blanked_len = ATTR.len() + end;
        out.push_str(&" ".repeat(blanked_len));
        rest = &after_attr[end..];
    }
    out
}

fn check_dependency_graph(root: &Path, problems: &mut Vec<String>) {
    let expected = expected_dependency_table();
    for (&crate_name, &allowed) in &expected {
        let output = Command::new("cargo")
            .args([
                "tree", "-p", crate_name, "-e", "normal", "--depth", "1", "--prefix", "none",
            ])
            .current_dir(root)
            .output()
            .unwrap_or_else(|e| panic!("failed to run cargo tree -p {crate_name}: {e}"));
        if !output.status.success() {
            problems.push(format!(
                "cargo tree -p {crate_name} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let actual: Vec<&str> = stdout
            .lines()
            .skip(1) // the crate itself
            .filter_map(|line| line.split_whitespace().next())
            .filter(|name| name.starts_with("forskscope-"))
            .collect();

        for dep in &actual {
            if !allowed.contains(dep) {
                problems.push(format!(
                    "{crate_name} depends on {dep}, which RFC-093 §1's table does not allow \
                     (allowed: {})",
                    if allowed.is_empty() {
                        "none".to_string()
                    } else {
                        allowed.join(", ")
                    }
                ));
            }
        }
        // Deliberately one-directional: the table is a ceiling on what a
        // crate may depend on, not a floor it must exercise. A table entry
        // this crate does not currently use (forskscope-ui-logic and
        // forskscope-report, today) is not a boundary violation - nothing
        // crossed a line that should have stayed closed - so it is not
        // checked for here.
    }

    // An unknown workspace crate — one not named in RFC-093 §1's table at
    // all — also fails: read every `crates/*/Cargo.toml` package name and
    // compare against the table's keys, so a new or renamed crate added to
    // the workspace without a matching table entry is caught here rather
    // than silently skipped.
    let crates_dir = root.join("crates");
    let Ok(entries) = fs::read_dir(&crates_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest = path.join("Cargo.toml");
        let Ok(manifest_text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let Some(name) = package_name(&manifest_text) else {
            continue;
        };
        if !expected.contains_key(name.as_str()) {
            problems.push(format!(
                "crates/{} is a workspace crate RFC-093 §1's table does not name at all",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ));
        }
    }
}

/// Reads `name = "..."` from the `[package]` section of a `Cargo.toml`'s
/// raw text — minimal, matching the rest of this file's own TOML reads
/// (`extract_workspace_value`), not a full TOML parse.
fn package_name(manifest_text: &str) -> Option<String> {
    for line in manifest_text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('"')
                    && let Some(end) = rest.find('"')
                {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_inline_cfg_test_blocks_removes_a_braced_mod() {
        let src = "fn a() {}\n#[cfg(test)]\nmod tests {\n    fn inner() { nested() }\n}\nfn b() {}";
        let blanked = blank_inline_cfg_test_blocks(src);
        assert!(!blanked.contains("nested"));
        assert!(blanked.contains("fn a()"));
        assert!(blanked.contains("fn b()"));
    }

    #[test]
    fn blank_inline_cfg_test_blocks_handles_nested_braces() {
        let src = "#[cfg(test)]\nmod tests {\n    fn f() {\n        if true { do_write() }\n    }\n}\nfn after() {}";
        let blanked = blank_inline_cfg_test_blocks(src);
        assert!(!blanked.contains("do_write"));
        assert!(blanked.contains("fn after()"));
    }

    #[test]
    fn find_fs_write_fn_imports_catches_the_bare_name_evasion() {
        // review 181 §3's own falsification: a qualified-call pattern like
        // `fs::write(` never appears here at all.
        let src =
            "use std::fs::write;\nfn f(p: &std::path::Path, b: &[u8]) { write(p, b).unwrap(); }";
        let stripped = strip_comments_and_strings(src);
        let found = find_fs_write_fn_imports(&stripped);
        assert_eq!(found, vec!["write"]);
    }

    #[test]
    fn find_fs_write_fn_imports_catches_a_braced_multi_import() {
        let src = "use std::fs::{create_dir_all, read_to_string, write};";
        let stripped = strip_comments_and_strings(src);
        let mut found = find_fs_write_fn_imports(&stripped);
        found.sort_unstable();
        assert_eq!(found, vec!["create_dir_all", "write"]);
    }

    #[test]
    fn find_fs_write_fn_imports_ignores_a_qualified_call_site() {
        // `fs::write(` with no preceding `use` - already caught by
        // WRITE_CALL_PATTERNS, and must not also double-report here.
        let src = "fn f() { std::fs::write(\"x\", []).unwrap(); }";
        let stripped = strip_comments_and_strings(src);
        assert!(find_fs_write_fn_imports(&stripped).is_empty());
    }

    #[test]
    fn find_fs_write_fn_imports_ignores_an_unrelated_read_import() {
        let src = "use std::fs::read_to_string;";
        let stripped = strip_comments_and_strings(src);
        assert!(find_fs_write_fn_imports(&stripped).is_empty());
    }

    #[test]
    fn find_fs_write_fn_imports_does_not_match_a_longer_identifier_sharing_the_prefix() {
        // `write_metrics` starts with "write" but is a different, unrelated
        // identifier - not a boundary match for the single-import form.
        let src = "use std::fs::write_metrics;";
        let stripped = strip_comments_and_strings(src);
        assert!(find_fs_write_fn_imports(&stripped).is_empty());
    }

    #[test]
    fn find_fs_write_fn_imports_braced_form_does_not_match_a_longer_identifier() {
        let src = "use std::fs::{read_to_string, write_metrics};";
        let stripped = strip_comments_and_strings(src);
        assert!(find_fs_write_fn_imports(&stripped).is_empty());
    }

    #[test]
    fn blank_inline_cfg_test_blocks_handles_a_brace_less_item() {
        let src = "#[cfg(test)]\nuse std::fs;\nfn after() { real_code() }";
        let blanked = blank_inline_cfg_test_blocks(src);
        assert!(!blanked.contains("use std::fs"));
        assert!(blanked.contains("real_code"));
    }

    #[test]
    fn collect_production_rs_files_excludes_tests_directory_and_tests_rs() {
        let dir = std::env::temp_dir().join(format!(
            "fsk-xtask-crate-boundaries-{}-{}",
            std::process::id(),
            line!()
        ));
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::write(dir.join("lib.rs"), "fn a() {}").unwrap();
        fs::write(dir.join("tests.rs"), "mod foo_tests;").unwrap();
        fs::write(dir.join("tests").join("foo_tests.rs"), "fn t() {}").unwrap();
        fs::write(dir.join("real.rs"), "fn b() {}").unwrap();

        let files = collect_production_rs_files(&dir);
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();
        assert!(names.contains(&"lib.rs"));
        assert!(names.contains(&"real.rs"));
        assert!(!names.contains(&"tests.rs"));
        assert!(
            files
                .iter()
                .all(|p| !p.to_string_lossy().contains("tests/"))
        );

        fs::remove_dir_all(&dir).ok();
    }
}
