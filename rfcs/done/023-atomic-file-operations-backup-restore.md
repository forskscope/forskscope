# RFC 023 — Atomic File Operations, Backup, and Restore

**Status.** Implemented (v0.44.0) — core complete; restore-from-manifest UI (restore picker dialog) deferred to UI layer

## Status
Partially implemented in v0.44.0. The `save_text` atomic write + sibling
backup + fingerprint conflict detection shipped in v0.27.0 (RFC-007). The
batch manifest layer ships in v0.44.0:

- **`BatchManifest`** — operation ID, app version, timestamp, per-entry
  `EntryOutcome` (Copied with backup path, Skipped, Failed). Serializes to
  deterministic JSON via `to_json()` / `write_to_dir()`.
- **`batch_copy`** — runs a `Vec<BatchItem>` with `BackupPolicy` and
  `BatchFailurePolicy` (StopOnFirst or ContinueOnFailure). Records every
  outcome; skips remaining entries on stop-on-first failure; returns the
  completed manifest.
- **`restore_from_manifest`** — copies each `.bak` backup back to its
  original destination path; skips entries without backups (new files).
- **`OperationId`** — `op-<unix_secs>-<pid>` format, used as the manifest
  filename anchor.
- **9 tests** covering all RFC-023 acceptance criteria.

Remaining open: the restore-from-manifest *UI* (the restore picker dialog
from RFC-023 §"Restore UI"), per-entry pre-confirmation (RFC-022 §preview),
and the digest-cache lifetime policy (RFC-037 open item).

## Summary

Define safe write, overwrite, backup, and restore behavior for file-level and directory-level merge operations.

## Problem

A diff/merge app can destroy user data if writes are performed casually. ForskScope must treat file writes as a safety-critical subsystem.

## Goals

- Provide atomic or best-effort-atomic writes.
- Detect external file modifications before saving.
- Create backups for risky writes.
- Support restore after failed or unwanted batch operations.
- Produce operation logs suitable for diagnostics and user trust.

## Non-goals

- Filesystem snapshot integration.
- Cloud backup.
- Full transactionality across all filesystems.
- Kernel-level atomic guarantees beyond what the platform supports.

## Save pipeline

```text
Prepare
  → validate target
  → re-stat source/target
  → detect external change
  → create backup if required
  → write temporary file
  → fsync temporary file where supported
  → atomic rename or platform-safe replace
  → fsync parent directory where supported
  → verify post-write metadata
  → record operation log
```

## File operation API

```rust
pub enum FileWriteIntent {
    SaveExisting,
    SaveAs,
    CopyLeftToRight,
    CopyRightToLeft,
    BatchDirectoryOperation,
}

pub struct WriteRequest {
    pub intent: FileWriteIntent,
    pub target_path: PathBuf,
    pub content: WriteContent,
    pub expected_fingerprint: Option<FileFingerprint>,
    pub backup_policy: BackupPolicy,
    pub newline_policy: NewlineWritePolicy,
    pub encoding_policy: EncodingWritePolicy,
}

pub enum WriteContent {
    Text { text: String },
    Bytes { bytes: Vec<u8> },
    CopyFromPath { source: PathBuf },
}

pub enum BackupPolicy {
    Always,
    OnOverwrite,
    OnRisk,
    NeverUnsafeForTestsOnly,
}
```

## Backup naming

Default backup pattern:

```text
<filename>.forskscope-backup.<yyyyMMdd-HHmmss>.<short-id>
```

Example:

```text
main.rs.forskscope-backup.20260608-103012.a1b2c3
```

For directory batch operations, backups should be grouped under an operation id where possible:

```text
.forskscope-backups/
  op-20260608-103012-a1b2c3/
    manifest.json
    src/main.rs
    docs/readme.md
```

## Restore manifest

```rust
pub struct RestoreManifest {
    pub operation_id: OperationId,
    pub created_at: SystemTime,
    pub app_version: String,
    pub entries: Vec<RestoreEntry>,
}

pub struct RestoreEntry {
    pub original_path: PathBuf,
    pub backup_path: PathBuf,
    pub operation: RestoreOperationKind,
    pub before_fingerprint: Option<FileFingerprint>,
    pub after_fingerprint: Option<FileFingerprint>,
}
```

## Save confirmation dialog

```text
+----------------------------------------------------------+
| Save Confirmation                                        |
+----------------------------------------------------------+
| Target: /right/src/main.rs                               |
| Status: file exists and was modified externally          |
| Action: overwrite target with current right buffer       |
| Backup: will create backup before overwrite              |
|                                                          |
| Risk: external change detected since file was opened.    |
|                                                          |
| [Cancel] [Show Details] [Save As...] [Overwrite Safely]  |
+----------------------------------------------------------+
```

## Restore UI

```text
+----------------------------------------------------------+
| Restore from Backup                                      |
+----------------------------------------------------------+
| Operation: op-20260608-103012-a1b2c3                     |
| Files: 8                                                 |
|                                                          |
| [ ] src/main.rs       restore previous version           |
| [ ] docs/readme.md    restore previous version           |
|                                                          |
| [Cancel] [Restore Selected]                              |
+----------------------------------------------------------+
```

## Failure policy

When an operation fails:

- Stop the current file operation.
- Continue batch only if the batch policy permits continuation.
- Record failure reason.
- Preserve all backups.
- Present recovery instructions.

## Acceptance criteria

- Save existing file uses temp write and safe replace.
- External modification is detected before overwrite.
- Backups are created for overwrites.
- Batch operation creates a manifest.
- Restore can recover at least text and binary copied files.
- Failure reports are visible to the user.
- Tests simulate permission failure and partial batch failure.

## Test strategy

- Temporary directory write tests.
- External modification tests.
- Backup creation tests.
- Restore manifest tests.
- Cross-platform path and rename behavior tests.
- Manual tests on Linux, Windows, macOS.

## Dependencies

- RFC 007 Save/session/file safety.
- RFC 017 Error taxonomy.
- RFC 021 Document/result-buffer model.
- RFC 022 Directory merge and batch operations.
