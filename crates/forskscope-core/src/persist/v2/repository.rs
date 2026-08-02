//! Shared file-write mechanics for the RFC-076 explicit-path repositories.
//!
//! Operates on already-serialized strings and bytes, not on `T: Serialize`
//! generically — with only two concrete repositories ([`super::settings`],
//! [`super::session`]), a generic repository type would trade a small amount
//! of duplication for a layer of indirection neither call site needs. What
//! *is* shared (envelope construction, atomic write, the migration-backup
//! dance) lives here so both repositories call the same tested primitives.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Outcome of a durable write. `backup_path` is set only by
/// [`ensure_pre_v2_backup`]-based commits — an ordinary [`save`] has nothing
/// to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceSaveOutcome {
    pub backup_path: Option<PathBuf>,
}

/// A file-write failure. Deliberately not [`crate::error::CoreError`]: that
/// type's variants (`Conflict`, document-oriented `IoOperation`s) are shaped
/// for document saves under RFC-007, not preference files with no fingerprint
/// or dirty-content concept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceIoError(pub String);

impl std::fmt::Display for PersistenceIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PersistenceIoError {}

/// Reads `path`, distinguishing "does not exist" from every other failure —
/// the latter must not be silently treated as an absent file (see
/// [`super::PersistenceError::Io`]).
pub(super) fn read_to_string_or_missing(path: &Path) -> Result<Option<String>, PersistenceIoError> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(Some(raw)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(PersistenceIoError(e.to_string())),
    }
}

/// Builds the pretty-printed v2 envelope JSON for `payload`. `created_unix`
/// is read from whatever currently exists at `path` (best-effort — any
/// failure falls back to now; this is a cosmetic timestamp, not a
/// correctness concern), so a resave does not reset the file's original
/// creation time.
pub(super) fn build_envelope_json<T: Serialize>(
    schema_name: &str,
    schema_version: u32,
    path: &Path,
    payload: &T,
) -> String {
    let created_unix = existing_created_unix(path).unwrap_or_else(unix_now);
    let value = serde_json::json!({
        "schema_name": schema_name,
        "schema_version": schema_version,
        "app_version": env!("CARGO_PKG_VERSION"),
        "created_unix": created_unix,
        "updated_unix": unix_now(),
        "payload": payload,
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

fn existing_created_unix(path: &Path) -> Option<u64> {
    let raw = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value.get("created_unix")?.as_u64()
}

/// Atomically writes `json` to `path` (temp file in the same directory, then
/// rename), reusing the same primitive [`crate::save::save_text`] uses for
/// document saves.
pub(super) fn atomic_write_envelope(path: &Path, json: &str) -> Result<(), PersistenceIoError> {
    crate::save::atomic_replace(path, json.as_bytes())
        .map_err(|e| PersistenceIoError(e.to_string()))
}

/// Copies `original_bytes` to `<name>.pre-v2.bak` next to `path`, unless that
/// backup already exists. RFC-076: "without overwriting an existing
/// backup" — a retried migration commit (e.g. after a crash between the
/// backup and the atomic replace) must not clobber the first attempt's
/// preserved original with a second, possibly-already-migrated, copy.
/// Returns the backup path whether it was just created or already existed.
pub(super) fn ensure_pre_v2_backup(
    path: &Path,
    original_bytes: &[u8],
) -> Result<PathBuf, PersistenceIoError> {
    let backup = pre_v2_backup_path(path);
    if !backup.exists() {
        fs::write(&backup, original_bytes).map_err(|e| PersistenceIoError(e.to_string()))?;
    }
    Ok(backup)
}

fn pre_v2_backup_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "forskscope".into());
    let backup_name = format!("{name}.pre-v2.bak");
    match path.parent() {
        Some(parent) => parent.join(backup_name),
        None => PathBuf::from(backup_name),
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
