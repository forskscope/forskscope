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
        ("row_inserted_near_top", row_inserted_near_top),
        ("row_appended_at_bottom", row_appended_at_bottom),
        ("row_deleted_near_top", row_deleted_near_top),
        ("row_inserted_near_bottom", row_inserted_near_bottom),
        ("insert_and_delete_balanced", insert_and_delete_balanced),
        ("cells_edited_in_place", cells_edited_in_place),
        ("insert_plus_edit_below", insert_plus_edit_below),
        (
            "identical_rows_away_from_edit",
            identical_rows_away_from_edit,
        ),
        (
            "display_identical_formulas_differ",
            display_identical_formulas_differ,
        ),
        ("identical_rows_one_deleted", identical_rows_one_deleted),
        (
            "identical_rows_no_structure_change",
            identical_rows_no_structure_change,
        ),
        (
            "relative_formulas_row_inserted",
            relative_formulas_row_inserted,
        ),
        ("formulas_above_the_edit", formulas_above_the_edit),
        ("relative_formulas_edits_only", relative_formulas_edits_only),
        (
            "relative_formulas_real_formula_edit",
            relative_formulas_real_formula_edit,
        ),
        ("two_sheets_mixed", two_sheets_mixed),
        (
            "string_literal_formulas_row_inserted",
            string_literal_formulas_row_inserted,
        ),
        ("range_total_row_inserted", range_total_row_inserted),
        (
            "range_total_row_inserted_zero",
            range_total_row_inserted_zero,
        ),
        (
            "value_and_explained_formula_in_one_cell",
            value_and_explained_formula_in_one_cell,
        ),
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

// ── Alignment corpus (F132; handoff 069 §9) ──────────────────────────────────
//
// Each case is a recipe, not a hand-made pair. The base sheet has a header row
// and four columns: `A` a unique id, `B` text derived from the id, `C` a number
// derived from the id, `D` a plain number derived from `C`. Formulas appear only
// where a case is about them. Every case's truth is the exact change made to the
// base, and `xlsx.rs`'s tests state that truth beside the outcome it expects.
// Rows are numbered as Excel numbers them: the header is row 1, the first record
// row 2.

use rust_xlsxwriter::Worksheet;

