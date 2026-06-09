# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.35.0] — 2026-06-09

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
