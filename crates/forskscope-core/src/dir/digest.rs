//! Digest equality comparison (RFC-008, RFC-023 of the v0.22 baseline).
//!
//! These are exact byte-equality checks (size first, then streamed
//! comparison), matching v0.22.x explorer indicator semantics. They are
//! intentionally synchronous and uncached; the UI runs them as background
//! jobs (RFC-008) and may add caching later (RFC-037).

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::error::{CoreError, IoOperation, Result};

const BUFFER: usize = 8 * 1024;

/// Byte-equality of two files: compare sizes, then stream-compare contents.
pub fn file_digest_equal(a: &Path, b: &Path) -> Result<bool> {
    let meta_a = fs::metadata(a).map_err(|e| CoreError::io(a, IoOperation::Metadata, &e))?;
    let meta_b = fs::metadata(b).map_err(|e| CoreError::io(b, IoOperation::Metadata, &e))?;
    if meta_a.len() != meta_b.len() {
        return Ok(false);
    }
    let mut fa = File::open(a).map_err(|e| CoreError::io(a, IoOperation::Read, &e))?;
    let mut fb = File::open(b).map_err(|e| CoreError::io(b, IoOperation::Read, &e))?;
    let mut buf_a = [0u8; BUFFER];
    let mut buf_b = [0u8; BUFFER];
    loop {
        let na = fa
            .read(&mut buf_a)
            .map_err(|e| CoreError::io(a, IoOperation::Read, &e))?;
        let nb = fb
            .read(&mut buf_b)
            .map_err(|e| CoreError::io(b, IoOperation::Read, &e))?;
        if na != nb {
            return Ok(false);
        }
        if na == 0 {
            return Ok(true);
        }
        if buf_a[..na] != buf_b[..nb] {
            return Ok(false);
        }
    }
}

/// Recursive byte-equality of two directories: equal names and equal
/// contents at every level. Symlinks are followed by the underlying
/// metadata calls; cycle protection is left to the caller's depth limits.
pub fn dir_digest_equal(a: &Path, b: &Path) -> Result<bool> {
    let (mut dirs_a, mut files_a) = read_split(a)?;
    let (mut dirs_b, mut files_b) = read_split(b)?;
    dirs_a.sort();
    dirs_b.sort();
    files_a.sort();
    files_b.sort();
    if dirs_a != dirs_b || files_a != files_b {
        return Ok(false);
    }
    for f in &files_a {
        if !file_digest_equal(&a.join(f), &b.join(f))? {
            return Ok(false);
        }
    }
    for d in &dirs_a {
        if !dir_digest_equal(&a.join(d), &b.join(d))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn read_split(dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(dir, IoOperation::ListDir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry
            .metadata()
            .map_err(|e| CoreError::io(dir, IoOperation::Metadata, &e))?;
        if meta.is_dir() {
            dirs.push(name);
        } else if meta.is_file() {
            files.push(name);
        }
    }
    Ok((dirs, files))
}
