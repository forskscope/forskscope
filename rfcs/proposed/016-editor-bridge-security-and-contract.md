# RFC-016 — Editor Bridge Security and Contract

**Status.** Proposed

## 1. Summary

This RFC defines the security, isolation, and API contract for the Dioxus editor bridge.

ForskScope adopts Dioxus partly because a web editor surface can reduce the risk of building a complex text editor from scratch. This introduces a boundary between Rust product state and WebView/editor state. That boundary must be narrow, versioned, and testable.

## 2. Motivation

The editor is the highest-risk UI component because it handles:

- raw file text;
- keyboard input;
- selections and clipboard;
- line decorations;
- scroll synchronization;
- text edits;
- possible JavaScript integration.

If the bridge is loose, the app may become hard to debug, unsafe to save from, or dependent on hidden DOM behavior.

## 3. Goals

- Define a stable editor adapter API.
- Keep Rust core as product truth.
- Prevent arbitrary bridge calls from mutating files.
- Define message validation and versioning.
- Allow mock editor implementation for tests.
- Constrain JavaScript usage to editor integration only.

## 4. Non-Goals

- This RFC does not ban all JavaScript.
- This RFC does not require a pure Rust editor.
- This RFC does not define browser-based deployment.
- This RFC does not expose a plugin API to user scripts.

## 5. Trust Boundary

```text
+----------------------------+      validated messages      +-----------------------+
| Rust Core                  | <---------------------------> | Editor Adapter        |
| - session truth            |                              | - CodeMirror binding  |
| - diff/merge model         |                              | - DOM/editor state    |
| - save safety              |                              | - decorations         |
+----------------------------+                              +-----------------------+
```

The editor adapter is trusted to display and collect edits. It is not trusted to decide save safety, merge correctness, or file identity.

## 6. Message Versioning

Every bridge message must include a version and session/tab identity.

```rust
pub struct BridgeEnvelope<T> {
    pub protocol_version: u32,
    pub session_id: SessionId,
    pub tab_id: TabId,
    pub editor_id: EditorId,
    pub revision: EditorRevision,
    pub payload: T,
}
```

Messages with unknown protocol versions must be rejected with a structured bridge error.

## 7. Rust to Editor Messages

```rust
pub enum RustToEditorMessage {
    InitDocument(InitDocumentPayload),
    ApplyTextPatch(ApplyTextPatchPayload),
    SetDecorations(SetDecorationsPayload),
    SetReadOnly(SetReadOnlyPayload),
    ScrollTo(ScrollToPayload),
    SetActiveHunk(SetActiveHunkPayload),
    Dispose,
}
```

## 8. Editor to Rust Messages

```rust
pub enum EditorToRustMessage {
    Ready(EditorReadyPayload),
    TextEdited(TextEditedPayload),
    SelectionChanged(SelectionChangedPayload),
    ScrollChanged(ScrollChangedPayload),
    FocusChanged(FocusChangedPayload),
    CommandRequested(CommandRequestedPayload),
    Error(EditorErrorPayload),
}
```

## 9. Validation Rules

The Rust side must validate:

- session ID exists;
- tab ID exists;
- editor ID belongs to the tab;
- base revision is current or reconcilable;
- text edit ranges are valid UTF-8 character boundaries or documented offset units;
- message size is under limit;
- command ID is known;
- read-only tabs do not send text edits.

Invalid messages must not panic. They must create structured diagnostics.

## 10. Offset Units

The bridge must define one offset unit.

Recommended policy:

```text
Bridge offsets use UTF-16 code units only inside the editor adapter if required by the web editor.
Core offsets use Rust string byte offsets or line/column positions.
Adapter converts between them at the boundary.
Public core model should prefer line/column and stable text ranges.
```

This is important because web editors often use JavaScript string semantics while Rust strings are UTF-8.

## 11. Security Rules

- Do not load remote editor scripts at runtime.
- Bundle editor assets with the application.
- Do not allow arbitrary file paths through editor messages.
- Do not allow editor messages to call save directly.
- Do not allow editor messages to spawn processes.
- Do not expose unrestricted application command execution to the editor.
- Log bridge errors without dumping full sensitive file contents.

## 12. Content Security Direction

The desktop app should use the strictest feasible local asset policy.

Allowed:

- local bundled editor assets;
- local CSS;
- generated decorations;
- controlled bridge messages.

Not allowed:

- loading remote JS/CSS;
- evaluating user-provided scripts;
- using file contents as HTML without escaping;
- injecting diff text as raw HTML.

## 13. Mock Editor

A test mock must implement the same adapter trait:

```rust
pub trait EditorAdapter {
    fn init_document(&mut self, payload: InitDocumentPayload) -> Result<()>;
    fn apply_patch(&mut self, payload: ApplyTextPatchPayload) -> Result<()>;
    fn set_decorations(&mut self, payload: SetDecorationsPayload) -> Result<()>;
    fn set_read_only(&mut self, payload: SetReadOnlyPayload) -> Result<()>;
    fn dispose(&mut self) -> Result<()>;
}
```

The mock editor allows core/editor tests without WebView automation.

## 14. Error Handling

Bridge errors should be classified:

```rust
pub enum BridgeErrorKind {
    ProtocolVersionMismatch,
    UnknownSession,
    UnknownTab,
    UnknownEditor,
    StaleRevision,
    InvalidRange,
    MessageTooLarge,
    ReadOnlyViolation,
    InternalEditorError,
}
```

User-facing bridge errors should be rare and plain:

```text
The editor view became out of sync with the comparison model.
Reload this tab to continue safely.
[Reload Tab] [Close Tab]
```

## 15. Testing Requirements

- Reject unknown protocol version.
- Reject stale tab ID.
- Reject edit from read-only editor.
- Convert offsets for multibyte text correctly.
- Apply decorations without changing text.
- Dispose editor and reject later messages.
- Simulate editor crash and recover tab.
- Verify no raw HTML injection from file content.

## 16. Acceptance Criteria

- The editor bridge is versioned.
- All editor messages are validated.
- The core remains source of truth.
- The editor can be mocked in tests.
- Local bundled assets are used.
- Bridge failures do not silently corrupt session state.

## 17. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| JS editor owns hidden truth | Critical | Core ownership and revision validation |
| UTF-16/UTF-8 offset mismatch | High | Explicit conversion and tests |
| Raw HTML injection | High | Escape file text; no raw injection |
| Remote asset dependency | Medium | Bundle editor assets |
| Bridge debugging is difficult | Medium | Structured diagnostics |

## 18. Open Questions

- Which editor implementation is the first target: CodeMirror, Monaco, or a minimal custom editor?
- Should the bridge be implemented through Dioxus eval calls or a dedicated JS module boundary?
- Should bridge messages be logged in development builds by default?
