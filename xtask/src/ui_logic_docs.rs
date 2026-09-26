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
//!
//! **A row the check cannot read a module name from is reported, never
//! skipped (F107).** Every table body row must start with a backticked
//! module name. A stale row written in a slightly different style —
//! `| compare/ghost_module | ... |`, no backticks — used to be dropped
//! silently and the check passed on it; now it is a problem, naming the
//! document and the row's text. There is no allowlist.
//!
//! ## Which table rows are body rows
//!
//! A table line is a line starting with `|`. Of those, exactly two kinds are
//! not body rows, and both are recognised by *structure*, not by wording:
//!
//! - the **separator** row (`|---|---|`, `|:--|--:|`): every cell is only
//!   `-`, `:` and spaces;
//! - the **header** row: the row immediately *before* a separator row, which
//!   is what a markdown table header is by definition. Recognising it this
//!   way (rather than by the words `Module`/`File`) means a differently
//!   worded header is still skipped, and a body row is never mistaken for one.
//!
//! Everything else is a body row.

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
    let arch_table = table_rows_until_next_heading(arch_section);
    let arch_rows: BTreeSet<String> = arch_table
        .modules
        .iter()
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
    let testing_table = table_rows_until_next_heading(testing_section);
    let testing_rows: BTreeSet<String> = testing_table.modules.iter().cloned().collect();

    let mut problems = Vec::new();

    if arch_count != disk_set.len() {
        problems.push(format!(
            "architecture.md's `{ARCHITECTURE_HEADING_PREFIX}{arch_count})` heading says {arch_count}, but there are {} ui-logic leaf modules on disk",
            disk_set.len()
        ));
    }
    report_diff(&mut problems, "architecture.md", &arch_rows, &disk_set);
    report_diff(&mut problems, "testing.md", &testing_rows, &disk_set);
    report_unreadable(&mut problems, "architecture.md", &arch_table);
    report_unreadable(&mut problems, "testing.md", &testing_table);

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

/// F107: a row the check cannot read a module name from is a problem, named,
/// not silently dropped.
fn report_unreadable(problems: &mut Vec<String>, doc: &str, table: &TableRows) {
    for row in &table.unreadable {
        problems.push(format!(
            "{doc} has a table row with no backticked module name: `{row}`"
        ));
    }
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

/// What was read from a section's tables: the module names, and every body
/// row a name could not be read from (F107).
#[derive(Debug, Default, PartialEq, Eq)]
struct TableRows {
    modules: Vec<String>,
    /// The text of each body row whose first cell has no backticked name.
    unreadable: Vec<String>,
}

/// `true` for a markdown table separator row: every cell is only `-`, `:` and
/// spaces (`|---|---|`, `| :--- | ---: |`), and there is at least one cell.
fn is_separator_row(line: &str) -> bool {
    let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| !c.trim().is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

/// Reads the markdown table rows of `section`, stopping at the next `## `
/// heading (or the end of `section`). The separator row and the header row
/// (the row immediately before a separator) are not body rows; every body
/// row must yield a backticked module name or is reported in `unreadable`.
fn table_rows_until_next_heading(section: &str) -> TableRows {
    let mut out = TableRows::default();
    let mut body = |line: &str| match first_backtick_cell(line) {
        Some(name) => out.modules.push(name),
        None => out.unreadable.push(line.trim().to_string()),
    };
    // The last table line seen, not yet known to be a header or a body row.
    let mut pending: Option<&str> = None;
    for line in section.lines() {
        if line.starts_with("## ") {
            break;
        }
        if !line.starts_with('|') {
            if let Some(p) = pending.take() {
                body(p);
            }
            continue;
        }
        if is_separator_row(line) {
            pending = None; // the row before a separator is the header
            continue;
        }
        if let Some(previous) = pending.replace(line) {
            body(previous);
        }
    }
    if let Some(p) = pending {
        body(p);
    }
    out
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
        let rows = table_rows_until_next_heading(section);
        assert_eq!(rows.modules, vec!["explore::align", "compare::save_error"]);
        assert!(rows.unreadable.is_empty());
    }

    #[test]
    fn skips_non_table_prose_lines() {
        let section = "\nAll tests are inline. Integration tests live in `tests/css_coverage.rs`.\n\n| `explore/align` | covers this |\n";
        assert_eq!(
            table_rows_until_next_heading(section).modules,
            vec!["explore/align"]
        );
    }

    /// F107's unit tests for the row-classification rule: a header row, a
    /// separator row, a normal row and an unbackticked row.
    #[test]
    fn a_header_and_a_separator_are_not_body_rows() {
        // Both document shapes: architecture.md's and testing.md's headers.
        for header in ["| Module | Purpose |", "| File | Covers | RFC |"] {
            let section = format!("\n{header}\n|---|---|---|\n| `a/b` | c | d |\n");
            let rows = table_rows_until_next_heading(&section);
            assert_eq!(rows.modules, vec!["a/b"], "{header}");
            assert!(
                rows.unreadable.is_empty(),
                "{header}: {:?}",
                rows.unreadable
            );
        }
    }

    /// The header is recognised by structure — the row before the separator —
    /// so a header worded differently is still skipped, and it is *only* that
    /// row: a body row is never mistaken for one.
    #[test]
    fn the_header_is_the_row_before_the_separator_whatever_it_says() {
        let section = "\n| Anything at all | here |\n| :--- | ---: |\n| `a` | x |\n| `b` | y |\n";
        let rows = table_rows_until_next_heading(section);
        assert_eq!(rows.modules, vec!["a", "b"]);
        assert!(rows.unreadable.is_empty());
    }

    #[test]
    fn a_normal_row_yields_its_backticked_name() {
        let rows = table_rows_until_next_heading("| `explore/align` | covers this |\n");
        assert_eq!(rows.modules, vec!["explore/align"]);
        assert!(rows.unreadable.is_empty());
    }

    /// The F107 probe: a stale row with no backticks is reported, not dropped —
    /// wherever it sits in the table, including as the last row and without a
    /// header above it.
    #[test]
    fn an_unbackticked_row_is_reported_not_skipped() {
        let section = "\n| File | Covers |\n|---|---|\n| `a/b` | ok |\n| compare/ghost_module | unbackticked stale row | RFC-000 |\n| `c/d` | ok |\n| trailing/ghost | last row |\n";
        let rows = table_rows_until_next_heading(section);
        assert_eq!(rows.modules, vec!["a/b", "c/d"]);
        assert_eq!(
            rows.unreadable,
            vec![
                "| compare/ghost_module | unbackticked stale row | RFC-000 |",
                "| trailing/ghost | last row |",
            ]
        );
    }

    #[test]
    fn a_separator_row_is_recognised_by_its_cells_only() {
        for sep in ["|---|---|", "| --- | --- |", "|:--|--:|", "| :---: |"] {
            assert!(is_separator_row(sep), "{sep}");
        }
        for not in ["| a | b |", "| `x` | --- |", "|  |  |", "| - a | b |"] {
            assert!(!is_separator_row(not), "{not}");
        }
    }

    #[test]
    fn heading_line_and_rest_finds_the_right_line() {
        let src = "intro\n## `ui-logic` modules (13)\n\nbody\n";
        let (heading, rest) = heading_line_and_rest(src, "## `ui-logic` modules (").unwrap();
        assert_eq!(heading, "## `ui-logic` modules (13)");
        assert_eq!(rest, "\n\nbody\n");
    }
}
