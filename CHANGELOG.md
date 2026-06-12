# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.79.0] — 2026-06-12

Maintainer documentation rewrite — architecture and testing docs updated to
reflect the complete v0.79.0 codebase.

### Changed

- **`docs/src/maintainers/architecture.md`** — complete rewrite:

  **Core modules table** now covers all 26 modules (was ~18). Added:
  `command`, `conflict_nav`, `diff_decoration`, `edit_op`, `line_map`,
  `watcher`; updated `encoding` (added `BomPolicy`, `BomPresence`),
  `error` (added `AppError`, `ErrorId`, `TechnicalDetail`), `job` (added
  `JobStatus`, `JobStatusRecord`, `JobRegistry`), removed duplicate
  `session` entry, corrected `diff` to mention `NewlineCompareMode`.

  **`ui-logic` modules table** (new — was one sentence). Lists all 7
  modules with purpose and RFC cross-reference.

  **UI modules table** expanded from 9 to 13 rows: added `dir_pane`,
  `deep_compare`, `search`, `keybindings`.

  **Core ownership rule** updated to explicitly mention `ui-logic` and
  `result_text()`/`apply_left_to_right()`/`undo()`.

- **`docs/src/maintainers/testing.md`** — complete rewrite:

  **Test count table** accurate at 692 (was not present).

  **`forskscope-core` test module table** now covers all 34 test files
  with module name, coverage description, and RFC column (was 8 entries).

  **`forskscope-ui-logic` section** now lists all 7 modules with coverage
  descriptions and RFC cross-references (was "two modules").

  **Integration test** listed (patch round-trip with GNU `patch`).

---

## [0.78.0] — 2026-06-12

Documentation audit and corrections pass.

### Fixed

**`rfcs/README.md` — structural correction.** The README had 20 done RFCs
listed under the "Proposed" section (with correct `done/` link paths but
wrong section placement). Rebuilt with three clean sections: Implemented
(39), Proposed (9), Archive (1). Each entry has the correct path and a
concise "Shipped in / deferred" note.

**`rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`** —
checklist updated from v0.72.0 to v0.78.0:
- RFC inventory: 39 done, 9 proposed (was 38/10)
- Engineering gates: added `[x] ui-logic tests pass (85 tests, 7 modules)`
- `[x] ROADMAP.md up to date` added to Documentation section

**`ROADMAP.md`** — updated from v0.73.0 to v0.78.0:
- Test count: 692 (was 629)
- RFC count: 39 (was 38)
- `ui-logic` description updated: 7 modules with 85 tests providing tested
  view-models (was "component stubs not yet wired")

**`rfcs/notes/core-completion-summary-v0.72.md`** — fully rewritten for
v0.78.0: correct `ui-logic` module table (7 modules with test counts),
correct total test count (692), updated RFC counts, full UI wiring priority
list keyed to ROADMAP.md slice numbers.

### Audit findings (no code changes needed)

- All 39 done RFC status fields verified correct against actual code
- All 9 proposed RFC status fields correctly say "Proposed"
- RFC-018 correctly in `archive/` (Withdrawn)
- RFC numbering gap 043–053 documented in README (reserved range)
- No code-vs-RFC discrepancies found

---

## [0.77.0] — 2026-06-12

Deep compare filter and summary view-model in `forskscope-ui-logic`.

### Added

- **`explore::deep_filter`** — filter and summary for `DeepCompareView`
  (RFC-037 §"Filter", RFC-038).

  **`DeepFilter`** — `Different | All | Equal` (default: `Different`).
  `matches(entry)` returns whether a `RecEntry` passes the filter.
  `label()` for button text. `button_class(active)` → `"filter-btn active"`
  or `"filter-btn"` for the filter-selector buttons. Replaces the inline
  `DeepFilter` enum in `deep_compare.rs`.

  **`DeepCompareSummary`** — derived counts: `total`, `different`
  (`Changed | LeftOnly | RightOnly`), `equal`, `computing`, `visible`
  (count matching the active filter). `from_entries(entries, filter)`.
  `footer_text()` → `"3 different · 12 equal · 15 total"`.
  `is_fully_computed()` → true when no `Computing` entries remain.
  `is_empty()`. Replaces the inline arithmetic in `deep_compare.rs`.

  **`apply_filter(entries, filter) → Vec<&RecEntry>`** — returns only
  the entries that pass the filter.

- **15 new tests** in `explore/deep_filter.rs`: `DeepFilter::matches` for
  all statuses under all three filters, label non-empty, `button_class`
  active/inactive, `DeepCompareSummary` all counts, `footer_text`,
  `is_fully_computed` false/true, `is_empty`, `apply_filter` returns
  correct entries. Total ui-logic count: 85.

---

## [0.76.0] — 2026-06-12

Compare summary and navigation state view-models in `forskscope-ui-logic`.

### Added

- **`compare::summary`** — status bar and navigation state view-models.

  **`CompareStatusSummary`** — single tested snapshot for the status bar
  and tab dirty indicator. Fields: `change_text` (`"+12 / -5"`,
  `"Files are identical"`, or `"N change(s)"`), `encoding_label`,
  `is_dirty`, `is_saveable`, `changed_hunks`, `is_identical`.
  `from_fields(stats, is_dirty, is_saveable, encoding)` consolidates the
  logic that was duplicated between `statusbar.rs` and `tabs.rs`.
  `dirty_marker()` → `"●"` or `""`. `dirty_css_class()`.

  **`DiffNavigationState`** — focused hunk position for the toolbar
  navigation buttons. `new(focused_change, total_changes)`. `has_changes()`,
  `prev_available()` / `next_available()` (both wrap, so always true when
  changes exist). `position_label()` → `"3 of 7"` or `""`.
  `prev_aria_label()` / `next_aria_label()` — ARIA labels mentioning
  position and wrapping behavior.

- **15 new tests** in `compare/summary.rs`: identical/changed/whitespace-
  only/single-hunk texts, dirty flag, unsaveable tab, encoding passthrough,
  no-changes nav state, first/middle/last position labels, prev/next ARIA
  labels (position and wrap cases), single-change nav.
  Total ui-logic count: 70.

---

## [0.75.0] — 2026-06-12

Explorer status view-model and tab state bridge in `forskscope-ui-logic`.

### Added

- **`explore::status`** — maps `EqualityEvidence` → display model for
  explorer tree rows (RFC-054, RFC-037, RFC-059 §M5).

  **`RowStatusKind`** — `Equal | Different | LeftOnly | RightOnly |
  Computing | Error`. `from_evidence(ev)` covers all 10 `EqualityEvidence`
  variants including `MetadataOnly` → `Computing` and `Unknown` → `Computing`.
  Each kind has `glyph()` (distinct char), `css_class()` (`status-*` prefix),
  `aria_label()` (non-empty), `needs_action()` predicate.

  **`StatusRow`** — fully-resolved badge snapshot with all four fields owned.
  `from_evidence(ev)` and `computing()` constructors. Replaces the ad-hoc
  `DigestState` enum in `ui/dir_pane.rs`.

- **`compare::tab_state`** — `TabStateSnapshot → CommandContext` bridge.

  **`TabStateSnapshot`** — 12-bool snapshot of tab state (same fields as
  `CommandContext`). `Default::default()` is all-false (no tab open).

  **`context_from_snapshot(snap) → CommandContext`** — field-by-field
  mapping so `build_toolbar(&reg, &ctx)` receives the correct flags from a
  Dioxus `TabSnapshot` without the component needing to know about
  `CommandContext` internals.

- **21 new tests** across both modules:
  - `status`: all 10 `EqualityEvidence` → `RowStatusKind` mappings, CSS
    prefix, glyph distinctness, aria labels, `needs_action`, `StatusRow`
    constructor correctness.
  - `tab_state`: default context is all-false, dirty-tab context has correct
    fields, end-to-end `TabStateSnapshot → CommandContext → build_toolbar →
    item enabled/disabled`, `AvailabilityRule` inverse verification.
  Total ui-logic count: 55.

---

## [0.74.0] — 2026-06-12

Command bar view-model in `forskscope-ui-logic`.

### Added

- **`compare::command_bar`** in `forskscope-ui-logic` — toolbar item
  view-model (RFC-019 §5, §6).

  **`ToolbarItem`** — fully-resolved toolbar button snapshot: `command_id`,
  `label`, `enabled`, `disabled_reason`, `shortcut_hint`. All fields are
  owned so the Dioxus toolbar component can hold a snapshot without
  lifetime issues.

  **`ToolbarSection`** — labelled group of `ToolbarItem`s. Five sections
  in display order: File | Navigate | Merge | Edit | View.

  **`build_toolbar(registry, ctx) → Vec<ToolbarSection>`** — the main
  entry point. Evaluates `AvailabilityRule` for every item against the
  current `CommandContext` and returns a fully-resolved snapshot. Replaces
  the ad-hoc `if can_save { ... }` guards currently in `ui/diff.rs`.

  **`find_item(sections, id) → Option<&ToolbarItem>`** — look up by
  command ID. **`enabled_count(sections) → usize`** — count enabled items.

- **`forskscope-ui-logic` now depends on `forskscope-core`** (direct
  path dependency). This is correct per RFC-020 §5a: `ui-logic` is the
  view-model layer and needs core types; it still has no Dioxus or GTK
  dependency.

- **12 new tests** in `command_bar` inline test module: section count,
  section labels, Save disabled/enabled by context, Next Difference
  enabled when hunks exist, Copy Hunk enabled with editable active hunk,
  Undo/Redo asymmetry, Command Palette always enabled, Ctrl+S shortcut
  hint, `enabled_count` minimum in empty context, `find_item` miss,
  all labels non-empty. Total ui-logic test count: 34.

---

## [0.73.0] — 2026-06-12

ROADMAP.md; RFC-020 promoted to done; RFC-042 updated.

### Added

- **`ROADMAP.md`** at the project root — the primary orientation document
  for the UI implementation phase. Contains:
  - Delivered milestones table (v0.23–v0.72)
  - 8 UI implementation slices with core types consumed and acceptance
    criteria for each
  - Remaining proposed RFC table with "when" column
  - Non-goals reference

### RFC promotion

- **RFC-020** (`Developer Architecture, CI, and Test Gates`) → `done/`.
  Crate architecture (three crates, dependency rules) settled in v0.48.0.
  CI gate documentation complete. Packaging smoke tests deferred to RFC-010.

### Updates

- RFC-042 status updated: "v0.73.0 — core layer complete, UI phase begins"
- RFC-041 checklist accurate at v0.72.0

**Done count: 39** (was 38). **Proposed: 9** — editor-adapter track (4),
platform/packaging (2), documentation (1), governance (2).

---

## [0.72.0] — 2026-06-12

Final core-layer promotion pass. RFC done count: 38. Core layer complete.

### RFC promotions (3)

| RFC | Title | Core shipped | Deferred |
|---|---|---|---|
| 008 | Directory Comparison and Background Job Model | v0.58.0 + v0.68.0 | Async background job runner, UI progress panel |
| 037 | Scalable Directory Compare Index and Incremental Refresh | v0.42.0 + v0.58.0 | Persistent on-disk index cache, incremental refresh |
| 059 | Explorer and Compare UI/UX Audit Remediation | v0.41.0 | H2/H3/M/L items cross-referenced in done RFCs |

**Done count: 38** (was 35). **Proposed: 10** — all editor-adapter track,
platform/packaging, process/governance, or documentation.

### Documentation

