# RFC-002 — Diff Engine Migration to `similar` v3 and Normalized Diff Model

**Status.** Implemented (v0.23.0)

---toml
project = "ForskScope"
rfc = "002"
title = "Diff Engine Migration to similar v3 and Normalized Diff Model"
status = "implemented"
phase = "M2"
depends_on = ["RFC-001"]
---

## 1. Summary

Migrate ForskScope's diff calculation from the current `similar` v2 usage to `similar` v3, and define a normalized diff model that supports line-level hunks, inline character/word-level highlights, stable hunk IDs, and future merge transactions.

This RFC is not only a dependency upgrade. It defines the contract that the Dioxus UI and editor adapter will consume.

## 2. Motivation

The current implementation exposes diff results in UI-oriented blocks with `diff_index`, `diff_kind`, line vectors, and later lazy character diffing. This is useful, but it is not sufficient for a robust merge workflow. A merge tool needs stable hunk identity, line ranges, side mapping, and a clear distinction between original left/right documents and the mutable working document.

## 3. Goals

- Upgrade the Rust diff dependency to `similar` v3.
- Define a stable `DiffDocument` independent from the UI.
- Preserve side-by-side line diff behavior.
- Support inline decoration data for changed lines.
- Define hunk IDs suitable for navigation, merge operations, undo/redo, and editor decorations.
- Add timeout/deadline or size-threshold policy for expensive diff operations.

## 4. Non-Goals

- Implement a full semantic diff engine.
- Implement language-aware syntax parsing.
- Implement three-way merge in this RFC.
- Replace all possible diff algorithms with user-configurable algorithms immediately.

## 5. Diff Model

### 5.1 Core Types

```rust
pub struct DiffDocument {
    pub diff_id: DiffId,
    pub left_document_id: DocumentId,
    pub right_document_id: DocumentId,
    pub options: DiffOptions,
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
    pub warnings: Vec<DiffWarning>,
}

pub struct DiffHunk {
    pub hunk_id: HunkId,
    pub kind: HunkKind,
    pub left_range: LineRange,
    pub right_range: LineRange,
    pub rows: Vec<DiffRow>,
}

pub enum HunkKind {
    Equal,
    Insert,
    Delete,
    Replace,
}

pub struct DiffRow {
    pub row_id: RowId,
    pub left: Option<SideLine>,
    pub right: Option<SideLine>,
    pub inline: Option<InlineDiff>,
}
```

### 5.2 Line and Inline Decoration

```rust
pub struct SideLine {
    pub original_line_number: Option<u32>,
    pub content: String,
    pub newline: NewlineMarker,
}

pub struct InlineDiff {
    pub left_spans: Vec<InlineSpan>,
    pub right_spans: Vec<InlineSpan>,
}

pub struct InlineSpan {
    pub kind: InlineKind,
    pub text: String,
}

pub enum InlineKind {
    Equal,
    Insert,
    Delete,
    Replace,
}
```

Inline spans are decorations, not independent document truth.

## 6. Hunk Identity Rule

`HunkId` must be deterministic within one diff calculation and stable enough for UI navigation during a session. It does not need to survive arbitrary re-diff after edits.

Recommended construction:

```text
HunkId = hash(diff_id, hunk ordinal, left_range, right_range, hunk kind)
```

When a document changes, a new `diff_id` is generated. Old hunk IDs become stale and should not be reused for merge operations.

## 7. Diff Options

```rust
pub struct DiffOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub inline_mode: InlineMode,
    pub algorithm: DiffAlgorithm,
    pub max_inline_chars_per_hunk: usize,
    pub max_file_bytes_for_full_diff: u64,
    pub deadline_ms: Option<u64>,
}

pub enum InlineMode {
    None,
    Lazy,
    EagerForSmallHunks,
}
```

Default for MVP:

```text
ignore_whitespace = false
ignore_case = false
inline_mode = Lazy or EagerForSmallHunks
algorithm = default similar text algorithm
```

## 8. External UI Contract

The UI receives a view model derived from `DiffDocument`:

```rust
pub struct DiffViewModel {
    pub title: String,
    pub left_path: Option<String>,
    pub right_path: Option<String>,
    pub hunks: Vec<DiffHunkView>,
    pub stats: DiffStatsView,
    pub can_merge: bool,
    pub warnings: Vec<UserWarning>,
}
```

The UI must not depend on `similar` crate types directly.

## 9. Inline Diff Strategy

Inline diffing can be expensive. It should be lazy or bounded:

1. Compute line hunks first.
2. For equal hunks, do not compute inline spans.
3. For replace hunks below threshold, compute inline spans immediately.
4. For large replace hunks, show line-level diff and offer lazy inline expansion.

## 10. Large File Policy

If a file exceeds configured thresholds:

- Warn the user before full diff.
- Allow binary/metadata-only comparison.
- Allow line-only comparison with inline disabled.
- Do not freeze the UI.

The core should return `DiffWarning::LargeFilePolicyApplied` instead of silently degrading.

## 11. Testing Requirements

### 11.1 Unit Tests

- Equal files produce one equal hunk or a compact equivalent.
- Insert/delete/replace hunks preserve correct line ranges.
- Inline spans preserve Unicode boundaries.
- Newline markers are retained.
- Diff options affect output predictably.

### 11.2 Golden Tests

Use text fixtures with:

- ASCII changes.
- Japanese/multibyte text.
- CRLF/LF newline differences.
- Long lines.
- Large replace blocks.
- Empty left or empty right document.

### 11.3 Compatibility Tests

Compare representative current outputs against the new normalized model through a mapping layer. The new model does not need identical JSON, but it must preserve user-visible meaning.

## 12. Acceptance Criteria

- `forskscope-core::diff` uses `similar` v3 or a pinned accepted v3-compatible version.
- No UI layer imports `similar` types directly.
- Hunks have stable IDs within a diff document.
- Inline spans are Unicode-safe.
- Large file thresholds are tested.
- The diff model can drive both read-only diff display and later merge transactions.

## 13. Risks

| Risk | Mitigation |
|---|---|
| Inline diff is too slow for large hunks | Lazy inline mode and thresholds. |
| Hunk identity becomes invalid after edits | Recompute diff and mark old hunk IDs stale. |
| Similar v3 API details differ from assumptions | Pin dependency and update this RFC during implementation. |
| UI wants a more display-oriented shape | Use DTO/view-model mapping, not core model mutation. |

## 14. Open Questions

- Should word-level inline diff be offered in addition to char-level diff?
- Should whitespace-only changes have a distinct visual kind?
- Should the first release expose diff options in settings or keep defaults hidden?

## Deferred work (v0.23.0)

Word-level inline diff (§14, open question) is deferred; char-level is implemented. User-visible algorithm selection via settings (§7) is deferred to RFC-028 compare profiles. Whitespace-error highlighting is deferred.
