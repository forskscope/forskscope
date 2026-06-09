# Architecture

## Crate layout

```
forskscope/
  crates/
    forskscope-core/      GUI-independent product logic
    forskscope-ui-dioxus/ Dioxus 0.7 desktop frontend
```

The core crate has no dependency on Dioxus, Tauri, or any UI framework.  It is
the canonical owner of product truth; the UI reads from it and dispatches
commands to it.  A future Iced frontend could be added as a third crate without
touching core.

## Core modules

| Module | Responsibility |
|---|---|
| `error` | Error taxonomy; no panics for user-facing failures. |
| `path` | Lenient canonicalization; platform-safe display helpers. |
| `file_kind` | File classification: Text, Binary, ExcelXlsx, Missing. |
| `encoding` | Decode with chardetng + encoding_rs; encode for save. |
| `document` | Load a path into a `LoadedDocument` with fingerprint. |
| `diff` | `similar` v3 diff engine; normalized model (hunks, stable IDs, inline spans). |
| `merge` | `MergeSession` (two-way) with transaction log, undo/redo, dirty state, `result_text()`; `ThreeWayMergeSession` (base-aware diff3, structured conflicts, resolution + undo/redo). |
| `patch` | Unified-diff patch export from file and directory comparisons (RFC-039). |
| `save` | Conflict detection, backup, atomic write. |
| `dir` | Directory listing and recursive digest equality. |
| `xlsx` | Excel adapter: derive comparable text via `sheets-diff`. |

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