/// How one record's `D` cell is written.
#[derive(Clone)]
enum DCell {
    /// A plain number.
    Plain(f64),
    /// `=C{row}*k`: the row-relative form, which Excel rewrites on every row move.
    Times(u32),
    /// `=$C$2*2`: refers to row 2, which the cases never move.
    AbsTwo,
    /// `=IF(C{row}>1000,"big","small")`: a formula with a string literal, which upstream
    /// declines to map (handoff 070 §1).
    IfBigSmall,
    /// `=SUM(C2:C{last})` over every data record above it: the total row of a sheet
    /// whose range is rewritten when a row is inserted (handoff 070 F6).
    SumDataAbove,
    /// A formula written verbatim, without the leading `=`, whatever row it lands on.
    Verbatim { text: &'static str, cached: f64 },
}

#[derive(Clone)]
struct Rec {
    id: String,
    text: String,
    c: f64,
    d: DCell,
}

/// The `i`-th base record (1-based): derived entirely from `i`.
fn base_rec(i: u32) -> Rec {
    let id = format!("ID-{i:04}");
    Rec {
        text: format!("text-{id}"),
        id,
        c: f64::from(i) * 10.0,
        d: DCell::Plain(f64::from(i) * 2.5),
    }
}

/// `n` base records, numbered 1 to `n`.
fn base(n: u32) -> Vec<Rec> {
    (1..=n).map(base_rec).collect()
}

/// A record that appears only in the new sheet.
fn inserted(tag: &str) -> Rec {
    Rec {
        id: format!("NEW-{tag}"),
        text: format!("inserted {tag}"),
        c: 5555.0,
        d: DCell::Plain(1388.75),
    }
}

/// The index of the record that sits at Excel row `row`.
fn at(row: u32) -> usize {
    (row - 2) as usize
}

fn insert_at(v: &mut Vec<Rec>, row: u32, rec: Rec) {
    v.insert(at(row), rec);
}

fn delete_at(v: &mut Vec<Rec>, row: u32) {
    v.remove(at(row));
}

/// Makes the record at Excel row `to` identical to the one at `from`, in every column.
fn copy_row(v: &mut [Rec], from: u32, to: u32) {
    let source = v[at(from)].clone();
    v[at(to)] = source;
}

/// The record that marks the total row. Its `D` is `SumDataAbove`.
fn total_row() -> Rec {
    Rec {
        id: "TOTAL".into(),
        text: "total".into(),
        c: 0.0,
        d: DCell::SumDataAbove,
    }
}

fn write_records(sheet: &mut Worksheet, recs: &[Rec]) -> Result<(), rust_xlsxwriter::XlsxError> {
    // The last data row is the last record that is not the total row.
    let data_rows = recs
        .iter()
        .filter(|r| !matches!(r.d, DCell::SumDataAbove))
        .count() as u32;
    let last_data_excel = data_rows + 1;
    let data_sum: f64 = recs
        .iter()
        .filter(|r| !matches!(r.d, DCell::SumDataAbove))
        .map(|r| r.c)
        .sum();
    sheet.write(0, 0, "ID")?;
    sheet.write(0, 1, "Text")?;
    sheet.write(0, 2, "Number")?;
    sheet.write(0, 3, "Value")?;
    let first_c = recs.first().map(|r| r.c).unwrap_or(0.0);
    for (k, rec) in recs.iter().enumerate() {
        let row = k as u32 + 1; // zero-based sheet row; Excel row is `row + 1`
        let excel = row + 1;
        sheet.write(row, 0, rec.id.as_str())?;
        sheet.write(row, 1, rec.text.as_str())?;
        sheet.write(row, 2, rec.c)?;
        match &rec.d {
            DCell::Plain(v) => {
                sheet.write(row, 3, *v)?;
            }
            DCell::Times(k) => {
                sheet.write_formula(
                    row,
                    3,
                    Formula::new(format!("C{excel}*{k}"))
                        .set_result(format!("{}", rec.c * f64::from(*k))),
                )?;
            }
            DCell::AbsTwo => {
                sheet.write_formula(
                    row,
                    3,
                    Formula::new("$C$2*2").set_result(format!("{}", first_c * 2.0)),
                )?;
            }
            DCell::IfBigSmall => {
                let cached = if rec.c > 1000.0 { "big" } else { "small" };
                sheet.write_formula(
                    row,
                    3,
                    Formula::new(format!("IF(C{excel}>1000,\"big\",\"small\")")).set_result(cached),
                )?;
            }
            DCell::SumDataAbove => {
                sheet.write_formula(
                    row,
                    3,
                    Formula::new(format!("SUM(C2:C{last_data_excel})"))
                        .set_result(format!("{data_sum}")),
                )?;
            }
            DCell::Verbatim { text, cached } => {
                sheet.write_formula(row, 3, Formula::new(*text).set_result(format!("{cached}")))?;
            }
        }
    }
    Ok(())
}

/// A workbook with one sheet per `(name, records)` entry, in order.
fn book(sheets: &[(&str, &[Rec])]) -> Result<Workbook, rust_xlsxwriter::XlsxError> {
    let mut workbook = new_workbook()?;
    for (name, recs) in sheets {
        let sheet = workbook.add_worksheet().set_name(*name)?;
        write_records(sheet, recs)?;
    }
    Ok(workbook)
}

/// Saves a pair built from single-sheet record lists.
fn save_records(dir: &Path, old: &[Rec], new: &[Rec]) -> Result<(), rust_xlsxwriter::XlsxError> {
    save_pair(dir, book(&[("Sheet1", old)])?, book(&[("Sheet1", new)])?)
}

fn row_inserted_near_top(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    insert_at(&mut new, 3, inserted("top"));
    save_records(dir, &old, &new)
}

/// Added beyond handoff 069 §9, because none of its named cases ties: A3 and C4 were
/// both decided on count alone. Appending a row moves nothing, so the positional
/// and aligned results both count the new row's four cells, a tie, and the tie
/// rule is the only thing that chooses between them.
fn row_appended_at_bottom(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    insert_at(&mut new, 202, inserted("bottom"));
    save_records(dir, &old, &new)
}

fn row_deleted_near_top(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    delete_at(&mut new, 3);
    save_records(dir, &old, &new)
}

fn row_inserted_near_bottom(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    insert_at(&mut new, 199, inserted("bottom"));
    save_records(dir, &old, &new)
}

fn insert_and_delete_balanced(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    insert_at(&mut new, 10, inserted("ten"));
    delete_at(&mut new, 150);
    save_records(dir, &old, &new)
}

fn cells_edited_in_place(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    for row in [40, 90, 140] {
        let i = at(row);
        new[i].text = format!("edited {row}");
    }
    save_records(dir, &old, &new)
}

fn insert_plus_edit_below(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = base(200);
    let mut new = base(200);
    insert_at(&mut new, 3, inserted("top"));
    let i = at(120);
    new[i].text = "edited 120".into();
    save_records(dir, &old, &new)
}

/// Rows 150 and 151 are identical in every column, in both sheets. The insertion
/// is the only difference between old and new.
fn identical_rows_away_from_edit(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = base(200);
    copy_row(&mut old, 150, 151);
    let mut new = old.clone();
    insert_at(&mut new, 3, inserted("top"));
    save_records(dir, &old, &new)
}

/// Rows 150 and 151 share `A`, `B` and `C`, and their `D` formulas differ in text but
/// give the same result. In the new sheet those two rows are swapped, with their
/// formula text carried over verbatim. Every `D` is a formula in this case only.
fn display_identical_formulas_differ(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old: Vec<Rec> = base(200)
        .into_iter()
        .map(|mut r| {
            r.d = DCell::Times(2);
            r
        })
        .collect();
    copy_row(&mut old, 150, 151);
    let i150 = at(150);
    let i151 = at(151);
    old[i150].d = DCell::Verbatim {
        text: "2*C150",
        cached: old[i150].c * 2.0,
    };
    old[i151].d = DCell::Verbatim {
        text: "C151+C151",
        cached: old[i151].c * 2.0,
    };
    let mut new = old.clone();
    insert_at(&mut new, 3, inserted("top"));
    let (a, b) = (at(150), at(151));
    new.swap(a, b);
    save_records(dir, &old, &new)
}

/// Rows 100 and 101 are identical; the new sheet deletes one of them. Which one was
/// deleted is not determinable from the content.
fn identical_rows_one_deleted(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = base(200);
    copy_row(&mut old, 100, 101);
    let mut new = old.clone();
    delete_at(&mut new, 101);
    save_records(dir, &old, &new)
}

/// Rows 100 and 101 are identical, and the new sheet carries B1's three edits.
fn identical_rows_no_structure_change(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = base(200);
    copy_row(&mut old, 100, 101);
    let mut new = old.clone();
    for row in [40, 90, 140] {
        let i = at(row);
        new[i].text = format!("edited {row}");
    }
    save_records(dir, &old, &new)
}

fn with_formulas(d: DCell) -> Vec<Rec> {
    base(200)
        .into_iter()
        .map(|mut r| {
            r.d = d.clone();
            r
        })
        .collect()
}

fn relative_formulas_row_inserted(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::Times(2));
    let mut new = with_formulas(DCell::Times(2));
    insert_at(&mut new, 3, inserted("top"));
    save_records(dir, &old, &new)
}

