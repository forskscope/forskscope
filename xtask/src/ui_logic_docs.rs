//! F93: `cargo xtask ui-logic-docs` — verifies that
//! `docs/src/maintainers/architecture.md`'s `` `ui-logic` modules (N) ``
//! table and `docs/src/maintainers/testing.md`'s
//! `` `forskscope-ui-logic` test modules `` table both name exactly the
//! `ui-logic` leaf modules that exist on disk (handoff 034 §C).
//!
//! F93 was fixed by hand twice (review 106 §4) and each fix missed a
//! document — once because nobody re-checked `testing.md` after
//! `architecture.md` was corrected, once because this handoff's own
//! `ui-logic` cleanup (handoff 033) deleted two modules and nobody updated
//! either table. A check that runs on every push is the only fix that
//! survives a third repetition.
//!
//! ## What counts as a leaf module
//!
//! Every `.rs` file under `crates/forskscope-ui-logic/src`, except:
//! - `lib.rs` (the crate root, not a module);
//! - a "parent module file" — `foo.rs` sitting beside a `foo/` directory
//!   (`compare.rs`, `explore.rs`, `session.rs`, `settings.rs` today) is
//!   the `mod` declaration for the directory's contents, not a module of
//!   its own;
//! - a file named exactly `tests.rs` (an external `#[cfg(test)] mod
//!   tests;` file, e.g. `compare/load_identity/tests.rs`) — a test
//!   module, not a leaf module.
//!
//! A leaf module's name for comparison purposes is its path relative to
//! `src/`, without the `.rs` extension: `compare/load_guard`,
//! `settings/settings_view`. `architecture.md`'s table spells these with
//! `::` (`compare::load_guard`); `testing.md`'s spells them with `/`
//! (`compare/load_guard`) — both are normalized to the `/` form before
//! comparing against disk.
//!
//! ## What is and isn't checked
//!
//! Only *which* modules are listed, and (`architecture.md` only) whether
//! the heading's `(N)` matches how many there are. Row *content* — what a
//! row says a module covers — is not parsed or checked; review 106 §4
//! was explicit that wording stays a human judgment. Every mismatch is
//! reported in one run, in both directions (missing from a doc, and
//! present in a doc but absent on disk) — not just the first found.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const ARCHITECTURE_HEADING_PREFIX: &str = "## `ui-logic` modules (";
const TESTING_HEADING: &str = "## `forskscope-ui-logic` test modules";

pub fn run(root: &Path) {
    let disk_set: BTreeSet<String> = leaf_modules(&root.join("crates/forskscope-ui-logic/src"))
        .into_iter()
        .collect();

    let arch_path = root.join("docs/src/maintainers/architecture.md");
    let arch_src = fs::read_to_string(&arch_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", arch_path.display()));
    let (arch_heading, arch_section) =
        heading_line_and_rest(&arch_src, ARCHITECTURE_HEADING_PREFIX).unwrap_or_else(|| {
            panic!(
                "{}: could not find a `{ARCHITECTURE_HEADING_PREFIX}N)` heading",
                arch_path.display()
            )
        });
    let arch_count = parse_paren_count(arch_heading).unwrap_or_else(|| {
        panic!(
            "{}: heading `{arch_heading}` has no parseable (N) count",
            arch_path.display()
        )
    });
    let arch_rows: BTreeSet<String> = table_rows_until_next_heading(arch_section)
        .into_iter()
        .map(|s| s.replace("::", "/"))
        .collect();

    let testing_path = root.join("docs/src/maintainers/testing.md");
    let testing_src = fs::read_to_string(&testing_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", testing_path.display()));
    let (_testing_heading, testing_section) = heading_line_and_rest(&testing_src, TESTING_HEADING)
        .unwrap_or_else(|| {
            panic!(
                "{}: could not find `{TESTING_HEADING}`",
                testing_path.display()
            )
        });
    let testing_rows: BTreeSet<String> = table_rows_until_next_heading(testing_section)
        .into_iter()
        .collect();

    let mut problems = Vec::new();

    if arch_count != disk_set.len() {
        problems.push(format!(
            "architecture.md's `{ARCHITECTURE_HEADING_PREFIX}{arch_count})` heading says {arch_count}, but there are {} ui-logic leaf modules on disk",
            disk_set.len()
        ));
    }
    report_diff(&mut problems, "architecture.md", &arch_rows, &disk_set);
    report_diff(&mut problems, "testing.md", &testing_rows, &disk_set);

    if !problems.is_empty() {
        eprintln!("ui-logic-docs check failed:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        process::exit(1);
    }

    println!(
        "ui-logic-docs check passed: architecture.md and testing.md both list exactly the {} ui-logic modules on disk.",
        disk_set.len()
    );
}

fn report_diff(
    problems: &mut Vec<String>,
    doc: &str,
    doc_set: &BTreeSet<String>,
    disk_set: &BTreeSet<String>,
) {
    for missing in disk_set.difference(doc_set) {
        problems.push(format!("{doc} is missing module `{missing}`"));
    }
    for extra in doc_set.difference(disk_set) {
        problems.push(format!(
            "{doc} lists `{extra}`, which does not exist on disk"
        ));
    }
}

/// `true` if `dir` contains at least one `.rs` file that is not itself an
/// external test-module file (i.e. not named `tests.rs`) - a real
/// submodule, distinguishing a genuine `mod`-declaration directory from a
/// leaf module's own `mod tests;` sibling directory.
fn dir_has_non_test_rs_file(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_stem().and_then(|s| s.to_str()) != Some("tests")
    })
}

fn leaf_modules(src_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect_leaf_modules(src_root, src_root, &mut out);
    out
}

fn collect_leaf_modules(src_root: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_leaf_modules(src_root, &path, out);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if path == src_root.join("lib.rs") {
            continue;
        }
        if path.file_stem().and_then(|s| s.to_str()) == Some("tests") {
            continue;
        }
        // Parent module file: a same-named sibling directory exists *and*
        // holds a real submodule of its own - not just an external test
        // file. `compare/load_identity.rs` has a sibling
        // `compare/load_identity/` directory that holds only `tests.rs`;
        // that makes `load_identity.rs` a leaf module with an external
        // test file, not a `mod`-declaration file for a submodule tree.
        let sibling_dir: PathBuf = path.with_extension("");
        if sibling_dir.is_dir() && dir_has_non_test_rs_file(&sibling_dir) {
            continue;
        }
        let rel = path.strip_prefix(src_root).expect("path under src_root");
        let rel_no_ext = rel.with_extension("");
        out.push(rel_no_ext.to_string_lossy().replace('\\', "/"));
    }
}