- `rfcs/notes/core-completion-summary-v0.72.md` — comprehensive state
  document: all 38 done RFCs, all 10 remaining proposed, module inventory
  (21 core modules, 2 ui-logic modules), test counts, and UI implementation
  phase roadmap.
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` —
  checklist updated to v0.72.0: all 8 must-stabilise targets ✓,
  engineering gates ✓ (599 core tests, 0 failures).

---

## [0.71.0] — 2026-06-12

FileChangeMonitor trait boundary and MockFileChangeMonitor (RFC-036);
RFC-036 promoted to done.

### Added

- **`forskscope-core::watcher`** — file change monitor trait boundary
  (RFC-036 §"Watcher Boundary").

  **`FileChangeMonitor`** trait: `watch(path) → Result<WatchToken, WatchError>`,
  `poll_events() → Vec<FileChangeEvent>`, `unwatch(token)`, `is_active()`.
  The trait is `Send`; real platform backends implement it. The watcher is
  an optimization layer only — save safety always validates via
  `check_external_state`, never relies solely on watcher events.

  **`FileChangeEvent { token, path, kind }`** — one change event.
  `FileChangeKind`: `Modified | Deleted | Created | Renamed | Unknown`.

  **`WatchToken(u64)`** — opaque handle from `watch`, passed back to `unwatch`.

  **`WatchError`** — `PathNotFound | BackendUnavailable | AlreadyWatched | Other`.
  All variants have non-empty `Display`.

  **`MockFileChangeMonitor`** — test-only implementation. `inject_event`
  queues synthetic events; `poll_events` drains the queue; `set_active(false)`
  simulates backend failure. Includes a rustdoc example.

- **15 new tests** in `tests/watcher_tests.rs` + 1 new doctest:
  active state, distinct tokens, empty poll, inject+drain, multiple events,
  unwatch removes path, unwatch unknown is no-op, inactive monitor error,
  `FileChangeEvent` fields, `FileChangeKind` distinctness, `WatchError`
  display, advisory-not-authoritative safety-rule test.
  Total: 599 core + 6 doctest.

### RFC promotion

- **RFC-036** (`Live Reload, File Watcher, External Modification Handling`)
  → `done/`. Core complete: `ExternalFileState` + `check_external_state`
  (v0.53.0) + `FileChangeMonitor` trait + `MockFileChangeMonitor` (v0.71.0).
  Deferred: `notify`-backed platform watcher implementation, reconciliation
  dialog UI. **Done count: 35** (was 34).

---

## [0.70.0] — 2026-06-12

External tool built-in presets (RFC-029); five RFC promotions.

### Added

- **`ExternalToolCommand::file_manager_reveal()`** — built-in preset that
  expands to `xdg-open {Path}` (Linux default). ID: `builtin.file_manager_reveal`.
  Users can override in settings with a configurable `ExternalToolCommand`
  for their specific file manager (e.g. `nautilus --select {Path}`).

- **`ExternalToolCommand::vscode_open()`** — preset: `code --goto {Path}`.
  ID: `builtin.vscode_open`.

- **`ExternalToolCommand::system_open()`** — preset: `xdg-open {Path}` for
  opening in the system default application. ID: `builtin.system_open`.

- **`ExternalToolCommand::builtin_presets()`** — returns all three built-in
  presets in display order.

- **`ToolKind`** — `Editor | FileManager | Terminal | Custom` — functional
  role classification for an external tool.

- **7 new tests** in `external_tool_tests.rs`: preset IDs and args,
  `file_manager_reveal` path expansion, VS Code `--goto` flag, system open
  placeholder, preset uniqueness, non-empty names, `ToolKind` distinctness.
  Total core test count: 586.

### RFC promotions (5)

Core scope of each RFC is complete; remaining items are UI components.

| RFC | Title | Core shipped | Deferred |
|---|---|---|---|
| 013 | Large File, Performance, Virtualization | v0.59.0 | Row virtualization UI, decoration batching (RFC-004) |
| 014 | Search, Filter, Navigation | v0.43.0 | Explorer filter UI, command palette integration |
| 022 | Directory Merge and Batch Operations | v0.52.0 | Batch preview dialog, deletion confirmation |
| 023 | Atomic File Operations, Backup, Restore | v0.44.0 | Restore picker dialog UI |
| 029 | Integration with External Tools | v0.70.0 | Settings UI for custom commands |

**Done count: 34** (was 29).

---

## [0.69.0] — 2026-06-12

BOM preservation policy (RFC-012 §7.2 bullet 5); RFC-012 promoted to done.

### Added

- **`BomPresence`** in `forskscope-core::encoding` (RFC-012 §7.2).

  `Absent | Utf8 | Utf16Le | Utf16Be` — records whether a Byte Order Mark
  was detected at the start of a loaded file and which variant. Default:
  `Absent`. `is_present()` predicate. `bytes()` returns the raw BOM byte
  sequence for each variant (empty for `Absent`).

- **`BomPolicy`** in `forskscope-core::encoding` (RFC-012 §7.2 bullet 5).

  `Preserve | Strip | AddUtf8` — governs BOM handling on save. Default:
  `Preserve` ("preserve BOM policy unless the user changes it"). `resolve_bytes(original)`
  returns the bytes to prepend before file content: `Preserve` re-emits
  the original BOM bytes (or nothing for `Absent`); `Strip` always returns
  empty; `AddUtf8` always returns `[EF BB BF]`.

- **`detect_bom(bytes: &[u8]) → (BomPresence, &[u8])`** — strips a leading
  BOM from a byte slice and reports the kind found. Returns the remaining
  bytes after the BOM (unchanged when absent). Used by the file-load path
  to strip the BOM before text decoding and record it for save round-trip.

- **16 new tests** in `encoding_tests.rs`: `detect_bom` absent/UTF-8/
  UTF-16LE/UTF-16BE detection and stripping, `is_present` predicate,
  `bytes()` sequences, `BomPolicy::Preserve` preserves/absent, `Strip`
  always empty, `AddUtf8` always UTF-8 BOM, defaults. Total: 579 core tests.

### RFC promotion

- **RFC-012** (`Text Encoding, Newline, and Binary Policy`) → `done/`.
  Core complete: `EditabilityClass` + `NewlinePolicy` (v0.50.0) +
  `BomPresence` + `BomPolicy` + `detect_bom` (v0.69.0).
  Deferred UI: charset/newline pane footer, encoding-warning dialog.
  **Done count: 29** (was 28).

---

## [0.68.0] — 2026-06-12

Job lifecycle state machine (RFC-008 slice).

### Added

- **`JobStatus`** in `forskscope-core::job` (RFC-008 §6–§7).

  `Queued | Running | Completed | Cancelled | Failed(String)` — the complete
  forward-only lifecycle state machine for background jobs. Predicates:
  `is_active()` (Queued or Running), `is_terminal()`, `is_success()`.

- **`JobStatusRecord`** — binds a `JobId` to its current `JobStatus` and
  last-known `JobProgress`. Constructed via `new(job_id, kind)` (starts
  `Queued`). Transitions: `start()` (Queued → Running), `complete()`,
  `cancel()`, `fail(message)` — all no-ops on already-terminal records,
  preventing double-transition bugs.

- **`JobRegistry`** — in-memory collection of all active and recently-
  completed job records. Methods: `register(id, kind)`, `get(id)`,
  `get_mut(id)`, `active()` (iterator over non-terminal records),
  `prune_terminal()` (remove completed/failed/cancelled records after
  display). Used by the UI progress indicator panel.

- **16 new tests** in `job_tests.rs`: all five `JobStatus` predicates, all
  lifecycle transitions (Queued→Running→Completed, →Cancelled, →Failed),
  no-op on double-transition, `JobRegistry` register/get/active filter/
  prune. Total core test count: 567.

---

## [0.67.0] — 2026-06-12

`AppError` structured error envelope (RFC-017); batch RFC promotion pass.

### Added

- **`AppError`** in `forskscope-core::error` (RFC-017 §5).

  Complete structured error envelope: `error_id: ErrorId`, `kind:
  AppErrorKind`, `severity: ErrorSeverity`, `message: UserMessage`,
  `technical: TechnicalDetail`, `recovery: Vec<RecoveryAction>`.

  **`AppError::from_core(err: &CoreError)`** — constructs from the
  lower-level `CoreError` using the standard mappings from `AppErrorKind::
  from_core`, `default_severity`, `for_kind`, `default_recovery_actions`.
  `technical.detail` carries `err.to_string()`.

  **`AppError::new(kind, technical_detail)`** — constructs from an
  application-layer `AppErrorKind` directly (for errors that don't originate
  in `CoreError`, e.g. `FileTooLarge` from the `FileSizeClass` check).

  **`AppError::is_blocking()`** — `severity >= Blocking`.

  **`AppError::is_recoverable()`** — `recovery` contains at least one
  non-`Dismiss` action.

  **`ErrorId`** — millisecond-timestamp + PID identifier for log correlation.

  **`TechnicalDetail { code, detail }`** — machine-readable code string +
  full diagnostic text; shown only in the copy-diagnostics panel.

- **8 new tests** in `app_error_tests.rs`: `from_core` for IO-read and
  Conflict, `new` with explicit kind, `is_blocking` true/false,
  `is_recoverable`, `ErrorId` prefix, `TechnicalDetail` fields.
  Total core test count: 551.

### RFC promotions (7)

Core scope of each RFC is complete; deferred items are UI components.

| RFC | Title | Shipped in | Deferred |
|---|---|---|---|
| 009 | Settings, Theme, Localization, Accessibility | v0.60.0 | Settings dialog UI, LocaleBundle |
| 017 | Error Taxonomy, Diagnostics, UX | v0.67.0 | Diagnostics panel UI, error toast |
| 019 | Command, Shortcut, Palette, Accessibility | v0.63.0 | Command palette UI |
| 024 | Diff Visual Semantics and Decoration Contract | v0.61.0 | Renderer wiring in Dioxus |
| 032 | Text Editing Operation Model | v0.62.0 | EditBuffer dispatch in Dioxus |
| 034 | Conflict Resolution Workspace | v0.64.0 | Four-region workspace UI |
| 035 | Scroll Sync, Line Mapping, Decoration Engine | v0.61.0 | Scroll-sync wiring in Dioxus |

RFC index (`rfcs/README.md`) updated. **Done count: 28** (was 21).

---

## [0.66.0] — 2026-06-12

`NewlineCompareMode::IgnoreDifference` wired into diff engine; RFC-028 and
RFC-011 promoted to done.

### Added

- **`DiffOptions::ignore_newlines: bool`** — new field (default `false`).
  When `true`, `line_key()` in the diff engine uses only the line's content
  for comparison, excluding the newline suffix. LF (`\n`) and CRLF (`\r\n`)
  lines with identical content then hash to the same key and are treated as
  equal by the `similar` algorithm (RFC-028 §`NewlineCompareMode`).

- **`CompareProfile::to_diff_options()`** — now maps
  `NewlineCompareMode::IgnoreDifference` to `ignore_newlines: true`.
  Previously `NewlineCompareMode` had no effect in the engine; this closes
  the last open core item for RFC-028.

- **7 new tests** in `compare_profile_tests.rs`:
  `ignore_newlines` default is `false`; `IgnoreDifference` profile sets the
  field; `Significant` profile leaves it unset; LF vs CRLF same-content lines
  are equal when flag is set; LF vs CRLF differ when flag is unset; content
  differences are still reported even when newlines are ignored; Code Review
  profile does not ignore newlines.
  Total core test count: 543.

### RFC promotions

- **RFC-028** (`Preferences, Profiles, and Compare Options`) → `done/`.
  All core scope complete: compare option types (v0.50.0), profile
  persistence via `UserSettings` (v0.60.0), `NewlineCompareMode` engine
  wiring (v0.66.0). Deferred post-v1: toolbar profile selector UI.

- **RFC-011** (`Workspace Session Persistence`) → `done/`.
  All core scope complete: `WorkspaceSession` model, JSON persistence,
  `CloseResult`, `RecentSessionEntry`, schema version guard, 21 tests
  (v0.56.0). Deferred to schema v2: tab list JSON persistence.
  Deferred UI: session restore picker, crash recovery journal.

RFC index (`rfcs/README.md`) updated. Done count: 21 (was 19).

---

## [0.65.0] — 2026-06-10

Clippy clean pass and documentation update.

### Fixed

Eight `cargo clippy -- -D warnings` errors resolved across four files:

- **`dir/batch.rs`** — collapsed nested `if bp.exists() { if copy(...) }` into
  `if bp.exists() && copy(...).is_ok()`.
- **`patch/model.rs`** — replaced manual `Default` impl on `PatchFormat` with
  `#[derive(Default)]` + `#[default]` on `Unified`; removed duplicate `#[derive]`
  that caused conflicting trait impl errors.
- **`session.rs`** — removed redundant closure: `.map_err(|e| PayloadError(e))`
  → `.map_err(PayloadError)`.
- **`settings.rs`** — renamed three `from_str` methods to `from_id` (avoids
  confusion with `std::str::FromStr::from_str`); replaced manual `Default` impl
  on `UserSettings` with `#[derive(Default)]`; replaced `.min(50).max(6)` with
  `.clamp(6, 50)`.
- **`ui-logic/search_index.rs`** — renamed `next()` → `advance()` and
  `prev()` → `retreat()` (avoids confusion with `std::iter::Iterator::next`).
  Updated all callers in `ui/search.rs`.

`cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` is now
clean. All 536 core tests and 22 ui-logic tests continue to pass.

### Docs

- `docs/src/maintainers/architecture.md` — added 8 new core modules introduced
  in v0.61.0–v0.64.0: `diff_decoration`, `line_map`, `edit_op`, `command`,
  `conflict_nav`, `settings`, `session`; updated `xlsx` entry.
- `docs/src/maintainers/testing.md` — added 8 new test module entries with RFC
  column.
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` — updated
  release readiness checklist against v0.65.0 actual state (7/8 must-stabilise
  targets ✓).

---

### Added

- **`forskscope-core::conflict_nav`** — conflict navigator view-model
  (RFC-034 §"Conflict navigator"). See previous entry for full details.
  22 new tests. Total core test count: 536.

### Fixed (clippy)

Eight clippy lint errors fixed across four files:

- `dir/batch.rs`: collapsed nested `if` into `if a && b`.
- `patch/model.rs`: replaced manual `Default` impl on `PatchFormat` with
  `#[derive(Default)]` + `#[default]` on the `Unified` variant; removed
  duplicate `#[derive]` that caused conflicting trait impls.
- `session.rs`: removed redundant closure `|e| SessionParseError::PayloadError(e)`
  → `SessionParseError::PayloadError`.
- `settings.rs`: renamed three `from_str` methods to `from_id` (avoids
  confusion with `std::str::FromStr::from_str`); replaced manual
  `Default` impl on `UserSettings` with `#[derive(Default)]`; replaced
  `.min(50).max(6)` with `.clamp(6, 50)`.
- `ui-logic/search_index.rs`: renamed `next()` → `advance()` and `prev()`
  → `retreat()` (avoids confusion with `std::iter::Iterator::next`).
  Updated all callers in `ui/search.rs`.

All 536 core tests and 22 ui-logic tests pass after these changes.
`cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
is now clean.

### Docs

- `docs/src/maintainers/architecture.md` — added 8 new core modules
  (`diff_decoration`, `line_map`, `edit_op`, `command`, `conflict_nav`,
  `settings`, `session`, updated `xlsx`).
- `docs/src/maintainers/testing.md` — added 8 new test module entries.
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` —
  updated release readiness checklist against v0.64.0 actual state.

---

## [0.64.0] — 2026-06-10

Conflict navigator view-model (RFC-034 slice).

### Added

- **`forskscope-core::conflict_nav`** — conflict navigator view-model
  (RFC-034 §"Conflict navigator").

  **`ConflictStatusDisplay`** — glyph + text label for one `ConflictStatus`.
  `for_status(status)` maps each of the six variants to the RFC-034 table:
  `! unresolved`, `L left`, `R right`, `B both`, `~ manual`, `- ignored`.
  Both glyph and text are always present; color is never the sole cue
  (RFC-009 §7 accessibility requirement).

  **`ConflictNavigatorEntry`** — one row in the navigator rail: `conflict_id`,
  `display_num` (1-based), `status`, `display`, `is_focused`. `css_class()`
  returns a stable `fsk-conflict-*` token for the status badge.

  **`NavigatorSummary`** — `total`, `resolved`, `unresolved`, `auto_merged`
  counts derived from `ThreeWayMergeSession::stats()`. `progress_fraction()`
  returns `resolved / total` (1.0 for empty session).

  **`ConflictFilter`** — `All` (default) or `UnresolvedOnly`. Controls which
  entries appear; `has_hidden_entries()` signals the UI to show a "show all"
  toggle.

  **`ConflictNavigator::build(session, focused_id, filter)`** — constructs
  the full navigator from a `ThreeWayMergeSession`. Methods: `focused_entry()`,
  `next_id()` (wraps), `prev_id()` (wraps), `first_unresolved_id()`,
  `is_fully_resolved()`, `has_hidden_entries()`.

