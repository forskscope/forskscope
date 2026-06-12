# RFC 035: Scroll Sync, Line Mapping, and Diff Decoration Engine

**Status.** Implemented (v0.61.0) — core complete; scroll-sync wiring in Dioxus panes, mini-map rendering deferred to UI layer

## Status
Proposed. (Originally proposed in RFC package v0.4.)

## Summary

Define how ForskScope synchronizes scrolling between panes, maps lines across diffed documents, and renders diff decorations in the Dioxus editor workspace.

## Motivation

Diff/merge usability depends heavily on stable visual correspondence. If panes drift, decorations lag, or line numbers become stale during editing, users lose trust. This is especially risky when using an embedded editor surface through a Dioxus/WebView bridge.

## Goals

- Support synchronized scrolling across two-way and three-way layouts.
- Map visual rows between left/right/base/result documents.
- Render line-level and inline decorations from core state.
- Recompute decorations safely after edits.
- Support large-file degradation modes.

## Non-Goals

- Pixel-perfect clone of WinMerge.
- Semantic language-aware diff rendering.
- Always-on full inline diff for huge files.

## External Design

### User Controls

```text
View Menu:
  [x] Synchronized Scrolling
  [x] Align Added/Deleted Blocks
  [x] Inline Character Differences
  [ ] Ignore Whitespace in View
  [ ] Collapse Equal Blocks
```

### Visual Semantics

```text
line state indicators:
  = equal
  - removed from right/result
  + added on right/result
  ~ changed
  ! conflict
  ? unknown / not computed
```

### Wireframe

```text
+--------------------------------------------------------------+
| Diff View                                                    |
+------------------------------+-------------------------------+
| 001 = use crate::foo;         | 001 = use crate::foo;          |
| 002 ~ let timeout = 30;       | 002 ~ let timeout = 60;        |
| 003 - old_value();            |     ·                           |
|     ·                         | 003 + new_value();             |
+------------------------------+-------------------------------+
| Mini hunk map: [==][~~][-+][==][!!][==]                      |
+--------------------------------------------------------------+
```

## Internal Design

### Line Mapping Types

```rust
pub struct LineMap {
    pub left_revision: RevisionId,
    pub right_revision: RevisionId,
    pub rows: Vec<AlignedRow>,
}

pub struct AlignedRow {
    pub row_id: RowId,
    pub left: Option<LineSpan>,
    pub right: Option<LineSpan>,
    pub state: RowState,
}

pub enum RowState {
    Equal,
    Inserted,
    Deleted,
    Modified,
    Conflict,
    Unknown,
}
```

### Decoration Types

```rust
pub struct DiffDecorationSet {
    pub document: DocumentId,
    pub revision: RevisionId,
    pub line_decorations: Vec<LineDecoration>,
    pub inline_decorations: Vec<InlineDecoration>,
}
```

Decorations are invalidated when the document revision changes. Incremental strategies may be added later, but correctness comes first.

## Scroll Synchronization Strategy

The editor adapter emits scroll events with visible line ranges:

```rust
pub struct VisibleRangeEvent {
    pub pane: PaneId,
    pub document: DocumentId,
    pub first_visible_line: LineNumber,
    pub last_visible_line: LineNumber,
}
```

The core/UI maps the anchor row to other panes and requests scroll alignment:

```rust
pub struct ScrollToMappedRowCommand {
    pub target_pane: PaneId,
    pub row_id: RowId,
    pub alignment: ScrollAlignment,
}
```

### Loop Prevention

Every scroll sync command carries a source event ID. The adapter must suppress feedback loops caused by programmatic scrolling.

## Large-File Mode

If file size, line count, or diff cost exceeds configured thresholds:

```text
- line-level diff remains available where possible
- inline character diff is disabled by default
- full decoration rebuild is throttled
- collapsed unchanged blocks may be forced
- user is told that large-file mode is active
```

## Acceptance Criteria

- Two-pane scroll sync works across insertions and deletions.
- Programmatic scroll does not cause infinite event loops.
- Diff decorations are regenerated from core state after edit operations.
- Inline decorations can be disabled without breaking line-level diff.
- Large-file mode is visible and testable.

## Dependencies

- RFC 002 — similar v3 Diff Engine
- RFC 013 — Large File Performance
- RFC 024 — Diff Visual Semantics
- RFC 032 — Text Editing Operation Model
- RFC 040 — Editor Adapter Verification Harness
