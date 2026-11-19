# Testing

## Running tests

```sh
# Each engine crate, individually (none requires GTK) - RFC-093
cargo test -p forskscope-compare
cargo test -p forskscope-report
cargo test -p forskscope-edit
cargo test -p forskscope-desktop

# forskscope-commands (RFC-096): compare/mcp subcommand library - its own
# unit tests only, no compiled binary involved (those moved to
# forskscope-ui/tests/, below)
cargo test -p forskscope-commands

# View-model layer (no GTK required)
cargo test -p forskscope-ui-logic

# Headless test gate
cargo test -p forskscope-commands -p forskscope-compare -p forskscope-desktop -p forskscope-edit -p forskscope-report -p forskscope-ui-logic

# Clippy (must pass without warnings)
cargo clippy -p forskscope-commands -p forskscope-compare -p forskscope-desktop -p forskscope-edit -p forskscope-report -p forskscope-ui-logic -- -D warnings

# Crate-boundary rules: no file writes in compare/report, no process spawn
# there or in forskscope-commands outside show_to_person's one named call
# site, outside #[cfg(test)], and every crate's dependencies stay within
# RFC-093 §1's table
cargo xtask crate-boundaries

# forskscope compare --format json validates against
# docs/schemas/forskscope.comparison.v1.json (RFC-094 §3, RFC-096 §5)
cargo xtask json-schema

# Full workspace gates used by CI when GTK/WebKitGTK dependencies are present
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings

# cfg(windows) code (main.rs, webview2.rs), type-checked on a Linux runner —
# no linker involved, so it needs no MinGW toolchain (F109)
rustup target add x86_64-pc-windows-gnu
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings

# Format and generated CSS gates
cargo fmt --check
cargo fmt --manifest-path xtask/Cargo.toml --check
cargo xtask css --check

# Security audit (uses .cargo/audit.toml for reviewed advisories)
cargo audit

# Enforce the reviewed dependency paths behind audit exceptions
cargo xtask audit-deps

# Every removed/added/replaced colour pair stays distinguishable under
# simulated colour vision deficiency (RFC-088, RFC-090)
cargo xtask colour-vision

# No text/control below WCAG contrast, no control edge below 3:1, no text
# below 12px, outside a documented, reasoned allowlist (F214-F216)
cargo xtask contrast

# The same checks as above, but against the real, rendered app rather than
# CSS source text, across nine kinds of layout defect and every view
# (RFC-098; see "Checking rendered layout in the running app" below). A
# hard gate as of review 222 - a finding not on its own
# EXPECTED_FAILURES list (xtask/src/layout_check.rs) fails the build,
# and so does a listed one that stops occurring.
#
# Opens real forskscope windows and, on an unexpected failure, screenshots
# one - never run this from a normal desktop shell (review 223 §2.2): it
# refuses outside CI or this project's own nested session,
# .git-exclude/gui/session.sh --fixture <any> --backend x11 --no-app --
# cargo xtask layout-check
cargo xtask layout-check

# The in-app keyboard reference and keyboard.md name the same keys (F165)
cargo xtask keyboard-docs

# The committed .xlsx fixtures match what the generator produces
cargo xtask xlsx-fixtures --check

# Release metadata and localization gates
cargo xtask version-sync
cargo xtask i18n

# RFC schedule, and the ui-logic module tables' connectivity and docs (F54, F93)
cargo xtask rfc-sync
cargo xtask ui-logic-connectivity
cargo xtask ui-logic-docs
```

A non-default umask (077) is also exercised in CI, in its own step, for the
one test that can only fail under one (F41) — not reproduced above since it
needs a dedicated shell, not a bare `cargo test` invocation; see
`ci.yml`'s own "File permission tests under a non-default umask" step.

