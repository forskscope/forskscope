//! F134: generates the `.xlsx` test fixtures under
//! `crates/forskscope-core/src/tests/fixtures/xlsx/<case>/{old,new}.xlsx`,
//! making checkable the claim their own README used to make uncheckable -
//! that they "were written with `rust_xlsxwriter` 0.99.0 from a throwaway
//! program, with a fixed creation timestamp so regenerating gives identical
//! files", with no such program tracked anywhere.
//!
//! Every case's exact shape (sheet names, cell addresses and values,
//! formulas) is derived directly from `forskscope-core/src/xlsx.rs`'s own
//! test assertions against these fixtures - the ground truth for what each
//! one must contain already existed there; this just makes it reproducible
//! rather than committed-and-unexplained.
//!
//! Run with `cargo xtask xlsx-fixtures` to regenerate, or `--check` to
//! verify the committed files are exactly what this produces today (exits
//! non-zero, naming the first mismatched file, otherwise).

use std::path::Path;
use std::process;

use rust_xlsxwriter::{Chart, ChartType, DocProperties, ExcelDateTime, Formula, Workbook};

type CaseGenerator = fn(&Path) -> Result<(), rust_xlsxwriter::XlsxError>;

/// 2026-09-26: the same fixed date the README already documented for the
/// F129/F131 cases, now applied to every case this generates, so the whole
/// directory regenerates identically on every run, not case by case.
fn fixed_properties() -> Result<DocProperties, rust_xlsxwriter::XlsxError> {
    let date = ExcelDateTime::from_ymd(2026, 9, 26)?;
    Ok(DocProperties::new().set_creation_datetime(&date))
}

fn new_workbook() -> Result<Workbook, rust_xlsxwriter::XlsxError> {
    let mut workbook = Workbook::new();
    workbook.set_properties(&fixed_properties()?);
    Ok(workbook)
}

