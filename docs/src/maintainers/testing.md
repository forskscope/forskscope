# Testing

## Running tests

```sh
# Core domain logic (no GTK required)
cargo test -p forskscope-core

# View-model layer (no GTK required)
cargo test -p forskscope-ui-logic

# Headless test gate
cargo test -p forskscope-core -p forskscope-ui-logic

# Clippy (must pass without warnings)
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings

# Full workspace gates used by CI when GTK/WebKitGTK dependencies are present
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Format and generated CSS gates
cargo fmt --check
cargo xtask css --check

# Security audit (uses .cargo/audit.toml for reviewed advisories)
cargo audit

# Enforce the reviewed dependency paths behind audit exceptions
cargo xtask audit-deps

# Release metadata and localization gates
cargo xtask version-sync
cargo xtask i18n
```

The UI crate (`forskscope-ui`) requires WebKitGTK/GTK3 to build and cannot
be tested in environments without a display server. Core and ui-logic tests
run anywhere Rust is installed.

## `cargo audit`'s cadence (F55, refined per review 059 N1)

`cargo audit` runs in three different places, since 2026-08-13:

- **Release preflight** (`release.yml`) still hard-blocks: it runs against
  a fixed, tagged ref, so its result is deterministic, and a release must
  never ship a known-vulnerable dependency. Unchanged by F55.