- **22 new tests** in `tests/conflict_nav_tests.rs`: all six status glyphs
  distinct, all text labels non-empty, `!` for Unresolved, empty/one-conflict
  sessions, summary count invariants, display nums sequential, all entries
  initially unresolved, focused entry set/unset, next/prev wrap on one entry,
  next/prev None on empty, filter hides/shows resolved, resolve updates
  summary, first unresolved before/after resolve, CSS prefix, progress
  fraction 0/1/empty. Total core test count: 536.

---

## [0.63.0] — 2026-06-10

Command model and registry (RFC-019 slice).

### Added

- **`forskscope-core::command`** — command definition, registry, and
  availability model (RFC-019 §5, §6, §7).

  **`CommandId(&'static str)`** — stable dotted-namespace identifier, e.g.
  `"file.save"`, `"merge.copy_left_to_right"`. RFC-041 requires these to
  stabilise before v1; they are all `const` values in the `cmd` submodule.
  25 built-in IDs covering File, Edit, Navigate, Compare, Merge, View,
  Settings, and External categories.

  **`CommandDefinition`** — `{ id, label, description, category,
  default_shortcuts, availability, danger_level }`. `is_available(ctx)`
  evaluates the rule against the current `CommandContext`.

  **`AvailabilityRule`** — 11 variants: `Always`, `ActiveDiffTab`,
  `DirtyAndSaveable`, `ActiveCompareTab`, `ActiveHunkEditable`, `HasHunks`,
  `ActiveConflict`, `AnyConflictUnresolved`, `CanUndo`, `CanRedo`,
  `SelectedPathExists`. `evaluate(ctx) → Availability` returns either
  `Available` or `Unavailable(reason)` with a human-readable tooltip string.

  **`CommandContext`** — minimal state snapshot (11 bool fields) populated
  by the UI at render time. The toolbar, keyboard handler, and command
  palette all derive enabled state from the same evaluation.

  **`CommandDangerLevel`** — `Safe | MayDiscardWork | Destructive`. Ordered.
  `requires_confirmation()`.

  **`CommandCategory`** — 10 variants with `label()`. Used to group commands
  in the palette and menu.

  **`Shortcut { modifiers, key }`** and **`Modifiers`** — keyboard shortcut
  descriptor. `Modifiers::CTRL`, `::ALT`, `::CTRL_SHIFT`, `::NONE` constants.

  **`CommandRegistry`** — `builtin()` constructs all 20+ built-in commands.
  Methods: `get(id)`, `all()`, `by_category(cat)`, `search(query)` (case-
  insensitive label+description match), `find_by_shortcut(shortcut)`.

- **25 new tests** in `tests/command_tests.rs`: availability rule evaluation
  for all 11 rules, unavailable-reason non-emptiness for all rules, danger
  level ordering and confirmation predicate, category labels, registry
  non-empty + ID uniqueness + label non-empty, `get` success and miss,
  `by_category` filtering, `search` case-insensitive + empty + no-match,
  `find_by_shortcut` Ctrl+S → Save, unbound shortcut, `Modifiers::NONE.is_none()`,
  save/undo context wiring.
  Total core test count: 514.

---

## [0.62.0] — 2026-06-10

Text editing operation model — RFC-032 core types.

### Added

- **`forskscope-core::edit_op`** — text editing operation model (RFC-032).

  **`DocumentId`** — stable document identity for the lifetime of a tab.

  **`RevisionId(u64)`** — monotonically increasing document revision.
  `initial()` starts at 0; `next()` increments; `is_initial()` tests.
  Ordering is derived so `RevisionId(n) < RevisionId(n+1)`.

  **`TextOffset(usize)`** — byte offset within document text.

  **`TextRange { start, end }`** — byte range (start inclusive, end
  exclusive). Methods: `len()`, `is_empty()`, `contains(offset)`,
  `overlaps(other)`, `empty_at(offset)`.

  **`TextEditOperation`** — `Insert { offset, text }` / `Delete { range }`
  / `Replace { range, text }`, all tagged with `document` and
  `base_revision`. Methods: `document_id()`, `base_revision()`,
  `affected_range()`, `inserts_text()`, `deletes_text()`.

  **`OperationAck`** — core's acceptance response: `new_revision`,
  `affected_range`, `diff_invalidated` (signals UI to reschedule diff).

  **`OperationReject`** — core's rejection response: `current_revision` and
  `RejectReason` (`StaleRevision | OutOfBounds | DocumentNotEditable`).

  **`is_revision_compatible(op_rev, current_rev) → bool`** — RFC-032 rule 2:
  exact match required; no last-write-wins semantics.

  **`TransactionId`**, **`TransactionLabel`** — transaction identity and
  human-readable undo-menu label. Well-known labels:
  `merge_hunk_left_to_right()`, `merge_hunk_right_to_left()`,
  `manual_edit()`, `paste()`, `delete_selection()`.

  **`EditTransaction`** — `{ id, label, operations, inverse, timestamp }`.
  Merge commands and manual edits both become transactions. `is_empty()`,
  `is_reversible()`.

- **23 new tests** in `tests/edit_op_tests.rs`: `RevisionId` initial/next/
  ordering, `TextRange` len/empty/contains/overlaps/adjacent, all three
  `TextEditOperation` variants (document id, base revision, affected range,
  inserts/deletes predicates, empty-text edge cases), revision compatibility
  (same = ok, stale/future = reject), `OperationReject` fields,
  `TransactionLabel` well-known labels, `EditTransaction` empty/reversible,
  `TransactionId` equality. Total core test count: 489.

---

## [0.61.0] — 2026-06-10

Diff decoration model (RFC-024) and line map / scroll sync model (RFC-035).

### Added

- **`forskscope-core::diff_decoration`** — semantic decoration set (RFC-024).

  **`DiffDecorationSet::from_diff(doc, focused_hunk_id)`**: derives all
  decorations from a `DiffDocument` in one pass. The Dioxus diff component
  receives this and maps to CSS/gutter; no diff logic lives in the component.

  **`LineDecorationKind`** — 7 variants: `Unchanged, Added, Deleted, Modified,
  EmptyCounterpart, Conflict, MergeApplied`. Each has `css_class()` (stable
  `fs-line-*` token), `gutter_symbol()` (`+/-/~/·/!/✓/ `), and `aria_label()`
  for screen-reader accessibility (RFC-009 §7).

  **`InlineDecorationKind`** — 4 variants: `InsertedChars, DeletedChars,
  ReplacedChars, WhitespaceOnly`. Each has `css_class()` (`fs-inline-*`).

  **`LineDecoration`** — `(side, row_index, kind, hunk_id)`.

  **`InlineDecoration`** — `(side, row_index, start_col, end_col, kind)`;
  byte-offset columns matching `InlineSpan`.

  **`HunkDecoration`** — `(hunk_id, start_row_index, end_row_index, is_focused)`;
  drives the hunk navigator and mini-map highlight.

  **`DecorationWarning`** — wraps `DiffWarning` as a banner message with kind
  (`LargeFile, DeadlineExpired, InlineSkipped`).

- **`forskscope-core::line_map`** — aligned row sequence and scroll model (RFC-035).

  **`LineMap::from_diff(doc)`**: builds the full aligned row sequence from a
  `DiffDocument`. Each `AlignedRow` carries `(row_id, left, right, state, hunk_id)`.
  Methods: `row(id)`, `changed_rows()`, `next_changed_row(from)`,
  `prev_changed_row(from)`, `is_identical()`.

  **`RowState`** — `Equal, Inserted, Deleted, Modified, Conflict, Collapsed,
  Unknown`. `is_changed()` predicate. `gutter_symbol()` distinct for all 7.

  **`AlignedRow::is_paired()`** — true when both left and right have a line.

  **`ScrollAnchor`** — `(row_index, row_fraction)` shared by both panes for
  synchronized scrolling. `at_top()`, `clamped(row, fraction)`.

  **`build_mini_map(map) → Vec<MiniMapSegment>`** — collapses consecutive
  same-state rows into segments with weights; total weight equals total row
  count (invariant tested).

- **31 new tests**: 18 in `diff_decoration_tests` (CSS class uniqueness and
  prefix, gutter symbols, aria labels, identical/insert/delete/replace diffs,
  focused hunk, unfocused default) and 13 in `line_map_tests` (RowState
  predicates, gutter symbol uniqueness, identical/insert/delete/replace maps,
  navigation, pairing, ScrollAnchor clamping, mini-map merging and weight sum).
  Total core test count: 466.

---

## [0.60.0] — 2026-06-10

User settings model and JSON persistence (RFC-009 slice).

### Added

- **`forskscope-core::settings`** — user settings model (RFC-009 §4, §6, §10).

  **`UserSettings`**: top-level settings record with four sections.
  Defaults represent a valid first-run state.

  **`AppearanceSettings`**: `theme: ThemeId` (Dark/Light/Night, default Dark),
  `density: Density` (Comfortable/Compact/Spacious), `font_family:
  FontFamilySetting` (SystemMono/SystemSans/SystemSerif), `font_size: u8`
  (clamped 6–50 on load, default 14).

  **`DiffSettings`**: `compare_profile: CompareProfile` (default profile),
  `show_line_numbers: bool` (true), `wrap_long_lines: bool` (false). Reuses
  `CompareProfile` from RFC-028 — the profile name is serialised to JSON and
  looked up in `all_presets()` on load; unknown names fall back to default.

  **`FileSettings`**: `newline_policy: NewlinePolicy` (Preserve), `performance:
  PerformanceLimits` (not persisted — always default; future RFC), `restore_session:
  bool` (true), `recent_limit: usize` (20).

  **`LocaleSettings`**: `locale: LocaleId`. Default is `"en"`.

  **`ThemeTokens::css_var_names(ThemeId) → Vec<(&str, &str)>`**: returns the 12
  CSS variable names (`--fsk-*`) for a theme. The Dioxus app injects these as
  `:root` variables; core is not involved in rendering.

  **`UserSettings::to_json` / `from_json`**: persist via `VersionedEnvelope`
  with `SchemaName::Settings` and `SETTINGS_SCHEMA_VERSION = 1`. `from_json`
  enforces the migration policy (error on `TooNew`). Per RFC-009 §10: unknown
  or corrupt payload fields silently fall back to defaults rather than rejecting
  the file — the envelope is the version gate, not the payload parser.

- **15 new tests** in `tests/settings_tests.rs`: default values, all
  `ThemeId`/`Density`/`FontFamilySetting` round-trips, CSS variable name
  count and prefix, JSON round-trip for defaults and non-defaults, schema
  version in output, newer-schema rejection, corrupt-payload fallback,
  `LocaleId` helpers, font_size clamping on load.
  Total core test count: 435.

---

## [0.59.0] — 2026-06-10

Application error taxonomy (RFC-017 slice) and file-size classification (RFC-013 slice).

### Added

- **`AppErrorKind`** in `forskscope-core::error` (RFC-017 §5).

  25-variant enum covering the full taxonomy of user-facing situations:
  path/filesystem errors, encoding, comparison, merge/save, background
  jobs, session, VCS, spreadsheet, and internal faults.

  **`default_severity(self) → ErrorSeverity`** — each kind's baseline
  severity level (Info / Warning / Recoverable / Blocking).

  **`default_recovery_actions(self) → &[RecoveryAction]`** — the typed
  set of dialog buttons appropriate for each kind. The UI pattern-matches
  the returned slice to render action buttons without hard-coding
  per-error-code logic.

  **`from_core(err: &CoreError) → AppErrorKind`** — best-effort mapping
  from the lower-level `CoreError` taxonomy to the application-layer kind.
  All `CoreError` variants are covered.

- **`RecoveryAction`** in `forskscope-core::error` (RFC-017 §"Recovery Actions").

  12-variant enum: `Dismiss`, `ChooseAnotherFile`, `Reload`, `SaveAs`,
  `OverwriteAnyway`, `OpenLimitedDiff`, `OpenAsBinary`, `Retry`,
  `RetryWithoutInline`, `Cancel`, `StartFresh`, `ReportBug`.

  **`token(self) → &'static str`** — stable string for keybinding / i18n
  lookup. All tokens are unique and non-empty.

  **`is_destructive(self) → bool`** — true for `OverwriteAnyway` and
  `StartFresh`; used by the UI to add an extra confirmation step.

- **`UserMessage`** in `forskscope-core::error` (RFC-017 §"UserMessage").

  `{ short: String, detail: String }` pair. `short` fits a toast or
  dialog title; `detail` fits a dialog body.

  **`for_kind(AppErrorKind) → UserMessage`** — standard copy for all 25
  kinds. Non-empty `short` guaranteed for every variant (test-verified).

- **`FileSizeClass`** in `forskscope-core::job` (RFC-013 §5).

  `Small | Medium | Large | VeryLarge` — derives `PartialOrd/Ord`
  ascending by severity.

  **`classify(bytes, limits) → FileSizeClass`** — maps a byte count to
  a class using `PerformanceLimits` thresholds.

  Predicates: `inline_diff_eager()` (Small only), `requires_user_prompt()`
  (Large + VeryLarge), `too_large_for_diff()` (VeryLarge only).

- **`PerformanceLimits`** in `forskscope-core::job` (RFC-013 §5).

  `Default`: Small ≤ 512 KiB, Medium ≤ 4 MiB, Large ≤ 64 MiB,
  VeryLarge > 64 MiB. Also: `max_inline_diff_chars_per_hunk: 2_000`,
  `max_directory_entries_eager: 500`, `max_eager_lines: 50_000`.

