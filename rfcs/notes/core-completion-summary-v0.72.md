# Core Layer Completion Summary — v0.72.0

**Date:** 2026-06-12
**Status:** Core data layer complete. Ready for UI implementation phase.

---

## RFC implementation summary

| Done | Proposed |
|------|---------|
| 38   | 10      |

### Done RFCs (38)

000, 001, 002, 003, 005, 006, 007, 008, 009, 011, 012, 013, 014, 015, 017,
019, 021, 022, 023, 024, 025 (governance), 027, 028, 029, 031, 032, 033,
034, 035, 036, 037, 038, 039, 054, 055, 056, 057, 058, 059

### Remaining proposed (10)

| RFC | Category | Reason still proposed |
|-----|----------|-----------------------|
| 004 | Editor adapter | Requires GTK/WebView + CodeMirror |
| 010 | Packaging/QA | Requires cross-platform CI |
| 016 | Editor bridge security | Requires editor adapter (RFC-004) |
| 020 | CI/architecture gates | Process document, no code |
| 025 | Editor adapter prototype | Requires editor adapter (RFC-004) |
| 026 | Cross-platform WebView | Requires cross-platform CI |
| 030 | User documentation | No code |
| 040 | Editor harness & golden corpus | Requires editor adapter (RFC-004) |
| 041 | v1 stabilization governance | Process document |
| 042 | Roadmap and execution plan | Process document |

---

## Core modules (v0.72.0)

`forskscope-core` has **21 modules** covering the complete domain:

| Module | Domain |
|--------|--------|
| `cancel` | `CancellationToken` |
| `command` | `CommandRegistry`, `AvailabilityRule`, 25 command IDs |
| `conflict_nav` | `ConflictNavigator`, navigator view-model |
| `diff` | `DiffDocument`, `DiffHunk`, `compute_diff`, `compute_inline_diff` |
| `diff_decoration` | `DiffDecorationSet`, CSS class tokens, gutter symbols |
| `dir` | `DirectoryIndex`, `EqualityEvidence`, `pair_entries`, batch ops |
| `document` | `LoadedDocument`, `ExternalFileState`, `check_external_state` |
| `edit_op` | `TextEditOperation`, `RevisionId`, `EditTransaction` |
| `encoding` | `decode_bytes`, `BomPolicy`, `BomPresence`, `NewlinePolicy` |
| `error` | `CoreError`, `AppError`, `AppErrorKind`, `RecoveryAction` |
| `external_tool` | `ExternalToolCommand`, `expand_args`, built-in presets |
| `file_kind` | `FileKind`, `EditabilityClass` |
| `ignore` | `IgnoreRules`, gitignore-style pattern matching |
| `job` | `JobStatus`, `JobRegistry`, `FileSizeClass`, `PerformanceLimits` |
| `line_map` | `LineMap`, `AlignedRow`, `ScrollAnchor`, `build_mini_map` |
| `merge` | `ThreeWayMergeSession`, `TransactionLog`, conflict resolution |
| `patch` | `PatchDocument`, unified-diff export |
| `path` | Path normalization and validation |
| `persist` | `VersionedEnvelope`, `MigrationPolicy` |
| `report` | `FileComparisonReport`, `DirComparisonReport` |
| `save` | `save_text`, `AtomicSaveStrategy`, `BackupPolicy` |
| `session` | `WorkspaceSession`, `WorkspaceTab`, JSON persistence |
| `settings` | `UserSettings`, `ThemeId`, `BomPolicy` integration |
| `vcs` | `VcsProvider`, `GitProvider`, `detect` |
| `watcher` | `FileChangeMonitor`, `MockFileChangeMonitor` |
| `xlsx` | `SpreadsheetDiff`, sheets-diff v2 adapter |

`forskscope-ui-logic` has **2 modules**:
- `explore::align` — aligned row merging for explorer panes
- `compare::search_index` — in-diff search match index (`advance`/`retreat`)

---

## Test count at v0.72.0

| Suite | Count |
|-------|-------|
| `forskscope-core` unit | 599 |
| `forskscope-core` integration | 2 |
| `forskscope-ui-logic` | 22 |
| Doctests | 6 |
| **Total** | **629** |

All pass. `cargo clippy -- -D warnings` clean.

---

## What the UI implementation phase needs

The Dioxus UI layer (`forskscope-ui`) consumes the complete core model.
The primary remaining UI work, in priority order:

1. **Diff/merge workspace** — `DiffDecorationSet` → renderer; `LineMap` →
   scroll-sync; `CommandRegistry` → toolbar/keyboard; merge transactions → UI.
2. **Explorer workspace** — `DirectoryIndex` + `EqualityEvidence` → digest
   icons; `JobRegistry` → progress indicator; `ExternalToolCommand` presets
   → "reveal in file manager".
3. **Settings dialog** — `UserSettings` JSON round-trip → settings form.
4. **Three-way merge workspace** — `ConflictNavigator` → navigator rail;
   `ThreeWayMergeSession` → four-region layout.
5. **Error dialogs** — `AppError` + `RecoveryAction` → dialog buttons.
6. **Editor adapter** (RFC-004, gated) — `TextEditOperation` → CodeMirror
   bridge; `DiffDecorationSet` → editor decorations.