fn formulas_above_the_edit(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::AbsTwo);
    let mut new = with_formulas(DCell::AbsTwo);
    insert_at(&mut new, 3, inserted("top"));
    save_records(dir, &old, &new)
}

fn relative_formulas_edits_only(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::Times(2));
    let mut new = with_formulas(DCell::Times(2));
    for row in [40, 90, 140] {
        let i = at(row);
        new[i].text = format!("edited {row}");
    }
    save_records(dir, &old, &new)
}

fn relative_formulas_real_formula_edit(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::Times(2));
    let mut new = with_formulas(DCell::Times(2));
    insert_at(&mut new, 3, inserted("top"));
    let i = at(120);
    new[i].d = DCell::Times(3);
    save_records(dir, &old, &new)
}

/// Two sheets: `Sheet1` is the insertion case, `Sheet2` the in-place edits case.
fn two_sheets_mixed(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old1 = base(200);
    let mut new1 = base(200);
    insert_at(&mut new1, 3, inserted("top"));
    let old2 = base(200);
    let mut new2 = base(200);
    for row in [40, 90, 140] {
        let i = at(row);
        new2[i].text = format!("edited {row}");
    }
    save_pair(
        dir,
        book(&[("Sheet1", &old1), ("Sheet2", &old2)])?,
        book(&[("Sheet1", &new1), ("Sheet2", &new2)])?,
    )
}