- **`ci.yml`'s per-push job no longer runs `cargo audit` at all**,
  unconditionally. It read a mutable external database (RustSec) from a
  blocking per-push gate, so a green result was a property of the commit
  *and the clock*, not the commit alone — F50 demonstrated this directly:
  two CI runs on `main` minutes apart, one green and one red, with no
  dependency change between them. A blocking per-push gate on that kind of
  check rewards making CI green *fast*, and the fastest path is widening
  `.cargo/audit.toml`'s ignore list — exactly what the disposition process
  (reachability, owner, review date, upgrade trigger) exists to prevent.
  `cargo xtask audit-deps` (dependency-path shape against
  `.cargo/audit.toml`'s reviewed exceptions) stays in this job — it
  depends only on `Cargo.lock`, not a live external database, so it has
  none of `cargo audit`'s flakiness and belongs on a deterministic gate.
- **`.github/workflows/audit.yml`** runs `cargo audit` on three triggers,
  each answering a different question: a **daily schedule** (did the
  advisory database change under code that didn't? — loud on failure via
  GitHub's default scheduled-workflow-failure email), **`push`/
  `pull_request` filtered to `Cargo.lock`/`**/Cargo.toml` only** (did
  *this* dependency-graph change introduce something vulnerable? —
  audited immediately, on the commit that actually made the change, so a
  future security bump like F50's own fix still gets audited on the
  commit that makes it, not just the next day), and **`workflow_dispatch`**
  (on-demand runs against any ref — how the workflow's fail/pass behavior
  was verified without waiting a day). **Both outcomes were demonstrated by
  dispatch, not assumed:** against `main`, run `31699200117` completed
  `success`; against a throwaway branch that downgraded `webbrowser` to
  `1.2.1`, reproducing F50's exact advisory, run `31699489630` completed
  `failure` with that advisory in the log. The branch was deleted. Unrelated pushes match none of the
  path-filtered triggers, so the non-determinism removed from `ci.yml`
  stays removed for the common case.

## What `cargo xtask i18n` actually guarantees (F39)

`cargo xtask i18n` verifies that every `t(lang, "key")` call site in
`forskscope-ui` has a matching Japanese translation in `i18n.rs`. This is a
**closed-world check over call sites that already reach `t()`** — it cannot
see a user-visible string that never calls `t()` in the first place, because
there is nothing structural distinguishing "user-visible string" from any
other `String`/`&str` in the crate for a source scan to key on.

Five error paths currently bypass `t()` this way, all going to a toast via
`store.notify(...)`:

- `ui/view/diff_actions.rs` — `handle_result`'s `Err(e) => store.notify(e.to_string())`
- `ui/view/diff_actions.rs` — `describe_block(...)`
- `ui/overlay/modals/recovery.rs` (two sites) — settings/session reset-with-backup failure
- `state/compare.rs` — tab ID allocation failure

**Decision (review 049 N1 / F39, 2026-08-13): these are not being translated,
and the gate's scope is documented here rather than widened to catch them.**
All five carry `CoreError`'s `Display` output — for the `Io`/`Decode`/
`Unsupported`/`InternalInvariant` variants, that `message` field is generated
by the OS or a dependency at the moment of the error (e.g. `std::io::Error`'s
platform-native text), not authored copy in this codebase, so there is no
fixed string to put in a translation map. `describe_block`'s `Binary`/
`Spreadsheet` arms are the one exception — genuine static literals that could
be translated — but translating only those two without a working detector
for future bypasses would recreate exactly the gap this decision documents,
so they stay matched to the established `e.to_string()` precedent (review 048
C2) instead of splitting the function's i18n treatment silently.

A structured taxonomy for exactly this problem already exists —
`AppError`/`AppErrorKind`/`SaveErrorView` (RFC-017) map `CoreError` into
translatable categories with detail kept separate — but `forskscope-ui` does
not use it anywhere; see **F52**. Routing these five sites through it would be
the real fix, and is future work, not a mechanical `t()`-wrapping of the
current passthrough text. **F52 is not tidiness** (review 058 §5.1): every
save/IO failure reaches the user as raw `CoreError` `Display` text, in
English, regardless of locale, until it's done — this gate's narrow scope is
a symptom of that gap, not a separate problem with its own fix.

## Test counts

**Not recorded here as numbers, deliberately.** This section used to carry a
hand-maintained table (last stamped v0.165.0, total 943). It drifted in both
directions — `forskscope-ui-logic` unit tests fell from 241 to 204 when F75(b)
deleted four obsolete view-models, while `forskscope-core` unit tests rose from
643 to 736 — and nothing checked it (F1). A count that must be updated by hand
on every change is a claim that will be wrong most of the time.

To see the current counts per suite:

```sh
cargo test --workspace 2>&1 | grep -E '^\s+Running|^test result'
```

The suites, for orientation:

| Suite | Where |
|-------|-------|
| `forskscope-core` unit | `crates/forskscope-core/src/tests/` |
| `forskscope-core` integration | `crates/forskscope-core/tests/` — `diff_corpus`, `merge_corpus`, `patch_apply` (shells out to real `git apply` and `patch -p1`) |
| `forskscope-ui-logic` unit | inline `#[cfg(test)]` modules |
| `forskscope-ui-logic` integration | `crates/forskscope-ui-logic/tests/css_coverage.rs` |
| `forskscope-ui` lib and bin | inline `#[cfg(test)]` modules, compiled twice (library and binary targets) |
| doctests | `forskscope-core` |

## `forskscope-core` test modules

Tests live in `crates/forskscope-core/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `app_error_tests` | `AppError::from_core`, `is_blocking`, `is_recoverable`, `ErrorId`, `TechnicalDetail`. | RFC-017 |
| `batch_tests` | `batch_copy`, `restore_from_manifest`, `BatchManifest`. | RFC-023 |
| `cancel_tests` | `CancellationToken::new`/`cancel`/idempotent; clone propagation (original→clone, clone→original, clone-of-clone); `Default`; `Debug`. | RFC-037, RFC-008 |
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
| `file_kind_tests` | `FileKind::is_mergeable_text` all variants; `classify` via tempfiles (text/binary/xlsx/uppercase-xlsx/empty/directory). | RFC-001 |
| `file_size_tests` | `FileSizeClass::classify` against `PerformanceLimits` thresholds. | RFC-013 |
| `ignore_tests` | `IgnoreRules::from_settings`, extension and directory pattern matching. | RFC-056 |
| `job_tests` | `JobStatus` lifecycle transitions, `JobStatusRecord`, `JobRegistry` register/get/active/prune. | RFC-008 |
| `line_map_tests` | `LineMap` row states, navigation, `ScrollAnchor` clamping, `build_mini_map` weight sum. | RFC-035 |
| `merge_plan_tests` | `plan_operations`, `execute_plan`, `OperationPlan` safety. | RFC-022 |
| `merge_tests` | `MergeSession` apply/undo/redo, dirty state, `result_text`, transaction log. | RFC-006 |
| `patch_tests` | `patch_from_file_diff`, `to_unified`; GNU `patch` round-trip integration. | RFC-039 |
| `path_tests` | `split_parent_name`, `has_extension` (case-insensitive, dotfile edge cases), `display`, `canonicalize_lenient` (nonexistent/absolute/edge inputs). | RFC-001 |
| `persist_tests` | `VersionedEnvelope` round-trip, `MigrationPolicy`, newer-schema rejection. | RFC-031 |
| `platform_tests` | `PlatformInfo::collect` non-panic; `os`/`arch`/`app_version` non-empty; `to_report` format; home redaction (`***`); determinism; `logical_cpus` positive. | RFC-026 |
| `report_tests` | `FileComparisonReport`, `DirComparisonReport`, Markdown/JSON output. | RFC-027 |
| `save_tests` | `save_text` with fingerprint match, `TargetPrecondition`, `BackupPolicy`. | RFC-007 |
| `session_tests` | `WorkspaceSession` tab lifecycle, dirty state, `CloseResult`, JSON round-trip, schema-version guard. | RFC-011 |
| `settings_tests` | `UserSettings` defaults, round-trip JSON, theme/density/font round-trips, CSS var count, fallback. | RFC-009 |
| `three_way_tests` | `ThreeWayMergeSession` conflicts, resolution, undo/redo, `can_save`, `result_text`. | RFC-033 |
| `transaction_log_tests` | `TransactionLog` push/undo/redo/mark_saved, `is_dirty`. | RFC-015 |
| `vcs_tests` | `GitProvider::detect`, `VcsProvider` trait contract. | RFC-038 |
| `watcher_tests` | `MockFileChangeMonitor` watch/inject/poll/drain, `WatchError`, `FileChangeKind`. | RFC-036 |
| `xlsx_tests` | Fail-closed spreadsheet comparison behavior while XLSX parsing is security-disabled. | RFC-058 |

Integration tests in `tests/`:

| File | Count | Covers |
|---|---|---|
| `diff_corpus` | 27 | Corpus-driven `compute_diff` correctness: identical, insertions, deletions, reordered, empty, LF vs CRLF, no-final-newline, whitespace/trailing/tabs, case, function edit, Unicode, UTF-8 BOM, large files (200 lines), binary classification. Fixtures in `tests/fixtures/`. |
| `merge_corpus` | 16 | Corpus-driven `ThreeWayMergeSession` correctness across 6 fixture triples: no-conflict auto-merge, conflict detection and resolution (left/right), identical-both-sides dedup, one-sided insert, CRLF preservation, multiple conflicts. Fixtures in `tests/fixtures/merge/`. |
| `patch_apply` | 2 | Generates a unified-diff patch and verifies it applies with GNU `patch`. |

## `forskscope-ui-logic` test modules

All tests are inline (`#[cfg(test)]` inside each module file), except
`compare/load_identity`, which keeps its tests in a sibling `tests.rs`
(`mod tests;`) because of the fixture setup its fifteen cases share.
Integration tests live in `tests/css_coverage.rs`.

**Rebuilt against the module tree (`cargo xtask ui-logic-docs`, F93,
review 106 §4), not the prior table**: `command_bar`, `hunk_decorations`,
`scroll_sync`, `summary` and `tab_state` were deleted in `d69c83b`/`8f1af77`
and never removed here; `conflict_nav_view` and `palette_view` were deleted
by handoff 033's connectivity cleanup; `compare/load_identity`,
`compare/startup`, `session/persistence_recovery` and
`settings/persistence_recovery` existed but were never listed.

| File | Covers | RFC |
|---|---|---|
| `explore/align` | `compute_aligned_rows`: pairing, ordering, one-sided entries, recursion depth, selection state; field propagation (`is_selected` left/right, `depth`, `abs_path`/`rel_path`, `is_expanded`); both-sides-selected merges into one row. | RFC-059 |
| `explore/dir_verdict` | Each verdict (`Different`, `MetadataMatch`, `Unknown`); a one-sided entry, a size mismatch and a `Changed` file are `Different`; unreadable entry, symlink, unreadable root and a common file missing a size are `Unknown`; a definite `Different` stands over an unreadable entry in **both** orders of input, and `Unknown` outranks only a match. | RFC-080 |
| `explore/tier1_trigger` | The debounce as a pure state machine on a fake clock: twenty rows passed through start no walk; a rest past 250 ms starts exactly one; the selection moving cancels the walk and never leaves two in flight; navigation cancels; re-selecting the same row keeps its timer. | RFC-080 |
| `explore/deep_filter` | `DeepFilter::matches` for all `RecStatus` variants; `DeepCompareSummary::from_entries` counts (`total`/`changed`/`left_only`/`right_only`/`equal`/`unreadable`/`computing`/`visible`), including that `Unreadable` is excluded from both `different` and `equal`; `apply_filter`; `is_fully_computed`/`is_empty`. | RFC-037, RFC-038 |
| `explore/status` | `RowStatusKind::from_evidence` for all 10 `EqualityEvidence` variants; CSS prefix; glyph distinctness; aria labels; `needs_action`; `StatusGlyph` parity with `RowStatusKind` for every variant (F82). | RFC-054 |
| `compare/load_guard` | `guard_for_sizes` / `guard_for_sizes_with_limits`: all four `FileSizeClass` branches, worst-of-pair logic, boundary values (at-limit and one-over), message non-empty, distinct large/very-large labels, default-limit smoke tests. | RFC-013 |
| `compare/load_identity` | `CompareTabIdAllocator`/`LoadGeneration` monotonicity and no-reuse; `completion_decision` acceptance and every rejection case (wrong tab, stale/future generation, wrong state); zero values reserved; exhaustion never wraps. | RFC-075 |
| `compare/save_error` | `action_label` all variants non-empty; `SaveErrorView::from_error`: external-mod action set, primary ≠ Overwrite, `FileWriteFailed`/`InternalFault` actions; path passthrough; title/body non-empty; button labels non-empty; exactly one primary. | RFC-007, RFC-017 |
| `compare/search_index` | `MatchIndex` build/advance/retreat/wrap, `matching_hunk_ids`, empty index; `len`/`is_empty` consistency; `focused()` returns correct `hunk_id` and `row_index`; `focused_number` at start and after advance; `advance`/`retreat` return `None` on empty index. | RFC-014 |
| `compare/startup` | `parse_startup_args` for 0/2/3-argument forms and the rejected 1/4-argument cases (with a non-empty, count-naming error); `into_compare_request`'s `SaveDestination` per mode, including that mergetool mode never saves to the compared `remote` input. | RFC-077 |
| `session/persistence_recovery` | `SessionRecoveryView::from_resolution` for Fresh/Current (no notice, no dialog), committed migration (notice, no dialog), deferred-by-conflict (neither), failed-commit/Incompatible/Corrupt (each dialog's exact action set); every `RecoveryDialogAction` has a non-empty label. | RFC-076 |
| `settings/persistence_recovery` | Same shape as `session/persistence_recovery`, for `SettingsRuntimeResolution`/`SettingsRecoveryView`. | RFC-076 |
| `settings/settings_view` | `theme_choices`: covers all three themes, round-trips via `ThemeId::from_id`, non-empty labels, no duplicate values; `clamp_font_size` stays within the shipped 8-32 bound at both extremes (F53). | RFC-009 |

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
