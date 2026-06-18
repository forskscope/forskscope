//! Directory listing (RFC-005, RFC-020 of the roadmap).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

use crate::error::{CoreError, IoOperation, Result};

/// One file entry in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub len: u64,
    /// Human-readable size, e.g. `12.0 KB`.
    pub human_size: String,
    /// Exact byte count with separators, e.g. `12,288 bytes`.
    pub bytes_size: String,
    /// Local last-modified timestamp, or empty when unavailable.
    pub last_modified: String,
    /// `true` when a NUL-byte sniff classifies this file as binary (RFC-066).
    pub is_binary: bool,
}

/// A directory listing with directories and files separated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryListing {
    pub current_dir: PathBuf,
    pub dirs: Vec<String>,
    pub files: Vec<FileEntry>,
}

/// List a directory, separating subdirectories from files. `None` lists the
/// current working directory.
pub fn list_dir(path: Option<&Path>) -> Result<DirectoryListing> {
    let dir: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|e| CoreError::io("", IoOperation::ListDir, &e))?,
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|e| CoreError::io(&dir, IoOperation::ListDir, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(&dir, IoOperation::ListDir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            dirs.push(name);
        } else if meta.is_file() {
            let path = entry.path();
            let is_binary = matches!(crate::file_kind::classify(&path), Ok(crate::file_kind::FileKind::Binary));
            files.push(FileEntry {
                name,
                len: meta.len(),
                human_size: human_size(meta.len()),
                bytes_size: bytes_size(meta.len()),
                last_modified: last_modified(&meta),
                is_binary,
            });
        }
    }
    dirs.sort();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(DirectoryListing {
        current_dir: dir,
        dirs,
        files,
    })
}

fn human_size(len: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    if len < 1024 {
        return format!("{len} bytes");
    }
    let mut value = len as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn bytes_size(len: u64) -> String {
    let digits = len.to_string();
    let mut out = String::new();
    let bytes = digits.as_bytes();
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c as char);
    }
    format!("{out} bytes")
}

fn last_modified(meta: &fs::Metadata) -> String {
    meta.modified()
        .ok()
        .map(|t| {
            let dt: DateTime<Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_default()
}
