# Architecture

ForskScope is a Cargo workspace with three crates.

```text
forskscope/
  crates/
    forskscope-core/      domain logic, no UI dependency
    forskscope-ui-logic/  view-model layer, no Dioxus/GTK dependency
    forskscope-ui/        Dioxus 0.7 desktop frontend (shell, workspaces, dialogs)
```

The core crate has no dependency on Dioxus, Tauri, or any UI framework. It is
the canonical owner of product truth; the UI reads from it and dispatches
commands to it. `forskscope-ui-logic` holds pure presentation logic that is
testable without a display server. A future alternative frontend (e.g. Iced)
could be added as a fourth crate without touching core.

## Core modules (26)

| Module | Responsibility |
|---|---|
| `cancel` | `CancellationToken` — lightweight `Arc<AtomicBool>` for cancellable background jobs. |
| `command` | `CommandDefinition`, `CommandRegistry`, `AvailabilityRule`, `CommandContext` — all user-visible commands, IDs, shortcuts (RFC-019). |
| `conflict_nav` | `ConflictNavigator`, `ConflictStatusDisplay`, `NavigatorSummary` — conflict rail view-model over `ThreeWayMergeSession` (RFC-034). |
| `diff` | `similar` v3 diff engine; normalized `DiffDocument` + hunk model, stable IDs, inline spans. `CompareProfile`, `WhitespaceMode`, `NewlineCompareMode`, `DiffOptions` (RFC-002, RFC-028). |
| `diff_decoration` | `DiffDecorationSet` derived from `DiffDocument` — CSS class tokens, gutter symbols, aria labels, inline spans, `HunkDecoration` (RFC-024). |
| `dir` | Directory listing, recursive digest equality, `DirectoryIndex`, `EqualityEvidence`, `pair_entries`. `batch_copy` + `BatchManifest` (RFC-023). `recursive_diff_with_cancel` (RFC-037). `plan_operations` + `execute_plan` (RFC-022). |
| `document` | Load a path into `LoadedDocument` with `FileFingerprint`. `ExternalFileState` / `check_external_state` (RFC-036). |
| `edit_op` | `TextEditOperation`, `RevisionId`, `TextRange`, `OperationAck/Reject`, `EditTransaction` — editor adapter boundary types (RFC-032). |
| `encoding` | `decode_bytes` with chardetng + encoding_rs. `NewlinePolicy`, `NewlineStyle`, `detect_newline_style`. `BomPresence`, `BomPolicy`, `detect_bom` (RFC-012). |
| `error` | `CoreError` (internal), `AppError` (UI envelope), `AppErrorKind` (25 variants), `ErrorId`, `TechnicalDetail`, `RecoveryAction`, `UserMessage` (RFC-017). |
| `external_tool` | `ExternalToolCommand`, `expand_args`, `ToolKind`, built-in presets (file manager reveal, VS Code, system open). No shell execution (RFC-029). |
| `file_kind` | `FileKind` (Text, Binary, ExcelXlsx, Missing). `EditabilityClass`, `requires_save_guard()` (RFC-012). |
| `ignore` | `IgnoreRules` — extension and directory-pattern filtering (RFC-056). |
| `job` | `JobProgress`, `JobHandle`, `JobStatus`, `JobStatusRecord`, `JobRegistry` — background job lifecycle. `FileSizeClass`, `PerformanceLimits` (RFC-013). |
| `line_map` | `LineMap`, `AlignedRow`, `RowId`, `ScrollAnchor`, `build_mini_map` — aligned row sequence for synchronized scroll (RFC-035). |
| `merge` | `MergeSession` (two-way) with transaction log, undo/redo, dirty state, `result_text()`; `ThreeWayMergeSession` (base-aware diff3, structured conflicts, resolution + undo/redo). `TransactionLog`, `TransactionKind` (RFC-015, RFC-033). |
| `patch` | `PatchDocument`, unified-diff export from file and directory diffs (RFC-039). |
| `path` | Lenient canonicalization; platform-safe display helpers. |
| `persist` | `VersionedEnvelope` + `MigrationPolicy` — schema-versioned JSON wrapper for all persisted data (RFC-031). |
| `platform` | `PlatformInfo::collect()` — runtime diagnostic snapshot (OS, arch, CPUs, app version, redacted home, config dir). `to_report()` for clipboard copy in the About panel (RFC-026). |
| `report` | `FileComparisonReport` + `DirComparisonReport` — Markdown and JSON report export (RFC-027). |
| `save` | `save_text`, `AtomicSaveStrategy`, `BackupPolicy`, `SaveRequest`, `SaveOutcome` — conflict detection, backup, atomic write (RFC-007). |
| `session` | `WorkspaceSession`, `WorkspaceTab`, `CloseResult`, `RecentSessionEntry` — session model and JSON persistence (RFC-011). |
| `settings` | `UserSettings`, `AppearanceSettings`, `DiffSettings`, `ThemeId`, `FontFamilySetting` — persisted user preferences (RFC-009). |
| `vcs` | `VcsProvider` trait + `GitProvider` — read-only VCS context (status, file at revision, merge base). `detect(path)` (RFC-038). |
| `watcher` | `FileChangeMonitor` trait, `WatchToken`, `FileChangeEvent`, `WatchError`, `MockFileChangeMonitor` — file-watcher boundary (RFC-036). |
| `xlsx` | `SpreadsheetDiff` structured model, sheets-diff v2 adapter, panic-free Result API, cancellation (RFC-058). |

