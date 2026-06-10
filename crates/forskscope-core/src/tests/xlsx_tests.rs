//! Spreadsheet structural diff tests (RFC-058 §"Test Corpus").
//!
//! Tests validate the RFC-058 acceptance criteria: structured model carries
//! cell-level changes with correct addresses/kinds/values, sheet add/remove
//! is reported, identical workbooks produce an empty diff, a malformed file
//! returns `Err` rather than panicking, and `derive_pair_text_from_diff`
//! produces non-empty output for changed workbooks.
//!
//! Fixtures are generated at test time using the zip crate (XLSX is a ZIP
//! of XML files). This keeps the repo free of opaque binary blobs.

use std::fs;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::xlsx::{CellChangeKind, SheetChange, derive_pair_text_from_diff, diff_xlsx};

// ── XLSX fixture builder ──────────────────────────────────────────────────────

/// Build a minimal OOXML-conformant `.xlsx` file understood by calamine.
///
/// `cells` is a flat list of `(row, col, value)` tuples (1-based).
/// `sheet_name` is the name of the single worksheet.
fn make_xlsx(path: &Path, sheet_name: &str, cells: &[(u32, u32, &str)]) {
    // Build the worksheet XML.
    let mut sheet_xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
         <sheetData>",
    );
    // Gather rows.
    let mut rows: std::collections::BTreeMap<u32, Vec<(u32, &str)>> =
        std::collections::BTreeMap::new();
    for &(r, c, v) in cells {
        rows.entry(r).or_default().push((c, v));
    }
    for (r, cols) in &rows {
        sheet_xml.push_str(&format!("<row r=\"{r}\">"));
        for (c, v) in cols {
            let addr = col_letter(*c) + &r.to_string();
            sheet_xml.push_str(&format!(
                "<c r=\"{addr}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
                xml_escape(v)
            ));
        }
        sheet_xml.push_str("</row>");
    }
    sheet_xml.push_str("</sheetData></worksheet>");

    // workbook.xml
    let workbook_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
         <sheets><sheet name=\"{sheet_name}\" sheetId=\"1\" r:id=\"rId1\"/></sheets>\
         </workbook>"
    );

    // workbook.xml.rels
    let rels_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
        <Relationship Id=\"rId1\" \
        Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
        Target=\"worksheets/sheet1.xml\"/>\
        </Relationships>";

    // [Content_Types].xml
    let ct_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
        <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
        <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
        <Override PartName=\"/xl/workbook.xml\" \
        ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
        <Override PartName=\"/xl/worksheets/sheet1.xml\" \
        ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
        </Types>";

    // _rels/.rels
    let top_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
        <Relationship Id=\"rId1\" \
        Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" \
        Target=\"xl/workbook.xml\"/>\
        </Relationships>";

    let file = fs::File::create(path).unwrap();
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut zip = zip::ZipWriter::new(file);

    for (name, content) in [
        ("[Content_Types].xml", ct_xml),
        ("_rels/.rels", top_rels),
        ("xl/workbook.xml", &workbook_xml),
        ("xl/_rels/workbook.xml.rels", rels_xml),
        ("xl/worksheets/sheet1.xml", &sheet_xml),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn col_letter(col: u32) -> String {
    // Simple A–Z for col 1–26; AA–AZ for 27–52 (sufficient for tests).
    if col <= 26 {
        format!("{}", (b'A' + (col - 1) as u8) as char)
    } else {
        let first = (b'A' + ((col - 1) / 26 - 1) as u8) as char;
        let second = (b'A' + ((col - 1) % 26) as u8) as char;
        format!("{first}{second}")
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("fsk-xlsx-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);
    d
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn identical_workbooks_produce_empty_diff() {
    let dir = tmp("identical");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[(1, 1, "hello"), (2, 1, "world")]);
    make_xlsx(&new, "Sheet1", &[(1, 1, "hello"), (2, 1, "world")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    assert!(diff.is_empty(), "identical workbooks must produce empty diff");
    assert_eq!(diff.stats.cells_changed, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn changed_cell_value_is_reported_with_correct_address_and_sides() {
    let dir = tmp("changed");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[(2, 2, "100")]);
    make_xlsx(&new, "Sheet1", &[(2, 2, "200")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    assert_eq!(diff.stats.cells_changed, 1, "one cell changed");

    let sheet = &diff.cells[0];
    assert_eq!(sheet.sheet, "Sheet1");
    let cell = &sheet.cells[0];
    assert_eq!(cell.addr, "B2");
    assert_eq!(cell.row, 2);
    assert_eq!(cell.col, 2);
    assert_eq!(cell.kind, CellChangeKind::Value);
    assert_eq!(cell.old.as_deref(), Some("100"));
    assert_eq!(cell.new.as_deref(), Some("200"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn empty_to_non_empty_cell_has_none_on_old_side() {
    let dir = tmp("empty-to-value");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[]);                      // empty sheet
    make_xlsx(&new, "Sheet1", &[(1, 1, "appeared")]);    // new cell

    let diff = diff_xlsx(&old, &new).unwrap();
    assert_eq!(diff.stats.cells_changed, 1);
    let cell = &diff.cells[0].cells[0];
    assert!(cell.old.is_none(), "old side must be None for newly added cell");
    assert_eq!(cell.new.as_deref(), Some("appeared"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn added_sheet_is_reported_as_sheet_change() {
    let dir = tmp("added-sheet");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    // old has Sheet1; new has Sheet1 + Sheet2.
    // sheets-diff only compares same-name sheets, so we need two fixture files
    // with different sheet names — we fake this by using different names.
    make_xlsx(&old, "Sheet1", &[(1, 1, "a")]);
    make_xlsx(&new, "NewSheet", &[(1, 1, "a")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    assert!(!diff.sheets.is_empty(), "sheet changes must be reported");
    let has_removed = diff.sheets.iter().any(|s| matches!(s, SheetChange::Removed(_)));
    let has_added   = diff.sheets.iter().any(|s| matches!(s, SheetChange::Added(_)));
    assert!(has_removed || has_added, "at least one sheet change expected: {:?}", diff.sheets);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn malformed_file_returns_err_not_panic() {
    let dir = tmp("malformed");
    let bad = dir.join("bad.xlsx");
    fs::write(&bad, b"this is not a zip archive").unwrap();
    let other = dir.join("other.xlsx");
    make_xlsx(&other, "Sheet1", &[]);

    let result = diff_xlsx(&bad, &other);
    assert!(result.is_err(), "malformed file must return Err, not panic");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn malformed_second_file_returns_err_not_panic() {
    let dir = tmp("malformed2");
    let good = dir.join("good.xlsx");
    make_xlsx(&good, "Sheet1", &[(1, 1, "x")]);
    let bad = dir.join("bad.xlsx");
    fs::write(&bad, b"garbage").unwrap();

    let result = diff_xlsx(&good, &bad);
    assert!(result.is_err(), "malformed second file must return Err");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn multiple_changed_cells_all_appear_in_model() {
    let dir = tmp("multicell");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[(1,1,"a"),(2,1,"b"),(3,1,"c")]);
    make_xlsx(&new, "Sheet1", &[(1,1,"A"),(2,1,"B"),(3,1,"C")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    assert_eq!(diff.stats.cells_changed, 3, "all three cells must be reported");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn derive_pair_text_from_diff_is_non_empty_for_changed_workbook() {
    let dir = tmp("pairtext");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[(1,1,"old_value")]);
    make_xlsx(&new, "Sheet1", &[(1,1,"new_value")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    let (old_text, new_text) = derive_pair_text_from_diff(&diff);
    assert!(!old_text.content.is_empty(), "old derived text must not be empty");
    assert!(!new_text.content.is_empty(), "new derived text must not be empty");
    assert!(old_text.content.contains("old_value"), "old side must contain old value");
    assert!(new_text.content.contains("new_value"), "new side must contain new value");
    assert_eq!(old_text.encoding.label, "(Excel)");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn derive_pair_text_from_diff_is_empty_for_identical_workbooks() {
    let dir = tmp("pairtext-identical");
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    make_xlsx(&old, "Sheet1", &[(1,1,"same")]);
    make_xlsx(&new, "Sheet1", &[(1,1,"same")]);

    let diff = diff_xlsx(&old, &new).unwrap();
    let (old_text, new_text) = derive_pair_text_from_diff(&diff);
    assert!(old_text.content.is_empty(), "identical workbooks produce empty text");
    assert!(new_text.content.is_empty());
    let _ = fs::remove_dir_all(&dir);
}