The UI crate (`forskscope-ui`) requires WebKitGTK/GTK3 to build and cannot
be tested in environments without a display server. The four engine crates
and ui-logic run anywhere Rust is installed.

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
| `forskscope-compare`/`-report`/`-edit`/`-desktop` unit | `crates/<crate>/src/tests/` (RFC-093) |
| `forskscope-compare`/`-report`/`-edit`/`-desktop` integration | `crates/<crate>/tests/` — `diff_corpus`, `f138_decline_before_delegating` (compare); `patch_apply` (report, shells out to real `git apply` and `patch -p1`); `merge_corpus` (edit) |
| `forskscope-commands` unit | `crates/forskscope-commands/src/mcp/tools.rs`'s own `#[cfg(test)]` module — `call_show_to_person_starting` against a fake script, no server/protocol machinery (RFC-096 §5) |
| `forskscope-ui` integration (`compare`/`mcp`) | `crates/forskscope-ui/tests/` — `compare_exit_codes.rs` (every exit-code path, via the compiled `forskscope compare` subcommand); `patch_matches_gui_export.rs` (stdout byte-compared against `patch_from_file_diff` + `to_unified` directly, RFC-094 §2); `mcp_protocol.rs`; `mcp_show_to_person.rs` — moved here from the removed `forskscope-cli`'s own `tests/` (RFC-096 §5) |
| `forskscope-ui-logic` unit | inline `#[cfg(test)]` modules, plus `command`/`diff_decoration`/`line_map`'s external `tests/*_tests.rs` files (moved in from the old `forskscope-core`, RFC-093 §1 step 5) |
| `forskscope-ui-logic` integration | `crates/forskscope-ui-logic/tests/css_coverage.rs` |
| `forskscope-ui` lib and bin | inline `#[cfg(test)]` modules, compiled twice (library and binary targets) |
| doctests | `forskscope-compare`, `forskscope-edit`, `forskscope-desktop` |

## `forskscope-compare` test modules

Tests live in `crates/forskscope-compare/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `app_error_tests` | `AppError::from_core`, `is_blocking`, `is_recoverable`, `ErrorId`, `TechnicalDetail`. | RFC-017 |
| `cancel_tests` | `CancellationToken::new`/`cancel`/idempotent; clone propagation (original→clone, clone→original, clone-of-clone); `Default`; `Debug`. | RFC-037, RFC-008 |
| `compare_profile_tests` | `CompareProfile` presets, `to_diff_options`, `NewlineCompareMode` engine wiring. | RFC-028 |
| `diff_tests` | `compute_diff`, hunk kinds, inline spans, equal/insert/delete/replace, whitespace/case modes. | RFC-002 |
| `dir_cancel_tests` | `recursive_diff_with_cancel`, cancellation mid-scan. | RFC-037 |
| `dir_ignore_tests` | Ignore rules reach the recursive directory walk: an ignored subtree is never walked or reported, shown by making it unreadable so a walk that descended would fail. | RFC-056 |
| `dir_tests` | Directory listing, recursive digest equality, `file_digest_equal`. | RFC-022 |
| `dir_unreadable_tests` | An unreadable entry is always visible — never silently dropped, never asserted as a verdict — and an unopenable root is distinguishable from an empty tree. | F79 |
| `document_tests` | `LoadedDocument`, `FileFingerprint`, `check_external_state`, `ExternalFileState`. | RFC-036 |
| `editability_tests` | `EditabilityClass::from_kind`, `requires_save_guard`, `NewlinePolicy::resolve`. | RFC-012 |
| `encoding_tests` | `decode_bytes`, `detect_newline_style`, `BomPresence`, `BomPolicy`, `detect_bom`. | RFC-012 |
| `error_tests` | `CoreError` variants, `AppErrorKind::from_core`, `RecoveryAction` defaults. | RFC-017 |
| `external_state_tests` | `check_external_state` with mocked fingerprints. | RFC-036 |
| `file_kind_tests` | `FileKind::is_mergeable_text` all variants; `classify` via tempfiles (text/binary/xlsx/uppercase-xlsx/empty/directory). | RFC-001 |
| `file_size_tests` | `FileSizeClass::classify` against `PerformanceLimits` thresholds. | RFC-013 |
| `ignore_tests` | `IgnoreRules::from_settings`, extension and directory pattern matching. | RFC-056 |
| `job_tests` | `JobStatus` lifecycle transitions, `JobStatusRecord`, `JobRegistry` register/get/active/prune. | RFC-008 |
| `path_tests` | `split_parent_name`, `has_extension` (case-insensitive, dotfile edge cases), `display`, `canonicalize_lenient` (nonexistent/absolute/edge inputs). | RFC-001 |
| `support` | No tests of its own — shared `chmod`-based "make this unreadable" helpers for every permission-dependent test in this crate. | F143 |
| `xlsx_tests` | Fail-closed spreadsheet comparison behavior: an unreadable pair errors instead of comparing as empty. Built only with the `spreadsheet` feature (on by default). | RFC-058 |

Integration tests in `tests/`:

| File | Count | Covers |
|---|---|---|
| `diff_corpus` | 27 | Corpus-driven `compute_diff` correctness: identical, insertions, deletions, reordered, empty, LF vs CRLF, no-final-newline, whitespace/trailing/tabs, case, function edit, Unicode, UTF-8 BOM, large files (200 lines), binary classification. Fixtures in `tests/fixtures/`. |
| `f138_decline_before_delegating` | 2 | A crafted CFB header declines before the full-file allocation it would otherwise trigger, capping this process's own address space first so the regression fails loudly (F138). Unix only. |

## `forskscope-report` test modules

Tests live in `crates/forskscope-report/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `build_tests` | `build_dir_report`/`BuildOutcome` (read-hash-diff every entry, stale-on-re-read refusal, the content cap); `checksum_if_fingerprinted`; `build_file_report`'s own `from_diff` + `with_checksums` wiring. Moved in from `forskscope-ui` (RFC-093 §1 step 6). | RFC-092 §5 |
| `patch_tests` | `patch_from_file_diff`, `to_unified`; GNU `patch` round-trip integration. | RFC-039 |
| `report_tests` | `FileComparisonReport`/`DirComparisonReport`'s model: counts, path privacy, the one-sentence summary, options/profile resolution, checksums, change hunks. Rendering tests (`html`, `text`, `json`) are inline in those modules instead (review 182 §2). | RFC-027 |
| `support` | No tests of its own — shared `chmod`-based "make this unreadable" helpers for every permission-dependent test in this crate. | F143 |

