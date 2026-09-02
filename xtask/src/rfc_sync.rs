//! F83: `cargo xtask rfc-sync` — verifies that `ROADMAP.md`'s "Remaining
//! proposed RFCs" table and the *unshipped* RFC folders name exactly the
//! same RFCs (handoff 009).
//!
//! **5-folder variant, adopted 2026-09-02.** RFC-000 offers a fifth folder,
//! `accepted/` ("review complete; implementer may start"), between
//! `proposed/` and `done/`. The owner adopted it, so "unshipped" is now
//! `proposed/` ∪ `accepted/` — both are work the register must still list,
//! and neither has shipped. Only `done/` and `archive/` remove a row.
//!
//! RFC-000 (`.git-exclude/rules/000-rfc-lifecycle-policy.md`) makes the
//! folder the source of truth for an RFC's lifecycle state. This check
//! enforces that the table agrees with the folder, not the reverse: if
//! they disagree, the folder is right and the table is wrong.
//!
//! Four conditions are checked, and every violation found is reported
//! (not just the first — a maintainer who moved three RFCs wants all
//! three named in one run):
//!
//! 1. An RFC file exists in `rfcs/proposed/` or `rfcs/accepted/` with no
//!    row in the table.
//! 2. A table row names an RFC that is in `rfcs/done/` or `rfcs/archive/`.
//! 3. A table row names an RFC that exists nowhere under `rfcs/`.
//! 4. A file in `rfcs/proposed/` or `rfcs/accepted/` has no
//!    `**Scheduling.**` line.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process;

const TABLE_HEADING: &str = "## Remaining proposed RFCs";
/// The folders holding RFCs that have not shipped. Both are listed in the
/// register's table; `done/` and `archive/` are not.
const UNSHIPPED_FOLDERS: [&str; 2] = ["proposed", "accepted"];
const SCHEDULING_MARKER: &str = "**Scheduling.**";

pub fn run(root: &Path) {
    let roadmap_path = root.join("ROADMAP.md");
    let roadmap = fs::read_to_string(&roadmap_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", roadmap_path.display()));

    let table_numbers = match table_section(&roadmap).map(parse_table_numbers) {
        Ok(numbers) => numbers,
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    };

    // Unshipped = proposed/ + accepted/ (5-folder variant). `accepted/` is
    // optional: a project mid-migration may not have created it yet, so a
    // missing directory reads as empty rather than failing the gate.
    let mut unshipped_numbers = BTreeSet::new();
    let mut unshipped_files: Vec<(String, String, &'static str)> = Vec::new();
    for folder in UNSHIPPED_FOLDERS {
        let dir = root.join("rfcs").join(folder);
        if !dir.is_dir() {
            continue;
        }
        unshipped_numbers.extend(list_numbered_md_files(&dir));
        let entries = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"));
        for path in entries {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            unshipped_files.push((name, content, folder));
        }
    }

    let mut violations = diff_violations(&table_numbers, &unshipped_numbers, |number| {
        locate_outside_unshipped(root, number)
    });

    let for_scheduling: Vec<(String, String)> = unshipped_files
        .iter()
        .map(|(name, content, folder)| (format!("{folder}/{name}"), content.clone()))
        .collect();
    violations.extend(
        missing_scheduling_line(&for_scheduling)
            .into_iter()
            .map(|name| format!("rfcs/{name} has no \"{SCHEDULING_MARKER}\" line")),
    );

    if !violations.is_empty() {
        eprintln!("RFC schedule sync check failed:");
        for violation in &violations {
            eprintln!("  - {violation}");
        }
        process::exit(1);
    }

    println!(
        "RFC schedule sync check passed: {} RFCs agree between ROADMAP.md and \
         rfcs/proposed/ + rfcs/accepted/.",
        table_numbers.len()
    );
}

// ── Table parsing ────────────────────────────────────────────────────────────

/// Slices `roadmap` to the lines between the `## Remaining proposed RFCs`
/// heading (exclusive) and the next `## ` heading (exclusive), or end of
/// file. Fails loudly if the heading is absent - §7c's guard against the
/// vacuous-green failure this project has repeatedly caught elsewhere: a
/// renamed or deleted heading must not silently parse as "zero rows,
/// check passes."
fn table_section(roadmap: &str) -> Result<Vec<&str>, String> {
    let lines: Vec<&str> = roadmap.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim() == TABLE_HEADING)
        .ok_or_else(|| {
            format!(
                "ROADMAP.md has no \"{TABLE_HEADING}\" heading - the RFC sync check has nothing to \
             anchor its table parse to. A renamed or removed heading must fail this check, not \
             silently pass with zero rows found (F83 §7c)."
            )
        })?;

    let mut section = Vec::new();
    for line in &lines[start + 1..] {
        if line.trim_start().starts_with("## ") {
            break;
        }
        section.push(*line);
    }
    Ok(section)
}

/// Extracts every RFC number appearing as a table row's first cell within
/// `section`. Anchored to lines that actually start with `|` and whose
/// first cell is all-digit, so this does not match `ROADMAP.md`'s other
/// pipe tables (the findings register, the milestone table) - those live
/// outside this section entirely - nor the header row (`RFC`) or the
/// separator row (`----`), neither of which is all-digit.
fn parse_table_numbers(section: Vec<&str>) -> BTreeSet<String> {
    section
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('|') {
                return None;
            }
            let first_cell = trimmed.trim_start_matches('|').split('|').next()?.trim();
            (!first_cell.is_empty() && first_cell.bytes().all(|b| b.is_ascii_digit()))
                .then(|| first_cell.to_string())
        })
        .collect()
}

