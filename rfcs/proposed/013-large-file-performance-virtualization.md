# RFC-013 — Large File, Performance, and Virtualization Strategy

**Status.** Proposed — threshold policy and job model slice implemented (v0.46.0); virtualization open

## Status
Partially implemented in v0.46.0:

- **Threshold policy constants** in `forskscope-core::job`: `LARGE_FILE_INLINE_DIFF_BYTES`
  (512 KB), `VERY_LARGE_FILE_BYTES` (10 MB), `LARGE_HUNK_AUTO_EXPAND_LINES`
  (10 000), `LARGE_DIRECTORY_VIRTUAL_THRESHOLD` (5 000),
  `DIGEST_CONCURRENCY_LIMIT` (32). These constants are now the single source
  of truth for large-file behaviour, replacing the previously ad-hoc values
  in `DiffOptions` defaults.
- **`JobKind`**, **`JobProgress`**, **`JobHandle`** in `forskscope-core::job`:
  the types the UI needs to show progress bars and cancellation buttons for
  background operations. `JobProgress::fraction()` and `is_complete()` are
  tested. `JobHandle::new()` pairs a `JobId` with a `CancellationToken`.
- **14 tests** covering fraction computation edge cases, `is_complete`,
  `JobKind::label`, and `JobHandle` cancel propagation.

Remaining open: row virtualization for large directories (§7.2) and the
editor-view decoration batching (§7.1, blocked on the editor adapter track,
RFC-004).

A diff/merge app must remain responsive even when users open large files or directory trees. The current application performs useful operations, but the Dioxus migration must make expensive work cancellable, bounded, and observable.

## 2. Motivation

Dioxus gives the project a practical path to rich text UI through a web editor surface. However, a WebView-based UI can still freeze if the app pushes too much DOM, too many decorations, or too many large snapshots across the bridge.

The core risk is:

```text
large input → expensive diff → huge view model → huge editor decorations → frozen UI
```

This RFC creates guardrails before large-file support becomes accidental behavior.

## 3. Goals

- Define soft and hard thresholds for expensive operations.
- Virtualize line rendering and explorer rows where possible.
- Make inline character diff lazy and cancellable.
- Avoid sending full file snapshots repeatedly across the editor bridge.
- Provide clear UI states for large-file mode.
- Establish performance regression tests.

## 4. Non-Goals

- This RFC does not require infinite-size file support.
- This RFC does not introduce memory-mapped editing in the first migration.
- This RFC does not implement a custom native text renderer.
- This RFC does not guarantee identical performance on all platforms.

## 5. Threshold Policy

The first implementation should define configurable thresholds.

```rust
pub struct PerformanceLimits {
    pub max_eager_text_bytes: u64,
    pub max_eager_lines: usize,
    pub max_inline_diff_chars_per_hunk: usize,
    pub max_editor_decoration_count: usize,
    pub max_directory_entries_eager: usize,
}
```

Recommended initial policy:

| Condition | Default Behavior |
|---|---|
| Small text file | Full diff and editable view |
| Medium text file | Full line diff, lazy inline diff |
| Large text file | Ask user before full diff; read-only or limited-edit mode |
| Very large file | Metadata/binary-style summary unless explicitly forced |
| Large directory | Background comparison with virtualized rows |
| Huge inline hunk | Show line-level replace; inline diff on demand |

## 6. Large File UI

```text
+--------------------------------------------------------------+
| Large File Mode                                              |
+--------------------------------------------------------------+
| This comparison may be expensive.                            |
|                                                              |
| Left:  84 MB, 1,230,000 lines                                |
| Right: 91 MB, 1,310,000 lines                                |
|                                                              |
| Recommended mode: Line diff without inline character diff     |
|                                                              |
| [Open Limited Diff] [Open Read-Only Preview] [Cancel]         |
+--------------------------------------------------------------+
```

## 7. Virtualization Areas

### 7.1 Editor View

If CodeMirror is used, it already avoids rendering all visible lines as raw DOM in the same naive way as simple text rendering. The project must still avoid creating excessive decorations.

Rules:

- Store all hunks in the core.
- Convert only visible or nearby hunks into editor decorations when possible.
- Batch decoration updates.
- Avoid full editor reinitialization when switching active hunk.
- Avoid sending full text after every edit.

### 7.2 Explorer View

Directory comparison must support row virtualization for large directories.

```text
ExplorerViewModel
  visible_range: 120..180
  total_rows: 42,000
  rows_loaded: window + buffer
```

### 7.3 Inline Diff

Inline diff must be lazy:

1. Compute line-level diff first.
2. Mark replace hunks as `inline_status = NotComputed`.
3. Compute inline segments only for visible hunk or user-expanded hunk.
4. Cache inline result by hunk identity and input fingerprint.

## 8. Background Job Model

Large operations must be jobs:

```rust
pub enum JobKind {
    ReadFile,
    DecodeFile,
    LineDiff,
    InlineDiff,
    DirectoryDigest,
    SavePreflight,
}

pub struct JobProgress {
    pub job_id: JobId,
    pub kind: JobKind,
    pub phase: String,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub cancellable: bool,
}
```

## 9. Cancellation

Cancellation must be best-effort but safe.

Rules:

- Cancelled jobs must not leave partial session truth as if complete.
- Partial results may be displayed only if marked partial.
- Save jobs should not be cancellable after the final atomic replacement step begins.
- UI must show whether cancellation is pending or complete.

## 10. Editor Bridge Performance Contract

The editor adapter must support incremental operations:

```rust
pub enum EditorPatch {
    ReplaceAll { revision: EditorRevision, text: String },
    ApplyEdits { base_revision: EditorRevision, edits: Vec<TextEdit> },
    SetDecorations { revision: EditorRevision, ranges: Vec<DecorationRange> },
    ScrollToLine { line: usize, align: ScrollAlign },
}
```

Prohibited for normal editing:

```text
on every keystroke → send full text from editor to Rust → recalculate all diff → replace whole editor doc
```

Allowed:

```text
on edit → send transaction summary → update core buffer → schedule bounded diff refresh
```

## 11. Performance Diagnostics

The diagnostics panel should expose:

- file size;
- line count;
- decode time;
- line diff time;
- inline diff time;
- decoration count;
- memory estimate;
- number of active background jobs;
- cancellation count.

## 12. Testing Requirements

Create synthetic fixture generators for:

- 10k-line text file;
- 100k-line text file;
- single huge line;
- many small hunks;
- few huge replace hunks;
- directory with 10k files;
- directory with nested tree depth;
- invalid/binary files.

Tests must verify:

- no UI-blocking synchronous command path for expensive jobs;
- inline diff is not eager for all hunks;
- cancellation returns to stable UI state;
- performance metrics are recorded.

## 13. Acceptance Criteria

- Large-file mode exists and is understandable.
- Inline diff is lazy and bounded.
- Directory comparison does not block the UI.
- Editor updates are incremental where possible.
- Performance gates exist in CI or manual release QA.

## 14. Risks

| Risk | Severity | Mitigation |
|---|---:|---|
| WebView freezes due to decorations | High | Decoration budget and visible-range strategy |
| Core recalculates too often | High | Debounce, revisions, incremental edit summaries |
| Users expect unlimited file size | Medium | Clear large-file mode and documented limits |
| Cancellation corrupts state | Critical | Partial results must be explicitly marked |

## 15. Open Questions

- Should large-file editing be disabled in v1?
- Should the app expose performance limit settings to users?
- Should extremely large files use an external diff command fallback later?
