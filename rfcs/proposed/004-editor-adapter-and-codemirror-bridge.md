# RFC-004 — Editor Adapter and CodeMirror Bridge

**Status.** Proposed

---toml
project = "ForskScope"
rfc = "004"
title = "Editor Adapter and CodeMirror Bridge"
status = "proposed"
phase = "M4"
depends_on = ["RFC-002", "RFC-003"]
---

## 1. Summary

Define the editor adapter boundary for the editable text surface required by ForskScope's diff/merge workflow. The preferred implementation path is a Dioxus desktop shell with a web editor component such as CodeMirror behind a narrow adapter. The adapter must provide editable text, line decorations, inline decorations, cursor/selection events, scroll sync, and command integration while keeping canonical product state in Rust.

This RFC directly addresses the Iced-vs-Dioxus concern: the editor surface is the reason Dioxus is adopted for now.

## 2. Goals

- Provide a practical route to WinMerge-class text interaction.
- Avoid building a custom text editor widget from scratch.
- Keep editor internals replaceable.
- Ensure Rust core remains the source of truth.
- Support two synchronized editor panes.
- Support hunk and inline diff decorations.
- Support edit events converted into model-backed transactions.

## 3. Non-Goals

- Commit permanently to CodeMirror if another editor proves superior.
- Allow JavaScript editor state to own merge decisions.
- Implement language-server or IDE features.
- Implement full three-way merge.
- Support every possible CodeMirror extension in the first release.

## 4. Adapter Boundary

```text
Dioxus Component
  EditorPane / DiffEditorPair
        │
        ▼
EditorAdapter Rust-facing API
        │ serialized events/commands
        ▼
Web editor component inside WebView
```

The editor adapter is the only place where JavaScript/editor-specific details may exist.

## 5. Editor Adapter API

### 5.1 Commands from Rust to Editor

```rust
pub enum EditorCommand {
    SetDocument { pane: PaneId, revision: DocumentRevision, text: String },
    ApplyDecorations { pane: PaneId, decorations: Vec<EditorDecoration> },
    RevealLine { pane: PaneId, line: u32, align: RevealAlign },
    SetReadOnly { pane: PaneId, read_only: bool },
    Focus { pane: PaneId },
    SetSelection { pane: PaneId, selection: EditorSelection },
    ScrollTo { pane: PaneId, position: ScrollPosition },
}
```

### 5.2 Events from Editor to Rust

```rust
pub enum EditorEvent {
    DocumentChanged {
        pane: PaneId,
        base_revision: DocumentRevision,
        changes: Vec<TextChange>,
    },
    CursorMoved { pane: PaneId, position: TextPosition },
    SelectionChanged { pane: PaneId, selection: EditorSelection },
    ScrollChanged { pane: PaneId, position: ScrollPosition },
    FocusChanged { pane: PaneId, focused: bool },
    CommandRequested { command: EditorCommandRequest },
}
```

### 5.3 Decorations

```rust
pub enum EditorDecoration {
    LineBackground { line: u32, class_name: DecorationClass },
    InlineSpan { from: TextPosition, to: TextPosition, class_name: DecorationClass },
    GutterMarker { line: u32, marker: GutterMarkerKind },
    HunkBoundary { line: u32, hunk_id: HunkId },
}
```

## 6. Source of Truth Rule

The editor maintains a fast local editing surface, but Rust owns canonical session truth.

```text
User types in editor
→ editor emits TextChange with base revision
→ Rust validates and applies transaction
→ Rust updates document revision
→ Rust sends accepted patch/decorations back to editor
```

If the editor emits a change based on a stale revision, Rust must reject or rebase explicitly. Silent drift is not allowed.

## 7. Two-Pane Editor Model

```text
┌───────────────────────────────┬───────────────────────────────┐
│ LeftEditorPane                 │ RightEditorPane                │
│  gutter: line no + markers     │  gutter: line no + markers     │
│  text surface                  │  text surface                  │
│  inline decorations            │  inline decorations            │
│  hunk backgrounds              │  hunk backgrounds              │
└───────────────────────────────┴───────────────────────────────┘
```

Each pane has independent text but shared hunk navigation.

## 8. Scroll Synchronization

The adapter must support at least two modes:

```rust
pub enum ScrollSyncMode {
    Off,
    ApproximateByLine,
    HunkAware,
}
```

MVP may use approximate line sync. Hunk-aware sync can follow after the diff/merge model stabilizes.

## 9. Keyboard Precedence

When editor focus is active:

1. Text-editing shortcuts belong to editor first.
2. Global safe commands such as save may still be handled by the app.
3. Merge commands require explicit hunk focus or toolbar action if there is ambiguity.
4. Dangerous commands must not trigger from normal text typing.

## 10. Editing Modes

```rust
pub enum EditorMode {
    ReadOnlyDiff,
    EditableLeft,
    EditableRight,
    EditableBoth,
    MergeResultOnly,
}
```

Recommended MVP: `ReadOnlyDiff` plus controlled merge commands, then `EditableRight` or `MergeResultOnly`. Fully editable both sides can be deferred until transaction handling is robust.

## 11. Dioxus Integration Pattern

The Dioxus component should treat the adapter as a child surface:

```text
DiffMergeWorkspace
  DiffToolbar
  DiffEditorPair
    EditorBridgeMount
  DiffFooter
```

Dioxus props pass only IDs and view-model data. The adapter handles low-level editor commands.

## 12. Testing Requirements

### 12.1 Adapter Contract Tests

- `SetDocument` followed by editor ready event produces expected revision.
- `DocumentChanged` with valid base revision applies to Rust model.
- Stale revision change is rejected or rebased deterministically.
- Decorations can be applied and cleared.
- Scroll event can be observed.
- Reveal line works for a known line.

### 12.2 UI Smoke Tests

- Open small text pair and show both panes.
- Navigate next/previous hunk.
- Apply a merge command and verify editor updates.
- Type in editable mode and verify dirty state changes.
- Undo and redo through app command model.

## 13. Security and Safety

- Editor content is local user file content; no network loading is allowed by the editor adapter.
- Any JavaScript bridge must expose only narrow, app-specific commands.
- The adapter must not evaluate file content as script.
- CSP and local asset loading should be configured during implementation.

## 14. Acceptance Criteria

- A proof-of-concept editor pair runs inside the Dioxus app.
- Rust can set both documents and apply line decorations.
- Editor can emit text changes into Rust as structured changes.
- Scroll and focus events are observable.
- The app can run in read-only diff mode without editor-owned merge state.
- A documented fallback exists if the chosen editor cannot meet requirements.

## 15. Fallback Options

If CodeMirror or the first chosen editor is unsuitable:

1. Use another web editor surface behind the same adapter.
2. Defer free editing and ship controlled merge commands first.
3. Re-evaluate Iced only after core and merge model are stable.

## 16. Risks

| Risk | Mitigation |
|---|---|
| JavaScript bridge complexity grows | Keep adapter commands small and typed. |
| Editor state diverges from Rust model | Revision-based transactions. |
| Scroll sync is imperfect | Start approximate; add hunk-aware sync later. |
| Editor shortcuts conflict with app shortcuts | Define focus precedence and tests. |
| WebView behavior differs by OS | Add platform smoke tests in RFC-010. |
