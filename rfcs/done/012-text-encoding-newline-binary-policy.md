# RFC-012 — Text Encoding, Newline, and Binary Policy

**Status.** Implemented (v0.50.0 + v0.69.0) — core complete; charset/newline pane footer and encoding-warning dialog deferred to UI layer

## Status

Core implementation complete across two releases:

- **v0.50.0**: `EditabilityClass` (`ReadOnly | ReadWriteWithGuard | ReadWrite | Unsupported`), `requires_save_guard()`, `NewlinePolicy` (`Preserve | ForceLf | ForceCrlf`), `NewlinePolicy::resolve(detected)`, `detect_newline_style()`. 15 tests.
- **v0.69.0**: `BomPresence` (`Absent | Utf8 | Utf16Le | Utf16Be`), `BomPolicy` (`Preserve | Strip | AddUtf8`), `BomPolicy::resolve_bytes(original)`, `detect_bom(bytes)`. Defaults are `Preserve` / `Absent` per RFC-012 §7.2 bullet 5: "preserve BOM policy unless the user changes it." 16 tests.

**Remaining (deferred to UI layer):** charset and newline display in the pane footer (§9.1), the encoding-warning save dialog (§9.2), and a settings toggle to change `BomPolicy` / `NewlinePolicy` per-session.

## Status
Partially implemented in v0.50.0:

- **`EditabilityClass`** (`Unsupported < ReadOnly < ReadWriteWithGuard <
  ReadWrite`, `Ord`) added to `file_kind.rs`. `FileKind::editability(had_decode_errors, encoding_label)` derives the class. Predicates: `is_editable()`, `is_saveable()`, `requires_save_guard()`. 21 tests.
- **`NewlinePolicy`** (`Preserve` default / `ForceLf` / `ForceCrlf`) added to `encoding.rs`. `resolve(detected_style) -> Option<&str>` returns the newline string to use when saving. `Preserve` on mixed/None returns `None` (caller keeps original endings). 14 tests.

Remaining open: the Charset + Newline metadata display (§9.1, pane footer), the encoding-warning save dialog (§9.2, UI), BOM preservation policy (§7.2 bullet 5), and the explicit editable vs read-only save guard wired in the `save_text` path.

This RFC defines the policy for reading, comparing, editing, and saving text and non-text files in the Dioxus migration.

The current implementation already detects UTF-8 and other encodings, has special labels for binary and Excel content, and writes using a selected charset. The next design must make this behavior explicit, safe, and testable before enabling model-backed editing and saving.

## 2. Motivation

A diff/merge app can corrupt files if it treats encoding, newline style, binary detection, or save behavior casually.

WinMerge-like usage implies users may compare:

- UTF-8 source files;
- Shift_JIS or other legacy-encoded files;
- files with CRLF, LF, or mixed newlines;
- binary files that should only be compared as bytes;
- Excel files rendered as textual diff views;
- files with invalid byte sequences.

The app must clearly distinguish what it can safely edit from what it can only display.

## 3. Goals

- Preserve encoding metadata for each side.
- Detect newline style and expose it to save logic.
- Define editable vs read-only classifications.
- Avoid silent binary corruption.
- Make decoding errors visible.
- Define default save behavior.

## 4. Non-Goals

- This RFC does not require perfect encoding detection.
- This RFC does not implement rich binary editing.
- This RFC does not define Excel structural editing.
- This RFC does not introduce arbitrary file conversion tools.

## 5. File Content Classification

```rust
pub enum ContentKind {
    Text(TextContentMeta),
    Binary(BinaryContentMeta),
    Excel(ExcelContentMeta),
    Unsupported(UnsupportedContentMeta),
}

pub struct TextContentMeta {
    pub encoding_label: String,
    pub had_decode_errors: bool,
    pub newline_style: NewlineStyle,
    pub has_mixed_newlines: bool,
    pub bom: BomKind,
    pub editable: bool,
}
```

## 6. Newline Policy

```rust
pub enum NewlineStyle {
    Lf,
    Crlf,
    Cr,
    Mixed,
    None,
    Unknown,
}
```

Rules:

1. Preserve the original newline style by default.
2. If a file has mixed newlines, preserve exact line endings where possible.
3. If the user edits text through the editor and the editor normalizes newlines, the save plan must warn if exact preservation is impossible.
4. Newline conversion should be an explicit option, not an implicit side effect.

## 7. Encoding Policy

### 7.1 Read Policy

When reading a file:

1. Read bytes first.
2. Detect binary before text decoding.
3. Try UTF-8, including BOM handling.
4. If UTF-8 fails, use encoding detection.
5. Record whether decoding had errors.
6. Preserve the detected or selected encoding label in the session.

