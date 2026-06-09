# RFC-001 — Core Extraction and Canonical Domain Model

**Status.** Implemented (v0.23.0)

---toml
project = "ForskScope"
rfc = "001"
title = "Core Extraction and Canonical Domain Model"
status = "implemented"
phase = "M1"
depends_on = ["RFC-042"]
---

## 1. Summary

Extract ForskScope's product logic from the current Tauri command layer into a GUI-independent Rust core crate. The extracted core becomes the canonical owner of file loading, file identity, text decoding metadata, diff requests, merge sessions, dirty state, and save policy inputs.

This RFC is the foundation for every later Dioxus RFC. Without this boundary, the migration would risk becoming a UI rewrite that preserves the current state-management problems.

## 2. Motivation

The current implementation mixes backend commands, file reading, diff calculation, and frontend state. That is workable for a small Tauri/Svelte app, but it is fragile for a WinMerge-class tool. The migration must make the core testable without a GUI and reusable by any future UI layer, including Dioxus now and possibly Iced later.

## 3. Goals

- Create a `forskscope-core` crate with no Dioxus, Tauri, WebView, or JavaScript dependency.
- Define canonical domain types for files, sessions, documents, diffs, hunks, merge operations, and save plans.
- Preserve current feature intent: text comparison, binary/hex fallback, Excel comparison adapter, directory listing, and directory digest comparison.
- Replace panic-prone path and file operations with explicit error results.
- Provide unit tests and golden tests for core behavior.

## 4. Non-Goals

- Implement the full Dioxus UI.
- Implement the final editor adapter.
- Solve all large-file performance problems immediately.
- Guarantee final public API stability before RFC-002 and RFC-007 are complete.

## 5. Target Repository Shape

```text
crates/
  forskscope-core/
    src/
      lib.rs
      error.rs
      path.rs
      file_kind.rs
      document.rs
      encoding.rs
      diff/
      merge/
      save/
      dir/
      tests_support.rs
  forskscope-ui-dioxus/
    src/
      main.rs
      app.rs
      workspaces/
      components/
      bridge/
  forskscope-editor-adapter/
    src/
      lib.rs
      model.rs
      events.rs
```

A temporary compatibility crate may be used during migration:

```text
crates/forskscope-tauri-compat/
```

This crate may wrap the new core in the existing Tauri command shape to prove behavior parity before the old app is retired.

## 6. Canonical Domain Model

### 6.1 File Identity

```rust
pub struct FileId {
    pub canonical_path: PathBuf,
    pub display_path: String,
}

pub struct FileFingerprint {
    pub len: u64,
    pub modified_unix_nanos: Option<i128>,
    pub digest: Option<ContentDigest>,
}
```

The fingerprint is used for external modification detection. Digest calculation may be lazy because hashing large files has cost.

### 6.2 File Kind

```rust
pub enum FileKind {
    Text,
    Binary,
    ExcelXlsx,
    Missing,
    Unsupported { reason: String },
}
```

The current behavior treats `.xlsx` specially and falls back to binary/hex for non-text content. The new model should make this explicit.

### 6.3 Loaded Document

```rust
pub struct LoadedDocument {
    pub file_id: Option<FileId>,
    pub fingerprint_at_load: Option<FileFingerprint>,
    pub kind: FileKind,
    pub bytes_len: u64,
    pub text: Option<TextDocument>,
    pub binary_preview: Option<BinaryPreview>,
    pub warnings: Vec<LoadWarning>,
}

pub struct TextDocument {
    pub content: String,
    pub encoding: TextEncoding,
    pub newline_style: NewlineStyle,
    pub had_decode_errors: bool,
}
```

### 6.4 Compare Session

```rust
pub struct CompareSession {
    pub session_id: SessionId,
    pub left: LoadedDocument,
    pub right: LoadedDocument,
    pub diff: Option<DiffDocument>,
    pub merge: Option<MergeSession>,
    pub created_at: SystemTime,
    pub dirty: DirtyState,
}
```

A compare session is the root object for a tab.

### 6.5 Error Model

