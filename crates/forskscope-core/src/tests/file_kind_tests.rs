//! Tests for FileKind predicates and the classify() function (RFC-001 §6.2).

use std::io::Write;

use crate::file_kind::{FileKind, classify};

// ── FileKind::is_mergeable_text ───────────────────────────────────────────────

#[test]
fn text_is_mergeable() {
    assert!(FileKind::Text.is_mergeable_text());
}

#[test]
fn binary_is_not_mergeable() {
    assert!(!FileKind::Binary.is_mergeable_text());
}

#[test]
fn excel_is_not_mergeable() {
    assert!(!FileKind::ExcelXlsx.is_mergeable_text());
}

#[test]
fn missing_is_not_mergeable() {
    assert!(!FileKind::Missing.is_mergeable_text());
}

#[test]
fn unsupported_is_not_mergeable() {
    let k = FileKind::Unsupported { reason: "test".into() };
    assert!(!k.is_mergeable_text());
}

// ── classify: missing path ────────────────────────────────────────────────────

#[test]
fn classify_missing_path_returns_missing() {
    let p = std::path::Path::new("/this/path/definitely/does/not/exist.txt");
    let kind = classify(p).expect("classify must not error on missing path");
    assert_eq!(kind, FileKind::Missing);
}

// ── classify: regular files via temp files ────────────────────────────────────

fn with_temp_file(ext: &str, content: &[u8]) -> (tempfile::NamedTempFile, std::path::PathBuf) {
    let mut f = tempfile::Builder::new()
        .suffix(&format!(".{ext}"))
        .tempfile()
        .expect("temp file creation must succeed");
    f.write_all(content).expect("write must succeed");
    f.flush().expect("flush must succeed");
    let path = f.path().to_path_buf();
    (f, path)
}

#[test]
fn classify_utf8_text_file_returns_text() {
    let (_f, path) = with_temp_file("txt", b"hello world\nline two\n");
    assert_eq!(classify(&path).unwrap(), FileKind::Text);
}

#[test]
fn classify_file_with_nul_byte_returns_binary() {
    let content = b"some text\x00with a nul byte";
    let (_f, path) = with_temp_file("bin", content);
    assert_eq!(classify(&path).unwrap(), FileKind::Binary);
}

#[test]
fn classify_xlsx_extension_returns_excel_before_content_check() {
    // Even a text-content file with .xlsx extension is Excel
    let (_f, path) = with_temp_file("xlsx", b"not really xlsx content");
    assert_eq!(classify(&path).unwrap(), FileKind::ExcelXlsx);
}

#[test]
fn classify_xlsx_case_insensitive() {
    // .XLSX (uppercase) must also be classified as ExcelXlsx
    let (_f, path) = with_temp_file("XLSX", b"content");
    assert_eq!(classify(&path).unwrap(), FileKind::ExcelXlsx);
}

#[test]
fn classify_empty_file_returns_text() {
    // An empty file has no NUL bytes — classified as text
    let (_f, path) = with_temp_file("txt", b"");
    assert_eq!(classify(&path).unwrap(), FileKind::Text);
}

#[test]
fn classify_rust_source_returns_text() {
    let (_f, path) = with_temp_file("rs", b"fn main() { println!(\"hello\"); }\n");
    assert_eq!(classify(&path).unwrap(), FileKind::Text);
}

// ── classify: directory (not a regular file) ──────────────────────────────────

#[test]
fn classify_directory_returns_unsupported() {
    let dir = tempfile::tempdir().expect("tempdir creation must succeed");
    let kind = classify(dir.path()).expect("classify must not error on directory");
    assert!(matches!(kind, FileKind::Unsupported { .. }),
        "directory must classify as Unsupported, got: {kind:?}");
}