Integration tests in `tests/`:

| File | Count | Covers |
|---|---|---|
| `patch_apply` | 5 | Generates a unified-diff patch and verifies it applies with GNU `patch` and `git apply` (CRLF, mixed-newline, space-in-path conformance; RFC-039, RFC-084). |

## `forskscope-edit` test modules

Tests live in `crates/forskscope-edit/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `batch_tests` | `batch_copy`, `restore_from_manifest`, `BatchManifest`. | RFC-023 |
| `compare_prep_tests` | `save_target_from_loaded`/`inspect_save_target` — the prepared-comparison save-target derivation for normal compare and Git mergetool mode. | RFC-077 |
| `dir_tests` | `copy_file`'s backup-and-overwrite and destination-parent-dir creation — the write half of `dir`; the read half's own `dir_tests` lives in `forskscope-compare`. | RFC-093 §1 |
| `edit_op_tests` | `TextEditOperation` variants, `RevisionId`, `TextRange`, revision compatibility, `EditTransaction`. | RFC-032 |
| `merge_plan_tests` | `plan_operations`, `execute_plan`, `OperationPlan` safety. | RFC-022 |
| `merge_tests` | `MergeSession` apply/undo/redo, dirty state, `result_text`, transaction log. | RFC-006 |
| `save_target_tests` | `TargetPrecondition`/`check_precondition` and the no-clobber commit (`persist_noclobber`) directly, as primitives — `save_tests` covers their integration through `save_text`. | RFC-077 |
| `save_tests` | `save_text` with fingerprint match, `TargetPrecondition`, `BackupPolicy`. | RFC-007 |
| `support` | No tests of its own — shared `chmod`-based "make this unreadable" helpers for every permission-dependent test in this crate. | F143 |
| `three_way_tests` | `ThreeWayMergeSession` conflicts, resolution, undo/redo, `can_save`, `result_text`. | RFC-033 |
| `transaction_log_tests` | `TransactionLog` push/undo/redo/mark_saved, `is_dirty`. | RFC-015 |

Integration tests in `tests/`:

| File | Count | Covers |
|---|---|---|
| `merge_corpus` | 16 | Corpus-driven `ThreeWayMergeSession` correctness across 6 fixture triples: no-conflict auto-merge, conflict detection and resolution (left/right), identical-both-sides dedup, one-sided insert, CRLF preservation, multiple conflicts. Fixtures in `tests/fixtures/merge/`. |

## `forskscope-desktop` test modules

Tests live in `crates/forskscope-desktop/src/tests/` and are declared in `tests.rs`.

| File | Covers | RFC |
|---|---|---|
| `env_passthrough_tests` | `record_before_override`/`restore_onto` — a recorded variable is restored onto a `Command`, an unset one is removed rather than left at its overridden value, and an unrecorded variable is left untouched. | RFC-091 |
| `external_tool_tests` | `expand_args` placeholder expansion; shell safety (spaces, semicolons, $HOME, backticks); built-in presets. | RFC-029 |
| `persist_v2_repository_tests` | Explicit-path repository safe-write tests: `Missing` on an absent file, save/load round-trip, migration-commit backup semantics (created once, never overwritten by a retry), no stray temp file after an ordinary save. | RFC-076 |
| `persist_v2_runtime_tests` | The settings/session runtime adapter (`resolve_and_commit`) end to end against real temp-path repositories: fresh/current pass through unchanged, legacy files migrate, future/corrupt sources disable writes. | RFC-076 |
| `persist_v2_schema_enum_wire_format_tests` | Per-variant exact wire-string assertions for every schema enum `PersistedSettings` reuses directly — independent of any fixture, so a variant rename fails here even where a fixture's own value happens not to exercise it (F26). | RFC-076 |
| `persist_v2_session_tests` | Session schema v2 routing, migration and validation; the legacy v1 envelope (never shipped) is preserved as `Corrupt`, never silently reinterpreted. | RFC-076 |
| `persist_v2_settings_tests` | Settings schema v2 routing, migration and validation: current-envelope round-trip, exact field-for-field legacy v0 migration, future/corrupt/unrecognized-legacy handling, unknown payload fields tolerated, range/index normalization. | RFC-076 |
| `platform_tests` | `PlatformInfo::collect` non-panic; `os`/`arch`/`app_version` non-empty; `to_report` format; home redaction (`***`); determinism; `logical_cpus` positive. | RFC-026 |
| `support` | No tests of its own — shared `chmod`-based "make this unreadable" helpers for every permission-dependent test in this crate. | F143 |
| `vcs_tests` | `GitProvider::detect`, `VcsProvider` trait contract. | RFC-038 |
| `watcher_tests` | `MockFileChangeMonitor` watch/inject/poll/drain, `WatchError`, `FileChangeKind`. | RFC-036 |

## `forskscope-ui-logic` test modules

All tests are inline (`#[cfg(test)]` inside each module file), except
`compare/load_identity`, which keeps its tests in a sibling `tests.rs`
(`mod tests;`) because of the fixture setup its fifteen cases share, and
`command`/`diff_decoration`/`line_map`, moved in from `forskscope-core`
with their own external `tests/*_tests.rs` files intact (RFC-093 §1 step
5) rather than split and inlined - `command_tests.rs` alone spans two
source files (`command.rs` and `command/registry.rs`). Integration tests
live in `tests/css_coverage.rs`.