/// F137 (handoff 069 §6, §9 D): the size cases, written to `dir` and never into the
/// repository, because a committed 40,000-row workbook is weight for no gain over
/// its recipe. Old and new each keep the recipe's own shape; sheet names are the
/// generator's defaults.
pub fn run_scale(dir: &Path) {
    let mut headline_new = base(2_000);
    insert_at(&mut headline_new, 3, inserted("top"));
    let pairs: [(&str, Vec<Rec>, Vec<Rec>); 2] = [
        (
            "asymmetric_five_against_forty_thousand",
            base(5),
            base(40_000),
        ),
        ("headline_2000", base(2_000), headline_new),
    ];
    for (case, old, new) in &pairs {
        let case_dir = dir.join(case);
        std::fs::create_dir_all(&case_dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", case_dir.display()));
        save_records(&case_dir, old, new).unwrap_or_else(|e| panic!("writing {case} failed: {e}"));
    }
    println!("wrote the size cases to {}", dir.display());
}

/// F5 (handoff 070 §5): a string literal in the formula. Every shifted row is declined,
/// so the gate keeps positional.
fn string_literal_formulas_row_inserted(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::IfBigSmall);
    let mut new = with_formulas(DCell::IfBigSmall);
    insert_at(&mut new, 3, inserted("top"));
    save_records(dir, &old, &new)
}

/// F6: the rows of A1 with a relative formula, and a total row summing the data
/// above it. The total's range is rewritten when a row is inserted.
fn range_total_row_inserted(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = with_formulas(DCell::Times(2));
    old.push(total_row());
    let mut new = with_formulas(DCell::Times(2));
    insert_at(&mut new, 3, inserted("top"));
    new.push(total_row());
    save_records(dir, &old, &new)
}

/// F7 (handoff 070 §5, the case most likely to be wrong on a first attempt): the F1
/// shape, plus row 120's `C` changed, so that row's `D` shows a new value and its
/// formula also shifted.
fn value_and_explained_formula_in_one_cell(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let old = with_formulas(DCell::Times(2));
    let mut new = with_formulas(DCell::Times(2));
    insert_at(&mut new, 3, inserted("top"));
    let i = at(120);
    new[i].c += 1000.0;
    save_records(dir, &old, &new)
}

/// F6b (added beyond the plan): as F6, but the inserted row's `C` is zero, so the
/// total's displayed value does not change. The total row then pairs by content, and
/// its rewritten range is explained, which F6 cannot show: its value changed, so it
/// was unpaired and nothing could explain its formula.
fn range_total_row_inserted_zero(dir: &Path) -> Result<(), rust_xlsxwriter::XlsxError> {
    let mut old = with_formulas(DCell::Times(2));
    old.push(total_row());
    let mut new = with_formulas(DCell::Times(2));
    let mut zero = inserted("top");
    zero.c = 0.0;
    zero.d = DCell::Plain(0.0);
    insert_at(&mut new, 3, zero);
    new.push(total_row());
    save_records(dir, &old, &new)
}
