# RFC 036: Live Reload, File Watcher, and External Modification Handling

**Status.** Proposed — external file state detection slice implemented (v0.53.0); file watcher and reconciliation UI open

## Status
Partially implemented in v0.53.0:

- **`ExternalFileState`** in `forskscope-core::document` — the six-state
  enum RFC-036 specifies: `Clean`, `DirtyInSession`, `ChangedOnDisk`,
  `DeletedOnDisk`, `ReplacedOnDisk`, `Unknown`. Predicates:
  `blocks_save()` (true for Changed/Deleted/Replaced) and
  `file_accessible()` (true for Clean/DirtyInSession/ChangedOnDisk).
- **`check_external_state(path, snapshot, is_session_dirty)`** — compares
  a live `fs::metadata` call against the `FileFingerprint` captured at load
  time. Detects: missing file (DeletedOnDisk), non-file replacement
  (ReplacedOnDisk), size change (ChangedOnDisk), mtime change
  (ChangedOnDisk where mtime resolution is sufficient). Never panics —
  metadata errors return `Unknown`. Called by the UI's save interlock.
- **15 tests** covering all RFC-036 acceptance criteria.

Remaining open: the `FileChangeMonitor` trait and platform file-watcher
backend (optimization layer; save safety must never rely solely on watcher
events per RFC-036 §"Watcher Boundary"), the reconciliation dialog UI
(Compare/Reload/Save As/Cancel), and the `ReplacedOnDisk` detection on
Windows (rename/replace sequences differ — needs platform CI).

## Summary

Define how ForskScope detects and reconciles files changed outside the application while a compare or merge session is open.

## Motivation

Diff/merge tools are often used while editors, build tools, VCS tools, or generators are also modifying files. ForskScope must not silently overwrite external changes. It must detect stale snapshots and offer safe reconciliation paths.

## Goals

- Detect external modifications for loaded files.
- Compare loaded snapshot with current disk state before save.
- Provide clear reload/reconcile choices.
- Avoid disruptive automatic reload during active edits.
- Support platforms where file watcher behavior differs.

## Non-Goals

- Build a full real-time collaboration system.
- Guarantee every filesystem event is delivered by the OS.
- Automatically merge all external changes.

## External Design

### Status Bar Indicators

```text
File state:
  clean              loaded snapshot matches disk
  dirty              user changes not saved
  externally changed disk changed since load
  missing            file was deleted or moved
  unknown            watcher unavailable; save will re-stat
```

### Reconciliation Dialog

```text
+--------------------------------------------------------------------+
| File Changed Outside ForskScope                                    |
+--------------------------------------------------------------------+
| The file has changed on disk since it was opened.                  |
|                                                                    |
| File: /project/src/main.rs                                         |
| Loaded: 2026-06-08 09:10:21, digest abc123                         |
| Disk:   2026-06-08 09:22:04, digest def456                         |
|                                                                    |
| Choose how to continue:                                            |
| [Compare Session Version with Disk Version]                        |
| [Reload from Disk and Discard Session Changes]                     |
| [Keep Session and Save As...]                                      |
| [Cancel]                                                           |
+--------------------------------------------------------------------+
```

### Save Interlock

Before overwriting a file:

```text
1. stat current file
2. compare with loaded snapshot
3. if changed, block overwrite
4. show reconciliation dialog
5. continue only through explicit user choice
```

## Internal Design

### File Snapshot

```rust
pub struct FileSnapshot {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub digest: ContentDigest,
    pub platform_file_id: Option<PlatformFileId>,
}
```

### File State

```rust
pub enum ExternalFileState {
    Clean,
    DirtyInSession,
    ChangedOnDisk,
    DeletedOnDisk,
    ReplacedOnDisk,
    Unknown,
}
```

### Watcher Boundary

Use a file watching backend as an optimization only. Save safety must never rely solely on watcher events.

```rust
pub trait FileChangeMonitor {
    fn watch(&mut self, path: &Path) -> Result<WatchToken, WatchError>;
    fn poll_events(&mut self) -> Vec<FileChangeEvent>;
}
```

## Reconciliation Actions

| Action | Behavior |
|---|---|
| Compare with Disk | Open a new diff between session buffer and current disk contents |
| Reload from Disk | Replace session document after confirmation |
| Keep and Save As | Preserve session changes by writing to a new path |
| Cancel | Return to session without writing |

## Platform Considerations

- Linux file watcher behavior may differ across native FS, network FS, and container mounts.
- Windows may report rename/replace sequences differently.
- macOS may coalesce events.
- Therefore, pre-save stat/digest verification is mandatory.

## Acceptance Criteria

- External disk changes are detected in common cases.
- Save refuses to overwrite changed files without explicit reconciliation.
- Missing file state is shown clearly.
- Watcher failure degrades to pre-save verification.
- Reconciliation actions are covered by tests.

## Dependencies

- RFC 007 — Save, Session, and File Safety
- RFC 023 — Atomic File Operations, Backup, and Restore
- RFC 032 — Text Editing Operation Model