**Rebuilt against the module tree (`cargo xtask ui-logic-docs`, F93,
review 106 §4), not the prior table**: `command_bar`, `hunk_decorations`,
`scroll_sync`, `summary` and `tab_state` were deleted in `d69c83b`/`8f1af77`
and never removed here; `conflict_nav_view` and `palette_view` were deleted
by handoff 033's connectivity cleanup; `compare/load_identity`,
`compare/startup`, `session/persistence_recovery` and
`settings/persistence_recovery` existed but were never listed. **F51
(0.199.0):** `conflict_nav` (moved in alongside the other three above) and
`tests/conflict_nav_tests` were deleted outright next, the same way
`conflict_nav_view` had been — the four-region workspace UI the whole
contract was for was never built.

| File | Covers | RFC |
|---|---|---|
| `explore/align` | `compute_aligned_rows`: pairing, ordering, one-sided entries, recursion depth, selection state; field propagation (`is_selected` left/right, `depth`, `abs_path`/`rel_path`, `is_expanded`); both-sides-selected merges into one row. `pair_by_index` (compact): by position, not name; the shorter side padded at the end; rows outside the root dropped. | RFC-059, RFC-068 |
| `explore/classify_pair` | `classify_two_files` against the automatic-digest cap: a same-size pair at the cap is never read (proven with a `chmod 000` side); a size mismatch at the cap is still `Different`; under the cap nothing changes; a pair at the cap never reaches `NeedsDigest` either way it resolves. | RFC-080 §5, F77, F145 |
| `explore/dir_verdict` | Each verdict (`Different`, `MetadataMatch`, `Unknown`); a one-sided entry, a size mismatch and a `Changed` file are `Different`; unreadable entry, symlink, unreadable root and a common file missing a size are `Unknown`; a definite `Different` stands over an unreadable entry in **both** orders of input, and `Unknown` outranks only a match. | RFC-080 |
| `explore/tier1_trigger` | The debounce as a pure state machine on a fake clock: twenty rows passed through start no walk; a rest past 250 ms starts exactly one; the selection moving cancels the walk and never leaves two in flight; navigation cancels; re-selecting the same row keeps its timer. | RFC-080 |
| `explore/tier2_verdict` | Each verdict (`Identical`, `Different`, `Unknown`) over a fully-digested scan; an all-`Equal` scan is `Identical`; a `Changed` file or one-sided entry is `Different`; a `Computing` entry surviving the full walk (a mid-file cancellation) is `Unknown`, never `Identical`; unreadable entry, symlink and unreadable root are `Unknown`; a definite `Different` stands over an unreadable entry in both orders of input; a one-sided entry beneath an unreadable directory is `Unknown`, not `Different`. | RFC-080 tier 2 |
| `explore/deep_filter` | `DeepFilter::matches` for all `RecStatus` variants; `DeepCompareSummary::from_entries` counts (`total`/`changed`/`left_only`/`right_only`/`equal`/`unreadable`/`computing`/`visible`), including that `Unreadable` is excluded from both `different` and `equal`; `apply_filter`; `is_fully_computed`/`is_empty`; `demote_entries_under_an_unreadable_root` demotes every entry under an unreadable root (both directions, a `Symlink` entry too, a real `chmod 000` root), leaves entries untouched when neither root is unreadable, and is a no-op when both are. | RFC-037, RFC-038, F110 |
| `explore/status` | `RowStatusKind::from_evidence` for all 11 `EqualityEvidence` variants (including `TreeIdentical`, RFC-080 tier 2); CSS prefix; glyph distinctness; aria labels; `needs_action`; `StatusGlyph` parity with `RowStatusKind` for every variant (F82). | RFC-054, RFC-080 |
| `explore/sync_panes` | `mirror_target`: in-step descent and ascent mirror the triggering pane's path relative to its own anchor onto the other pane's anchor; ascending all the way back to your own anchor mirrors to the other pane's anchor exactly; navigating above your own anchor, or an unrelated jump, mirrors nothing even when the anchors happen to share a leaf name; the mirrored path is the triggering pane's relative path, not a coincidence. | F151, issue #148; anchored model from review 139 §3 |
| `explore/home_access` | `explorer_access`: outside a Flatpak always `Available` regardless of `home_granted`; inside one, `HomeRequired` iff `home_granted` is false, `Available` otherwise. | RFC-095 §4.3 |
| `compare/data_location` | `resolve_data_location`: every row of RFC-091 §1's table, the `--private`/`--data-dir` conflict, a marker with no program folder falling through to Installed (not treated as "found"). `extract_data_location_flags`: both flags extracted in any position with the remaining file arguments preserved in order, `--data-dir` with no following value, an unrecognized `--` flag rejected rather than passed through as a file name. | RFC-091 |
| `compare/load_guard` | `guard_for_sizes` / `guard_for_sizes_with_limits`: all four `FileSizeClass` branches, worst-of-pair logic, boundary values (at-limit and one-over), message non-empty, distinct large/very-large labels, default-limit smoke tests. | RFC-013 |
| `compare/load_identity` | `CompareTabIdAllocator`/`LoadGeneration` monotonicity and no-reuse; `completion_decision` acceptance and every rejection case (wrong tab, stale/future generation, wrong state); zero values reserved; exhaustion never wraps. | RFC-075 |
| `compare/save_error` | `action_label` all variants non-empty; `SaveErrorView::from_error`: external-mod action set, primary ≠ Overwrite, `FileWriteFailed`/`InternalFault` actions; path passthrough; title/body non-empty; button labels non-empty; exactly one primary. | RFC-007, RFC-017 |
| `compare/search_index` | `MatchIndex` build/advance/retreat/wrap, `matching_hunk_ids`, empty index; `len`/`is_empty` consistency; `focused()` returns correct `hunk_id` and `row_index`; `focused_number` at start and after advance; `advance`/`retreat` return `None` on empty index. | RFC-014 |
| `compare/startup` | `parse_startup_args` for 0/2/3-argument forms and the rejected 1/4-argument cases (with a non-empty, count-naming error); `into_compare_request`'s `SaveDestination` per mode, including that mergetool mode never saves to the compared `remote` input. | RFC-077 |
| `session/persistence_recovery` | `SessionRecoveryView::from_resolution` for Fresh/Current (no notice, no dialog), committed migration (notice, no dialog), deferred-by-conflict (neither), failed-commit/Incompatible/Corrupt (each dialog's exact action set); every `RecoveryDialogAction` has a non-empty label. | RFC-076 |
| `settings/persistence_recovery` | Same shape as `session/persistence_recovery`, for `SettingsRuntimeResolution`/`SettingsRecoveryView`. | RFC-076 |
| `settings/settings_view` | `theme_choices`: covers all three themes, round-trips via `ThemeId::from_id`, non-empty labels, no duplicate values; `clamp_font_size` stays within the shipped 8-32 bound at both extremes (F53). | RFC-009 |
| `settings/ignore_list` | Normalizing and deduplicating a field's current entries; adding one typed entry, several pasted at once, and a duplicate (by normalized value, both against what's already there and within one pasted batch) adding nothing; normalization matching the engine's own on the same input; removing a badge, including rewriting a pre-existing file's own un-normalized text. | RFC-099 §3 |
| `update_check` | `Version::parse`: a plain `X.Y.Z` round-trips; a `v` prefix, missing/extra component, pre-release suffix, or a hostile shape (`"0.187.0; rm -rf /"`, `"$(whoami)"`) all reject rather than guess; numeric not lexical ordering (`0.10.0 > 0.9.0`). `decide`: each of the five states from its outcome; the F166 falsification that no failure outcome ever decides `UpToDate`. `AppChannel::from_label`: unset and every unrecognised label fall back to `Direct`. `release_page_url`: built from the version alone, and structurally can only ever contain digits and `.`. | F180/F181 |
| `test_support` | No tests of its own - a shared helper exercised indirectly by every permission-dependent test in this crate that calls `permission_guard_failed`. | F143 |
| `command/registry` | Covered by `tests/command_tests`, below (the file spans both `command.rs` and `command/registry.rs`). | RFC-019 §5, §6, §7 |
| `diff_decoration` | Covered by `tests/diff_decoration_tests`, below. | RFC-024 |
| `line_map` | Covered by `tests/line_map_tests`, below. | RFC-035 |
| `tests/command_tests` | `AvailabilityRule::evaluate` across every rule variant and `CommandContext` combination; `CommandRegistry` lookup, builtin command coverage, shortcut/category/danger-level invariants. | RFC-019 §5, §6, §7 |
| `tests/diff_decoration_tests` | `DiffDecorationSet` derivation from a `DiffDocument`; `LineDecorationKind`/`InlineDecoration` CSS class and gutter-symbol stability. | RFC-024 |
| `tests/line_map_tests` | `LineMap` row alignment, `RowState` gutter symbols, mini-map weights, scroll-anchor clamping, next/prev-changed-row traversal. | RFC-035 |

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

