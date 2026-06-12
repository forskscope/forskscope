# Testing

## Running tests

```sh
# Core domain logic (no GTK required)
cargo test -p forskscope-core

# View-model layer (no GTK required)
cargo test -p forskscope-ui-logic

# Both (the CI-equivalent command)
cargo test -p forskscope-core -p forskscope-ui-logic

# Clippy (must pass without warnings)
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
```

The UI crate (`forskscope-ui`) requires WebKitGTK/GTK3 to build and cannot
be tested in environments without a display server. Core and ui-logic tests
run anywhere Rust is installed.

## Test counts (v0.87.0)

| Suite | Count |
|-------|-------|
| `forskscope-core` unit | 599 |
| `forskscope-core` integration | 2 |
| `forskscope-ui-logic` | 189 |
| Doctests | 6 |
| `forskscope-ui-logic` integration | 1 |
| **Total** | **797** |

## `forskscope-core` test modules

Tests live in `crates/forskscope-core/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `app_error_tests` | `AppError::from_core`, `is_blocking`, `is_recoverable`, `ErrorId`, `TechnicalDetail`. | RFC-017 |
| `batch_tests` | `batch_copy`, `restore_from_manifest`, `BatchManifest`. | RFC-023 |
| `command_tests` | `AvailabilityRule` evaluation, `CommandRegistry` uniqueness/search/shortcut lookup, `CommandDangerLevel`. | RFC-019 |
| `compare_profile_tests` | `CompareProfile` presets, `to_diff_options`, `NewlineCompareMode` engine wiring. | RFC-028 |
| `conflict_nav_tests` | `ConflictNavigator` build/focus/prev/next/filter, `ConflictStatusDisplay` glyphs, summary counts, progress fraction. | RFC-034 |
| `diff_decoration_tests` | `DiffDecorationSet` from diff, CSS class uniqueness/prefix, gutter symbols, aria labels, focused hunk marking. | RFC-024 |
| `diff_tests` | `compute_diff`, hunk kinds, inline spans, equal/insert/delete/replace, whitespace/case modes. | RFC-002 |
| `dir_cancel_tests` | `recursive_diff_with_cancel`, cancellation mid-scan. | RFC-037 |
| `dir_index_tests` | `DirectoryIndex`, `EqualityEvidence`, `pair_entries`, one-sided entries. | RFC-037 |
| `dir_tests` | Directory listing, recursive digest equality, `file_digest_equal`. | RFC-022 |
| `document_tests` | `LoadedDocument`, `FileFingerprint`, `check_external_state`, `ExternalFileState`. | RFC-036 |
| `edit_op_tests` | `TextEditOperation` variants, `RevisionId`, `TextRange`, revision compatibility, `EditTransaction`. | RFC-032 |
| `editability_tests` | `EditabilityClass::from_kind`, `requires_save_guard`, `NewlinePolicy::resolve`. | RFC-012 |
| `encoding_tests` | `decode_bytes`, `detect_newline_style`, `BomPresence`, `BomPolicy`, `detect_bom`. | RFC-012 |
| `error_tests` | `CoreError` variants, `AppErrorKind::from_core`, `RecoveryAction` defaults. | RFC-017 |
| `external_state_tests` | `check_external_state` with mocked fingerprints. | RFC-036 |
| `external_tool_tests` | `expand_args` placeholder expansion; shell safety (spaces, semicolons, $HOME, backticks); built-in presets. | RFC-029 |
| `file_size_tests` | `FileSizeClass::classify` against `PerformanceLimits` thresholds. | RFC-013 |
| `ignore_tests` | `IgnoreRules::from_settings`, extension and directory pattern matching. | RFC-056 |
| `job_tests` | `JobStatus` lifecycle transitions, `JobStatusRecord`, `JobRegistry` register/get/active/prune. | RFC-008 |
| `line_map_tests` | `LineMap` row states, navigation, `ScrollAnchor` clamping, `build_mini_map` weight sum. | RFC-035 |
| `merge_plan_tests` | `plan_operations`, `execute_plan`, `OperationPlan` safety. | RFC-022 |
| `merge_tests` | `MergeSession` apply/undo/redo, dirty state, `result_text`, transaction log. | RFC-006 |
| `patch_tests` | `patch_from_file_diff`, `to_unified`; GNU `patch` round-trip integration. | RFC-039 |
| `persist_tests` | `VersionedEnvelope` round-trip, `MigrationPolicy`, newer-schema rejection. | RFC-031 |
| `report_tests` | `FileComparisonReport`, `DirComparisonReport`, Markdown/JSON output. | RFC-027 |
| `save_tests` | `save_text` with fingerprint match, `AtomicSaveStrategy`, `BackupPolicy`. | RFC-007 |
| `session_tests` | `WorkspaceSession` tab lifecycle, dirty state, `CloseResult`, JSON round-trip, schema-version guard. | RFC-011 |
| `settings_tests` | `UserSettings` defaults, round-trip JSON, theme/density/font round-trips, CSS var count, fallback. | RFC-009 |
| `three_way_tests` | `ThreeWayMergeSession` conflicts, resolution, undo/redo, `can_save`, `result_text`. | RFC-033 |
| `transaction_log_tests` | `TransactionLog` push/undo/redo/mark_saved, `is_dirty`. | RFC-015 |
| `vcs_tests` | `GitProvider::detect`, `VcsProvider` trait contract. | RFC-038 |
| `watcher_tests` | `MockFileChangeMonitor` watch/inject/poll/drain, `WatchError`, `FileChangeKind`. | RFC-036 |
| `xlsx_tests` | `derive_pair_text`, structured diff output, sheets-diff v2 API. | RFC-058 |

Integration tests in `tests/`:

| File | Covers |
|---|---|
| `patch_round_trip` | Generates a unified-diff patch and verifies it applies correctly with GNU `patch`. |

## `forskscope-ui-logic` test modules

All tests are inline (`#[cfg(test)]` inside each module file).
One integration test exists in `tests/` (patch round-trip via `hunk_decorations`).

