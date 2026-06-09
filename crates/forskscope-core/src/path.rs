//! Path model helpers (RFC-001 §6.1).
//!
//! The core never joins paths with a hard-coded separator. Display strings
//! are derived from `Path` values; canonicalization is lenient so that a
//! not-yet-existing save target still produces a stable identity.

use std::path::{Path, PathBuf};

/// Canonicalize when possible; otherwise normalize lexically against the
/// current working directory. Never fails: a save target may not exist yet.
pub fn canonicalize_lenient(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|_| path.to_path_buf())
            }
        }
    }
}

/// Human-readable display string for a path.
pub fn display(path: &Path) -> String {
    path.display().to_string()
}

/// Split a path into `(parent_display, file_name)`, both as display strings.
/// Used by UI headers that ellipsize the parent but never the file name.
pub fn split_parent_name(path: &Path) -> (String, String) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let parent = path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    (parent, name)
}

/// `true` when the path has the given extension (ASCII case-insensitive).
pub fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case(ext))
        .unwrap_or(false)
}