## `ui-logic` modules (14)

Framework-independent view-model logic. All modules are testable with
`cargo test -p forskscope-ui-logic` — no GTK or display server required.

| Module | Purpose |
|---|---|
| `explore::align` | `compute_aligned_rows` — merges two flat tree row lists into an aligned two-pane sequence (RFC-059). |
| `explore::deep_filter` | `DeepFilter`, `DeepCompareSummary`, `apply_filter` — filter state and counts for recursive directory compare (RFC-037, RFC-038). |
| `explore::status` | `RowStatusKind`, `StatusRow` — maps `EqualityEvidence` to CSS class, glyph, and aria label for tree row badges (RFC-054). |
| `compare::command_bar` | `build_toolbar(registry, ctx)` → `Vec<ToolbarSection>` — evaluates `AvailabilityRule` for all commands; replaces ad-hoc `if can_save` guards (RFC-019). |
| `compare::conflict_nav_view` | `ConflictNavView::from_navigator(nav, can_save)` — complete navigator rail snapshot: rows with glyphs/CSS, progress text, prev/next IDs (RFC-034, Slice 6). |
| `compare::hunk_decorations` | `DecorationIndex::from_set(dec)` — O(1) `(row_index, side)` → `RowDecoration` lookup; replaces inline `match hunk.kind` CSS logic in `hunk.rs` (RFC-024, RFC-035). |
| `compare::load_guard` | `guard_for_sizes(left, right)` → `LoadGuard` — pre-diff decision: Proceed / WarnBanner / ConfirmPrompt derived from `FileSizeClass` thresholds (RFC-013, Slice 1). |
| `compare::palette_view` | `build_palette(registry, ctx, query)` → `Vec<PaletteRow>` — filtered, availability-evaluated, sorted palette results (RFC-019, Slice 7). |
| `compare::save_error` | `SaveErrorView::from_error(err, path)` — maps `AppError` to dialog title, body, and ordered `Vec<RecoveryButton>` (RFC-007, RFC-017, Slice 3). |
| `compare::scroll_sync` | `ScrollSyncState` — `scrollTop` ↔ `ScrollAnchor` arithmetic for synchronized pane scrolling; `scroll_to_row` for hunk navigation (RFC-035, Slice 1). |
| `compare::search_index` | `MatchIndex` — in-diff search match navigation with `advance()`/`retreat()` (RFC-014). |
| `compare::summary` | `CompareStatusSummary`, `DiffNavigationState` — status bar content and hunk navigation position (RFC-006). |
| `compare::tab_state` | `TabStateSnapshot`, `context_from_snapshot` — bridges `TabSnapshot` fields to `CommandContext` for toolbar evaluation (RFC-003, RFC-019). |
| `settings::settings_view` | `theme_choices`, `density_choices`, `font_family_choices`, `profile_presets` — picker metadata and validators (`validate_font_size`, `validate_context_lines`, `find_active`) for the settings dialog (RFC-009, Slice 5). |

## UI modules

| Module | Responsibility |
|---|---|
| `state` | `Store` (shared Dioxus signals), `CompareTab`, `AppSettings`, `open_compare`, session persistence. |
| `app` | Root component; provides store context; CSS injection; startup pair; git mergetool mode. |
| `ui/header` | Brand, Settings button, keyboard reference shortcut. |
| `ui/tabs` | Tab bar with dirty-dot markers and close. |
| `ui/explorer` | Two `DirectoryTreeView` panes; aligned row display; digest status; pick and compare. |
| `ui/diff` | Diff workspace; hunk rendering from `MergeSession`; toolbar with progressive disclosure. |
| `ui/dir_pane` | Tree row building blocks (`PathBar`, `TreeRow`), `NavHistory`, `FilteringExecutor`. |
| `ui/deep_compare` | Recursive directory compare with incremental digest progress. |
| `ui/settings` | Settings modal + persistence; `ModalLayer` dispatcher. |
| `ui/statusbar` | Passive context: file names, encoding, diff stats, local-only marker. |
| `ui/search` | Inline search bar, `SearchCtx`, scroll-to-focused. |
| `ui/keybindings` | Keyboard reference modal. |
| `i18n` | English passthrough + Japanese key map. |
| **Shim re-exports** | `command_bar`, `compare_summary`, `conflict_nav`, `deep_filter`, `explore_status`, `explorer_align`, `hunk_decorations`, `load_guard`, `palette_view`, `save_error`, `scroll_sync`, `search_index`, `settings_view`, `tab_state` — one shim per `ui-logic` module providing a stable import path for future wiring. |

## Core ownership rule

The Rust core owns product truth: documents, diff, merge state, dirty state,
and save policy. `ui-logic` holds tested view-model derivations. Dioxus owns
rendering and interaction dispatch. The UI must never reconstruct merge results
from rendered content or `contenteditable` state; it must read from
`result_text()` and write through `apply_left_to_right()` / `undo()`.
