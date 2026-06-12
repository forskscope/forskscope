# Core and View-Model Layer Completion Summary — v0.99.0

**Date:** 2026-06-12
**Status:** Core data layer complete. `ui-logic` view-model layer complete (14 modules, 189 tests). All 7 ROADMAP slices have view-model coverage. UI stabilisation ongoing; tests at 875, docs complete, RFC-041 checklist 12/16 items ticked.

> Originally written at v0.72.0; updated at v0.78.0, v0.87.0, v0.99.0.

---

## RFC implementation summary

| Done | Proposed |
|------|---------|
| 39   | 9       |

### Done RFCs (39)

000, 001, 002, 003, 005, 006, 007, 008, 009, 011, 012, 013, 014, 015, 017,
019, 020, 021, 022, 023, 024, 027, 028, 029, 031, 032, 033, 034, 035, 036,
037, 038, 039, 054, 055, 056, 057, 058, 059

### Remaining proposed (9)

| RFC | Category | Reason still proposed |
|-----|----------|-----------------------|
| 004 | Editor adapter | Requires GTK/WebView + CodeMirror |
| 010 | Packaging/QA | Requires cross-platform CI |
| 016 | Editor bridge security | Requires editor adapter (RFC-004) |
| 025 | Editor adapter prototype | Requires editor adapter (RFC-004) |
| 026 | Cross-platform WebView | Requires cross-platform CI |
| 030 | User documentation | No code |
| 040 | Editor harness & golden corpus | Requires editor adapter (RFC-004) |
| 041 | v1 stabilization governance | Process document |
| 042 | Roadmap and execution plan | Process document |

---

## `forskscope-core` modules (27, v0.99.0)

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

## `forskscope-ui-logic` modules (14, v0.87.0)

| Module | Purpose | Tests |
|--------|---------|-------|
| `explore::align` | Two-pane tree row alignment | ~22 |
| `explore::deep_filter` | Deep compare filter + `DeepCompareSummary` | 15 |
| `explore::status` | `EqualityEvidence` → status badge | 18 |
| `compare::command_bar` | Toolbar items from `CommandRegistry` + ctx | 12 |
| `compare::search_index` | In-diff match navigation (advance/retreat) | ~8 |
| `compare::summary` | Status bar text + `DiffNavigationState` | 15 |
| `compare::tab_state` | `TabStateSnapshot` → `CommandContext` bridge | 10 |

---

## Test count at v0.78.0

| Suite | Count |
|-------|-------|
| `forskscope-core` unit | 599 |
| `forskscope-core` integration | 2 |
| `forskscope-ui-logic` | 85 |
| Doctests | 6 |
| **Total** | **692** |

All pass. `cargo clippy -- -D warnings` clean.

---

## What the UI implementation phase needs

The Dioxus UI layer (`forskscope-ui`) can now wire to complete view-models.
Priority order per `ROADMAP.md`:

1. **Diff view** — `DiffDecorationSet` → renderer; `LineMap` → scroll-sync;
   `build_toolbar(reg, ctx_from_snapshot(snap))` → toolbar; `DiffNavigationState` → nav buttons
2. **Merge** — `TextEditOperation` → core; `TransactionLog` → undo/redo
3. **Save** — `save_text` + `check_external_state` + `AppError` → dialogs
4. **Explorer** — `StatusRow::from_evidence` → tree badges; `DeepFilter`/`DeepCompareSummary` → deep compare
5. **Settings** — `UserSettings::to_json`/`from_json` → settings form
6. **Three-way merge** — `ConflictNavigator::build` → navigator rail
7. **Command palette** — `CommandRegistry::search` + `CommandContext` → palette
8. **Editor adapter** (gated, RFC-004) — `TextEditOperation` → CodeMirror bridge
