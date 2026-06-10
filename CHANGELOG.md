# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
