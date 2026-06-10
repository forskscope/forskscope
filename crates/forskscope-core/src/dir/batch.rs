//! Batch file-copy with a restore manifest (RFC-023 §"Batch operation manifest").
//!
//! Single-file copy already exists in [`super::copy`]. This module adds
//! the *batch* layer: an atomic operation ID, per-entry outcome tracking,
//! a stop-or-continue failure policy, and a JSON manifest written to the
//! backup directory so every batch operation is reversible from first
//! principles.
//!
//! The manifest is written *after* all attempted copies so it reflects the
//! actual outcome rather than the plan. Backups created by successful copies
//! are preserved even when a later entry fails.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dir::copy::copy_file;
use crate::error::{CoreError, IoOperation, Result};
use crate::save::BackupPolicy;

// ── Public types ──────────────────────────────────────────────────────────────

/// Unique identifier for one batch operation, used in the manifest filename
/// and as a human-readable anchor in diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationId(pub String);

impl OperationId {
    /// Generate a new ID from the current wall time: `op-<unix_secs>-<pid>`.
    pub fn new() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self(format!("op-{secs}-{}", std::process::id()))
    }
}

impl Default for OperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What to do when one entry in a batch fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatchFailurePolicy {
    /// Stop the batch at the first failure. Later entries are not attempted.
    #[default]
    StopOnFirst,
    /// Continue attempting remaining entries; collect all failures.
    ContinueOnFailure,
}

/// One planned copy in a batch.
#[derive(Debug, Clone)]
pub struct BatchItem {
    pub src: PathBuf,
    pub dst: PathBuf,
}

/// Outcome of one entry.
#[derive(Debug, Clone)]
pub enum EntryOutcome {
    Copied {
        bytes:       u64,
        backup_path: Option<PathBuf>,
    },
    Skipped {
        reason: String,
    },
    Failed {
        error: String,
    },
}

impl EntryOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Copied { .. })
    }
}

/// Per-entry record in the manifest.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    pub src:     PathBuf,
    pub dst:     PathBuf,
    pub outcome: EntryOutcome,
}

/// The restore manifest written to disk at the end of a batch.
#[derive(Debug, Clone)]
pub struct BatchManifest {
    pub operation_id:     OperationId,
    pub app_version:      String,
    pub created_unix_sec: u64,
    pub entries:          Vec<ManifestEntry>,
    /// Path where the manifest JSON was written, set after [`BatchResult::write_manifest`].
    pub manifest_path:    Option<PathBuf>,
}

impl BatchManifest {
    fn new(op_id: OperationId) -> Self {
        Self {
            operation_id: op_id,
            app_version: env!("CARGO_PKG_VERSION").into(),
            created_unix_sec: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            entries: Vec::new(),
            manifest_path: None,
        }
    }

