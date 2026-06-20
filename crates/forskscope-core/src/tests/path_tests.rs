//! Tests for path helper utilities (RFC-001 §6.1).

use std::path::{Path, PathBuf};

use crate::path::{canonicalize_lenient, display, has_extension, split_parent_name};

// ── split_parent_name ─────────────────────────────────────────────────────────

#[test]
fn split_parent_name_typical_path() {
    let (parent, name) = split_parent_name(Path::new("/home/user/project/main.rs"));
    assert_eq!(name,   "main.rs");
    assert!(parent.ends_with("project"), "parent should end with 'project'");
}

#[test]
fn split_parent_name_root_file() {
    let (parent, name) = split_parent_name(Path::new("/file.txt"));
    assert_eq!(name,   "file.txt");
    assert_eq!(parent, "/");
}

#[test]
fn split_parent_name_relative_path() {
    let (parent, name) = split_parent_name(Path::new("src/lib.rs"));
    assert_eq!(name,   "lib.rs");
    assert_eq!(parent, "src");
}

#[test]
fn split_parent_name_filename_only() {
    let (parent, name) = split_parent_name(Path::new("README.md"));
    assert_eq!(name,   "README.md");
    assert_eq!(parent, "");
}

#[test]
fn split_parent_name_dotfile() {
    let (_parent, name) = split_parent_name(Path::new("/home/user/.gitignore"));
    assert_eq!(name, ".gitignore");
}

// ── has_extension ─────────────────────────────────────────────────────────────

#[test]
fn has_extension_matches_exact() {
    assert!(has_extension(Path::new("file.rs"), "rs"));
}

#[test]
fn has_extension_case_insensitive() {
    assert!(has_extension(Path::new("data.XLSX"), "xlsx"),
        "extension check must be ASCII case-insensitive");
    assert!(has_extension(Path::new("data.xlsx"), "XLSX"));
}

#[test]
fn has_extension_no_match() {
    assert!(!has_extension(Path::new("file.rs"), "txt"));
}

#[test]
fn has_extension_no_extension() {
    assert!(!has_extension(Path::new("Makefile"), "mk"));
}

#[test]
fn has_extension_dotfile_no_extension() {
    // ".gitignore" has no extension in Rust's Path model
    assert!(!has_extension(Path::new(".gitignore"), "gitignore"));
}

#[test]
fn has_extension_xlsx_matches_xlsx() {
    assert!(has_extension(Path::new("/path/to/sheet.xlsx"), "xlsx"));
}

// ── display ──────────────────────────────────────────────────────────────────

#[test]
fn display_returns_path_string() {
    let p = PathBuf::from("/home/user/file.txt");
    let s = display(&p);
    assert!(s.contains("file.txt"));
}

#[test]
fn display_is_non_empty_for_non_empty_path() {
    let s = display(Path::new("some/path"));
    assert!(!s.is_empty());
}

// ── canonicalize_lenient ──────────────────────────────────────────────────────

#[test]
fn canonicalize_lenient_nonexistent_absolute_returns_input() {
    // A path that definitely doesn't exist — absolute so no cwd join
    let p = PathBuf::from("/this/path/definitely/does/not/exist/file.txt");
    let result = canonicalize_lenient(&p);
    // Should return the input unchanged (absolute, canonicalize failed)
    assert_eq!(result, p);
}

#[test]
fn canonicalize_lenient_existing_path_is_canonical() {
    // Use a path we know exists: /tmp
    let p = Path::new("/tmp");
    if p.exists() {
        let result = canonicalize_lenient(p);
        assert!(result.is_absolute(), "canonicalized path must be absolute");
    }
}

#[test]
fn canonicalize_lenient_never_panics_on_odd_input() {
    // Various edge cases — must not panic
    for path in &[
        Path::new(""),
        Path::new("."),
        Path::new(".."),
        Path::new("/"),
        Path::new("/nonexistent/../nonexistent"),
    ] {
        let _ = canonicalize_lenient(path); // must not panic
    }
}