// ── Filesystem ────────────────────────────────────────────────────────────────

/// The leading digit run of `dir`'s `*.md` filenames (`NNN-slug.md` →
/// `NNN`), as literal text - no numeric parsing, so this makes no
/// assumption about digit count (RFC-000: numbers are permanent; §13:
/// three digits today, not guaranteed forever).
fn list_numbered_md_files(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .filter_map(|p| leading_digits(&p.file_name()?.to_string_lossy()))
        .collect()
}

fn leading_digits(filename: &str) -> Option<String> {
    let digits: String = filename.chars().take_while(char::is_ascii_digit).collect();
    (!digits.is_empty()).then_some(digits)
}

/// Where `number` actually lives, if not in an unshipped folder -
/// `rfcs/done/` or `rfcs/archive/` (the `020`/`077` shape), or `None` if it
/// exists nowhere under `rfcs/`.
fn locate_outside_unshipped(root: &Path, number: &str) -> Option<&'static str> {
    for (label, rel) in [("done", "rfcs/done"), ("archive", "rfcs/archive")] {
        let dir = root.join(rel);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let found = entries.filter_map(|e| e.ok()).any(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            leading_digits(&name).as_deref() == Some(number)
        });
        if found {
            return Some(label);
        }
    }
    None
}

// ── Pure violation logic (fixture-testable) ─────────────────────────────────

/// §7b.1-3: every RFC present in exactly one of `table_numbers` /
/// `unshipped_numbers` is a violation - which one determines the message.
/// `locate` answers "if not unshipped, where (if anywhere) does this
/// number actually live?" - injected so this stays a pure function over
/// two sets and a lookup, testable with fixtures rather than real
/// directories.
fn diff_violations(
    table_numbers: &BTreeSet<String>,
    unshipped_numbers: &BTreeSet<String>,
    locate: impl Fn(&str) -> Option<&'static str>,
) -> Vec<String> {
    let mut violations = Vec::new();

    for number in unshipped_numbers.difference(table_numbers) {
        violations.push(format!(
            "RFC {number} exists in rfcs/proposed/ or rfcs/accepted/ but has no row in \
             ROADMAP.md's \"{TABLE_HEADING}\" table"
        ));
    }

    for number in table_numbers.difference(unshipped_numbers) {
        match locate(number) {
            Some(label) => violations.push(format!(
                "RFC {number} has a row in ROADMAP.md's \"{TABLE_HEADING}\" table but is in \
                 rfcs/{label}/, not rfcs/proposed/ or rfcs/accepted/"
            )),
            None => violations.push(format!(
                "RFC {number} has a row in ROADMAP.md's \"{TABLE_HEADING}\" table but does not \
                 exist anywhere under rfcs/"
            )),
        }
    }

    violations
}

