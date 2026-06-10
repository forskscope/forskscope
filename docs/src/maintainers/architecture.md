# Architecture

## Crate layout

```
forskscope/
  crates/
    forskscope-core/      GUI-independent product logic (diff/merge/save/dir/…)
    forskscope-ui-logic/  framework-independent presentation logic (view-model)
    forskscope-ui/        Dioxus 0.7 desktop frontend (shell, workspaces, dialogs)
```

The core crate has no dependency on Dioxus, Tauri, or any UI framework.  It is
the canonical owner of product truth; the UI reads from it and dispatches
commands to it.  `forskscope-ui-logic` holds pure presentation logic (row
alignment, search-match indexing) that is testable without a display server.
A future alternative frontend could be added as a fourth crate without
touching core.

## Core modules

| Module | Responsibility |
|---|---|
| `error` | Error taxonomy; `ErrorSeverity`, `RecoveryHint`; no panics for user-facing failures. |
| `path` | Lenient canonicalization; platform-safe display helpers. |
| `file_kind` | File classification: Text, Binary, ExcelXlsx, Missing. `EditabilityClass` (RFC-012). |
| `encoding` | Decode with chardetng + encoding_rs; encode for save. `NewlinePolicy` (RFC-012). |
| `document` | Load a path into a `LoadedDocument` with fingerprint. `ExternalFileState` / `check_external_state` (RFC-036). |
| `diff` | `similar` v3 diff engine; normalized model (hunks, stable IDs, inline spans). `CompareProfile` + `WhitespaceMode` (RFC-028). |
| `merge` | `MergeSession` (two-way) with transaction log, undo/redo, dirty state, `result_text()`; `ThreeWayMergeSession` (base-aware diff3, structured conflicts, resolution + undo/redo). `TransactionLog` / `SessionRevision` / `TransactionKind` (RFC-015). |
| `patch` | Unified-diff patch export from file and directory comparisons (RFC-039). |
| `save` | Conflict detection, backup, atomic write. |
| `dir` | Directory listing, recursive digest equality. `batch_copy` + `BatchManifest` (RFC-023). `recursive_diff_with_cancel` (RFC-037). `plan_operations` + `execute_plan` / `OperationPlan` (RFC-022). |
| `cancel` | `CancellationToken` — lightweight `Arc<AtomicBool>` for cancellable background jobs (RFC-037). |
| `ignore` | `IgnoreRules` — extension and directory-pattern filtering. |
| `job` | `JobProgress`, `JobHandle`, large-file threshold policy constants (RFC-013). |
| `persist` | `VersionedEnvelope` + `MigrationPolicy` — schema-versioned JSON wrapper for all persisted data (RFC-031). |
| `report` | `FileComparisonReport` + `DirComparisonReport` — Markdown and JSON report export (RFC-027). |
| `vcs` | `VcsProvider` trait + `GitProvider` — read-only VCS context (status, file at revision, merge base). `detect(path)` entry point (RFC-038). |
| `xlsx` | Excel adapter: `SpreadsheetDiff` structured model + panic-guarded `diff_xlsx` (RFC-058). |

## UI modules

| Module | Responsibility |
|---|---|
| `state` | `Store` (shared signals), `CompareTab`, settings, `open_compare`. |
| `app` | Root component; provides store context; CSS injection; startup pair. |
| `ui/header` | Brand, Explorer link, Settings button. |
| `ui/tabs` | Tab bar with dirty markers and close. |
| `ui/explorer` | Two `DirectoryTreeView` panes; pick and compare. |
| `ui/diff` | Diff workspace; hunk rendering from `MergeSession`; toolbar with progressive disclosure. |
| `ui/settings` | Settings modal + overwrite confirm; `app-json-settings` persistence. |
| `ui/statusbar` | Passive context: file names, encoding, local-only marker. |
| `i18n` | English passthrough + Japanese key map. |

## Core ownership rule

The Rust core owns product truth: documents, diff, merge state, dirty state,
and save policy.  Dioxus owns rendering and interaction dispatch.  The UI must
never reconstruct merge results from rendered content or from `contenteditable`
state.

See RFC-001 (core extraction) and RFC-042 (migration roadmap) for the full
rationale.