## Checking keyboard bindings in the running app

Unit tests call the handler functions directly, so they cannot show that a key
reaches the handler from the webview, or what the running app does with it. For
that, drive the built app with its accessibility tree and the keyboard. This is
the procedure used to verify the Explorer's path-bar keys (0.181.0). It was run
on a Linux Wayland session under niri, with `wtype`, `niri`, and the Python
`gi` AT-SPI bindings installed.

1. **Launch with accessibility on.** Start the app with `GTK_A11Y=atspi` in the
   environment. Without it the app never joins the accessibility bus, and it
   looks unreachable. Copy the binary to a distinct name first (for example
   `bin-<label>`): the AT-SPI application name and the process name are both
   the binary's name, and that makes it findable and killable on its own.

2. **Prepare the configuration.** Back up `~/.config/forskscope/settings.json`
   and `session.json` before any run. Set `payload.last_left_dir` and
   `payload.last_right_dir` to fixture folders, and set
   `payload.remember_explorer_dirs` to `true`. **The `payload` envelope is
   required.** A hand-written file without it makes the app show *Settings file
   could not be read*, and that dialog swallows every later keypress. The
   symptom looks like a focus failure, and it cost three negative results in one
   session. If in doubt, launch once against an empty config directory and edit
   what the app writes.