### 7.2 Save Policy

When saving a file:

1. Use the session's selected output encoding.
2. Default to the original side's encoding.
3. Warn when encoding cannot represent some characters.
4. Never silently replace characters without warning.
5. Preserve BOM policy unless the user changes it.

## 8. Editable Classification

| Kind | Viewable | Editable | Saveable | Notes |
|---|---:|---:|---:|---|
| UTF-8 text | Yes | Yes | Yes | Default safe path |
| Non-UTF text decoded cleanly | Yes | Yes | Yes with encoding guard | Warn on lossy save |
| Text with decode errors | Yes | Limited | Save guarded | Must show warning |
| Binary | Yes as hex/summary | No | No direct text save | Use binary compare only |
| Excel rendered as diff text | Yes | No in v1 | No | Export/save not supported |
| Unsupported | Limited | No | No | Explain reason |

## 9. UI Requirements

### 9.1 Charset and Newline Display

Each diff pane must show compact metadata:

```text
Left:  UTF-8 · LF · 12.4 KB
Right: Shift_JIS · CRLF · 12.8 KB · warning: conversion guarded
```

### 9.2 Encoding Warning

```text
+--------------------------------------------------------------+
| Encoding Warning                                             |
+--------------------------------------------------------------+
| This file was decoded as Shift_JIS. Some edited characters   |
| may not be representable in that encoding.                   |
|                                                              |
| Output encoding: [Shift_JIS v]                               |
|                                                              |
| [Save with warning] [Save as UTF-8...] [Cancel]              |
+--------------------------------------------------------------+
```

### 9.3 Binary Compare View

```text
+--------------------------------------------------------------+
| Binary Comparison                                            |
+--------------------------------------------------------------+
| These files are not editable as text.                        |
|                                                              |
| Left:  image-a.png · 48.2 KB · SHA-256 ...                   |
| Right: image-b.png · 48.2 KB · SHA-256 ...                   |
|                                                              |
| Status: different                                            |
|                                                              |
| [Open in File Manager] [Copy Path]                           |
+--------------------------------------------------------------+
```

## 10. Data Model

```rust
pub struct DecodedFile {
    pub identity: FileIdentity,
    pub bytes_fingerprint: FileFingerprint,
    pub kind: ContentKind,
    pub text: Option<TextBuffer>,
}

pub struct TextBuffer {
    pub normalized_text: String,
    pub line_endings: LineEndingMap,
    pub original_encoding: EncodingSpec,
    pub output_encoding: EncodingSpec,
}
```

`normalized_text` may use `\n` internally if the core also keeps a `LineEndingMap`. This allows diff calculation to be simpler while retaining save fidelity.

## 11. Internal Line Ending Map

For high-fidelity saves, the core should preserve line ending information at line boundaries:

```rust
pub enum OriginalLineEnding {
    Lf,
    Crlf,
    Cr,
    None,
}

pub struct LineEndingMap {
    pub endings: Vec<OriginalLineEnding>,
    pub mixed: bool,
}
```

When a user edits or inserts new lines, inserted lines should use the dominant newline style unless the user selected another output policy.

## 12. Save Preflight

Save preflight must return a structured result:

```rust
pub enum SavePreflightResult {
    Safe(SavePlan),
    Warning(SavePlan, Vec<SaveWarning>),
    Blocked(Vec<SaveBlocker>),
}
```

Examples of blockers:

- target file externally modified;
- output encoding cannot encode content and replacement is disabled;
- target is read-only;
- path points to directory;
- binary/Excel session is not text-saveable.

## 13. Testing Requirements

Fixtures must include:

- UTF-8 LF;
- UTF-8 CRLF;
- UTF-8 with BOM;
- Shift_JIS Japanese text;
- mixed newline file;
- binary file with null bytes;
- text-like file with invalid byte sequences;
- Excel comparison sample if supported by the existing behavior;
- save round-trip tests.

## 14. Acceptance Criteria

- The app never silently saves binary or unsupported content as text.
- Encoding and newline metadata are visible in the UI.
- Save preflight detects risky conversions.
- UTF-8 and non-UTF round trips are covered by tests.
- Newline preservation policy is deterministic.

## 15. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| Encoding detection misidentifies file | High | Show detected encoding and allow override |
| Editor normalizes newlines | Medium | Use core line-ending map and save warning |
| Binary file accidentally decoded | Critical | Binary detection before decode; save blocked |
| Non-UTF save loses characters | Critical | Save preflight with representability check |

## 16. Open Questions

- Should users be able to override detected encoding before diffing?
- Should the app expose a newline conversion command in v1?
- Should Excel diff output be considered exportable text or strictly read-only?
