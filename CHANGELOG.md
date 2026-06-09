# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