- **35 new tests**: 20 in `app_error_tests` (severity, recovery actions,
  `from_core` mapping, token uniqueness, destructive flag, `for_kind`
  exhaustiveness) and 15 in `file_size_tests` (boundary conditions,
  predicates, ordering, custom limits). Total: 420 core tests.

---

## [0.58.0] — 2026-06-10

Directory index model, equality evidence, and pair comparison (RFC-008 §5, RFC-037 §"Directory Index").

### Added

- **`forskscope-core::dir::index`** — directory index model and equality evidence (RFC-008 §5, RFC-037).

  **`DirectoryIndex`**: snapshot of one directory tree. Fields: `root`, `revision: IndexRevision`, `entries: Vec<DirectoryEntryRecord>`, `ignored_count`, `is_complete`. Constructors: `empty(root)`, `from_records(root, entries, is_complete)`. Methods: `get(rel)`, `files()`, `directories()`.

  **`DirectoryEntryRecord`**: one file in the index — `relative_path`, `entry_type: EntryType`, `size`, `modified`, `digest: Option<ContentDigest>`, `error`. Predicates: `has_error()`, `has_digest()`.

  **`ContentDigest`**: algorithm + hex pair. `fnv1a64(hex)` constructor. `matches(other)` — requires same algorithm and same hex (different-algorithm digests are incomparable, never equal).

  **`EqualityEvidence`** (RFC-008 §5): ten-variant enum encoding the comparison verdict for one path pair: `DigestEqual | MetadataEqual | MetadataOnly | LeftOnly | RightOnly | TypeMismatch | SizeDifferent | DigestDifferent | Error | Unknown`. Predicates: `is_equal()`, `is_different()`, `is_pending()`, `present_on_both_sides()`.

  **`pair_entries(left, right) → PairedEntrySet`** — pairs two `DirectoryIndex` instances by relative path and computes `EqualityEvidence` for each path, following the RFC-008 §5 strategy in order: missing-side → `LeftOnly`/`RightOnly`; error → `Error`; type mismatch → `TypeMismatch`; size differs → `SizeDifferent` (skip digest); both digests present → `DigestEqual`/`DigestDifferent`; same mtime → `MetadataEqual`; else → `MetadataOnly`.

  **`PairedEntrySet`**: `entries: Vec<PairedEntry>` with `equal_count()`, `different_count()`, `pending_count()`, `left_only_count()`, `right_only_count()`.

  **`IndexRevision`**: newtype `u64` with `next()`. Incremented on each rescan.

- **25 new tests** in `tests/dir_index_tests.rs`: empty index, `get`, `files`/`directories` iterators, `ContentDigest::matches` (same/different hex, different algorithm), all `EqualityEvidence` predicates, all 9 `pair_entries` comparison branches, `PairedEntrySet` counts, empty-both-sides, revision `next()`. Total core test count: 385.

---

## [0.57.0] — 2026-06-10

sheets-diff v2.2.1 migration — structured result, no catch_unwind,
formula text, cancellation, richer sheet changes (RFC-058).

### Changed

- **`forskscope-core`: sheets-diff upgraded `1.1` → `2.2.1`** (RFC-058
  re-implementation). The adapter boundary held perfectly: no `sheets-diff`
  types escaped `xlsx.rs`; no other file changed.

  **`xlsx.rs` rewritten for v2:**

  - **`catch_unwind` removed.** `compare_paths_with_options` returns
    `Result<WorkbookDiff, SheetsDiffError>`; the v1 panic risk is gone.

  - **One `CellChange` per address.** Value and formula changes on the same
    cell are now facets of one entry (Q1 resolution). Previously they could
    produce two separate rows (v1 artifact). `CellChangeKind` enum removed;
    replaced by `value_changed: bool` + `formula_changed: bool`.

  - **`CellChange` carries `old_formula`/`new_formula`** (`Option<String>`).
    Formula text is now surfaced at the adapter boundary without dropping
    into the upstream model (v2.2.1 `CellChangeRow::old_formula/new_formula`,
    FR2 addition).

  - **`SheetChange` extended.** New variants: `Modified(String)`,
    `Renamed { old_name, new_name }`, `Moved(String)` alongside existing
    `Added`/`Removed`. `derive_pair_text` renders `~` prefix for renames.

  - **`SpreadsheetDiffStats` from `wb.summary`.** `values_changed`,
    `formulas_changed`, `sheets_renamed`, `sheets_moved` now populated
    directly from `WorkbookDiff.summary` instead of manual counting.
    `sheets_modified` added.

  - **`CancellationToken` wired.** `diff_xlsx(old, new, cancel: Option<&CancellationToken>)`
    — token maps to v2's `Cancellation` trait via `move || tok.is_cancelled()`.
    Granularity is per-sheet (sub-sheet cancellation planned in sheets-diff;
    documented in FR2 reply). Pass `None` for existing callers.

  - **`drop(wb)` explicit after conversion.** All `cell_diffs` released
    immediately; only owned `SpreadsheetDiff` survives.

- **`xlsx_tests.rs`**: 9 existing tests updated for new API; 2 new tests
  added (`stats_are_driven_from_workbook_summary`, `cancellation_token_does_not_affect_small_workbook`).
  Sheet-structural test updated to accept `Renamed` (v2's heuristic sheet
  matching correctly classifies `Sheet1 → NewSheet` as a rename, not
  Added+Removed). Total core test count: 360.

---

## [0.56.0] — 2026-06-10

Workspace session model and JSON persistence (RFC-011 slice).

### Added

- **`forskscope-core::session`** — workspace session model (RFC-011).

  **`WorkspaceSession`**: canonical session record outside any Dioxus
  component state. Constructors: `empty()` (empty startup), `from_file_pair`
  (two-file startup args), `from_directory_pair` (two-directory args). Tab
  operations: `open_tab`, `close_tab` → `CloseResult`, `force_close_tab`,
  `mark_tab_dirty`, `mark_tab_clean`. Queries: `any_dirty()`, `dirty_tabs()`,
  `active_tab()`. `SessionId` and `TabId` are stable across redraws
  (RFC-011 §12).

  **`WorkspaceRoot`**: `Empty | FilePair(FilePairRoot) |
  DirectoryPair(DirectoryPairRoot)` — the top-level context for the workspace.

  **`WorkspaceTab`**: `Diff(DiffTabSession) | Binary(BinaryTabSession) |
  Excel(ExcelTabSession) | Error(ErrorTabSession)`. Only `DiffTabSession` has
  an `is_dirty` flag; all other tab kinds are always clean.

  **`CloseResult`**: `Closed | BlockedDirty | NotFound`. `BlockedDirty` is
  the signal for the UI to show the unsaved-changes dialog (RFC-011 §5.4).
  `force_close_tab` bypasses the check after user confirmation.

  **`RecentSessionEntry`**: metadata-only (title, paths, kind, timestamp).
  `paths_available()` checks whether both paths still exist on disk; missing
  paths are visible but marked unavailable (RFC-011 §9).

  **`WorkspaceSession::to_json` / `from_json`**: wraps in a
  `VersionedEnvelope` with `SchemaName::Session` and
  `SESSION_SCHEMA_VERSION = 1`. `from_json` enforces the migration policy:
  returns `SessionParseError::TooNew` when the file was written by a newer
  ForskScope (prevents silent overwrite of future-format data).

- **21 new tests** in `tests/session_tests.rs` covering all 10 RFC-011 §13
  testing requirements and all §14 acceptance criteria: empty/file-pair/
  directory-pair constructors, open multiple tabs, close clean tab, dirty-tab
  block, mark-clean-then-close, recent entries with existing/missing paths,
  JSON round-trip for all root kinds, newer-schema error, stable session
  identity, dirty-tab visibility, structural no-content guarantee.
  Total core test count: 358.

---

## [0.55.0] — 2026-06-10

External tool command model and safe argument expansion (RFC-029 slice).
endringer evaluation note recorded in `rfcs/notes/`.

### Added

- **`forskscope-core::external_tool`** — external tool command model and
  safe argument expansion (RFC-029 §"API sketch", §"Security policy").

  **`ExternalToolCommand`** — id, name, executable path, argument template
  (`Vec<ExternalToolArg>`).

  **`ExternalToolArg`** — `Literal(String)` or `Placeholder(...)`. The split
  means literal flags like `"--goto"` and typed placeholders like `{path}`
  are represented distinctly, making the template inspectable and serialisable.

  **`ExternalToolPlaceholder`** — five variants: `Path`, `LeftPath`,
  `RightPath`, `Line`, `Column`. `token()` returns the `{token}` string used
  in the settings UI. `from_token()` parses it. `all()` returns them in
  display order.

  **`expand_args(cmd, ctx) → Vec<String>`** — the core function. Expands a
  command template against an `ExpandContext`. The result is a plain
  `Vec<String>` ready for `std::process::Command::args` — **no shell string
  is ever constructed**. Missing context values (e.g. no line number when
  revealing in file manager) silently omit the argument rather than producing
  a literal `"None"` string or panicking.

  **`parse_arg(s)`** — used by the settings validator. Accepts known tokens
  and plain strings; rejects apparent `{token}` strings that are not in the
  supported set, protecting users from typos like `{pat}` silently becoming
  a literal argument.

  **`UnknownTokenError`** — structured error from `parse_arg`, carrying the
  rejected token and listing valid alternatives in its `Display`.

- **20 new tests** in `tests/external_tool_tests.rs` covering:
  literal pass-through, all five placeholder variants, mixed templates,
  the security contract (paths containing spaces, `;`, `$HOME`, and
  backticks each arrive as a single intact argument — no shell splitting),
  missing-context omission (not `"None"` string), `parse_arg` acceptance,
  typo rejection, token round-trips. Total core test count: 337.

### Notes

- **`rfcs/notes/endringer-evaluation-v0.22.0.md`** — evaluation note
  recording endringer v0.22.0 as the preferred path for a future RFC-038
  backend upgrade. No code change now. See note for the migration plan.

---

## [0.54.0] — 2026-06-10

VCS context integration — GitProvider and VcsProvider trait (RFC-038).

### Added

- **`forskscope-core::vcs`** — VCS context integration boundary (RFC-038).

  **`VcsProvider` trait** — read-only interface implemented by all providers:
  `root()`, `system_name()`, `status() → Vec<VcsFileStatus>`,
  `read_revision_file(rev, path) → Vec<u8>`, `merge_base(left, right)
  → Option<VcsRevision>`. No write operations are in scope.

  **`GitProvider`** — detects a repository by walking upward from a given
  path looking for `.git`. Implements all four trait methods via bounded,
  explicit `git` subprocesses (argument arrays, no shell string expansion).
  Status parsing covers Modified, Added, Deleted, Renamed, and Conflicted
  from `git status --porcelain -u`. File contents are read via `git show
  <rev>:<path>` and returned as raw bytes for the caller to decode through
  `load_path`. Merge base via `git merge-base`.

  **`VcsFileChange`** — `Modified | Added | Deleted | Renamed { from } |
  Conflicted | Other(String)`.

  **`VcsRevision`** — opaque string wrapper. `head()` → `"HEAD"`,
  `working_tree()` → `"WORKING"`.

  **`detect(path) → Option<Box<dyn VcsProvider>>`** — top-level entry
  point. Returns `None` outside any supported VCS; ForskScope works fully
  without VCS context.

- **13 new tests** in `tests/vcs_tests.rs` using real git repos in temp
  directories: detect inside/outside/from-subdirectory a repo; `root()` is
  the repo root; clean working tree has empty status; untracked file →
  `Added`; modified file → `Modified`; deleted file → `Deleted`; HEAD file
  content; nonexistent path → `Err`; merge-base of HEAD with itself;
  `GitProvider::detect` outside repo → `None`; revision `Display`.
  Total core test count: 317.

### RFC

- RFC-038 moved from `proposed/` to `done/`. Remaining open: VCS Changes
  Panel UI, JJ provider, conflicted-path surfacing in the three-way merge
  flow, and wiring `read_revision_file` to the "Compare with HEAD" action.

---

## [0.53.0] — 2026-06-10

External file state detection (RFC-036 slice).

### Added

- **`ExternalFileState`** in `forskscope-core::document` (RFC-036 §"File State").

  Six variants ordered by severity of action required:
  - `Clean` — file matches load-time snapshot, no session edits.
  - `DirtyInSession` — file matches snapshot, session has unsaved edits.
  - `ChangedOnDisk` — file differs from snapshot (size or mtime changed).
    Saving would overwrite the external change.
  - `DeletedOnDisk` — path no longer exists.
  - `ReplacedOnDisk` — path exists but is no longer a regular file (e.g.
    replaced by a directory).
  - `Unknown` — metadata unavailable; state indeterminate.

  Predicates:
  - `blocks_save()` — `true` for Changed, Deleted, Replaced. The UI uses
    this to gate the save button and trigger the reconciliation dialog.
  - `file_accessible()` — `true` for Clean, DirtyInSession, ChangedOnDisk
    (the file is reachable, whatever its content). `false` for Deleted,
    Replaced, Unknown.

- **`check_external_state(path, snapshot, is_session_dirty) → ExternalFileState`**
  — compares the live filesystem metadata against the `FileFingerprint`
  captured at load time. Detection order: missing → `DeletedOnDisk`;
  non-file → `ReplacedOnDisk`; size differs → `ChangedOnDisk`; mtime
  differs → `ChangedOnDisk`; same → `DirtyInSession` or `Clean`. Never
  returns `Err` — metadata failures return `Unknown`. This is the
  pre-save interlock specified in RFC-036 §"Save Interlock".

- **15 new tests** in `tests/external_state_tests.rs`:
  clean/dirty-in-session states, size change, mtime change (with note on
  coarse-grained filesystem clocks), deleted file, replaced-by-directory,
  never-panic guarantee, all predicate states. Total core test count: 304.

---

## [0.52.0] — 2026-06-10

Directory merge action planner and operation plan model (RFC-022 slice).

### Added