/// §7b.4: which of `files` (filename, content) lack the `**Scheduling.**`
/// marker.
fn missing_scheduling_line(files: &[(String, String)]) -> Vec<String> {
    files
        .iter()
        .filter(|(_, content)| !content.contains(SCHEDULING_MARKER))
        .map(|(name, _)| name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(numbers: &[&str]) -> BTreeSet<String> {
        numbers.iter().map(|s| s.to_string()).collect()
    }

    // ── table_section / parse_table_numbers ─────────────────────────────────

    #[test]
    fn extracts_numbers_from_the_remaining_proposed_rfcs_table_only() {
        let roadmap = "\
# ROADMAP

## Findings register

| F83 | some finding | owner | status |
|-----|---|---|---|

## Remaining proposed RFCs

Some intro prose.

| RFC | When | What |
|-----|------|------|
| 004 | Slice 8 | Editor adapter |
| 079 | Post-Gate-D | Store submission |

## Non-goals

| 999 | this must not be picked up | irrelevant |
";
        let section = table_section(roadmap).unwrap();
        let numbers = parse_table_numbers(section);
        assert_eq!(numbers, set(&["004", "079"]));
    }

    #[test]
    fn missing_heading_fails_rather_than_returning_zero_rows() {
        let roadmap = "\
# ROADMAP

## Some other heading

| 004 | Slice 8 | Editor adapter |
";
        let result = table_section(roadmap);
        assert!(
            result.is_err(),
            "a missing heading must fail, not silently parse as an empty table"
        );
    }

    #[test]
    fn heading_at_end_of_file_yields_an_empty_but_valid_section() {
        let roadmap = "\
# ROADMAP

## Remaining proposed RFCs
";
        let section = table_section(roadmap).unwrap();
        assert!(parse_table_numbers(section).is_empty());
    }

    #[test]
    fn separator_and_header_rows_are_not_mistaken_for_rfc_numbers() {
        let roadmap = "\
## Remaining proposed RFCs

| RFC | When | What |
|-----|------|------|
| 004 | Slice 8 | Editor adapter |

## Next
";
        let numbers = parse_table_numbers(table_section(roadmap).unwrap());
        assert_eq!(numbers, set(&["004"]));
    }

    // ── diff_violations ──────────────────────────────────────────────────────

    #[test]
    fn a_proposed_file_with_no_table_row_is_reported() {
        let table = set(&["004"]);
        let proposed = set(&["004", "099"]);
        let violations = diff_violations(&table, &proposed, |_| None);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("099"));
        assert!(violations[0].contains("no row"));
    }

    #[test]
    fn a_table_row_for_an_rfc_in_done_is_reported_as_such() {
        let table = set(&["077"]);
        let proposed = set(&[]);
        let violations = diff_violations(&table, &proposed, |n| (n == "077").then_some("done"));
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("077"));
        assert!(violations[0].contains("rfcs/done/"));
    }

    #[test]
    fn a_table_row_for_a_number_that_exists_nowhere_is_reported_as_such() {
        let table = set(&["999"]);
        let proposed = set(&[]);
        let violations = diff_violations(&table, &proposed, |_| None);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("999"));
        assert!(violations[0].contains("does not exist anywhere"));
    }

    #[test]
    fn a_matched_pair_produces_no_violation() {
        let table = set(&["004", "079"]);
        let proposed = set(&["004", "079"]);
        let violations = diff_violations(&table, &proposed, |_| None);
        assert!(violations.is_empty());
    }

    #[test]
    fn every_violation_is_reported_not_just_the_first() {
        let table = set(&["004", "077", "999"]);
        let proposed = set(&["004", "099"]);
        let violations = diff_violations(&table, &proposed, |n| (n == "077").then_some("done"));
        // 099: in proposed/, no row. 077: row, but in done/. 999: row, nowhere.
        assert_eq!(violations.len(), 3);
    }

    // ── missing_scheduling_line ──────────────────────────────────────────────

    #[test]
    fn a_file_without_the_scheduling_marker_is_reported() {
        let files = vec![
            (
                "004-a.md".to_string(),
                "**Scheduling.** Slice 8.".to_string(),
            ),
            ("099-b.md".to_string(), "no marker here".to_string()),
        ];
        let missing = missing_scheduling_line(&files);
        assert_eq!(missing, vec!["099-b.md".to_string()]);
    }

    #[test]
    fn every_file_present_produces_no_violation() {
        let files = vec![(
            "004-a.md".to_string(),
            "**Scheduling.** Slice 8.".to_string(),
        )];
        assert!(missing_scheduling_line(&files).is_empty());
    }
}