    /// Total entries attempted (Copied + Failed; not Skipped).
    pub fn attempted(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !matches!(e.outcome, EntryOutcome::Skipped { .. }))
            .count()
    }

    pub fn succeeded(&self) -> usize {
        self.entries.iter().filter(|e| e.outcome.is_success()).count()
    }

    pub fn failed(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.outcome, EntryOutcome::Failed { .. }))
            .count()
    }

    /// All backup paths created by this batch (for restore or cleanup).
    pub fn backup_paths(&self) -> Vec<&PathBuf> {
        self.entries
            .iter()
            .filter_map(|e| {
                if let EntryOutcome::Copied { backup_path: Some(bp), .. } = &e.outcome {
                    Some(bp)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Serialize to a deterministic JSON string (no serde dependency).
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{{");
        let _ = writeln!(s, "  \"operation_id\": {:?},", self.operation_id.0);
        let _ = writeln!(s, "  \"app_version\": {:?},", self.app_version);
        let _ = writeln!(s, "  \"created_unix_sec\": {},", self.created_unix_sec);
        let _ = writeln!(s, "  \"entries\": [");
        for (i, entry) in self.entries.iter().enumerate() {
            let comma = if i + 1 < self.entries.len() { "," } else { "" };
            let _ = writeln!(s, "    {{");
            let _ = writeln!(s, "      \"src\": {:?},", entry.src.display().to_string());
            let _ = writeln!(s, "      \"dst\": {:?},", entry.dst.display().to_string());
            match &entry.outcome {
                EntryOutcome::Copied { bytes, backup_path } => {
                    let _ = writeln!(s, "      \"outcome\": \"copied\",");
                    let _ = writeln!(s, "      \"bytes\": {bytes},");
                    let bp = backup_path
                        .as_ref()
                        .map(|p| format!("{:?}", p.display().to_string()))
                        .unwrap_or_else(|| "null".into());
                    let _ = writeln!(s, "      \"backup_path\": {bp}");
                }
                EntryOutcome::Skipped { reason } => {
                    let _ = writeln!(s, "      \"outcome\": \"skipped\",");
                    let _ = writeln!(s, "      \"reason\": {:?}", reason);
                }
                EntryOutcome::Failed { error } => {
                    let _ = writeln!(s, "      \"outcome\": \"failed\",");
                    let _ = writeln!(s, "      \"error\": {:?}", error);
                }
            }
            let _ = writeln!(s, "    }}{comma}");
        }
        let _ = writeln!(s, "  ]");
        let _ = write!(s, "}}");
        s
    }

    /// Write the manifest JSON to `dir/<operation_id>.json`. Stores the
    /// resulting path in `self.manifest_path`.
    pub fn write_to_dir(&mut self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)
            .map_err(|e| CoreError::io(dir, IoOperation::Write, &e))?;
        let path = dir.join(format!("{}.json", self.operation_id.0));
        fs::write(&path, self.to_json())
            .map_err(|e| CoreError::io(&path, IoOperation::Write, &e))?;
        self.manifest_path = Some(path);
        Ok(())
    }
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Execute a batch of file copies with backup and a restore manifest.
///
/// Each successful copy creates a `.bak` sibling of the destination (when
/// the destination exists) using the same policy as single-file save.
/// The manifest is written to `manifest_dir` when provided; if `None` the
/// manifest is returned but not persisted.
///
/// Returns the completed manifest. Check `manifest.failed()` to determine
/// whether any entries failed.
pub fn batch_copy(
    items:         &[BatchItem],
    backup:        BackupPolicy,
    failure_policy: BatchFailurePolicy,
    manifest_dir:  Option<&Path>,
) -> Result<BatchManifest> {
    let op_id = OperationId::new();
    let mut manifest = BatchManifest::new(op_id);

    for item in items {
        match copy_file(&item.src, &item.dst, backup) {
            Ok(outcome) => {
                manifest.entries.push(ManifestEntry {
                    src: item.src.clone(),
                    dst: item.dst.clone(),
                    outcome: EntryOutcome::Copied {
                        bytes:       outcome.bytes_copied,
                        backup_path: outcome.backup_path,
                    },
                });
            }
            Err(e) => {
                manifest.entries.push(ManifestEntry {
                    src: item.src.clone(),
                    dst: item.dst.clone(),
                    outcome: EntryOutcome::Failed { error: e.to_string() },
                });
                if failure_policy == BatchFailurePolicy::StopOnFirst {
                    // Mark remaining items as skipped.
                    let remaining_start = manifest.entries.len();
                    for skipped in &items[remaining_start..] {
                        manifest.entries.push(ManifestEntry {
                            src: skipped.src.clone(),
                            dst: skipped.dst.clone(),
                            outcome: EntryOutcome::Skipped {
                                reason: "batch stopped on earlier failure".into(),
                            },
                        });
                    }
                    break;
                }
            }
        }
    }

    if let Some(dir) = manifest_dir {
        manifest.write_to_dir(dir)?;
    }

    Ok(manifest)
}

/// Attempt to restore files from a manifest by copying each backup back to
/// its original destination. Returns the number of successfully restored entries.
///
/// Entries without a backup path (e.g. newly created files, skipped entries)
/// are silently skipped; they were never backed up so cannot be restored
/// by this path.
pub fn restore_from_manifest(manifest: &BatchManifest) -> usize {
    let mut restored = 0;
    for entry in &manifest.entries {
        if let EntryOutcome::Copied { backup_path: Some(bp), .. } = &entry.outcome {
            if bp.exists() {
                if fs::copy(bp, &entry.dst).is_ok() {
                    restored += 1;
                }
            }
        }
    }
    restored
}
