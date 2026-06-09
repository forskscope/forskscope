# RFC 037: Scalable Directory Compare Index and Incremental Refresh

**Status.** Proposed

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define a scalable model for directory comparison, indexing, incremental refresh, and batch operation previews.

## Motivation

Directory comparison can become expensive quickly. Users may compare source trees, generated output, configuration directories, or release packages. The UI must remain responsive, results must be explainable, and destructive batch operations must be previewed.

## Goals

- Support large directory trees without blocking the UI.
- Separate scanning, indexing, comparison, and rendering.
- Support incremental refresh after file changes.
- Provide cancellation and progress reporting.
- Support batch copy/delete/merge through preview.

## Non-Goals

- Replace rsync or backup tools.
- Provide network synchronization.
- Automatically delete or overwrite files without preview.

## External Design

### Directory Compare Workspace

```text
+--------------------------------------------------------------------------------+
| Directory Compare: /left/project  <->  /right/project                          |
+--------------------------------------------------------------------------------+
| Toolbar: [Refresh] [Stop] [Filter] [Copy ->] [<- Copy] [Export Report]          |
+-------------------------------+------------------------------------------------+
| Summary                       | Results                                        |
| Equal: 1820                   | path                      state      action   |
| Modified: 42                  | src/main.rs               modified   review   |
| Left only: 18                 | docs/old.md               left only  copy ->  |
| Right only: 7                 | target/generated.txt      right only ignore   |
| Conflicts: 3                  | config/app.toml           conflict   merge    |
+-------------------------------+------------------------------------------------+
| Progress: hashing 1240 / 1887 files | [Cancel]                               |
+--------------------------------------------------------------------------------+
```

## Internal Design

### Directory Index

```rust
pub struct DirectoryIndex {
    pub root: PathBuf,
    pub revision: IndexRevision,
    pub entries: Vec<DirectoryEntryRecord>,
    pub digest_policy: DigestPolicy,
    pub ignore_policy: IgnorePolicy,
}

pub struct DirectoryEntryRecord {
    pub relative_path: RelativePath,
    pub entry_type: EntryType,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub digest: Option<ContentDigest>,
    pub error: Option<EntryError>,
}
```

### Compare Result

```rust
pub enum DirectoryCompareState {
    Equal,
    LeftOnly,
    RightOnly,
    ModifiedMetadata,
    ModifiedContent,
    TypeMismatch,
    Unreadable,
    Ignored,
}
```

### Job Pipeline

```text
scan left tree
scan right tree
normalize paths
match entries by relative path
quick compare by type/size/mtime
hash candidate modified files
produce compare records
render incrementally
```

## Incremental Refresh

A refresh should update only affected paths where possible:

```rust
pub struct RefreshRequest {
    pub paths: Vec<RelativePath>,
    pub reason: RefreshReason,
}
```

If incremental refresh becomes unreliable, the app may fall back to full rescan with user-visible status.

## Batch Operation Preview

Before applying batch actions:

```text
+------------------------------------------------------+
| Batch Operation Preview                              |
+------------------------------------------------------+
| 18 files will be copied left -> right                |
| 2 files will be overwritten                          |
| 1 directory will be created                          |
| 0 files will be deleted                              |
|                                                      |
| [Export Plan] [Cancel] [Apply with Backup]           |
+------------------------------------------------------+
```

## Acceptance Criteria

- Directory compare jobs are cancellable.
- UI remains responsive during scanning and hashing.
- Incremental refresh works for changed, added, and removed files.
- Batch operations require preview.
- Errors on individual files do not abort the whole comparison.

## Dependencies

- RFC 008 — Directory Comparison Background Jobs
- RFC 022 — Directory Merge and Batch Operations
- RFC 023 — Atomic File Operations
- RFC 036 — File Watcher and External Modification Handling
