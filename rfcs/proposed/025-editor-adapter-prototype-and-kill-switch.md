# RFC 025 — Editor Adapter Prototype and Kill Switch

**Status.** Proposed

## Status

Proposed.

## Summary

Define a mandatory prototype gate for the editor adapter and a fallback/kill-switch strategy if the CodeMirror-like editor bridge proves unstable on target desktop platforms.

## Background

The decision to adopt Dioxus is partly motivated by the need for a practical text editor surface. However, integrating a web editor through a desktop WebView introduces risk:

- event synchronization bugs,
- focus and IME issues,
- scroll synchronization problems,
- bridge serialization cost,
- platform WebView differences,
- security concerns around injected JavaScript.

This RFC ensures the editor integration is validated before too much product logic depends on it.

## Goals

- Define editor-adapter acceptance gates.
- Require a prototype before full implementation.
- Define a fallback read-only diff viewer.
- Define a runtime kill switch for editor bridge issues.
- Keep editor state subordinate to Rust core state.

## Non-goals

- Choosing a final editor library forever.
- Implementing a full custom editor in Rust.
- Allowing arbitrary plugin JavaScript.

## Adapter boundary

```text
Rust core model
  ↕ typed commands/events
Dioxus editor adapter
  ↕ restricted bridge
Embedded editor implementation
```

The adapter must expose only typed operations:

```rust
pub enum EditorCommand {
    LoadDocument { buffer: BufferId, text: String, decorations: DiffDecorationSet },
    ApplyPatch { buffer: BufferId, patch: TextPatch },
    SetDecorations { buffer: BufferId, decorations: DiffDecorationSet },
    ScrollToLine { buffer: BufferId, line: usize },
    SetReadOnly { buffer: BufferId, read_only: bool },
    Focus { buffer: BufferId },
}

pub enum EditorEvent {
    TextEdited { buffer: BufferId, edit: TextEdit, editor_revision: u64 },
    CursorMoved { buffer: BufferId, position: TextPosition },
    SelectionChanged { buffer: BufferId, range: Option<TextRange> },
    ScrollChanged { buffer: BufferId, top_line: usize },
    BridgeError { message: String },
}
```

## Prototype acceptance gates

The editor prototype must prove:

1. Two editors can load 10,000-line files.
2. Scroll synchronization works in both directions.
3. Line decorations render correctly.
4. Inline decorations render correctly on representative examples.
5. Text edits are reported to Rust as structured events.
6. Rust-side merge commands can update editor text.
7. Undo/redo can be owned by Rust or coherently coordinated.
8. Clipboard works on Linux, Windows, and macOS.
9. IME does not catastrophically break normal input.
10. Fallback mode can be activated at runtime.

## Kill switch

A runtime setting should disable the advanced editor adapter:

```text
Settings → Advanced → Editor engine
  ( ) Advanced web editor
  ( ) Safe read-only diff viewer
```

Command-line override:

```text
forskscope --safe-editor
forskscope --disable-editor-bridge
```

Environment override:

```text
FORSKSCOPE_SAFE_EDITOR=1
```

## Safe viewer fallback

The fallback viewer must support:

- opening comparisons,
- line-level diff rendering,
- hunk navigation,
- copy hunk commands if model-safe,
- no free-form editing,
- save only for model-applied merge results.

Fallback wireframe:

```text
+--------------------------------------------------------------------------------+
| Safe Viewer Mode: advanced editor disabled                                      |
+----------------------------------------+----------------------------------------+
| Left file                              | Right file                             |
| line-level diff only                   | line-level diff only                   |
+----------------------------------------+----------------------------------------+
| [Prev] [Next] [Copy →] [Copy ←] [Open External Editor]                          |
+--------------------------------------------------------------------------------+
```

## Security restrictions

- The bridge must not evaluate arbitrary user file content as JavaScript.
- Only bundled editor assets may be loaded.
- Editor commands must be serialized through known schemas.
- File paths must not be exposed unnecessarily to injected code.
- Developer debug bridge must be disabled in production unless explicitly enabled.

## Acceptance criteria

- Prototype report is written before full editor implementation.
- All acceptance gates are tested or explicitly waived.
- Safe editor mode is implemented.
- Bridge errors are reported to diagnostics.
- The app can open and inspect diffs without the advanced editor.

## Test strategy

- Automated adapter contract tests.
- Manual platform tests.
- Large-file load tests.
- IME smoke tests where feasible.
- Bridge failure injection tests.

## Dependencies

- RFC 004 Editor adapter and CodeMirror bridge.
- RFC 016 Editor bridge security.
- RFC 021 Document model.
- RFC 024 Diff decoration contract.
- RFC 026 Cross-platform compatibility.