- **`forskscope-core::dir::merge_plan`** (RFC-022) — turns a directory
  comparison result into a previewable, executable operation plan.

  **`plan_operations(entries, left_root, right_root, direction, selection)
  → OperationPlan`** — accepts `Vec<RecEntry>` from `recursive_diff`,
  applies a `CopyDirection` (L→R or R→L) and an `EntrySelection` filter
  (AllNonEqual / ChangedOnly / SourceOnlyEntries), computes source/target
  paths for each entry, runs preflight checks, and returns a plan with a
  `RiskSummary`. Equal and Computing entries are excluded automatically.
  Entries that are on the wrong side for the chosen direction become
  `DirectoryMergeAction::Skip`.

  **`OperationPreflight`** — per-file pre-execution checks captured at plan
  time: `target_exists`, `target_writable` (best-effort), `backup_required`
  (true when target exists), `estimated_bytes`.

  **`RiskSummary`** — `total_files`, `new_files`, `overwrites`,
  `estimated_bytes`, `permission_blocks`. Drives the batch preview dialog:
  `OperationPlan::is_safe_to_execute()` is `true` when `permission_blocks
  == 0`.

  **`execute_plan(plan, BackupPolicy, BatchFailurePolicy) → PlanExecutionReport`**
  — creates missing parent directories, delegates to `batch_copy`, and
  returns per-file `FileOutcome` (Copied / Skipped / Failed) with backup
  presence reported.

- **15 new tests** covering: L→R / R→L direction, all `RecStatus` variants,
  `EntrySelection` filters, risk summary accuracy, preflight target detection,
  execute round-trip, backup creation on overwrite, skip count reporting, and
  empty entry list. Total core test count: 289.

---

## [0.51.0] — 2026-06-10

Versioned schema envelope and migration policy for all persisted data (RFC-031).

### Added

- **`forskscope-core::persist`** — versioned data envelope and schema
  migration policy (RFC-031 §"Versioned app data", §"Migration policy").

  Every persisted file (settings, profiles, sessions, manifests, reports)
  is wrapped in a `VersionedEnvelope` containing: `schema_name`, `schema_version`,
  `app_version`, `created_unix`, `updated_unix`, and a pre-serialized JSON
  payload. The envelope is independent of `serde` — serialization is
  hand-written via `std::fmt::Write`, consistent with the project pattern.

  **`SchemaName`** — `Settings | Profiles | Session | BatchManifest | Report
  | Unknown(String)`. `as_str()` / `from_str_pub()` round-trip through
  their canonical names.

  **`VersionedEnvelope::parse(json)`** — a minimal hand-written parser
  that extracts the envelope metadata and the raw payload JSON as a
  substring. Handles nested objects `{...}` and arrays `[...]` as payload
  via balanced-delimiter counting (no full JSON grammar needed for the
  envelope shape).

  **`MigrationPolicy`** — the four RFC-031 decisions:
  - `CompatibleRead` — version matches; use directly.
  - `ForwardMigration { from_version }` — older file; app may migrate.
  - `NewerSchema { file_version, app_version }` — newer file; do not
    overwrite, ask user to upgrade.
  - `UnknownSchema { schema_name }` — unrecognised schema; preserve untouched.

  Predicates: `is_compatible()`, `can_write()`.

- **19 new tests** covering: schema name round-trips, envelope JSON
  structure, payload embedding, nested-object and array payload parse,
  round-trip of all envelope fields, missing-field error cases, all four
  migration policy branches, and all policy predicates.
  Total core test count: 274.

### RFC

- RFC-031 moved from `proposed/` to `done/`. Remaining open: per-schema
  migration execution functions, settings/profile/session serialization
  driving this envelope, and crash-recovery journal integration.

---

## [0.50.0] — 2026-06-10

Editability classification, newline save policy (RFC-012 slice) and compare profiles (RFC-028 slice).

### Added

- **`EditabilityClass`** in `forskscope-core::file_kind` (RFC-012 §8).

  Ordered by capability (`Unsupported < ReadOnly < ReadWriteWithGuard <
  ReadWrite`). `FileKind::editability(had_decode_errors, encoding_label)`
  derives the class at load time. Predicates: `is_editable()`,
  `is_saveable()`, `requires_save_guard()`.

  Mapping: `Text` + UTF-8 + no errors → `ReadWrite`; `Text` + non-UTF-8
  or decode errors → `ReadWriteWithGuard` (warn before save); `Binary`,
  `ExcelXlsx`, `Missing` → `ReadOnly`; `Unsupported` → `Unsupported`.

