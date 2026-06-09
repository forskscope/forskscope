use std::fs;
use std::path::PathBuf;

use crate::document::{LoadOptions, hex_preview, load_path};
use crate::error::CoreError;
use crate::file_kind::{FileKind, classify};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-doc-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn classifies_text_binary_missing_and_unsupported() {
    let dir = temp_dir("classify");
    let text = dir.join("a.txt");
    fs::write(&text, "hello\nworld\n").unwrap();
    assert_eq!(classify(&text).unwrap(), FileKind::Text);

    let binary = dir.join("b.bin");
    fs::write(&binary, [0u8, 1, 2, 0, 255]).unwrap();
    assert_eq!(classify(&binary).unwrap(), FileKind::Binary);

    let missing = dir.join("nope.txt");
    assert_eq!(classify(&missing).unwrap(), FileKind::Missing);

    assert!(matches!(
        classify(&dir).unwrap(),
        FileKind::Unsupported { .. }
    ));
}

#[test]
fn xlsx_extension_classifies_as_excel() {
    let dir = temp_dir("xlsx");
    let f = dir.join("book.xlsx");
    fs::write(&f, [0u8; 4]).unwrap();
    assert_eq!(classify(&f).unwrap(), FileKind::ExcelXlsx);
}

#[test]
fn loading_text_retains_encoding_and_fingerprint() {
    let dir = temp_dir("load-text");
    let f = dir.join("c.txt");
    fs::write(&f, "line1\nline2\n").unwrap();
    let doc = load_path(&f, LoadOptions::default()).unwrap();
    assert_eq!(doc.kind, FileKind::Text);
    assert!(doc.fingerprint_at_load.is_some());
    assert_eq!(doc.diff_text(), "line1\nline2\n");
    assert_eq!(doc.text.unwrap().encoding.label, "UTF-8");
}

#[test]
fn missing_path_errors_unless_allowed() {
    let dir = temp_dir("missing");
    let f = dir.join("absent.txt");
    let err = load_path(&f, LoadOptions::default()).unwrap_err();
    assert!(matches!(err, CoreError::InvalidPath { .. }));

    let allowed = load_path(
        &f,
        LoadOptions {
            allow_missing: true,
        },
    )
    .unwrap();
    assert_eq!(allowed.diff_text(), "");
}

#[test]
fn binary_loads_as_hex_preview_not_editable_text() {
    let dir = temp_dir("binary-load");
    let f = dir.join("d.bin");
    fs::write(&f, [0u8, 0x41, 0x42]).unwrap();
    let doc = load_path(&f, LoadOptions::default()).unwrap();
    assert_eq!(doc.kind, FileKind::Binary);
    assert!(!doc.kind.is_mergeable_text());
}

#[test]
fn hex_preview_has_offset_and_ascii_columns() {
    let preview = hex_preview(b"AB");
    assert!(preview.starts_with("00000000  "));
    assert!(preview.trim_end().ends_with("AB"));
}