3. **Focus the window, then the tree.** Get the window id from
   `niri msg -j windows` (match on the app id, which is the binary name), and run
   `niri msg action focus-window --id <id>` **before** the AT-SPI grab. Then,
   among the `section` nodes that AT-SPI reports as `FOCUSABLE`, pick the one
   whose text **starts with `▸`**. That is the tree's rows, in either layout. Do not choose
   by child count, which varies with the folder listed. Do not choose by
   "contains `▸`": the app shell's text also contains `▸`, and a first attempt
   grabbed the shell and saw no navigation. Call
   `Atspi.Component.grab_focus` on it. It returns `True` when it succeeds.

4. **Send keys.** `wtype -M alt -k Left -m alt` sends Alt+←, and `wtype -k F6`
   sends F6. F6 switches which pane has focus, so use it before Alt+↑ or Alt+←
   on the right pane. Keep the window focused between keys.

5. **Build history without the keyboard.** `Atspi.Action.do_action(0)` on a
   button works. The path bar's `Go up one directory`, `Back` and `Forward`
   buttons appear twice, once per pane, and the first instance is the left pane.

6. **Observe through `settings.json`.** Read `payload.last_left_dir` (or
   `last_right_dir`) about two seconds after each key. Navigation writes the
   directory to the settings file before it changes the pane's signal. So a
   navigation that then panics can still show its new value in the file. Check
   the process as well, every time (step 7).