```rust
pub enum CoreError {
    InvalidPath { path: String, reason: String },
    Io { path: Option<PathBuf>, operation: IoOperation, message: String },
    Decode { path: Option<PathBuf>, message: String },
    Unsupported { message: String },
    Conflict { message: String },
    InternalInvariant { message: String },
}
```

No core operation should panic for normal user-facing failures.

## 7. Public Core Service Surface

```rust
pub trait FileService {
    fn load_path(&self, path: &Path, options: LoadOptions) -> Result<LoadedDocument, CoreError>;
    fn list_dir(&self, path: Option<&Path>) -> Result<DirectoryListing, CoreError>;
    fn fingerprint(&self, path: &Path, policy: FingerprintPolicy) -> Result<FileFingerprint, CoreError>;
}

pub trait CompareService {
    fn create_session(&self, left: CompareInput, right: CompareInput) -> Result<CompareSession, CoreError>;
    fn refresh_diff(&self, session: &mut CompareSession, options: DiffOptions) -> Result<(), CoreError>;
}
```

The concrete implementation may be a struct rather than trait-based at first, but the boundary should remain clear.

## 8. Migration Mapping from Current Code

| Current area | New target |
|---|---|
| `src-tauri/src/core/file.rs` | `forskscope-core::file`, `encoding`, `dir`, `save` |
| `src-tauri/src/core/diff.rs` | `forskscope-core::diff`, `merge` |
| `src-tauri/src/core/types.rs` | Domain model + UI DTO conversion layer |
| Tauri commands | Temporary compatibility adapter or removed |
| Svelte stores | Dioxus state derived from core sessions |

## 9. User-Facing Behavior Preservation

The following behaviors must remain recognizable:

- Opening two files starts a comparison.
- Opening one side and leaving the other empty is allowed where useful.
- Text files are compared as text.
- Binary files are not silently treated as editable text.
- Excel comparison remains supported as a separate adapter, not as ordinary text editing.
- Directory listing separates directories and files.

## 10. Testing Requirements

### 10.1 Unit Tests

- Path canonicalization on Linux, macOS, and Windows path forms.
- File kind detection for UTF-8 text, non-UTF-8 text, binary with NUL bytes, empty files, and `.xlsx` placeholder fixtures.
- Encoding metadata retention.
- Error mapping for missing paths and permission errors.

### 10.2 Golden Tests

Create `tests/golden/` fixtures:

```text
golden/
  text_utf8_equal/
  text_utf8_changed/
  text_shift_jis_or_legacy_encoding/
  binary_small/
  excel_simple/
  missing_left/
  missing_right/
```

Golden tests should validate stable domain objects after removing nondeterministic fields such as absolute temp paths.

### 10.3 Compatibility Tests

For a temporary period, compare new core outputs with current Tauri command outputs for representative files.

## 11. Acceptance Criteria

- `cargo test -p forskscope-core` passes without any GUI dependency.
- Core exposes explicit error results for invalid paths, missing files, permission failures, and unsupported content.
- Existing representative file comparisons can be modeled as `CompareSession` objects.
- No Dioxus, Tauri, Svelte, or JavaScript type appears in `forskscope-core`.
- Documentation explains how UI DTOs are derived from core domain objects.

## 12. Risks

| Risk | Mitigation |
|---|---|
| Core API becomes too abstract too early | Start with concrete structs and narrow traits. |
| Current behavior changes accidentally | Add compatibility fixtures before deleting Tauri path. |
| Encoding behavior is under-specified | Store encoding and decode warning metadata from the first extraction. |
| Excel behavior pollutes text model | Keep Excel behind a file-kind adapter. |

## 13. Open Questions

- Should binary preview be generated eagerly or lazily?
- Should file digest be part of initial load or a separate background job?
- Should legacy encodings be saved by default, or should conversion require explicit confirmation?

## Deferred work (v0.23.0)

Golden compatibility fixtures against the v0.22.x Tauri output (§10.3) are deferred to a follow-up once the Tauri baseline is preserved. The Excel placeholder `load_placeholder` is loaded at open-time but pair-text derivation requires both paths; this is correct behaviour.