/// Returns `(heading_line, everything_after_it)` for the first line
/// containing `heading_prefix`, or `None` if not found.
fn heading_line_and_rest<'a>(src: &'a str, heading_prefix: &str) -> Option<(&'a str, &'a str)> {
    let idx = src.find(heading_prefix)?;
    let line_start = src[..idx].rfind('\n').map(|o| o + 1).unwrap_or(0);
    let line_end = src[idx..].find('\n').map(|o| idx + o).unwrap_or(src.len());
    Some((&src[line_start..line_end], &src[line_end..]))
}

fn parse_paren_count(heading_line: &str) -> Option<usize> {
    let open = heading_line.rfind('(')?;
    let close = heading_line[open..].find(')')? + open;
    heading_line[open + 1..close].trim().parse().ok()
}

/// Collects the first backtick-quoted cell of every markdown table row in
/// `section`, stopping at the next `## ` heading (or end of `section`).
/// Header and separator rows are skipped naturally - neither starts with
/// a backtick immediately after the leading `|`.
fn table_rows_until_next_heading(section: &str) -> Vec<String> {
    let mut rows = Vec::new();
    for line in section.lines() {
        if line.starts_with("## ") {
            break;
        }
        if !line.starts_with('|') {
            continue;
        }
        if let Some(name) = first_backtick_cell(line) {
            rows.push(name);
        }
    }
    rows
}

fn first_backtick_cell(line: &str) -> Option<String> {
    let after_pipe = line.trim_start_matches('|').trim_start();
    let rest = after_pipe.strip_prefix('`')?;
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_count() {
        assert_eq!(parse_paren_count("## `ui-logic` modules (13)"), Some(13));
        assert_eq!(parse_paren_count("## `ui-logic` modules (11)"), Some(11));
    }

    #[test]
    fn extracts_table_rows_and_stops_at_next_heading() {
        let section = "\n\
| Module | Purpose |\n\
|---|---|\n\
| `explore::align` | does a thing |\n\
| `compare::save_error` | does another |\n\
\n\
## Next section\n\
| `not::a::module` | must not be counted |\n";
        assert_eq!(
            table_rows_until_next_heading(section),
            vec!["explore::align", "compare::save_error"]
        );
    }

    #[test]
    fn skips_non_table_prose_lines() {
        let section = "\nAll tests are inline. Integration tests live in `tests/css_coverage.rs`.\n\n| `explore/align` | covers this |\n";
        assert_eq!(
            table_rows_until_next_heading(section),
            vec!["explore/align"]
        );
    }

    #[test]
    fn heading_line_and_rest_finds_the_right_line() {
        let src = "intro\n## `ui-logic` modules (13)\n\nbody\n";
        let (heading, rest) = heading_line_and_rest(src, "## `ui-logic` modules (").unwrap();
        assert_eq!(heading, "## `ui-logic` modules (13)");
        assert_eq!(rest, "\n\nbody\n");
    }
}
