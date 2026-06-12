# RFC 032: Text Editing Operation Model and Editor Truth Boundary

**Status.** Proposed — TextEditOperation, RevisionId, TextRange, OperationAck/Reject, EditTransaction slice implemented (v0.62.0); EditBuffer and operation dispatch open

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define the authoritative text editing model for ForskScope. The Dioxus/CodeMirror editor surface may display and collect user edits, but it must not become the authoritative product state. All edits must cross an adapter boundary as typed operations owned and validated by `forskscope-core`.

## Motivation

A diff/merge application is not a normal note editor. User edits affect hunk mapping, merge decisions, save safety, undo/redo, conflict resolution, and patch generation. If the editor DOM becomes product truth, the application can lose the ability to explain, replay, validate, or safely save user changes.

This RFC addresses the concern that Iced may require a difficult custom text editor while Dioxus can leverage a mature web editor. The selected solution is Dioxus plus an explicit operation contract.

## Goals

- Support direct text edits in diff/merge panes.
- Keep document truth in Rust core.
- Represent editor actions as typed operations.
- Support deterministic replay and undo/redo.
- Preserve newline, encoding, and binary policies from the core.
- Enable future non-Dioxus UI backends by making the editor operation model GUI-independent.

## Non-Goals

- Implement a general IDE.
- Expose filesystem access to JavaScript.
- Trust CodeMirror internal state as durable merge state.
- Require a custom Iced editor in this migration phase.

## External Design

### User-Facing Behavior

Users can edit text in the result/editor pane depending on workspace mode:

```text
Compare Mode
  - left and right may be read-only by default
  - users can open a result buffer for merge output

Merge Mode
  - result buffer is editable
  - copy left/right hunk commands create core operations
  - manual edits create core operations

Manual Edit Mode
  - user may edit a selected side or result buffer
  - dirty state appears immediately in tab and status bar
```

### Wireframe

```text
+----------------------------------------------------------------------------------+
| ForskScope | Compare | Merge | Search | View | Help                              |
+----------------------------------------------------------------------------------+
| Toolbar: [Open Left] [Open Right] [Prev] [Next] [Copy ->] [<- Copy] [Save Result] |
+--------------------------+-------------------------------------------------------+
| Explorer / Sessions      | Diff/Merge Workspace                                  |
|                          | +----------------------+----------------------+       |
|                          | | Left Editor          | Right / Result      |       |
|                          | | line numbers         | line numbers        |       |
|                          | | read-only/editable   | editable if result  |       |
|                          | +----------------------+----------------------+       |
|                          | Status: rev=42 dirty=true external=clean encoding=UTF-8|
+--------------------------+-------------------------------------------------------+
```

## Internal Design

### Core Types

```rust
pub struct DocumentId(String);
pub struct RevisionId(u64);

pub struct TextDocument {
    pub id: DocumentId,
    pub revision: RevisionId,
    pub text: RopeText,
    pub encoding: EncodingPolicy,
    pub newline: NewlinePolicy,
    pub source_snapshot: Option<FileSnapshot>,
}

pub enum TextEditOperation {
    Insert {
        document: DocumentId,
        base_revision: RevisionId,
        offset: TextOffset,
        text: String,
    },
    Delete {
        document: DocumentId,
        base_revision: RevisionId,
        range: TextRange,
    },
    Replace {
        document: DocumentId,
        base_revision: RevisionId,
        range: TextRange,
        text: String,
    },
}

pub struct OperationAck {
    pub document: DocumentId,
    pub new_revision: RevisionId,
    pub affected_range: TextRange,
    pub diff_invalidated: bool,
}
```

### Operation Rules

1. The editor sends an operation with the document revision it observed.
2. The core accepts the operation only if the base revision is compatible.
3. The core applies the edit and returns a new revision.
4. The UI updates editor state from the acknowledged revision.
5. If the operation conflicts with a newer revision, the core rejects it and the UI reconciles.

### Transaction Model

```rust
pub struct EditTransaction {
    pub id: TransactionId,
    pub label: TransactionLabel,
    pub operations: Vec<TextEditOperation>,
    pub inverse: Vec<TextEditOperation>,
    pub timestamp: SystemTime,
}
```

Merge commands and manual edits both become transactions. This allows undo/redo to remain consistent across user text edits and merge actions.

## Dioxus / Editor Adapter Contract

```text
EditorAdapter responsibilities:
  - collect user edits
  - translate editor changes to core operations
  - render text from acknowledged core revisions
  - apply decorations from core-derived diff state
  - report selection/cursor changes
  - never read or write files directly
```

```text
Core responsibilities:
  - own text truth
  - validate edits
  - maintain revisions
  - compute dirty state
  - trigger diff invalidation/recomputation
  - own undo/redo transactions
```

## Error Handling

| Error | UI Behavior |
|---|---|
| Revision mismatch | Show non-destructive reconciliation prompt |
| Invalid range | Reject operation, log diagnostic, refresh editor from core |
| Encoding violation | Block edit/save or convert through explicit policy |
| Large edit threshold exceeded | Ask user to continue in large-file/manual mode |

## Acceptance Criteria

- Manual insert/delete/replace operations can be replayed deterministically.
- Copy-hunk merge actions and manual edits share the same undo/redo model.
- Editor state can be reconstructed from core state after UI reload.
- A test can simulate editor operations without launching Dioxus or WebView.
- The JavaScript bridge cannot access arbitrary files.

## Dependencies

- RFC 015 — Undo/Redo Transaction Log
- RFC 016 — Editor Bridge Security and Contract
- RFC 021 — Document and Result Buffer Model
- RFC 024 — Diff Visual Semantics Decoration Contract
- RFC 040 — Editor Adapter Verification Harness and Golden Corpus
