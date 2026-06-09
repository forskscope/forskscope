# RFC 021 — Document and Result Buffer Model

**Status.** Implemented (v0.28.0)

## Status

Proposed for Dioxus migration v3.

## Summary

Define the core model for opened files, compared documents, working buffers, result buffers, and merge output. This RFC prevents the editor surface from becoming the source of truth and establishes a stable foundation for editable diff/merge behavior.

## Background

The current application can display and merge content in memory, but the migration must support more reliable workflows:

- editable text panes,
- safe merge results,
- undo/redo,
- save conflict detection,
- directory-level batch operations,
- session persistence.

A WinMerge-like app cannot treat DOM text, `contenteditable`, or editor internal state as canonical. The Rust core must own the document state.

## Goals

- Define canonical Rust-owned document models.
- Separate original input, current working text, and save targets.
- Support left/right comparison and an optional result buffer.
- Enable merge transactions and undo/redo.
- Allow editor adapters to read/write through explicit model APIs.
- Preserve metadata required for save safety.

## Non-goals

- Full IDE text editing.
- Multi-user collaboration.
- Git merge conflict format support in this RFC.
- Language-specific parsing or semantic diff.

## External design

### User-facing concept

The user sees a comparison workspace with left and right documents. Depending on mode, the app may expose:

1. **Two-pane compare mode** — left and right files are compared directly.
2. **Two-pane merge mode** — one side is edited by applying changes from the other side.
3. **Result-buffer mode** — a separate output buffer is produced from selected merge decisions.

Initial migration may implement two-pane merge mode first and reserve result-buffer mode for later if UI complexity is high.

### Wireframe: two-pane merge mode

```text
+--------------------------------------------------------------------------------+
| Toolbar: Open | Compare | Prev Diff | Next Diff | Copy <- | Copy -> | Save ...  |
+-------------------------------+------------------------------------------------+
| Left: /path/old.txt           | Right: /path/new.txt                           |
| read-only or editable marker  | read-only or editable marker                   |
+-------------------------------+------------------------------------------------+
|  1 unchanged                  |  1 unchanged                                    |
|  2 - old line                 |  2 + new line                                    |
|  3 unchanged                  |  3 unchanged                                    |
+-------------------------------+------------------------------------------------+
| Status: 1 hunk selected | Right dirty | Backup enabled | UTF-8 | LF             |
+--------------------------------------------------------------------------------+
```

### Wireframe: result-buffer mode

```text
+--------------------------------------------------------------------------------+
| Toolbar: Open Pair | Recompute | Accept Left | Accept Right | Save Result ...   |
+----------------------+----------------------+----------------------------------+
| Left original         | Right original        | Result buffer                    |
+----------------------+----------------------+----------------------------------+
|  1 old                |  1 new                |  1 selected/edited result        |
|  2 old                |  2 new                |  2 selected/edited result        |
+----------------------+----------------------+----------------------------------+
| Status: result dirty | 3 unresolved hunks | output not saved                  |
+--------------------------------------------------------------------------------+
```

## Core data model

```rust
pub struct DocumentId(String);
pub struct BufferId(String);
pub struct ComparisonId(String);

pub struct DocumentSource {
    pub path: Option<PathBuf>,
    pub display_name: String,
    pub origin: DocumentOrigin,
    pub read_metadata: ReadMetadata,
}

pub enum DocumentOrigin {
    FileSystem,
    Temporary,
    SessionRestore,
    GeneratedResult,
}

pub struct ReadMetadata {
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub file_fingerprint: FileFingerprint,
    pub encoding: EncodingKind,
    pub newline: NewlineKind,
    pub binary_kind: BinaryKind,
}

pub struct TextBuffer {
    pub id: BufferId,
    pub source_document: Option<DocumentId>,
    pub text: String,
    pub normalized_text: Option<String>,
    pub dirty: bool,
    pub revision: u64,
}

pub struct ComparisonSession {
    pub id: ComparisonId,
    pub left_document: DocumentId,
    pub right_document: DocumentId,
    pub left_buffer: BufferId,
    pub right_buffer: BufferId,
    pub result_buffer: Option<BufferId>,
    pub diff_revision: u64,
    pub merge_mode: MergeMode,
}

pub enum MergeMode {
    CompareOnly,
    EditLeft,
    EditRight,
    ResultBuffer,
}
```

## Model ownership rule

The editor adapter may hold a local editor document for rendering and cursor behavior, but the Rust model remains canonical.

```text
Editor event
  → EditorAdapterEvent
  → Core command
  → Core model mutation
  → Model revision increment
  → UI/editor refresh instruction
```

The reverse direction is also explicit:

```text
Core merge command
  → TextBuffer mutation
  → Diff invalidation
  → Editor patch instruction
```

## Required core commands

```rust
pub enum DocumentCommand {
    OpenFile { path: PathBuf },
    CreateComparison { left: DocumentId, right: DocumentId },
    ApplyTextEdit { buffer: BufferId, edit: TextEdit },
    ApplyMergeHunk { comparison: ComparisonId, hunk: HunkId, direction: MergeDirection },
    RecomputeDiff { comparison: ComparisonId },
    SaveBuffer { buffer: BufferId, target: SaveTarget },
    CloseComparison { comparison: ComparisonId },
}
```

## Result-buffer policy

The result buffer is optional in the first Dioxus release. However, the core should be designed so that result-buffer mode can be added without replacing two-pane merge.

Recommended staged approach:

```text
Stage 1: CompareOnly + EditLeft/EditRight
Stage 2: ResultBuffer internal model
Stage 3: ResultBuffer UI
Stage 4: three-pane merge workflow, only if needed
```

## Save metadata

Each buffer must know:

- whether it is dirty,
- whether its source file changed externally,
- whether it has a writable target,
- whether a backup is required,
- whether newline/encoding preservation is possible.

## Acceptance criteria

- Opening two text files creates two documents and two buffers.
- Comparison sessions reference buffers, not raw paths.
- Editor edits update the core model revision.
- Merge actions update buffers through transactions.
- Dirty state is model-owned.
- Save preflight can inspect source metadata and current buffer state.
- Unit tests prove that DOM/editor state is not required to perform merge commands.

## Test strategy

- Unit tests for document creation.
- Unit tests for edit application.
- Unit tests for merge hunk mutation.
- Snapshot tests for serialized session metadata.
- Property-style tests for revision monotonicity.
- Integration tests using the acceptance corpus.

## Dependencies

- RFC 001 Core extraction.
- RFC 002 similar v3 diff engine.
- RFC 015 Undo/redo transaction log.
- RFC 023 Atomic file operations.
- RFC 025 Editor adapter prototype.

## Deferred work (v0.28.0)

Full history persistence across restarts is deferred. Drag-and-drop to set directory paths is deferred.