- **`NewlinePolicy`** in `forskscope-core::encoding` (RFC-012 §6).

  `Preserve` (default) / `ForceLf` / `ForceCrlf`. `resolve(detected_style)
  → Option<&str>` returns the newline string to use when writing. `Preserve`
  on `Mixed` or `None` returns `None` — the caller preserves per-line endings
  rather than normalizing (RFC-012 rule 2: "preserve exact line endings where
  possible for mixed-newline files").

- **`WhitespaceMode`**, **`NewlineCompareMode`**, **`CaseSensitivity`** in
  `forskscope-core::diff` (RFC-028 §"Compare option types"). Typed enums
  replacing the bare booleans in `DiffOptions` at the profile layer. All
  default to the "significant / sensitive" value matching existing behaviour.

- **`CompareProfile`** in `forskscope-core::diff` (RFC-028 §"Default
  profiles"). A named preset carrying whitespace, newlines, case,
  inline_mode, and algorithm. Four built-in presets via associated functions:
  `default_profile`, `code_review` (Histogram algorithm — better hunk
  alignment for code), `loose_text` (ignore trailing whitespace and newline
  differences), `large_file_safe` (inline diff disabled). `all_presets()`
  returns them in display order. `to_diff_options()` converts to the engine
  type. `Default` is `default_profile`.

- **35 new tests** (21 editability, 14 compare profile). Total: 255 core.

---

## [0.49.0] — 2026-06-10

Report export: Markdown and JSON comparison reports (RFC-027).

### Added

- **`forskscope-core::report`** — comparison report engine (RFC-027).

  **`FileComparisonReport`**: built from a `DiffDocument` with optional
  `TransactionLog` (for operation history) and optional path display.
  `to_markdown()` produces a structured Markdown document with Summary,
  Compare Options, Warnings, Changed Hunks, and Operation History sections.
  `to_json()` produces a JSON object with schema version 1.

  **`DirComparisonReport`**: built from `Vec<RecEntry>` with optional
  `BatchManifest` (for batch operation summary) and optional root paths.
  `to_markdown()` and `to_json()` follow the same section structure.

  **`ReportPathMode`** — `NameOnly` (default, safe to share) / `Relative` /
  `Absolute`. The default deliberately omits directory paths so reports can
  be shared without leaking project layout.

  **`ReportOptions`** — `include_hunks`, `include_history`,
  `include_options`, `include_warnings`, `include_sizes`, `path_mode`. All
  sections are on by default; callers opt out by field.

  **JSON schema v1**: `schema_version`, `app_version`, `kind`
  (`"file_comparison"` or `"directory_comparison"`), `summary`, `options`,
  `warnings`, `hunks` / `files`. No `serde` dependency — serialization is
  hand-written with `std::fmt::Write`, consistent with the project pattern
  in `BatchManifest::to_json()`.

- **20 new tests** in `tests/report_tests.rs`: Markdown title and section
  presence, identical vs different status, hunk table, options section, JSON
  object structure, schema version, kind field, identical flag, privacy
  (name-only strips absolute paths, absolute mode shows them), directory
  summary counts, equal files excluded from changed-files table, sizes in
  default mode, directory JSON files array. Total core test count: 220.

### RFC

- RFC-027 moved from `proposed/` to `done/`. HTML format, the export dialog
  UI, and CSV/PDF remain open (see RFC-027 §"Future formats" and §"Non-goals").

---

## [0.48.0] — 2026-06-10

Crate architecture: classify by function, not framework (RFC-020 §5a).

### Changed

- **`forskscope-explorer-align` → `forskscope-ui-logic`.** The crate had
  outgrown its name (it held alignment *and* search-index logic). It is now
  scoped to *all* framework-independent presentation logic — the view-model
  layer — and remains fully testable without a display server. Feature areas
  are now modules:
  - `explore::align` (was `align`)
  - `compare::search_index` (was `search_index`)
  - `settings` reserved for when pure settings logic emerges.

- **`forskscope-ui-dioxus` → `forskscope-ui`.** The `-dioxus` suffix
  documented an implementation choice the project already committed to
  (Dioxus is *the* UI target per RFC-042) and conveyed nothing about the
  crate's role. The library target is renamed `forskscope_ui`; the
  `forskscope` binary name is unchanged.

- Crate dependencies, workspace members, the two UI re-export shims
  (`ui/explorer_align.rs`, `ui/search_index.rs`), README, and maintainer
  docs updated to the new names. The shim pattern meant the rename touched
  only two lines of actual UI component code.

### RFC

- RFC-020 §5a records the settled three-crate architecture
  (`forskscope-core` / `forskscope-ui-logic` / `forskscope-ui`), the
  function-over-framework naming rationale, the module-vs-crate boundary
  policy (feature areas are modules until a concrete need — chiefly
  GTK-free testability — justifies a crate), and why per-widget crates are
  not adopted at current scale. The original §5 sketch (which named
  `forskscope-dioxus`) is retained but marked superseded.

### Notes

- Crate counts unchanged (3). Test counts unchanged (200 core + 2 patch
  integration + 22 ui-logic). No behavioral change; this is a structural
  and naming release.

---

## [0.47.0] — 2026-06-10

Transaction log and unified merge operation history (RFC-015).

### Added

- **`TransactionLog`** in `forskscope-core::merge` (RFC-015) — a companion
  struct that can be attached to either `MergeSession` (two-way) or
  `ThreeWayMergeSession` to provide enriched, queryable operation history.
  The existing session undo/redo stacks are unchanged; `TransactionLog` is
  the *metadata layer* RFC-015 calls for.

  Key API:
  - `push(TransactionKind)` — record a new operation; clears the redo branch.
  - `record_undo()` / `record_redo()` — sync with the session stack.
  - `mark_saved()` — set clean baseline.
  - `is_dirty()`, `can_undo()`, `can_redo()` — state queries.
  - `active_entries()`, `undone_entries()`, `all_entries()` — for the
    history panel: all entries are kept (including undone) so the panel can
    show the full session history.
  - `active_ops_since_save()` — count of dirty operations.

- **`TransactionKind`** — typed enum with variants for every current merge
  operation, each carrying its `HunkId` or `ConflictId`. `kind.label()`
  returns a human-readable English description for the history panel.

- **`SessionRevision`** — a typed `u64` newtype replacing the raw `usize`
  save-baseline offset. `INITIAL` is revision 0; each `push()` increments.
  Revisions are `Ord`, making dirty-state a direct comparison.

- **`TransactionEntry`** — one log record: `revision`, `kind`, `label`,
  `timestamp` (`UnixTimestamp`), and `active` (false when undone). Undone
  entries stay in the log for the history panel.

- **23 new tests** covering all RFC-015 §13 requirements: push/undo/redo
  semantics, revision tracking, dirty state and baseline, redo-branch
  discard on new push, entry visibility splits, labels, and integration
  with both session types. Total core test count: 200.

### RFC

- RFC-015 moved from `proposed/` to `done/`. The history panel UI (§10),
  persistent crash-recovery journal (deferred in RFC-015 §4), and
  editor-local vs core undo precedence (§9) remain open.

---

## [0.46.0] — 2026-06-10

Error severity/recovery model (RFC-017 slice) + job progress model and threshold policy (RFC-013 slice).

### Added

- **`ErrorSeverity`** and **`RecoveryHint`** in `forskscope-core::error`
  (RFC-017 §"Error Severity", §"Recovery Actions").

  Every `CoreError` now answers two questions without string parsing:
  - `severity()` → `Info | Warning | Recoverable | Blocking` — lets the UI
    choose a toast, inline warning, or blocking modal automatically.
  - `recovery_hint()` → `ChooseAnotherFile | Reload | SaveAs |
    OverwriteAnyway | CheckPermissions | Dismiss | ReportBug` — the primary
    recovery action to offer.
  - `is_user_recoverable()` — convenience predicate: `true` when severity
    is ≤ `Recoverable`.

  Severity mapping highlights: Conflict → Recoverable (Reload); read/listdir
  I/O → Recoverable (ChooseAnotherFile); write/rename I/O → Blocking (SaveAs);
  InternalInvariant → Blocking (ReportBug). `ErrorSeverity` implements `Ord`
  so the UI can compare levels directly.

- **Threshold policy constants** in `forskscope-core::job` (RFC-013
  §"Thresholds") — the single source of truth for large-file behaviour:

  | Constant | Value | Governs |
  |---|---|---|
  | `LARGE_FILE_INLINE_DIFF_BYTES` | 512 KB | disable inline diff auto-compute |
  | `VERY_LARGE_FILE_BYTES` | 10 MB | further constrain diff deadline |
  | `LARGE_HUNK_AUTO_EXPAND_LINES` | 10 000 | suppress auto-expand for collapsed hunks |
  | `LARGE_DIRECTORY_VIRTUAL_THRESHOLD` | 5 000 | switch explorer to windowed rendering |
  | `DIGEST_CONCURRENCY_LIMIT` | 32 | back-pressure on in-flight digest tasks |

- **`JobKind`**, **`JobProgress`**, **`JobHandle`** in `forskscope-core::job`
  (RFC-013 §"Background Job Model", RFC-008).

  `JobProgress { job_id, kind, phase, completed_units, total_units,
  cancellable }` is the snapshot the UI renders for progress bars.
  `fraction()` returns `Option<f32>` (0.0–1.0, clamped); `is_complete()`
  is true when `completed_units ≥ total_units`. `JobHandle::new(id)` pairs
  a `JobId` with a `CancellationToken` — caller holds the handle, worker
  holds the token clone.

- **35 new tests** (21 error, 14 job). Total core test count: 177.

---

## [0.45.0] — 2026-06-10

Spreadsheet structural diff adapter and test corpus (RFC-058).

### Added

- **`SpreadsheetDiff` model** in `forskscope-core::xlsx` (RFC-058) —
  app-owned, no `sheets-diff` types in the public API:
  `SpreadsheetDiff { sheets, cells, stats }`, `SheetChange`
  (Added/Removed), `SheetCellChanges`, `CellChange { addr, row, col, kind,
  old, new }`, `CellChangeKind` (Value/Formula), `SpreadsheetDiffStats`.

- **`diff_xlsx(old, new) -> Result<SpreadsheetDiff>`** — the
  `sheets-diff::Diff::new` call is wrapped in `std::panic::catch_unwind`.
  The upstream crate uses `.expect()` internally, which panics on any
  unreadable or corrupt workbook. The wrap converts a caught panic to
  `CoreError::Unsupported` so the core's no-panic contract is honoured.

- **`derive_pair_text_from_diff`** — replaces the previous approach of
  flattening `sheets-diff`'s own unified-text renderer. The derived text is
  now built from `SpreadsheetDiff`, preserving the user-visible format while
  making the structured data available to future UI layers.

- **Test corpus** (9 tests, fixtures generated at test time with the `zip`
  dev-dep — no opaque binary blobs committed):
  identical workbooks produce empty diff;
  changed cell reports correct `addr`, `row`, `col`, `old`, `new`;
  empty-to-value cell has `old: None`;
  sheet name difference produces `SheetChange`;
  malformed first or second file returns `Err`, not a panic;
  multiple changed cells all appear in the model;
  `derive_pair_text_from_diff` non-empty for changes, empty for identical.

### Changed

- `xlsx.rs` entirely rewritten. `load_placeholder` and `derive_pair_text`
  (the existing entry points used by the document loader) are preserved with
  identical signatures; `derive_pair_text` now delegates to the structured
  model path.

### RFC

- RFC-058 moved from `proposed/` to `done/`. The aligned cell-grid UI
  workspace and performance bounds for very large workbooks remain deferred
  (see RFC-058 §"Graduation Criteria").

---

## [0.44.0] — 2026-06-10

Batch copy with restore manifest (RFC-023 §"Batch operation manifest").

### Added

- **`batch_copy`** in `forskscope-core::dir` (RFC-023) — runs a slice of
  `BatchItem` (src/dst path pairs) with configurable `BackupPolicy` and
  `BatchFailurePolicy`. Each successful copy creates a `.bak` sibling of
  the destination (same policy as single-file save). Returns a
  `BatchManifest` recording every outcome.

- **`BatchManifest`** — carries an `OperationId` (`op-<unix_secs>-<pid>`),
  app version, timestamp, and a `Vec<ManifestEntry>` where each entry holds
  `(src, dst, EntryOutcome)`. `EntryOutcome` is `Copied { bytes, backup_path }`,
  `Skipped { reason }`, or `Failed { error }`. Convenience methods:
  `succeeded()`, `failed()`, `attempted()`, `backup_paths()`.

- **`BatchManifest::to_json()`** — deterministic JSON serialization using
  `std::fmt::Write` (no `serde` dependency added to core). Combined with
  `write_to_dir(dir)` which writes `<op-id>.json` to the provided directory
  and records the path in `manifest_path`.

- **`BatchFailurePolicy`** — `StopOnFirst` (default) marks remaining items
  as `Skipped` and stops; `ContinueOnFailure` attempts all items and
  collects all failures.

- **`restore_from_manifest`** — copies each `.bak` backup back to its
  original destination. Skips entries without a backup (newly created files
  have no prior state to restore). Returns the count of restored files.

- **9 new tests** in `tests/batch_tests.rs` validating: all-success path,
  backup creation on overwrite, stop-on-first skips remainder, continue
  collects all outcomes, manifest written to directory, manifest JSON
  structure, operation ID format, restore recovers files, restore skips
  entries without backup. Total core test count: 133.

---

## [0.43.0] — 2026-06-10

Search next/prev traversal and match navigation (RFC-014 slice).

### Added

- **`MatchIndex`** in `forskscope-explorer-align` (`search_index` module,
  RFC-014 §"Text Search") — a pure data engine with no Dioxus or GTK
  dependency. Builds an ordered list of `(hunk_id, row_index, MatchSide)`
  positions from a hunk snapshot and a query string, then exposes:
  `next()` / `prev()` (both wrapping), `reset_focus()`, `focused()` /
  `focused_number()`, `matching_hunk_ids()` (for auto-expand), and
  `is_focused()`. Case-insensitive substring matching; `MatchSide::Both`
  when a row matches on both sides. 13 unit tests.

- **`SearchBar` Prev/Next navigation** — the search bar now shows ▲ / ▼
  buttons (keyboard: Shift+Enter / Enter), a focused-match counter
  ("3 / 12"), and a "No matches" label with `aria-live` so screen-reader
  users are informed without polling.

- **Auto-expand on search** — hunks containing matches are automatically
  added to the expanded set so results are visible without manual expand.

- **Scroll-to-match** — `scrollIntoView` fires on first match, on Prev/Next,
  and on Enter/Shift+Enter in the search input.

- **F3 shortcut** — wired in the global `onkeydown` handler alongside F7/F8.

### Changed

- `forskscope-explorer-align` crate expanded into a two-module pure-logic
  crate: `align` (the existing aligned-row computation) and `search_index`
  (the new match index). Re-exports at the crate root keep existing
  `use` statements in the UI crate unchanged.

---

## [0.42.0] — 2026-06-10

Cancellable directory comparison and explicit symlink handling (RFC-037 slice).

### Added

- **`CancellationToken`** in `forskscope-core` (RFC-037 §"Cancellation") —
  a lightweight `Arc<AtomicBool>` wrapper usable from any blocking task.
  `cancel()` is observed by all clones; `is_cancelled()` is a cheap atomic
  read. No async machinery; the UI layer wires it to a tokio task or a
  thread-local handle as appropriate.

- **`recursive_diff_with_cancel`** and
  **`list_recursive_for_display_with_cancel`** — cancellable variants of the
  two recursive directory-scan functions. Cancellation is checked before the
  scan starts and at each directory entry; partial results are returned
  without blocking or panic. The original non-cancellable entry points are
  preserved as thin wrappers over the new variants so call sites are
  unchanged.

- **`RecStatus::Symlink`** — symlinks encountered during a recursive scan
  are now explicitly reported with this status rather than silently skipped
  by `.flatten()`. The patch-directory builder emits a `BinaryNotice` for
  symlinks when `include_binary_notices` is set.

- **8 new tests** in `tests/dir_cancel_tests.rs`:
  token unit tests (starts uncancelled, cancel propagates to all clones,
  clone cancel propagates back); pre-cancelled token returns no digest
  results; mid-scan cancel produces partial results without panic;
  uncancelled result matches the non-cancellable API; symlink reported as
  `RecStatus::Symlink` in both full-diff and fast-listing paths (Unix).
  Total core test count: 124 (plus 2 integration, 9 alignment).

---

## [0.41.0] — 2026-06-10

RFC triage + Explorer/Compare audit remediation (RFC-059 core slice).

### Changed

- **RFC-018 archived.** Migration Compatibility and Parity Plan withdrawn —
  the Dioxus migration is complete through v0.40.0 and parity was proven by
  the shipped feature set. The file moves to `rfcs/archive/` per RFC-000.

- **RFC-042 refreshed.** Roadmap and RFC Execution Plan updated to reflect
  shipped milestone reality (M0–M7 delivered at different versions than
  projected) and to add a forward roadmap for v0.41+.

### Added

- **`forskscope-explorer-align` crate** (RFC-059 §M5) — the pure
  aligned-row merge logic (`compute_aligned_rows`, `merge_level`, `RowData`,
  `AlignedRow`) extracted from `explorer.rs` into a standalone crate with no
  Dioxus or GTK dependency. Comes with 9 unit tests covering same-name
  pairing, one-sided rows (spacers), directories-before-files ordering,
  alphabetical ordering within type, recursive expansion, and correct
  relative-path computation.

### Fixed

- **CSS de-duplication** (RFC-059 H1) — `main.css` had three conflicting
  `.explorer` rules (two `flex-column`, one two-column `grid`) and two
  `.row` rules (5-column then 7-column). The orphaned grid rule and the
  superseded 5-column row rule are removed; one definition of each remains.
  The `deep-compare { grid-column: 1/-1 }` layout dependency now resolves
  correctly.

- **Typed `DigestKey` enum** (RFC-059 M2) — the stringly-typed
  `PathBuf::from("r:").join(rel)` namespace hack in `explorer.rs` is
  replaced with `DigestKey::Common(rel)` / `DigestKey::RightOnly(rel)`,
  removing the aliasing footgun for files literally named `r:` and making
  the left/right lookup unambiguous.

- **Removed unjustified `unsafe`** (RFC-059 L5) — `unsafe impl Send` and
  `unsafe impl Sync` on `FilteringExecutor` in `dir_pane.rs` are deleted.
  `IgnoreRules` is `Vec<String>` and is `Send + Sync` by the standard-library
  auto-impl; the manual assertions were unnecessary.

- **`explorer.rs` ELOC reduced** from 426 to 354 by the alignment extraction
  (RFC-059 §M5).

---

## [0.40.0] — 2026-06-09

Three-way merge model (RFC-033 core slice).

### Added

- **`forskscope-core::merge::ThreeWayMergeSession` — base-aware merge**
  (RFC-033)

  A new three-way merge model sits alongside the existing two-way
  `MergeSession`, which is unchanged and remains the default. Given base,
  left, and right texts, the session reconciles them with a conservative
  line-oriented diff3 engine and exposes:

  - **Automatic merge of non-conflicting changes** — a region changed on
    only one side takes that side; a region changed identically on both
    sides deduplicates; non-overlapping edits on different lines all apply.

  - **Structured conflict records** — divergent two-sided edits become
    `MergeConflict` entries carrying the base/left/right line content, a
    durable `ConflictId` (stable across resolution operations), and a
    `ConflictStatus`. Conflicts are metadata; conflict markers are never
    written into the result silently.

  - **Resolution operations** — `resolve_left`, `resolve_right`,
    `resolve_both` (left then right), `resolve_manual` (custom text),
    `ignore` (take base), and `reset`. Every operation is reversible
    through `undo` / `redo`, consistent with the two-way transaction model.

  - **Result reconstruction** — `result_text()` rebuilds the merged output
    with original line terminators preserved (LF / CRLF / CR / none).
    Unresolved conflicts contribute nothing until resolved.

  - **Save policy** — `can_save()` returns `false` while any conflict is
    unresolved, implementing the RFC-033 rule that unresolved conflicts
    block a direct save.

  The conflict-resolution *workspace* UI (RFC-034), editor-driven manual
  conflict edits (RFC-032), and marker-based conflict-file export are
  deferred to follow-up releases.

- **19 unit tests** covering one-sided changes, identical two-sided
  changes, non-overlapping edits, true conflicts, every resolution path,
  undo/redo, dirty/save-baseline tracking, CRLF preservation, and stale-id
  rejection. Total core test count: 116 (plus 2 integration tests).

---

## [0.39.0] — 2026-06-09

Patch export (RFC-039 export slice).

### Added

- **`forskscope-core::patch` — unified-diff patch export** (RFC-039)

  A new `patch` module adds deterministic patch generation from the
  existing diff model. Three public entry points are available:

  - `patch_from_file_diff(path, diff, options)` — builds a `PatchDocument`
    for a single two-file comparison. Returns `None` when the inputs are
    identical. The `PatchOptions` struct controls context line count
    (default 3), whether file-creation/deletion entries are included, and
    whether binary files emit a notice.

  - `patch_from_directories(left, right, diff_options, patch_options)` —
    walks both directory trees with `recursive_diff` and assembles one
    patch covering every differing file: `Modify` for changed files, `Add`
    for right-only files, `Delete` for left-only files.

  - `to_unified(patch)` — serialises a `PatchDocument` to a
    standards-conformant unified-diff string. Output is byte-for-byte
    reproducible. Format features:
    - git-style `--- a/` / `+++ b/` file headers; new files reference
      `/dev/null` on the old side, deleted files on the new side.
    - Standard `@@ -old +new @@` hunk headers; single-line ranges omit
      the `,1` count, matching `diff -u` and `git diff` exactly.
    - `\ No newline at end of file` marker emitted correctly when a source
      file lacks a trailing newline.
    - Path separators normalised to `/` for cross-platform portability.
    - Summary comment header (`# forskscope patch: N files, +A -D`).

  The module performs no I/O during export. The guarded *apply* workflow
  (preflight, atomic write, backup-protected application) is deferred to a
  follow-up release pending RFC-023 and RFC-027.

- **11 unit tests + 2 GNU-`patch` integration tests** — the integration
  tests feed generated patches to the system `patch` tool and verify the
  patched files match the expected right-side content, confirming format
  correctness against a real consumer. Total core test count: 97.

---

## [0.38.0] — 2026-06-09

Explorer row alignment and path bar polish.

### Added

- **Aligned two-pane row view** — same-name files and directories now share
  the same horizontal row across the left and right panes. Entries that exist
  only on one side produce a spacer row on the opposite side.  Directories
  appear before files within each level; both groups are sorted alphabetically.
  The tree expansion state of either pane drives the merged row list: expanding
  `src/` on the left inserts its child rows (with spacers on the right for any
  right-side entries that are not expanded or not present). Both tree states are
  managed at the Explorer level so the alignment is computed from a single
  consistent snapshot.

- **Directory diff status** — directories now show a status icon in the tree
  row: ✓ when the same-name directory exists on the other side, · when it
  exists only on this side. (Deep byte-for-byte recursive equality is shown in
  the Directory Report; the tree view shows existence status.)

### Fixed

- **Path bar single-line, leading part shortened** — the path bar no longer
  wraps to multiple lines on long paths. The breadcrumb uses `direction: rtl`
  in CSS so that when the path is too long for the available width, the
  leading ancestors overflow invisibly to the LEFT while the current
  directory stays visible on the right. No segment is ever truncated from
  the right end.

- **Compare: mobile vertical line broken** — the diff rows had no `min-width`
  constraint and the `.diff-scroll` container had no `overflow-x: auto`.
  On narrow viewports this caused the grid columns to collapse and the
  centre divider line to disappear. The diff view is now horizontally
  scrollable (`overflow-x: auto`) with a `min-width: 55ch` on each row to
  preserve the two-pane layout.

---



Explorer polish and diff alignment bug-fix.

### Fixed

- **Diff row vertical misalignment (sr-only grid bug)** — on Delete, Insert,
  and Replace hunks the row contained an extra `span.sr-only` (the
  screen-reader change label) as a raw grid child. With no `.sr-only {
  position: absolute }` rule, the span occupied the first grid column and
  shifted every subsequent cell in changed rows by one column, visually
  misaligning the two halves. Added the standard `.sr-only` rule so the span
  is removed from grid flow while remaining accessible.

### Added

- **Back and Forward navigation buttons restored** — the ← and → buttons return
  to the previous or next directory in per-pane history, matching the design
  from RFC-021 that was lost in the v0.36.0 rewrite.

- **Home button** — navigates the active pane to the user's home directory
  (`$HOME` / `%USERPROFILE%`).

- **Folder picker button** — the 📁 button opens a native folder-picker dialog
  (via `rfd::FileDialog::pick_folder`) so users can locate a directory without
  typing.

- **Editable path input** — clicking the ✎ button (or the current segment of
  the breadcrumb) switches the path bar to a text field. Press Enter to navigate
  if the typed path is a valid directory; press Escape or lose focus to revert.
  Invalid paths are shown with a red border before reverting.

- **All breadcrumb segments shown, each label capped at 18 chars** — instead of
  truncating the middle of the path, every ancestor segment is always shown, and
  long directory names are individually trimmed with "…". Users can always see
  the full depth of the path.

- **Digest status icons in tree rows** — each file node in the tree shows a
  status icon once its background digest comparison finishes: ✓ (equal), ⚠
  (different), · (exists only in this pane). A spinning ⟳ is shown while the
  comparison is running. No extension to `dioxus-swdir-tree` is needed;
  icons are added to the custom row rendering already in use.

- **Tab bar max height** — the tabbar is capped at `3rem`; individual tabs are
  capped at `2.2rem` height with overflow hidden so the toolbar cannot grow
  taller than one tab row.

- **Deep compare renamed to "Directory Report"** — the mode-toggle in the
  explorer footer is replaced by a two-button selector at the top of the
  explorer: "Browse" and "Directory Report". Both have title-attribute
  descriptions. This makes the purpose of each mode clear without requiring
  the user to click to find out.

---



Explorer redesign and ignore-pattern feature. Implements RFCs 054–057.

### Added

- **Explorer tree view (RFC-054)** — each pane now renders an expandable
  directory tree via `dioxus-swdir-tree`. Directories expand/collapse in-place;
  the full tree is navigable by keyboard (↑/↓/←/→/Home/End/Enter/Space).

- **Single-click select, double-click compare (RFC-054)** — single-clicking a
  file in either pane sets it as the pick for that side and shows its name beside
  the Compare button. Double-clicking a file auto-compares it with the
  same-named file picked in the opposite pane. Double-clicking a directory
  navigates into it.

- **Permanent Explorer tab in the tab bar (RFC-054 defect fix)** — the Explorer
  was previously only reachable via a header button that didn't reliably indicate
  the active workspace. The tab bar now shows a permanent Explorer tab as its
  first entry, styled as active when the explorer workspace is open, matching
  the comparison tabs in behaviour. The header Explorer button is removed.

- **Breadcrumb path navigation (RFC-055)** — the "up to parent directory" button
  is removed. In its place, each directory segment in the path bar is a
  clickable link that re-roots the pane at that ancestor (Nautilus-style).
  Deep paths are truncated with `…` to preserve the root and last two segments.
  `Alt+↑` continues to work as the keyboard shortcut for "go up one level".

- **Ignore patterns for files and directories (RFC-056)** — two new fields in
  Settings: *Ignore file extensions* (e.g. `o, class, tmp`) and *Ignore
  directory names* (e.g. `target, node_modules, *.cache`). Extensions are
  matched case-insensitively; directory names support a single `*` wildcard
  (prefix `tmp*`, suffix `*.cache`, infix `*backup*`). Ignored entries are
  stripped from tree scans before they enter the tree state machine, so they
  never appear in either pane. Settings are persisted to disk immediately.

- **About button moved to Settings header (RFC-057)** — the `ℹ` button is
  removed from the global header and added to the Settings dialog header row,
  where it is more discoverable next to the relevant "app information" context.

- **New profile form hidden by default (RFC-057)** — the always-visible profile
  creation form is replaced by a `+ New profile` button that reveals the form on
  demand (progressive disclosure). The form collapses after a profile is added or
  the action is cancelled.

### Core

- `IgnoreRules` struct in `forskscope-core` (`src/ignore.rs`) — `from_settings`,
  `is_file_ignored`, `is_dir_ignored`, `is_empty`. Public re-export from crate
  root. 10 new tests.

---



Hardening release from a full codebase audit. No new user-facing features; three
correctness/consistency findings fixed.

### Fixed

- **Panic risk from unchecked tab indexing** — five event handlers used
  `store.tabs.write()[index]`, which panics if `index` is out of bounds. After a
  tab is closed (Ctrl+W or ×) the remaining tabs shift indices, so a stale event
  fired for a closed component's captured index could panic. All five sites
  (`hunk.rs` apply, `diff.rs` undo/redo/char-mode/word-wrap) now use the safe
  `.get_mut(index)` pattern already used elsewhere in the codebase.

- **i18n gap in diff warnings and read-only notices** — eight strings added in
  v0.33.0 (three diff warnings, five kind-aware read-only notices) bypassed the
  `t(lang, …)` translation system and stayed English in Japanese mode. They now
  route through `t()` and have Japanese translations in `i18n.rs`.

- **CSS drift in the tab bar** — the tab container's class was renamed to
  `.tabbar` in v0.30.0 but no `.tabbar` rule existed, so the bar lost its
  `display:flex` and padding (tabs would stack vertically). Renamed the rule and
  removed four orphaned dead rules (`.tabs`, `.tab .close`, `.tab .dirty`,
  `.tab .name`) left over from the pre-v0.30.0 tab structure.

### Audit notes (no change required)

- `DiffAlgorithm::Lcs` is defined in core but intentionally not exposed in the UI
  selector; the enum must exhaustively map `similar`'s algorithms while the UI
  curates Myers/Patience/Histogram. The `DiffAlgorithmSetting → DiffAlgorithm`
  conversion is consistent.
- No production `.unwrap()`/`.expect()`/`panic!`/`todo!` calls outside tests.
- No `TODO`/`FIXME`/`HACK` markers in source.
- ELOC under the 300 soft limit across all files (`state.rs` 284 is the largest).

---

## [0.34.0] — 2026-06-09

### Fixed

- **`Alt+↑` and `Space` now work in the explorer** — these shortcuts were
  documented but not implemented. `Space` selects the focused file as a
  comparison candidate (equivalent to a single-click). `Alt+↑` navigates up
  one directory level from the keyboard. Both required adding `Modifiers::ALT`
  detection to `dir_pane.rs`'s `onkeydown` handler.

### Added

- **`Ctrl+W` closes the active tab** — standard tab-close shortcut, with the
  same dirty-state guard as the `×` button: if the merge session has unsaved
  changes, a confirmation modal appears before discarding.

- **`aria-pressed` on toolbar toggle buttons** — the five diff-toolbar toggles
  (Inline, Wrap, Ignore WS, Ignore case, algorithm) now carry `aria-pressed`
  attributes reflecting their current state. Combined with the existing
  `aria-label` attributes, these buttons are now fully navigable by assistive
  technology.

- **Modification time in explorer rows** — each file row shows the
  `last_modified` timestamp (already stored in `FileEntry`) in a
  `.dir-mtime` column. The column is suppressed on narrow viewports
  (< 540 px) via CSS `@media` to avoid crowding small windows.

- **Keyboard reference updated** — both `keyboard.md` and the in-app `?`
  modal now include `Alt+↑`, `Space`, and `Ctrl+W`.

- **13 new core tests** — total 76 (up from 63):
  - Diff: insertion/deletion counts for multi-insert, replace, and complete
    rewrites; ignore-whitespace false-positive guard; large-file warning
    absence for small files; hunk-ID uniqueness across successive calls.
  - Merge: `pending_changes()` tracking after apply, undo, and from identical diff.
  - Dir: empty directory listing; `last_modified` populated; `list_dir(None)`;
    recursive diff on two empty trees.

---

## [0.33.0] — 2026-06-09

### Added

- **Diff warning banner** — when the diff engine applies the large-file policy
  or the deadline expires, a yellow `⚠` banner now appears below the toolbar
  with a human-readable explanation. Three warning types are surfaced:
  `LargeFilePolicyApplied`, `DeadlineExpired`, and `InlineSkippedHunkTooLarge`.
  Previously these were silently discarded; users had no way to know that a
  result might be approximate.

- **Kind-aware read-only notices** — the generic "Merge/save unavailable for
  this file type" message is replaced with specific notices: "Binary file —
  read-only comparison (hex preview)", "Spreadsheet — read-only comparison",
  "One side is missing — read-only", and "File type not supported for merge —
  read-only." The correct message is chosen from `tab.left_doc.kind` and
  `tab.right_doc.kind` in `TabSnapshot::from_tab`.

- **ARIA on diff rows** — every diff row now carries `role="row"`. Changed rows
  (Delete, Insert, Replace) prepend a visually-hidden `.sr-only` span
  ("Deleted:", "Inserted:", "Changed:") so screen readers can announce the
  nature of each change without relying on colour or glyph alone. The
  `.sr-only` utility class follows the standard `position:absolute; clip:rect`
  pattern.

- **Dynamic window title** — a `use_effect` in `app.rs` subscribes to the
  active tab signal and updates the OS window title via `document.title`.
  The title reads "ForskScope — filename" when a comparison is active and
  "ForskScope" when the Explorer is shown.

- **Five documentation chapters** — five thin stub files replaced with full
  content:
  - `intermediate/keyboard.md` — all shortcuts, organised by context
  - `intermediate/cli.md` — all startup modes, git difftool/mergetool config,
    JJ integration, exit codes
  - `intermediate/diff-options.md` — all three algorithms with characteristics,
    ignore-whitespace/ignore-case, context lines, inline diff, compare profiles
  - `users/faq.md` — eight common questions with concrete answers
  - `users/settings.md` — every settings panel option with type, default, and
    description

---

## [0.32.0] — 2026-06-09

### Changed

- **`diff.rs` split** — the 330-ELOC file was split into `diff.rs`
  (Dioxus components: DiffWorkspace, DiffHeader, Toolbar, TabSnapshot: 238 ELOC)
  and the new `ui/diff_actions.rs` (pure action functions: apply_focused_hunk,
  move_focus, save_tab, save_as, build_request, handle_result, trunc, algo_val:
  108 ELOC). `diff.rs` re-exports the public action functions for external callers.

- **`save_text` creates parent directories** — "Save As" to a path in a
  directory that doesn't yet exist now succeeds. Previously the atomic
  temp-file write would fail with ENOENT on the missing parent.

### Added

- **Ctrl+Y redo shortcut** — `Ctrl+Y` re-applies the most recently undone
  merge. `Ctrl+Z` / `Ctrl+Y` are now a symmetric pair (Redo also available via
  the More ▼ toolbar). Keyboard reference updated.

- **Ignore-case toggle in diff toolbar** — "Ignore case: off/on" button in the
  advanced toolbar toggles the per-tab `ignore_case` option and immediately
  recomputes the diff, matching how the ignore-whitespace toggle works.

- **63 core tests** (up from 35) — 28 new tests covering:
  - `ignore_case` option: case-only change collapses; combined with `ignore_whitespace`
  - All three diff algorithms (Myers / Patience / Histogram) for equivalence
  - Empty files, no-trailing-newline, single-character diffs, multi-block changes
  - Diff stats accuracy (lines_inserted, lines_deleted, hunks_changed)
  - Inline span detection via `refine_pair`
  - `result_text()` before and after apply, partial apply correctness
  - Full undo/redo round-trips, multiple-hunk sessions
  - Save to nested parent directories
  - Conflict detection and fingerprint stability
  - `FileFingerprint` stability and change detection
  - `allow_missing` for absent file paths

- **Documentation** — three new user-guide chapters:
  - `comparing-files.md` — opening comparisons, reading the diff view, search, options
  - `merging.md` — apply/undo/redo model, save workflow, keyboard-first merge pattern
  - `directory-compare.md` — browse mode, filters/sort, deep recursive compare, batch copy

---

## [0.31.0] — 2026-06-09

### Changed

- **`settings.rs` split** — the 375-ELOC file was split into `settings.rs`
  (SettingsModal, persist/load, profile form: ~130 ELOC) and the new
  `ui/modals.rs` (all safety/action modals: ~200 ELOC), both well under the
  300-ELOC guideline.

### Added

- **Algorithm selector** — a dropdown in the diff toolbar advanced section
  (Myers / Patience / Histogram) recomputes the diff immediately on change.
  `DiffProfile` also carries an `algorithm` field so profiles can encode a
  preferred algorithm. A fourth built-in profile "Histogram" is now included.
  `DiffAlgorithm` is re-exported from `forskscope_core` for UI use.

- **Explorer name filter** — a text input in the filter bar filters both panes
  by filename substring (case-insensitive). Typing `rs` shows only `.rs` files;
  typing `Cargo` shows only files whose names contain "Cargo". Clears instantly.

- **Batch copy in deep compare** — when the deep compare table has changed or
  one-side-only files, two "Copy N →" / "← Copy N" buttons appear in the
  toolbar. Clicking opens a confirmation modal that lists the count and warns
  that existing files receive `.bak` backups. All copy operations run with the
  same `BackupPolicy::SiblingBak` safety model as single-file copy.

- **Keyboard reference modal** — a `?` button in the header (or Ctrl+/) opens
  a formatted shortcut table covering the diff view, explorer navigation, and
  app-level commands. The `ℹ` button retains the About panel separately. A new
  `ui/keybindings.rs` module holds the component.

- **README overhaul** — `README.md` rewritten with badges, a clear product
  statement, quick-start commands, git integration snippet, feature list,
  keyboard table, and doc links.

---

## [0.30.0] — 2026-06-09

### Added

- **Tab close button** — every comparison tab now has a `×` button. If the
  comparison has unsaved merge changes, a confirmation modal asks before
  discarding. Closing the last tab returns to the Explorer workspace. The
  session file is updated immediately after each close.

- **Tab dirty indicator** — a `●` dot appears before the tab title whenever
  the merge session has unsaved changes, giving an at-a-glance view of
  which comparisons need saving.

- **Custom compare profiles (RFC-009 complete)** — the Settings panel now
  shows a clickable profile list instead of a dropdown. Clicking a profile
  activates it. Built-in profiles (Exact, Ignore whitespace, Ignore case)
  are read-only. An inline form at the bottom lets users create named
  presets with their own combination of ignore-whitespace and ignore-case
  options. Custom profiles can be deleted with `×`; they are persisted to
  `settings.json`.

---

## [0.29.0] — 2026-06-09

### Added

- **Session persistence (RFC-035)** — open comparison tabs are saved to
  `session.json` (via `app-json-settings`) whenever the tab list changes.
  On the next launch with no explicit CLI arguments, tabs are restored
  automatically. Tabs whose files are gone are skipped silently; tabs with
  one missing side open gracefully with an empty document.

- **About panel** — the `?` button in the header opens a modal showing the
  version, build profile (debug/release), platform (OS + arch), UI framework,
  and diff engine. A **Copy diagnostics** button copies the information to the
  clipboard for easy bug reporting.

- **Enter to apply focused hunk** — pressing Enter in the diff workspace
  applies the currently focused change (left → right) and auto-advances to
  the next pending change, enabling rapid single-key merge flow.

- **Status bar diff stats** — the status bar now shows `+N / -N`
  (insertions/deletions) for the active comparison, together with the file
  names, encoding, and unsaved-changes marker.

- **Deep compare incremental progress (RFC-040 partial)** — the deep
  recursive compare now uses a two-phase approach: Phase 1 (fast file-system
  walk) fills the table immediately with `Computing` placeholders; Phase 2
  runs per-file `spawn_blocking` digest tasks and updates entries in-place.
  A live `checking N/total…` counter shows progress.

---

## [0.28.0] — 2026-06-09

### Added

- **Navigation history (RFC-021)** — each directory pane now keeps a per-pane
  back/forward history stack. ◀ and ▶ buttons navigate between previously
  visited directories, restoring the exact path. The stack is managed by the
  `nav()` helper that pushes on every `go` call and truncates the forward
  history on a new navigation, matching standard file-manager behaviour.

- **Explorer filter bar** — a compact toolbar above the directory panes lets
  the user choose: **All** (default), **Different** (changed + unique-to-one-side
  files only), or **Equal** (same on both sides). Filters apply to both panes
  simultaneously so the view stays aligned. Most useful in large directories:
  "Different only" hides hundreds of equal files and shows only what needs attention.

- **Sort by Name / Status / Size** — a dropdown in the filter bar. "Status" sort
  puts changed (⚠) files first, then computing (⊙), then equal (✓), then the rest.

- **Show/Hide hidden files** — a checkbox in the filter bar toggles files and
  folders whose names start with `.`.

- **Deep recursive compare (RFC-037, RFC-038)** — the `⟳ Deep compare` button
  switches the explorer into a recursive-scan mode. Both directory trees are
  walked in a `spawn_blocking` background task; the result is a flat, sorted
  table of every file with its status (⚠ changed, ← left-only, → right-only,
  ✓ equal). The same All/Different/Equal filter applies. A summary line shows
  total counts. Clicking **Compare** on any row opens a file comparison. This
  is the WinMerge-class "compare entire project tree" workflow.

- **Compare profiles (RFC-009)** — three built-in profiles in Settings:
  "Exact (default)", "Ignore whitespace", "Ignore case". The active profile
  is applied when opening a new comparison. Users can switch profiles mid-session
  from Settings without losing their open tabs.

- **`recursive_diff` core function** — `forskscope_core::dir::recursive_diff`
  returns a sorted `Vec<RecEntry>` covering every file in either tree. Covered
  by two new core tests (35 total; all pass).

---

## [0.27.0] — 2026-06-09

### Added

- **Directory file operations** — each file row in the explorer now carries a
  copy button (→ for the left pane, ← for the right pane) that appears on
  hover and focus. Clicking opens a confirmation modal that shows the exact
  source and destination paths, warns when the destination already exists, and
  creates a `.bak` sibling backup before overwriting — the same safety model as
  the text-merge save flow. The operation calls `forskscope_core::dir::copy_file`
  with the standard `BackupPolicy::SiblingBak`; no file is modified without
  explicit user confirmation (D-005, D-006).

- **Git mergetool mode** — `forskscope <local> <remote> <merged>` (3-arg
  invocation) opens a comparison of `<local>` vs `<remote>` and redirects
  **Save** to write the result to `<merged>`. The tab title carries a "(merge)"
  suffix. Compatible with standard `git mergetool` configuration; see
  `docs/src/intermediate/git-integration.md` for setup instructions.

- **Git integration documentation** — `docs/src/intermediate/git-integration.md`
  covers `git difftool`, `git mergetool`, and JJ/Jujutsu configuration.

- **GitHub Actions CI/CD** — `.github/workflows/ci.yml` runs `cargo test`,
  `cargo clippy -D warnings`, and a UI compile check on every push and PR.
  `.github/workflows/release.yml` builds Linux x86_64, macOS aarch64, and
  Windows x64 release binaries plus a source archive when a `vX.Y.Z` tag is
  pushed, and creates a draft GitHub release with all assets.

- **`copy_file` core function** — `forskscope_core::dir::copy_file` (with
  backup) is the safe file-copy primitive. It creates destination parent
  directories automatically and is covered by two new core tests (33 total).

---

## [0.26.0] — 2026-06-09

### Added

- **Colour-independent diff markers** — every changed row now carries a
  visible glyph (− for deletions, + for insertions, ~ for replacements) in
  the gutter alongside the colour cue, satisfying the accessibility
  requirement that colour must not be the sole indicator of change kind
  (RFC-019 §19.3). Equal rows show no mark. All markers carry
  `aria-hidden="true"` so screen readers are not flooded with symbols.

- **Word-wrap toggle** — in the advanced toolbar section, "Wrap: on/off"
  toggles word-wrapping for the active comparison. Off by default (code
  files); on is useful for prose/markdown. Stored per comparison tab; not
  persisted (tabs are created fresh).

- **Search within diff** — press the 🔍 button or Ctrl+F to open a compact
  search bar above the diff content. Matching rows are highlighted across both
  panes. A live match count shows "N matches". Esc closes the bar and clears
  the query.

- **Swap sides** — "⇄ Swap sides" in the advanced toolbar exchanges left/right
  documents, paths, and recomputes the diff. If the merge session has unsaved
  changes, a confirmation modal asks before discarding.

- **Context lines preference** — in Settings, a "Context lines" selector
  (0 / 3 / 5 / 10) controls how many lines of equal context are shown around
  each change before collapse. Persisted across launches.

- **Remember last directories** — when a pane navigates to a new directory the
  path is saved to `AppSettings` and loaded on the next launch. Stored
  separately for the left and right panes.

### Improved (accessibility — RFC-046)

- Every safety modal now carries `role="dialog"`, `aria-modal="true"`, and
  `aria-label`; the first button has `autofocus` so keyboard users land
  immediately on the safe default choice.
- Toast notifications carry `role="status"` and `aria-live="polite"`.
- The diff workspace region has `role="region"` and `aria-label`.
- Action buttons have explicit `aria-label` attributes where icon text is
  insufficient.

---

## [0.25.0] — 2026-06-09

### Added

- **Explorer: auto-compare on common-file click** — clicking a file that
  exists on both sides (⚠ or ✓ marker) opens the comparison immediately,
  without needing to pick each side manually.  Only left-only / right-only
  files require explicit single-side selection.  This is the core
  "Diff through Exploring" workflow (RFC-005).

- **Explorer: keyboard navigation** — the directory table accepts focus
  (tabindex) and responds to ↑/↓ (move row focus), Enter (navigate into
  folder or auto-compare file), and Tab (switch between panes via the
  browser focus order).

- **Explorer: directory summary counts** — when no files are selected the
  compare bar shows "N common · N changed · N left-only · N right-only" so
  the overall state of both directories is immediately visible.

- **Diff: Reload (↺)** — reloads both files from disk and recomputes the
  diff.  When the merge session has unsaved changes, a confirmation modal
  asks before discarding.  The button is always visible regardless of file
  type.

- **Packaging scripts** — `packaging/` directory with:
  - Linux: `.desktop` entry, `install.sh` (user-local or custom PREFIX),
    `PKGBUILD` for Arch Linux / AUR.
  - macOS: `build-dmg.sh` (requires `create-dmg`).
  - Windows: `build-zip.sh` (requires `zip` or `7z`).
  - `build-release.sh` — top-level script that builds a release binary
    and the source archive for the current platform.

---

## [0.24.0] — 2026-06-09

### Added

- **Explorer: flat directory comparison** — both panes now show a plain
  file-manager view (path bar + directory listing) instead of a tree widget.
  Same-name files are compared in the background via parallel `spawn_blocking`
  tasks; each file row shows a status marker: ✓ equal, ⚠ changed, ← left-only,
  → right-only, ⊙ computing.  This is the core "Diff through Exploring" identity
  feature (RFC-005 main design decision).

- **Context collapse** — long equal sections in the diff view are folded to a
  `··· N unchanged lines ···` divider by default (3 lines of context shown on
  each side).  Click the divider to expand.  Large diffs are now readable
  without raw scrolling (D-003).

- **Keyboard shortcuts** — F7 / F8 for previous / next change; Ctrl+S for save;
  Ctrl+Z for undo.  All operate on the active diff tab via a global `onkeydown`
  handler on the app root.

- **Scroll to focused hunk** — pressing F7/F8 or clicking Prev/Next now smoothly
  scrolls the view so the focused hunk is visible.

- **Save As** — a Save As button opens a modal where the target path can be
  edited.  The result is written to the new path and the tab's right-side path
  is updated.

- **Ignore-whitespace toggle** — in the advanced (More ▼) toolbar section, a
  toggle button recomputes the diff with `ignore_whitespace: true`, replacing
  the merge session while preserving all other tab state.

- **File path header** — the diff workspace shows both file paths in a compact
  header bar above the diff, with parent-path ellipsis when paths are long.

- **`DiffOptions` per tab** — each compare tab carries its own `DiffOptions`
  so that future compare-profile work (RFC-028) can tune per comparison.

### Changed

- Explorer panes no longer use `dioxus-swdir-tree` for the primary view.
  The flat listing approach is simpler, more WinMerge-like, and surfaces the
  digest comparison results directly.  The swdir-tree crate remains a workspace
  dependency for a planned deep-tree navigation mode.

- `diff.rs` split into `diff.rs` (coordination, ~250 ELOC) + `hunk.rs`
  (rendering, ~125 ELOC) to stay within the 300-ELOC per-file guideline.

---

## [0.23.0] — 2026-06-09

First release of the Dioxus migration.  Previous releases (through 0.22.x)
used Tauri v2 and Svelte 5; this version replaces that stack with a
GUI-independent Rust core and a Dioxus 0.7 desktop frontend.

### Added

- **`forskscope-core`** — GUI-independent crate with no Tauri, WebView, or
  JavaScript dependency.  Owns file identity, text decoding, binary/hex
  rendering, Excel comparison adapter, the normalized `similar` v3 diff model
  (line-level hunks, stable IDs, lazy inline character refinement), the
  model-backed merge session with a full undo/redo transaction log, save safety
  (fingerprint conflict detection, atomic write, `.bak` backup), and directory
  listing / recursive digest comparison.  31 unit tests validate the design
  specs from RFC-001 and RFC-002.

- **Model-backed merge** — the key correctness fix over v0.22.  Every merge
  action goes through a transaction log; the canonical result text is
  reconstructed from the model, never from rendered HTML or DOM state.

- **CLI startup pair** — `forskscope <left> <right>` now opens a comparison
  immediately.  The unwired `ready` command from v0.22 is replaced.

- **Settings persistence** — theme, language, and diff font size are saved to
  the OS config directory and restored on next launch (`app-json-settings`).

- **Explorer panes** — two directory-tree panes built on `dioxus-swdir-tree`
  (lazy loading, keyboard navigation).  Select a file on each side, click
  Compare.

- **Diff / merge workspace** — side-by-side hunk rendering from the merge
  session, prev / next navigation, per-hunk apply, undo, and save.

- **Progressive disclosure** — the default toolbar shows only navigation,
  undo, and save.  Advanced controls (inline character diff, redo) are behind
  a one-click disclosure; unused controls are hidden entirely for binary and
  Excel comparisons.

- **Themes** — dark (default), light, and night; diff font size configurable.

- **Localization** — English and Japanese.

- **Save safety** — external-modification detection before every save;
  overwrite requires explicit confirmation; `.bak` sibling created by default.

- **Merge / save disabled for non-text** — binary and Excel comparisons are
  explicitly read-only; attempting to save is impossible, not silently wrong.

### Changed

- Binary content now uses one normalized hex-preview format (address offset,
  hex bytes, ASCII column) instead of the two inconsistent formats in v0.22.

- Diff font size setting now takes effect in the rendered diff.  In v0.22 the
  preference was stored but ignored.

### Removed

- Tauri, Svelte, Node.js, and Vite build dependencies.

### Fixed

- `contenteditable` new-pane could be edited in v0.22 but changes were never
  reconciled back into the model.  The new pane is not free-form editable;
  every change goes through the merge session.

---

## [0.22.3] and earlier

Tauri v2 + Svelte 5 + similar v2 baseline.
See the [v0.22.x repository](https://github.com/forskscope/forskscope/tree/v0.22.3)
for the previous changelog.
