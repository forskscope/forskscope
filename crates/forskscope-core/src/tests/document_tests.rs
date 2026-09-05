use std::fs;
use std::path::PathBuf;

use crate::document::{LoadOptions, hex_preview, load_path};
use crate::encoding::BomPresence;
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

// ── New tests for v0.32.0 ─────────────────────────────────────────────────────

#[test]
fn empty_file_loads_as_empty_text_document() {
    let dir = temp_dir("doc-empty");
    let path = dir.join("empty.txt");
    std::fs::write(&path, "").unwrap();
    let doc = crate::document::load_path(
        &path,
        crate::document::LoadOptions {
            allow_missing: false,
        },
    )
    .unwrap();
    assert!(doc.kind.is_mergeable_text(), "empty .txt should be text");
    assert_eq!(doc.diff_text(), "", "empty file has empty diff text");
}

#[test]
fn fingerprint_changes_after_file_modification() {
    let dir = temp_dir("fingerprint");
    let path = dir.join("file.txt");
    std::fs::write(&path, "v1").unwrap();
    let fp1 = crate::document::FileFingerprint::capture(&path, None).unwrap();
    std::fs::write(&path, "v2-different").unwrap();
    let fp2 = crate::document::FileFingerprint::capture(&path, None).unwrap();
    assert_ne!(
        fp1, fp2,
        "fingerprint must change when file content changes"
    );
}

#[test]
fn fingerprint_is_stable_for_unchanged_file() {
    let dir = temp_dir("fp-stable");
    let path = dir.join("stable.txt");
    std::fs::write(&path, "constant content").unwrap();
    let fp1 = crate::document::FileFingerprint::capture(&path, None).unwrap();
    let fp2 = crate::document::FileFingerprint::capture(&path, None).unwrap();
    assert_eq!(fp1, fp2, "fingerprint must be stable for an unchanged file");
}

#[test]
fn allow_missing_loads_empty_document_for_absent_path() {
    use std::path::PathBuf;
    let absent = PathBuf::from("/tmp/this_file_definitely_does_not_exist_12345.txt");
    let doc = crate::document::load_path(
        &absent,
        crate::document::LoadOptions {
            allow_missing: true,
        },
    )
    .unwrap();
    assert!(
        doc.diff_text().is_empty(),
        "missing file with allow_missing yields empty document"
    );
}

// ── RFC-083 §2: BOM stripped at load, recorded, never left in content ────────

#[test]
fn a_utf8_bom_is_stripped_from_content_and_recorded() {
    let dir = temp_dir("bom-utf8");
    let path = dir.join("bommed.txt");
    fs::write(&path, [0xEFu8, 0xBB, 0xBF, b'h', b'i', b'\n']).unwrap();
    let doc = load_path(&path, LoadOptions::default()).unwrap();
    assert_eq!(
        doc.diff_text(),
        "hi\n",
        "the BOM must not survive into content"
    );
    assert_eq!(doc.text.as_ref().unwrap().bom, BomPresence::Utf8);
}

#[test]
fn a_utf16le_bom_decodes_to_the_right_text() {
    let dir = temp_dir("bom-utf16le");
    let path = dir.join("bommed.txt");
    // UTF-16LE BOM + "hi\n"
    fs::write(&path, [0xFFu8, 0xFE, b'h', 0, b'i', 0, b'\n', 0]).unwrap();
    let doc = load_path(&path, LoadOptions::default()).unwrap();
    assert_eq!(doc.diff_text(), "hi\n");
    assert_eq!(doc.text.as_ref().unwrap().bom, BomPresence::Utf16Le);
    assert_eq!(doc.text.as_ref().unwrap().encoding.label, "UTF-16LE");
}

/// Handoff 023 §6 test 2 — the one that proves §3 rather than merely
/// exercising it: a BOM'd file compared against an otherwise identical
/// non-BOM'd file must report **no difference on line 1**. Before RFC-083,
/// the BOM survived into `content` as a literal U+FEFF, so line 1 differed
/// with nothing visibly different. Falsify by reverting `load_path`'s
/// `FileKind::Text` branch to decode the raw bytes directly (no BOM strip)
/// — see the review request for the real failing output.
#[test]
fn a_bommed_file_diffs_identically_to_its_non_bommed_twin() {
    let dir = temp_dir("bom-diff-lie");
    let bommed = dir.join("bommed.txt");
    let plain = dir.join("plain.txt");
    fs::write(&bommed, [0xEFu8, 0xBB, 0xBF, b'h', b'i', b'\n']).unwrap();
    fs::write(&plain, b"hi\n").unwrap();

    let a = load_path(&bommed, LoadOptions::default()).unwrap();
    let b = load_path(&plain, LoadOptions::default()).unwrap();

    assert_eq!(
        a.diff_text(),
        b.diff_text(),
        "a BOM'd file and its non-BOM'd twin must produce identical \
         comparable text — otherwise the diff reports a line-1 change \
         with nothing visibly different"
    );
}