| File | Covers | RFC |
|---|---|---|
| `explore/align` | `compute_aligned_rows`: pairing, ordering, one-sided entries, recursion depth, selection state. | RFC-059 |
| `explore/deep_filter` | `DeepFilter::matches` for all `RecStatus` variants, `DeepCompareSummary` counts, footer text, `is_fully_computed`, `apply_filter`. | RFC-037, RFC-038 |
| `explore/status` | `RowStatusKind::from_evidence` for all 10 `EqualityEvidence` variants, CSS prefix, glyph distinctness, aria labels, `needs_action`, `StatusRow` constructors. | RFC-054 |
| `compare/command_bar` | `build_toolbar` section structure, `Save` enabled/disabled, `Undo`/`Redo` asymmetry, `CommandPalette` always enabled, shortcut hint, `find_item`. | RFC-019 |
| `compare/conflict_nav_view` | `ConflictNavView::from_navigator`: non-empty with conflicts, empty without, `display_num` ≥ 1, `!` glyph for unresolved, CSS prefix, progress text, `can_save` predicate, `len`. | RFC-034 |
| `compare/hunk_decorations` | `DecorationIndex::from_set`: added/deleted/modified kinds, gutter symbols, CSS prefix, aria labels, multi-hunk coverage, out-of-bounds safety, `RowDecoration` field invariants. | RFC-024, RFC-035 |
| `compare/load_guard` | `guard_for_sizes` / `guard_for_sizes_with_limits`: all four `FileSizeClass` branches, worst-of-pair logic, boundary values (at-limit and one-over), message non-empty, distinct large/very-large labels, default-limit smoke tests. | RFC-013 |
| `compare/palette_view` | `build_palette`: empty query returns all; query matches label; nonsense empty; case-insensitive; enabled before disabled; Save disabled in empty context; `enabled_count`; all labels non-empty. | RFC-019 |
| `compare/save_error` | `action_label` all variants non-empty; `SaveErrorView::from_error`: external-mod action set, primary ≠ Overwrite, `FileWriteFailed`/`InternalFault` actions; path passthrough; title/body non-empty; button labels non-empty; exactly one primary. | RFC-007, RFC-017 |
| `compare/scroll_sync` | `ScrollSyncState`: at-top, pixel→anchor→pixel round-trip, mid-row fraction, negative clamping, `scroll_to_row`, past-end clamping, `max_scroll_px`, zero row-height guard. | RFC-035 |
| `compare/search_index` | `MatchIndex` build/advance/retreat/wrap, `matching_hunk_ids`, empty index. | RFC-014 |
| `compare/summary` | `CompareStatusSummary` for identical/changed/whitespace-only/single-hunk, dirty marker, `DiffNavigationState` position labels and aria wrap cases. | RFC-006 |
| `compare/tab_state` | `context_from_snapshot` field mapping, `AvailabilityRule` inverse verification, end-to-end `TabStateSnapshot → CommandContext → build_toolbar`. | RFC-003, RFC-019 |
| `settings/settings_view` | `theme_choices` round-trip via `ThemeId::from_id`; density/font round-trips; `profile_presets` count and name; font-size validation boundaries; `clamp_font_size` extremes; context-lines boundary; `find_active` hit/miss; no duplicate values. | RFC-009 |

Doctest in `watcher.rs` (`MockFileChangeMonitor` usage example): 1 test.

## `forskscope-ui` tests (GTK-required)

The UI crate exposes a `[lib]` target so `#[cfg(test)]` blocks can be
written alongside component code. However, `dioxus-desktop` requires GTK3
at compile time, so these tests can only run in a full build environment.

Current GTK-free-in-theory, GTK-required-in-practice tests in `state.rs`:

| Function | Tests |
|---|---|
| `tab_title` | same filename, different filenames, left-only, both missing, dotfile, deeply nested |
| `SessionState` serde | round-trip with tabs, empty session |

These serve as the template for future state-layer tests once the project
has a GTK CI environment (RFC-010).
