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

## Core modules (28)

| Module | Responsibility |
|---|---|
| `cancel` | `CancellationToken` — lightweight `Arc<AtomicBool>` for cancellable background jobs. |
| `command` | `CommandDefinition`, `CommandRegistry`, `AvailabilityRule`, `CommandContext` — all user-visible commands, IDs, shortcuts (RFC-019). |
| `compare_prep` | `PreparedCompare`, `SaveTargetSnapshot`, `save_target_from_loaded`, `inspect_save_target` — separates what's compared from what's saved to, since normal two-file mode and Git mergetool mode disagree (RFC-077). |
| `conflict_nav` | `ConflictNavigator`, `ConflictStatusDisplay`, `NavigatorSummary` — conflict rail view-model over `ThreeWayMergeSession` (RFC-034). |
| `diff` | `similar` v3 diff engine; normalized `DiffDocument` + hunk model, stable IDs, inline spans. `CompareProfile`, `WhitespaceMode`, `NewlineCompareMode`, `DiffOptions` (RFC-002, RFC-028). |
| `diff_decoration` | `DiffDecorationSet` derived from `DiffDocument` — CSS class tokens, gutter symbols, aria labels, inline spans, `HunkDecoration` (RFC-024). |
| `dir` | Directory listing, recursive digest equality, `EqualityEvidence` (the comparison vocabulary). `batch_copy` + `BatchManifest` (RFC-023). `recursive_diff_with_cancel` (RFC-037). `plan_operations` + `execute_plan` (RFC-022). |
| `document` | Load a path into `LoadedDocument` with `FileFingerprint`. `ExternalFileState` / `check_external_state` (RFC-036). |
| `edit_op` | `TextEditOperation`, `RevisionId`, `TextRange`, `OperationAck/Reject`, `EditTransaction` — editor adapter boundary types (RFC-032). |
| `encoding` | `decode_bytes` with chardetng + encoding_rs; `decode_body` selects a BOM-implied encoding directly, `decode_with_label` re-decodes an explicit user choice (RFC-083). `NewlinePolicy`, `NewlineStyle`, `detect_newline_style`. `BomPresence`, `BomPolicy`, `detect_bom` (RFC-012). |
| `env_passthrough` | `record_before_override`/`restore_onto` — remembers an environment variable's value before this process overrides it (RFC-091 §2's portable/private `XDG_*` redirection) and reapplies the original onto a child `Command`, so a process ForskScope launches never inherits the override. |
| `error` | `CoreError` (internal), `AppError` (UI envelope), `AppErrorKind` (25 variants), `ErrorId`, `TechnicalDetail`, `RecoveryAction`, `UserMessage` (RFC-017). |
| `external_tool` | `ExternalToolCommand`, `expand_args`, `ToolKind`, built-in presets (file manager reveal, VS Code, system open). No shell execution (RFC-029). |
| `file_kind` | `FileKind` (Text, Binary, ExcelXlsx, Missing). `EditabilityClass`, `requires_save_guard()` (RFC-012). |
| `ignore` | `IgnoreRules` — extension and directory-pattern filtering (RFC-056). |
| `job` | `JobProgress`, `JobHandle`, `JobStatus`, `JobStatusRecord`, `JobRegistry` — background job lifecycle. `FileSizeClass`, `PerformanceLimits` (RFC-013). |
| `line_map` | `LineMap`, `AlignedRow`, `RowId`, `ScrollAnchor`, `build_mini_map` — aligned row sequence for synchronized scroll (RFC-035). |
| `merge` | `MergeSession` (two-way) with transaction log, undo/redo, dirty state, `result_text()`; `ThreeWayMergeSession` (base-aware diff3, structured conflicts, resolution + undo/redo). `TransactionLog`, `TransactionKind` (RFC-015, RFC-033). |
| `patch` | `PatchDocument`, unified-diff export from file and directory diffs (RFC-039). |
| `path` | Lenient canonicalization; platform-safe display helpers. |
| `persist` | `schema` — the canonical settings/session v2 schema, migration from the shipping UI's plain-JSON v0 files, and load outcomes via `PersistenceLoad` (RFC-076). RFC-031's original `VersionedEnvelope`/`MigrationPolicy` wrapper and v1 records were removed (RFC-076 patch 5, 2026-08-03): no released version ever wrote a v1 file. |
| `platform` | `PlatformInfo::collect()` — runtime diagnostic snapshot (OS, arch, CPUs, app version, redacted home, config dir). `to_report()` for clipboard copy in the About panel (RFC-026). |
| `report` | `FileComparisonReport` + `DirComparisonReport` — Markdown and JSON report export (RFC-027). |
| `save` | `save_text`, `TargetPrecondition`, `BackupPolicy`, `SaveRequest`, `SaveOutcome` — conflict detection, backup, temp-file-then-rename write (visibility-atomic, not a power-loss guarantee — F9/N2) via `atomic_replace`/`persist_noclobber` (RFC-007, RFC-077). |
| `settings` | `ThemeId`, `LocaleId`, `Density`, `FontFamilySetting`, `ThemeTokens` — display types `forskscope-ui`/`forskscope-ui-logic` and `persist::schema::settings::PersistedSettings` depend on (RFC-009). `UserSettings` (RFC-031's schema v1 record) was removed, never having shipped (RFC-076 patch 5). |
| `vcs` | `VcsProvider` trait + `GitProvider` — read-only VCS context (status, file at revision, merge base). `detect(path)` (RFC-038). |
| `watcher` | `FileChangeMonitor` trait, `WatchToken`, `FileChangeEvent`, `WatchError`, `MockFileChangeMonitor` — file-watcher boundary (RFC-036). |
| `xlsx` | Fail-closed spreadsheet comparison boundary between `sheets-diff` and ForskScope's app model, no `sheets-diff` types in the public API (RFC-058). Suspended for one release over an XML denial-of-service advisory chain, restored at `sheets-diff` 2.5.0 with cancellation polling inside a sheet (RFC-085); now at 3.0.0, which streams its read (F130). |

## `ui-logic` modules (28)

Framework-independent view-model logic. All modules are testable with
`cargo test -p forskscope-ui-logic` — no GTK or display server required.

F93, third time (review 106 §4, handoff 034): this table has now drifted
twice after being corrected — once when `d69c83b`/`8f1af77` deleted five
modules and neither deletion reached here, once when handoff 033's
`ui-logic` connectivity cleanup deleted `conflict_nav_view` and
`palette_view` (deferred post-v1 view-models, never wired) and this table
was not updated alongside the code. `cargo xtask ui-logic-docs` checks, on
every push, that the module names listed in this table and in `testing.md`'s,
and this heading's count, match the leaf modules on disk, and it reports a
table row it cannot read a module name from instead of ignoring it. It does not
read what a row says a module does; that stays a judgment.

| Module | Purpose |
|---|---|
| `explore::align` | `compute_aligned_rows` — merges two flat tree row lists into an aligned two-pane sequence (RFC-059); `pair_by_index` — the compact pairing, by position (RFC-068, handoff 071). |
| `explore::classify_pair` | `EntryClassification`, `classify_two_files` — a file pair's equality classification under the automatic-digest cap (RFC-080 §5 / F77, moved from `forskscope-ui` by F145). |
| `explore::dir_verdict` | `DirVerdict`, `dir_verdict` — what a fast recursive listing can conclude about a directory pair: `Different`, `MetadataMatch` (names and sizes match, contents unread) or `Unknown`, with the precedence written down (RFC-080 tier 1). |
| `explore::tier1_trigger` | `Tier1Trigger`, `TIER1_DEBOUNCE` — the pure debounce state machine deciding when a tier-1 walk starts: a row must be rested on for 250 ms, and moving through rows starts none (RFC-080 §5). |
| `explore::tier2_verdict` | `Tier2Verdict`, `tier2_verdict` — what a full, content-reading recursive comparison can conclude about a directory pair: `Identical`, `Different` or `Unknown` — the only tier entitled to claim identity (RFC-080 tier 2). |
| `explore::deep_filter` | `DeepFilter`, `DeepCompareSummary`, `apply_filter` — filter state and counts for recursive directory compare (RFC-037, RFC-038). |
| `explore::status` | `RowStatusKind`, `StatusGlyph` — maps `EqualityEvidence` to CSS class, glyph, and aria label for tree row badges (RFC-054). |
| `explore::sync_panes` | `mirror_target` — the anchored model (review 139 §3): an in-step descent or ascent mirrors the triggering pane's path *relative to its own anchor* onto the other pane's anchor; navigating above your own anchor, or an unrelated jump, mirrors nothing, even if the anchors happen to share a leaf name (F151, issue #148). |
| `compare::data_location` | `resolve_data_location`, `extract_data_location_flags` — where ForskScope keeps its data, decided once at startup from `--private`/`--data-dir` and the program's own folder, before any settings/session/web-view path is resolved (RFC-091). |
| `compare::load_guard` | `guard_for_sizes(left, right)` → `LoadGuard` — pre-diff decision: Proceed / WarnBanner / ConfirmPrompt derived from `FileSizeClass` thresholds (RFC-013, Slice 1). |
| `compare::load_identity` | Process-local `CompareTabId` allocation, monotonic `LoadGeneration`, and pure async completion validation; runtime tokens are never persisted (RFC-075). |
| `compare::save_error` | `SaveErrorView::from_error(err, path)` — maps `AppError` to dialog title, body, and ordered `Vec<RecoveryButton>` (RFC-007, RFC-017, Slice 3). |
| `compare::search_index` | `MatchIndex` — in-diff search match navigation with `advance()`/`retreat()` (RFC-014). |
| `compare::startup` | `StartupRequest`, `parse_startup_args`, `CompareRequest`, `SaveDestination` — CLI startup argument parsing and mergetool-to-compare conversion (RFC-077). |
| `session::persistence_recovery` | `SessionRecoveryView::from_resolution` — maps a session `SessionRuntimeResolution` to a recovery dialog's one-time notice and ordered actions (RFC-076). |
| `settings::persistence_recovery` | `SettingsRecoveryView::from_resolution` — maps a settings `SettingsRuntimeResolution` to a recovery dialog's one-time notice and ordered actions (RFC-076). |
| `settings::settings_view` | `theme_choices` — theme picker options; `clamp_font_size` — the diff-pane font-size bound the settings modal enforces (F53) (RFC-009, Slice 5). |
| `settings::field_debounce` | `FieldDebounce`, `FIELD_DEBOUNCE` — the pure debounce state machine behind the ignore-pattern settings fields: a keystroke always overwrites the pending value, and it commits once 400 ms have passed since the last one (handoff 062, F136). |
| `update_check` | `decide(current, outcome)` — the About dialog's *Check for updates* button: maps a parsed-or-not reply to `Checking` / `UpToDate` / `NewerAvailable` / `AheadOfRelease` / `CouldNotCheck(reason)`, never "up to date" from a failure (F166). `Version::parse` accepts only plain `X.Y.Z`. `AppChannel`/`update_action`/`release_page_url` pick the right instruction and build the release-page URL from the parsed version alone, never from raw reply text (F180/F181, handoff 073). |
| `test_support` | `permission_guard_failed` — shared by permission-dependent tests across this crate (`explore/deep_filter.rs`, `explore/classify_pair.rs`): panics unless this process is root, so a `chmod`-based test guard that has no effect fails loudly instead of skipping silently (F143, handoff 063 §1). Test-only (`#[cfg(test)]`), duplicated once per crate alongside the other RFC-093 crates' and `forskscope-ui`'s own copies - there is no shared test-support crate today. |
| `command/registry` | `CommandRegistry` — all `CommandDefinition`s indexed by `CommandId` (RFC-019 §5). Moved in from `forskscope-core` (RFC-093 §1 step 5); not yet wired into `forskscope-ui` - the toolbar/keyboard/command-palette consumer described in `command`'s own module doc was deferred post-v1 and never built. |
| `conflict_nav` | `ConflictNavigator` — the view-model between `ThreeWayMergeSession` and the four-region conflict-workspace UI: which conflict is focused, the navigator rail, prev/next traversal (RFC-034 §"Conflict navigator"). Moved in from `forskscope-core` (RFC-093 §1 step 5); not yet wired into `forskscope-ui` - `conflict_nav_view` (the rail component) was deleted outright by handoff 033's connectivity cleanup as deferred post-v1 UI, never built. |
| `diff_decoration` | `DiffDecorationSet` — derived from a `DiffDocument`, describes what to render (CSS classes, gutter symbols) without prescribing how (RFC-024). Moved in from `forskscope-core` (RFC-093 §1 step 5); not yet wired into `forskscope-ui` - see F48's note in `tests/css_coverage.rs`. |
| `line_map` | `LineMap` — the aligned row sequence a `DiffDocument` renders to (same-height panes, synchronized scrolling, the mini hunk map) (RFC-035). Moved in from `forskscope-core` (RFC-093 §1 step 5); not yet wired into `forskscope-ui`. |
| `tests/command_tests` | `command.rs`'s and `command/registry.rs`'s tests (RFC-019 §5, §6, §7): kept as its own file rather than split and inlined, since it spans both. Moved in from `forskscope-core` (RFC-093 §1 step 5). |
| `tests/conflict_nav_tests` | `conflict_nav.rs`'s tests (RFC-034). Moved in from `forskscope-core` (RFC-093 §1 step 5). |
| `tests/diff_decoration_tests` | `diff_decoration.rs`'s tests (RFC-024). Moved in from `forskscope-core` (RFC-093 §1 step 5). |
| `tests/line_map_tests` | `line_map.rs`'s tests (RFC-035). Moved in from `forskscope-core` (RFC-093 §1 step 5). |

## UI modules

| Module | Responsibility |
|---|---|
| `state` | `Store` (shared Dioxus signals and root-owned compare-tab ID allocator), `CompareTab`, token-guarded async open/reload completion, `AppSettings`, and session persistence. |
| `app` | Root component; provides store context; CSS injection; startup pair; git mergetool mode; the desktop window title (RFC-089). |
| `keyboard` | Global keyboard ownership, testable: `global_key_action` (pure state-in/action-out), `ControlZone`, `swallow_when_typing` (RFC-060). |
| `update_check` | Update-check network adapter: one bounded HTTPS GET, off the UI thread; the decision logic lives in `forskscope-ui-logic` (F180). |
| `webview2` | Windows-only WebView2 Runtime presence check, since `wry` panics with no console to explain why if it's missing (F109). |
| `ui/layout/tabs` | Tab bar: Explorer tab + file/directory comparison tabs (a WAI-ARIA tabs pattern, RFC-087), plus the *Settings* and *?* buttons moved here when the header was removed (RFC-089). |
| `ui/layout/statusbar` | Passive context: file names, encoding, diff stats, local-only marker. |
| `ui/view/explorer` | Two `DirectoryTreeView` panes; one tree component for both layouts (row pairs from `compute_aligned_rows` or `pair_by_index`); digest status; pick and compare. |
| `ui/view/diff` | Diff workspace; hunk rendering from `MergeSession`; toolbar with progressive disclosure. |
| `ui/view/dir_pane` | Tree row building blocks (`PathBar`, `TreeRow`), `NavHistory`, `FilteringExecutor`. |
| `ui/view/deep_compare` | Recursive directory compare with incremental digest progress. |
| `ui/view/settings` | Settings modal + persistence; `ModalLayer` dispatcher. |
| `ui/view/search` | Inline search bar, `SearchCtx`, scroll-to-focused. |
| `ui/overlay/keybindings` | Keyboard reference modal (`KbRow` rows; `cargo xtask keyboard-docs` ties these to `keyboard.md`). |
| `ui/overlay/modals` | Every safety/action dialog dispatched from `ModalLayer`: `file` (overwrite, save-as, reload, swap, option/encoding-change confirms, large-load, save-error), `tab` (`CloseTabModal`), plus `about`, `copy`, `recovery`. |
| `ui/component/notice` | `Notice` — a styled inline message strip (default/success/warning) reused across views and overlays. |
| `i18n` | English passthrough + Japanese key map. |

## Core ownership rule

The Rust core owns product truth: documents, diff, merge state, dirty state,
and save policy. `ui-logic` holds tested view-model derivations. Dioxus owns
rendering and interaction dispatch. The UI must never reconstruct merge results
from rendered content or `contenteditable` state; it must read from
`result_text()` and write through `apply_left_to_right()` / `undo()`.
