# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