7. **Check the process.** Use `pgrep -x <binary-name>` after two or three
   seconds. An app that is aborting can still show as alive for a second or so,
   so one early check is not enough. Use `pkill -x`, not `pkill -f`: `-f`
   matches the shell command line that contains the pattern, which kills the
   session's own shell.

8. **Close, confirm, restore.** Close the window with
   `niri msg action close-window --id <id>`. Wait for `pgrep -x` to return
   nothing before you touch the configuration again, because the app writes
   `settings.json` on exit when a setting has changed. Then restore the backups.

**What does not work in this environment, so nobody re-derives it:** pointer
injection. The app is a native Wayland window, `xdotool` cannot see it, and no
uinput tool is installed. Not needed for keyboard work, which the steps above
cover.

**Screenshots do work**, once an early round of attempts without a usable
destination wrongly concluded otherwise: `niri msg action screenshot-window
--id <id> --path <absolute path>` writes a PNG to that path. The path **must
be absolute** — the failures that led to the earlier, wrong conclusion were
runs with no usable `--path` at all. To get a fixed size for a reproducible
screenshot, `niri msg action toggle-window-floating --id <id>` first, then
`set-window-width`/`set-window-height`. So appearance is checkable by this
same procedure, not a case for deferring to someone else to look at the
window — though a released build run with real, non-invented data is still
the right source for anything public, never a screenshot from this
sandboxed procedure's own invented fixtures.

**What a unit test can and cannot show, from one case.** The Alt+↑ handler
(`dispatch_go_up`) does reproduce its borrow panic in a unit test, because the
test calls the same function the key handler calls. The webview dispatch itself
is not covered by any unit test, so the running app is the only check for it.

## Checking rendered layout in the running app

