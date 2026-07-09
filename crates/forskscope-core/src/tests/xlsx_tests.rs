//! Spreadsheet comparison security mitigation tests.

use std::fs;

use crate::error::CoreError;
use crate::file_kind::FileKind;
use crate::xlsx::{derive_pair_text, diff_xlsx, load_placeholder};

#[test]
fn diff_xlsx_fails_closed_without_parsing_workbook_content() {
    let dir = std::env::temp_dir().join(format!("fsk-xlsx-disabled-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    fs::write(&old, b"not a real workbook").unwrap();
    fs::write(&new, b"also not a real workbook").unwrap();

    let err = diff_xlsx(&old, &new, None).unwrap_err();
    assert!(matches!(err, CoreError::Unsupported { .. }));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn derive_pair_text_returns_empty_documents_when_xlsx_is_disabled() {
    let dir = std::env::temp_dir().join(format!("fsk-xlsx-pair-disabled-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.xlsx");
    let new = dir.join("new.xlsx");
    fs::write(&old, b"old").unwrap();
    fs::write(&new, b"new").unwrap();

    let (left, right) = derive_pair_text(&old, &new);
    assert!(left.content.is_empty());
    assert!(right.content.is_empty());
    assert_eq!(left.encoding.label, "(Excel)");
    assert_eq!(right.encoding.label, "(Excel)");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_placeholder_preserves_metadata_without_reading_workbook_xml() {
    let dir = std::env::temp_dir().join(format!("fsk-xlsx-placeholder-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("book.xlsx");
    fs::write(&path, b"local workbook bytes").unwrap();

    let doc = load_placeholder(&path).unwrap();
    assert_eq!(doc.kind, FileKind::ExcelXlsx);
    assert_eq!(doc.bytes_len, "local workbook bytes".len() as u64);
    assert!(doc.text.is_none());
    assert!(doc.fingerprint_at_load.is_some());

    let _ = fs::remove_dir_all(&dir);
}