pub fn run(root: &Path, check: bool) {
    let fixtures_dir = root.join("crates/forskscope-core/src/tests/fixtures/xlsx");
    let cases: Vec<(&str, CaseGenerator)> = vec![
        ("basic", basic),
        ("unchanged_sheet", unchanged_sheet),
        ("renamed", renamed),
        ("renamed_and_moved", renamed_and_moved),
        ("formula", formula),
        ("formula_cached_value_changed", formula_cached_value_changed),
        ("large", large),
        ("reorder_only", reorder_only),
        ("sheet_added", sheet_added),
        ("sheet_removed", sheet_removed),
        ("stray_far_cell", stray_far_cell),
        ("chart_sheet_not_compared", chart_sheet_not_compared),
        ("ambiguous_rename", ambiguous_rename),
        ("duplicate_row_keys", duplicate_row_keys),
    ];

    if check {
        let tmp = std::env::temp_dir().join(format!("fsk-xlsx-fixtures-check-{}", process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        generate_all(&tmp, &cases);

        let mut mismatches = Vec::new();
        for (case, _) in &cases {
            for side in ["old.xlsx", "new.xlsx"] {
                let committed = fixtures_dir.join(case).join(side);
                let generated = tmp.join(case).join(side);
                let committed_bytes = std::fs::read(&committed).unwrap_or_else(|e| {
                    eprintln!("cannot read {}: {e}", committed.display());
                    process::exit(1);
                });
                let generated_bytes = std::fs::read(&generated).unwrap_or_else(|e| {
                    panic!("generator did not write {}: {e}", generated.display())
                });
                if committed_bytes != generated_bytes {
                    mismatches.push(format!("{case}/{side}"));
                }
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);

        if mismatches.is_empty() {
            println!(
                "xlsx fixtures check passed: all {} cases match what the generator produces.",
                cases.len()
            );
        } else {
            eprintln!(
                "xlsx fixtures are STALE - these no longer match the generator's output: {}",
                mismatches.join(", ")
            );
            eprintln!("Run `cargo xtask xlsx-fixtures` to regenerate.");
            process::exit(1);
        }
        return;
    }

    generate_all(&fixtures_dir, &cases);
    println!(
        "wrote {} xlsx fixture pairs to {}",
        cases.len(),
        fixtures_dir.display()
    );
}

fn generate_all(dir: &Path, cases: &[(&str, CaseGenerator)]) {
    for (case, generator) in cases {
        let case_dir = dir.join(case);
        std::fs::create_dir_all(&case_dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", case_dir.display()));
        generator(&case_dir).unwrap_or_else(|e| panic!("generating {case} failed: {e}"));
    }
}

fn save_pair(
    dir: &Path,
    mut old: Workbook,
    mut new: Workbook,
) -> Result<(), rust_xlsxwriter::XlsxError> {
    old.save(dir.join("old.xlsx"))?;
    new.save(dir.join("new.xlsx"))?;
    Ok(())
}

/// `xlsx.rs::a_cell_value_change_is_reported_as_modified`: one sheet, one
/// cell, one value change.
fn basic(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().write(0, 0, "hello")?;
    let mut new = new_workbook()?;
    new.add_worksheet().write(0, 0, "world")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::an_unchanged_sheet_is_dropped_from_the_sheet_list`: two sheets,
/// one byte-for-byte untouched ("Same"), one with a real change ("Changed").
fn unchanged_sheet(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().set_name("Same")?.write(0, 0, "same")?;
    old.add_worksheet()
        .set_name("Changed")?
        .write(0, 0, "before")?;
    let mut new = new_workbook()?;
    new.add_worksheet().set_name("Same")?.write(0, 0, "same")?;
    new.add_worksheet()
        .set_name("Changed")?
        .write(0, 0, "after")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_renamed_sheet_at_the_same_position_is_reported_as_renamed`
/// and `a_pure_rename_renders_each_sides_own_name`: one sheet, same position,
/// same cell content, name only differs.
fn renamed(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet()
        .set_name("Original")?
        .write(0, 0, "same")?;
    let mut new = new_workbook()?;
    new.add_worksheet()
        .set_name("Renamed")?
        .write(0, 0, "same")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_sheet_renamed_and_moved_collapses_into_renamed_not_two_entries`
/// and `the_sheet_list_follows_each_sides_own_tab_order`: "Anchor" keeps its
/// name but swaps tab position with "ToRename", which is also renamed to
/// "RenamedSheet" - old tabs [Anchor, ToRename], new tabs [RenamedSheet, Anchor].
fn renamed_and_moved(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().set_name("Anchor")?.write(0, 0, "a")?;
    old.add_worksheet().set_name("ToRename")?.write(0, 0, "r")?;
    let mut new = new_workbook()?;
    new.add_worksheet()
        .set_name("RenamedSheet")?
        .write(0, 0, "r")?;
    new.add_worksheet().set_name("Anchor")?.write(0, 0, "a")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_formula_change_is_isolated_from_value_change`: formula text
/// changes, neither side sets a cached result, so both read as the same
/// empty value.
fn formula(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().write_formula(0, 0, "1+1")?;
    let mut new = new_workbook()?;
    new.add_worksheet().write_formula(0, 0, "2+2")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_formula_cells_changed_cached_value_is_still_reported_with_the_flag_off`
/// (F138 Part B): formula text `A1+A2` identical on both sides; only the
/// cached result (2 -> 3) differs.
fn formula_cached_value_changed(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    let sheet = old.add_worksheet();
    sheet.write(0, 0, 1)?;
    sheet.write(1, 0, 1)?;
    sheet.write_formula(2, 0, Formula::new("A1+A2").set_result("2"))?;
    let mut new = new_workbook()?;
    let sheet = new.add_worksheet();
    sheet.write(0, 0, 1)?;
    sheet.write(1, 0, 1)?;
    sheet.write_formula(2, 0, Formula::new("A1+A2").set_result("3"))?;
    save_pair(dir, old, new)
}

/// `xlsx.rs`'s "large" fixture family (F117, handoff 022 §4): 51,000
/// populated numeric cells in one sheet, no formulas. Deliberately
/// byte-identical on both sides (written once, copied) -
/// `every_fixture_pair_renders_two_different_sides_in_both_directions`
/// decides "the workbooks differ" from the files' raw bytes, and this is
/// the one fixture meant to render as no difference at all.
const LARGE_CELL_COUNT: u32 = 51_000;

fn large(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut workbook = new_workbook()?;
    let sheet = workbook.add_worksheet();
    for row in 0..LARGE_CELL_COUNT {
        sheet.write(row, 0, f64::from(row))?;
    }
    let old_path = dir.join("old.xlsx");
    workbook.save(&old_path)?;
    std::fs::copy(&old_path, dir.join("new.xlsx"))
        .unwrap_or_else(|e| panic!("cannot copy {} to new.xlsx: {e}", old_path.display()));
    Ok(())
}

/// `xlsx.rs::a_reorder_only_pair_shows_where_each_sheet_is_on_each_side`:
/// same two sheets, same content, tabs swapped.
fn reorder_only(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet()
        .set_name("Same")?
        .write(0, 0, "unchanged content")?;
    old.add_worksheet()
        .set_name("Other")?
        .write(0, 0, "other content")?;
    let mut new = new_workbook()?;
    new.add_worksheet()
        .set_name("Other")?
        .write(0, 0, "other content")?;
    new.add_worksheet()
        .set_name("Same")?
        .write(0, 0, "unchanged content")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::an_added_sheet_is_reported_and_marked_on_the_new_side_only`:
/// old has only "Kept"; new adds "Added".
fn sheet_added(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().set_name("Kept")?.write(0, 0, "same")?;
    let mut new = new_workbook()?;
    new.add_worksheet().set_name("Kept")?.write(0, 0, "same")?;
    new.add_worksheet()
        .set_name("Added")?
        .write(0, 0, "other content")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_removed_sheet_is_reported_and_marked_on_the_old_side_only`:
/// mirror of `sheet_added` - old has "Kept" and "Removed", new drops it.
fn sheet_removed(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet().set_name("Kept")?.write(0, 0, "same")?;
    old.add_worksheet()
        .set_name("Removed")?
        .write(0, 0, "other content")?;
    let mut new = new_workbook()?;
    new.add_worksheet().set_name("Kept")?.write(0, 0, "same")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_stray_cell_at_the_last_position_is_compared_not_refused_or_fatal`
/// (F123/F130): `A1` unchanged on both sides; the cell at Excel's last
/// position, `XFD1048576` (0-indexed row 1,048,575, column 16,383), is the
/// one real change.
fn stray_far_cell(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    const LAST_ROW: u32 = 1_048_575;
    const LAST_COL: u16 = 16_383;
    let mut old = new_workbook()?;
    let sheet = old.add_worksheet().set_name("Data")?;
    sheet.write(0, 0, "origin")?;
    sheet.write(LAST_ROW, LAST_COL, "x")?;
    let mut new = new_workbook()?;
    let sheet = new.add_worksheet().set_name("Data")?;
    sheet.write(0, 0, "origin")?;
    sheet.write(LAST_ROW, LAST_COL, "y")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_sheet_that_was_not_compared_is_a_warning_naming_it` and
/// `compare_pair_returns_the_warnings_with_the_text`: a real chartsheet
/// ("Chart1", `add_chartsheet`'s own default name) alongside an ordinary
/// "Data" sheet whose `A1` changes.
fn chart_sheet_not_compared(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    {
        let sheet = old.add_worksheet().set_name("Data")?;
        sheet.write(0, 0, "before")?;
        sheet.write(1, 0, 1)?;
    }
    let mut chart = Chart::new(ChartType::Column);
    chart.add_series().set_values("Data!$A$2:$A$2");
    old.add_chartsheet().insert_chart(0, 0, &chart)?;

    let mut new = new_workbook()?;
    {
        let sheet = new.add_worksheet().set_name("Data")?;
        sheet.write(0, 0, "after")?;
        sheet.write(1, 0, 1)?;
    }
    let mut chart = Chart::new(ChartType::Column);
    chart.add_series().set_values("Data!$A$2:$A$2");
    new.add_chartsheet().insert_chart(0, 0, &chart)?;

    save_pair(dir, old, new)
}

/// `xlsx.rs::an_ambiguous_sheet_match_is_one_warning_naming_the_candidates`:
/// two sheets removed, two added, each with distinct content so nothing
/// coincidentally content-matches across the rename.
fn ambiguous_rename(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    old.add_worksheet()
        .set_name("Alpha")?
        .write(0, 0, "alpha")?;
    old.add_worksheet().set_name("Beta")?.write(0, 0, "beta")?;
    let mut new = new_workbook()?;
    new.add_worksheet()
        .set_name("Gamma")?
        .write(0, 0, "gamma")?;
    new.add_worksheet()
        .set_name("Delta")?
        .write(0, 0, "delta")?;
    save_pair(dir, old, new)
}

/// `xlsx.rs::a_sheet_level_warning_is_read_as_well_as_the_workbook_level_ones`:
/// sheet "Keyed", column A has a duplicate key ("k" twice) under a row-key
/// alignment on column 1; column B's value at the duplicated key's second
/// row changes, independent of the key duplication itself.
fn duplicate_row_keys(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = new_workbook()?;
    {
        let sheet = old.add_worksheet().set_name("Keyed")?;
        for (row, (a, b)) in [("k", "one"), ("k", "two"), ("x", "three")]
            .into_iter()
            .enumerate()
        {
            sheet.write(row as u32, 0, a)?;
            sheet.write(row as u32, 1, b)?;
        }
    }
    let mut new = new_workbook()?;
    {
        let sheet = new.add_worksheet().set_name("Keyed")?;
        for (row, (a, b)) in [("k", "one"), ("k", "TWO"), ("x", "three")]
            .into_iter()
            .enumerate()
        {
            sheet.write(row as u32, 0, a)?;
            sheet.write(row as u32, 1, b)?;
        }
    }
    save_pair(dir, old, new)
}