`cargo xtask layout-check` (RFC-098) drives the real, built app and measures
nine kinds of layout defect against its actual rendered output, not against
CSS source text the way `cargo xtask contrast`/`colour-vision` do. It needs
no accessibility bus and no `wtype` — unlike the keyboard section above, it
measures entirely in-process, through `layout_probe.rs` (built only behind
the `layout-probe` Cargo feature) and a single `document::eval` call per
combination, so it is also usable headless under `xvfb-run` (CI's own job
runs this way; see `.github/workflows/ci.yml`'s `layout-check` job).

**Never run from a normal desktop shell (review 223 §2.2, review 224 §2).**
It opens real `forskscope` windows — the full matrix launches it five
times, a capture re-launch (below) once more per unexpected finding — and
on an unexpected finding, screenshots one of them; from a plain shell
there is no way to tell this run's own window from the maintainer's own,
real session, so `require_private_display` (`xtask/src/layout_check.rs`)
refuses to run unless `$GITHUB_ACTIONS == "true"` and `$DISPLAY` are set
(CI's own Xvfb — not the generic `$CI`, which a developer's own shell can
also set) or `$FORSKSCOPE_GUI_SESSION`, `$NIRI_SOCKET` and
`$SESSION_X11_DISPLAY` all are (this project's own nested `niri` session,
`.git-exclude/gui/session.sh`, which now exports all three). Every launch
sets `DISPLAY` explicitly from that value — never the caller's own,
inherited one, even once the checks above already passed. `--no-app`
(review 224 §2.3) starts the session itself with no app of its own, since
this driver launches its own (one, or several short-lived ones) — without
it, every local run would be a second instance alongside session.sh's:

```sh
.git-exclude/gui/session.sh --fixture <any> --backend x11 --no-app -- \
  cargo xtask layout-check
```

**The nine checks**, every one against the real, rendered DOM: overflow,
clipped text, off-screen controls, hit area (a touch/click target under
24×24px), small text (effective font size under 12px, 100% text size only),
rendered contrast (WCAG, against the resolved `color`/`background-color`,
not CSS source), label placement (an `aria-labelledby` target not
horizontally aligned with, or below, its label), dialog occlusion (nothing
but a toast may cover an open dialog), and overlap (two elements with their
own text or interactive role occupying the same pixels, outside a dialog's
own scrim).

**The matrix is two axis groups, not one flat product** (`xtask/src/
layout_check.rs`'s own module doc has the full design): layout checks run at
text size × language × window × view, one fixed colour setting; rendered
contrast runs at theme × colour vision × contrast × view, at two fixed
(window, text size) layouts, English only. Both groups cover all six views
today: Explorer, file comparison, Deep Compare, the merge tool, Settings
(with many ignore-list entries, to populate the advanced section's own
badges), and the keyboard shortcuts reference.

**`--inject <defect>`** plants one named CSS rule on Settings' own reachable
elements (one shape per check: an overflowing label, an undersized button, a
same-colour pair, 8px text, and so on), confirms the matching check's own
field comes back non-empty, then reverts — a `Drop` guard that also
regenerates `main.css`, so a run that fails partway never leaves the tree
dirty. Run it to sanity-check a change to the probe's own measurement script,
not as part of the normal gate set:

```
cargo xtask layout-check --inject overlap
```

`cargo xtask layout-check --help`-equivalent (`xtask/src/main.rs`'s own
`print_usage`) lists every defect name.

**Capture on an unexpected failure** (review 220 §5): when a finding isn't
on `EXPECTED_FAILURES`, a second, targeted launch reopens that exact
combination (view, text size, language, theme, contrast, colour vision,
window — `capture_failures`/`write_capture_plan`), pauses it on screen
(`layout_probe::Plan::pause_before_exit_ms`), and screenshots it to
`target/layout-check/failure-captures/`. Two capture routes, picked by
`$WAYLAND_DISPLAY`, not by a fixed CI/local split: CI's own Xvfb is a
plain, uncomposited X server, so `import -window root` reads the composited
frame straight off its root window; this project's own local sessions run
under a nested `niri` (Wayland), whose XWayland side never paints a
composited frame to its root window at all, so `niri msg action
screenshot-window --id <id> --path <absolute path>` is used there instead
(`capture_screen`, `xtask/src/layout_check.rs`). CI uploads the directory
as the `layout-check-failure-captures` artifact, only `if: failure()`.
`--inject <defect>` exercises the same capture path unconditionally (one
combination, always worth a look), which is how it was falsified: `import
-window root` under this project's own nested-`niri` dev sessions fails
outright ("missing an image filename") where the `niri` route succeeds and
writes a real screenshot of the failing view.

**The `niri` route finds the probe's own window by process id, never by
its app id** (review 223 §2.2): this project's own nested session is
still a real, shared compositor the maintainer's own, real ForskScope
could also be open in — app id alone (`"Forskscope"`) would pick
whichever window niri lists first, which can be either one.
`capture_via_niri` instead reads the probe's own pid from
`dbus_run_session`'s own direct child (unambiguous — no two running
processes ever share one) and matches `niri msg -j windows`'s own `pid`
field against it.

**A hard CI gate as of review 222.** The job runs on every push and pull
request, and exits non-zero on anything not named on
`EXPECTED_FAILURES` (`xtask/src/layout_check.rs`) — five findings today
(F227–F231), each a registered, scope-pending defect (`ROADMAP.md`),
not something this check decides for itself. An entry nothing in a run
matches is itself a failure: a list that could go stale silently would
defeat the entire point of naming findings on it. Getting here from the
first real run's own 2,033 findings (reply 189) took five named
checker-fidelity fixes (replies 189b/189c) — almost all of that count
was the checker's own false positives, not a sudden wave of app fixes.

## The `.xlsx` corpus, and the sizes it covers

The spreadsheet tests in `forskscope-compare/src/xlsx.rs` read real workbooks from
`src/tests/fixtures/xlsx/`, which `cargo xtask xlsx-fixtures` generates and
`--check` verifies. The corpus is described case by case in that directory's
`README.md`, with each case's recipe.

`cargo xtask xlsx-fixtures --check` runs in CI (the gate list above), so a
committed fixture that drifts from its own recipe fails the build, the same
as every other "two things must agree" gate in this list.

Two dimensions are named here, so a reader knows what the corpus must cover:

- **Row identity.** Rows inserted, deleted, edited in place, identical to another
  row, and repeated formulas. Each has a case in the corpus.
- **Size asymmetry (F137).** A one-row sheet against a 40,000-row sheet is the
  largest cascade positional comparison can produce, and the shape where the
  content-aligned comparison costs the most. It is not committed, because a
  40,000-row workbook is repository weight for no gain over its recipe. It is
  written on demand, and so is the 2,000-row headline case:

  ```sh
  cargo xtask xlsx-fixtures --scale /some/scratch/dir
  ```

  The text corpus has the same dimension in memory, in `tests/diff_corpus.rs`:
  five lines against forty thousand.

The measured cost of each size is in RFC-058's amendment and the threat model's
parser section, not repeated here. Measurements that read generated workbooks
are `#[ignore]`d tests that take a directory from an environment variable
(`FSK_D_DIR`, `FSK_DIR_LEFT`/`FSK_DIR_RIGHT`).

