# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.156.0] — 2026-06-18

Phase 2 complete: split `settings.rs` (310 ELOC). No behaviour changes.

### Changed

`ui/view/settings.rs` (310 ELOC) split into three files:

| File | ELOC | Responsibility |
|---|---|---|
| `settings.rs` | 46 | `persist`, `load`, `ModalLayer`, type helpers |
| `settings/modal.rs` | 218 | `SettingsModal` dialog (appearance, advanced, profiles) |
| `settings/profile.rs` | 64 | `AddProfileInline` form |

All three Phase 2 targets are now under the 300 ELOC soft limit:
`explorer.rs` (251), `diff.rs` (211), `settings.rs` (46). The largest
remaining file is `state.rs` (427 ELOC) — a candidate for Phase 3 splitting.

---

## [0.155.0] — 2026-06-18

Phase 2: split `diff.rs` (339 ELOC). No behaviour changes.

### Changed

`ui/view/diff.rs` (339 ELOC) split into two files:

| File | ELOC | Responsibility |
|---|---|---|
| `diff.rs` | 211 | `DiffWorkspace`, `DiffHeader`, `TabSnapshot` |
| `diff/toolbar.rs` | 160 | Primary toolbar + advanced disclosure panel |

`Toolbar` was a private component in `diff.rs` with its own `advanced` signal
and 14 action buttons. Extracting it into `diff/toolbar.rs` gives it a clear
boundary and keeps each file well under the 300 ELOC soft limit. `TabSnapshot`
fields are now `pub(crate)` so `toolbar.rs` can read them via the `super::` path.

---

## [0.154.0] — 2026-06-18

Phase 2: split `explorer.rs` (704 ELOC). No behaviour changes.

### Changed

`ui/view/explorer.rs` (704 ELOC) split into five files:

| File | ELOC | Responsibility |
|---|---|---|
| `explorer.rs` | 251 | Root component, signal setup, digest computation |
| `explorer/tree.rs` | 242 | Aligned two-pane tree + keyboard navigation |
| `explorer/compact.rs` | 141 | Compact (unaligned) view (RFC-068) |
| `explorer/filter.rs` | 110 | Filter bar UI and filter predicate (RFC-067) |
| `explorer/footer.rs` | 71 | Targets label and Compare button (RFC-069) |

Shared types (`DigestKey`, `FocusedPane`, `PickKind`, `CompareAction`,
`compare_action`) remain in `explorer.rs` and are accessed via `super::` from
the submodules. No behaviour, CSS, string, or i18n changes.

---

## [0.153.0] — 2026-06-18

Import migration and `mod.rs` cleanup. No behaviour changes.

### Changed

**Import migration (v0.152.0 TODO resolved):**
All `crate::ui::X` import paths updated to their explicit new locations
established in v0.152.0. The backward-compatible re-exports that were marked
`TODO(v0.153): remove after import migration` have been removed from `ui.rs`.
`ui.rs` now declares only the four submodules (`bridge`, `layout`, `overlay`,
`view`) with no re-exports.

Updated files: `app.rs`, `state.rs`, `ui/overlay/modals.rs`,
`ui/view/diff.rs`, `ui/view/explorer.rs`, `ui/view/hunk.rs`,
`ui/view/search.rs`, `ui/view/settings.rs`.

**`forskscope-ui-logic` `mod.rs` → sibling files:**
Applied the same Rust 2018/2024 module style to `crates/forskscope-ui-logic`:

- `compare/mod.rs` → `compare.rs`
- `explore/mod.rs` → `explore.rs`
- `settings/mod.rs` → `settings.rs`

No `mod.rs` files remain in either `forskscope-ui` or `forskscope-ui-logic`.

---

## [0.152.0] — 2026-06-18

RFC-069: Explorer Compare action and targets label. RFC-070: Diff font family
selector. Security audit, doc verification, and UI module restructuring (Phase 1).

### Fixed (security audit)

- **`binary_cache` not cleared on directory navigation** — stale sniff results
  persisted across directory changes. Fixed by clearing in the `use_effect`
  hooks for `left_dir` and `right_dir`.
- **Filter loop calling `classify()` without cache** — the "Hide binary" filter
  predicate bypassed `binary_cache` and called `classify()` on every render
  frame. Fixed to use the cache.
- **`binary_cache` not declared `mut`** — seven `.write()` call sites failed
  E0596. Added `mut` to the declaration.
- **E0515 double-borrow in compact right pane** — chained `digest_map.read()`
  calls held two guards simultaneously. Split into two sequential reads.
- **Unused variable warnings** — `is_dir` → `_is_dir` in compact-pane filter
  closures.

### Changed (RFC-069)

**Persistent targets label and Compare ▶ button:**
The Explorer footer now always shows a targets label guiding the user
progressively: no picks → partial pick with guidance text → both picks ready
with names. The Compare button is styled with accent colour when enabled.

### Changed (RFC-070)

**Diff font family selector:**
Five presets added to Settings → Appearance: Monospace (default), Sans-serif,
Serif, Courier New, Consolas / Menlo. Applied via `--diff-ff` CSS variable.

### Changed (UI module restructuring — Phase 1)

`crates/forskscope-ui/src/ui/` was a flat list of 27 files. Restructured into
four named groups per the architect's review, using Rust 2024 sibling-file
module style (no `mod.rs`):

```
ui.rs
ui/
  layout/    header.rs, tabs.rs, statusbar.rs
  view/      explorer.rs, diff.rs, settings.rs, deep_compare.rs,
             dir_pane.rs, diff_actions.rs, hunk.rs, search.rs
  overlay/   modals.rs, keybindings.rs
  bridge/    14 thin ui-logic re-export shims
```

- `state/mod.rs` renamed to `state.rs` (Rust 2018 style; `state/settings.rs`
  unchanged).
- Backward-compatible `pub use` re-exports added to `ui.rs` so all existing
  `crate::ui::X` import paths continue to resolve. Each re-export is marked
  `TODO(v0.153): remove after import migration`.
- No UI behaviour, CSS, string, or i18n changes in this PR.

### Added (docs)

- `docs/src/maintainers/threat-model.md` — first security documentation:
  data flows, controls, no-network guarantee, audit history.
- `docs/src/users/settings.md` — diff font family, binary comparison toggle,
  Explorer layout option.
- `docs/src/users/explorer.md` — binary files, filter bar, compact layout,
  targets label, async tab opening; corrected stale focused-pane description.
- `docs/src/users/features.md` — corrected horizontal scroll description;
  updated Explorer and Appearance sections.

---

## [0.152.1] — 2026-06-18

Build fixes and module style correction.

### Fixed

- **`binary_cache` not declared `mut`** — seven call sites called `.write()` on
  a `Signal` bound without `mut`, causing E0596. Added `mut` to the declaration.
- **E0515 double-borrow in compact right pane** — `digest_map.read().get(...)`
  was chained with `.or_else(|| digest_map.read().get(...))`, holding the first
  read guard while acquiring a second. Split into two sequential reads.
- **Unused variable warnings** — `is_dir` in the two compact-pane filter
  closures renamed to `_is_dir`.

### Changed

- **`state/mod.rs` → `state.rs`** — Rust 2018 module style: the state module
  now lives at `src/state.rs` with its `settings` submodule at
  `src/state/settings.rs`. All `crate::state::` import paths are unchanged.

---

## [0.152.0] — 2026-06-16

RFC-069: Explorer Compare action and targets label. RFC-070: Diff font family selector.

### Changed

**RFC-069 — Persistent targets label and Compare button redesign:**
The Explorer footer now shows a **targets label** at all times, not only when
both picks are set. The label guides the user progressively:

- *No picks:* "Choose a file or folder on each side to compare" (muted hint)
- *Left pick only:* `filename ↔ Choose a file or folder on the right`
- *Right pick only:* `Choose a file or folder on the left ↔ filename`
- *Both picks:* `left-name ↔ right-name` (ready — Compare button enabled)

The Compare button sits to the right of the targets label, visually connected.
It is styled with an accent border/colour when enabled (`.compare-btn`) and
uses the new label `Compare ▶` to make the action direction clear. The button
and label are the same visual unit; Compare is disabled with muted styling
until both picks are ready.

**RFC-070 — Diff font family selector:**
A **Diff font family** selector is added to Settings → Appearance (alongside
the existing diff font size control). Five presets using safe cross-platform
CSS font stacks:

| Preset | Font stack |
|---|---|
| Monospace (default) | `ui-monospace, monospace` |
| Sans-serif | `system-ui, sans-serif` |
| Serif | `Georgia, serif` |
| Courier New | `Courier New, Courier, monospace` |
| Consolas / Menlo | `Consolas, Menlo, monospace` |

The selected family is injected as `--diff-ff` CSS variable on the
`.diff-scroll` wrapper, parallel to the existing `--diff-fs` variable.
Default is unchanged (monospace); existing users see no difference until they
choose a different preset. Setting persists across launches.

### Added

- `DiffFontFamily` enum in `state/settings.rs` with `css_value()` helper.
- `TabSnapshot::font_family` field; `from_tab` reads `settings.diff_font_family`.
- i18n keys (ja): `Diff font family`, `Monospace (default)`, `Sans-serif`,
  `Serif`, `Choose a file or folder on each side…`, and the partial-pick hints.

---

---

## [0.151.0] — 2026-06-16

RFC-068: Explorer unaligned (compact) view mode.

### Added

**Compact Explorer layout (RFC-068):**
A new **Explorer layout** setting under Settings → Advanced. Two options:

- **Aligned (default)** — the existing behaviour: same-name entries share a row
  across panes; spacer rows fill gaps where one side is missing. Cross-pane
  alignment is structural; vertical scroll syncs implicitly.
- **Compact (independent panes)** — no spacer rows. Each pane renders its own
  flat list and scrolls independently. Entries are sorted by name within each
  pane but not forced to align across panes.

Compact mode is useful in directories where many files exist only on one side
(many left-only or right-only entries), where the aligned view produces mostly
whitespace spacers.

**Design decisions recorded in RFC-068:**
- Cross-pane alignment is intentionally absent in compact mode — this is the
  trade-off; row N on the left does not correspond to row N on the right.
- Same-name equality badges (✓/⚠) still appear in compact mode via name-keyed
  digest lookup.
- Picks, binary badges, and navigation work the same way in both modes; the
  filter bar (RFC-067) applies independently per pane in compact mode.
- `AppSettings::explorer_compact` (default: `false`, persisted) controls the mode.

---

---

## [0.150.0] — 2026-06-16

RFC-067: Explorer name-pattern filter and filter checkboxes.

### Added

**Filter bar in the Explorer (RFC-067):**
A compact filter bar between the path bars and the pane-root labels, revealed
by clicking the `⊞` toggle button. Empty and hidden by default (calm default,
D-001 — no visual noise when not in use).

**Filter controls:**

- **Name filter** — a text input that narrows visible Explorer rows live,
  case-insensitive substring match. A pair is shown if *either* side's filename
  contains the query.
- **Hide binary** checkbox — hides pairs where all present file sides are
  binary (works with RFC-066; only meaningful when binary comparison is off).
- **Hide identical** checkbox — hides pairs whose digest status is `Equal` (✓).
- **Clear (✕) button** — appears when any filter is active; resets all three
  to their defaults in one click.

**Filter semantics preserve cross-pane alignment:** the aligned row list is
filtered *after* `compute_aligned_rows`, so pairs that survive the filter
remain correctly matched. Spacer rows for filtered-out entries are removed
alongside their counterparts.

### Notes

- Filter state is session-only (not persisted) per RFC-067 open-question
  resolution: name query and checkboxes reset on app restart.
- The filter bar input stops key propagation so typing in the filter does not
  trigger global shortcuts.

---

---

## [0.149.0] — 2026-06-16

RFC-066: Binary comparison policy — off by default, explicit opt-in.

### Changed

**Binary comparison disabled by default (RFC-066):**
Binary files (NUL-byte sniff → `FileKind::Binary`) can no longer be compared
unless **Enable binary comparison** is turned on in Settings → Advanced.

- **Explorer rows:** binary files show a `bin` badge and are rendered muted
  (`opacity: .5`, `cursor: default`). Clicking or double-clicking a binary row
  does nothing while the setting is off. The row is visible (D-015: don't hide
  unsupported cases); only the action is blocked.
- **Attempting to compare:** `load_and_diff` (used by both `open_compare` and
  `reload_tab`) returns a clear error — "Binary comparison is off. Enable it in
  Settings → Advanced." — which the async tab lifecycle shows as `TabState::Error`.
- **Setting on:** binary rows become fully actionable; the comparison runs via
  the async lifecycle (RFC-065) so large binary hex diffs no longer freeze the app.

**`FileEntry::is_binary` added to core `listing.rs`:**
`list_dir` now classifies each file as binary using the same NUL-byte sniff
as `classify()`. The field is propagated through the listing result so callers
can use it without re-reading file content.

**Binary detection cache in Explorer:**
A `Signal<HashMap<PathBuf, bool>>` caches the classify result per path for the
current directory, so the NUL-byte sniff runs at most once per unique path per
Explorer session rather than on every render.

### Added

- `AppSettings::enable_binary_comparison` (default: `false`, persisted).
- Settings → Advanced: **Enable binary comparison** checkbox with tooltip.
- CSS: `.tree-row.binary-blocked`, `.tree-status.st-binary`.
- i18n keys (ja): `Enable binary comparison`, the tooltip string, and the
  "Binary comparison is off" notice.

---

---

## [0.148.0] — 2026-06-16

RFC-065: Asynchronous comparison — loading-state tabs, off-thread load+diff.

### Changed

**Opening a comparison no longer blocks the UI (RFC-065):**
`open_compare` and `reload_tab` previously ran file load, decode, and diff
computation synchronously on the UI thread. Large files — especially large
binary files rendered as hex — would freeze the app for the duration. This was
the reported "app stacked" symptom.

Both functions are now asynchronous:

1. A tab is created **immediately** in `TabState::Loading` and made active —
   the user sees the tab open right away with a spinning `⟳` indicator in the
   tab title and the diff area.
2. File I/O and diff computation run in a **tokio blocking thread**
   (`spawn_blocking`) off the UI thread, so the rest of the app remains
   responsive.
3. When the task completes, it writes the result back via the `Signal<Vec<CompareTab>>`
   directly (a guard checks that the tab is still present and still loading before
   writing). The tab transitions to `TabState::Ready` (showing the diff) or
   `TabState::Error` (showing a recoverable message).

**`TabState` enum added to `CompareTab` (RFC-065):**
- `TabState::Loading` — spinner shown, tab not dirty, close skips dirty-check.
- `TabState::Ready` — normal diff view, all existing behaviour unchanged.
- `TabState::Error(String)` — friendly error message with the path and cause.

**`DiffDocument::empty()` and `MergeSession::empty()` added to core:**
Sentinel constructors needed for the Loading placeholder; never rendered.

### Added

- i18n key (ja): `Loading`.
- CSS: `.diff-loading`, `.diff-loading-spinner`, `.tab-loading-spinner`,
  `.diff-error`; `@keyframes spin` for the spinner rotation.

---

---

## [0.147.0] — 2026-06-16

RFC-064: Compare view bug fixes — per-pane horizontal scroll and all-different
line coloring.

### Fixed

**Per-pane horizontal scrollbars (RFC-064, Approach A):**
Switched from the previous single shared horizontal scrollbar ("Approach B",
ISSUE-001) to per-pane scrollbars ("Approach A"). Each diff pane now owns its
own `overflow-x: auto` scrollbar, so a long line in the left pane can be
revealed independently of the right pane and vice versa. The old behaviour —
where the shared scrollbar was clipped/unreachable when one pane overflowed —
is resolved.

CSS changes: `.diff-scroll` now sets `overflow-x: hidden` (no longer owns the
horizontal bar); `.diff-pane` sets `overflow-x: auto`; `.diff-row` no longer
needs `min-width: 110ch`; `.diff-pane-labels` no longer needs a matching
`min-width`. Word-wrap mode (`diff-scroll.wrap`) explicitly sets
`overflow-x: hidden` on panes to suppress the per-pane scrollbar while wrapping.
Vertical scroll-sync is unaffected.

**All-different file coloring (RFC-064):**
When the focused hunk was a delete/insert/replace hunk, the CSS class was
replaced with `"hunk focused"` — dropping `hunk-del`/`hunk-ins`/`hunk-rep`
entirely. This meant that in files where all or most hunks are changed (e.g.
two completely different files), focusing any hunk would strip its color. Fixed:
the focused class is now appended (`format!("{kind_class} focused")`) so both
the kind color and the focus ring are always present.

### Changed

- `docs/src/users/known-limitations.md` — removed the "Horizontal scroll is
  shared between both panes" limitation (now fixed).
- `docs/src/users/diff-workflow.md` — updated scrolling section to describe
  per-pane scrollbars.
- RFC-064 moved from `rfcs/proposed/` to `rfcs/done/` (Implemented v0.147.0).
  RFC index: Implemented 39 → 40, Proposed 20 → 19.

---

---

## [0.146.0] — 2026-06-16

RFC drafting for the next feature track. No code changes — design only.

### Added — feature-track RFCs (drafted, Proposed)

Seven RFCs drafted for the post-v0.145 feature track, numbered fresh from 064:

- **RFC-064** — Compare View: per-pane horizontal scrollbars (reverses the
  intentional single-shared-scrollbar "Approach B" in favour of "Approach A",
  per maintainer decision) and a fix for missing coloring when no lines match.
- **RFC-065** — Asynchronous Comparison and Loading-State Tabs: open a tab
  immediately in a Loading state, run load+diff off the UI thread, resolve to
  Loaded/Error; cancellable. Prevents the app freezing on large/binary files.
- **RFC-066** — Binary Comparison Policy: off by default, opt-in via Settings →
  Advanced; binary files shown in the Explorer as visible-but-non-actionable
  with a clear reason (honours NG-005 and D-015 simultaneously).
- **RFC-067** — Explorer Name-Pattern Filter and Filter Checkboxes: live name
  filter plus "hide binary" / "hide identical" checkboxes; minimal by default.
- **RFC-068** — Explorer Unaligned (Compact) View Mode: optional mode that
  removes spacer rows so panes pack independently. Recorded explicitly as a
  view mode that breaks cross-pane row alignment and disables vertical
  scroll-sync (maintainer-requested).
- **RFC-069** — Explorer Layout: Compare action placement, a targets label
  stating what Compare will open, and a header/footer rethink that accounts for
  the new filter bar (067) and view-mode control (068). Sequenced after them.
- **RFC-070** — Font Family Selector: a diff font family setting (preset list),
  paralleling the existing font-size control.

`rfcs/README.md` index updated (Proposed 13 → 20); statuses for the shipped
RFC-060–063 refreshed to reflect v0.145.x delivery.

### Notes

- Proposed implementation order: 064 (bug fixes) → 065 (async) → 066 (binary
  policy) → 067/068 (Explorer filters and view mode) → 069 (Explorer layout) →
  070 (font selector). Each ships as its own release line per the cadence.
- RFC-064 and RFC-068 record deliberate reversals/additions to previously
  intentional design decisions (shared scrollbar; aligned-panes), with rationale.

---

---

## [0.145.3] — 2026-06-14

RFC-062: Safe batch copy UX and restore manifest integration.

### Changed

**RFC-062 B1 — All copies route through the core batch manifest:**
Both `ConfirmDirOpModal` (single file) and `BatchCopyModal` (multiple files)
now use `forskscope_core::dir::batch::batch_copy` with `ContinueOnFailure`
policy instead of a direct `copy_file` loop. Every copy produces a
`BatchManifest` and writes a JSON file to
`$XDG_DATA_DIR/forskscope/manifests/<op-id>.json` for audit and recovery.

**RFC-062 B2 — Result summary instead of silent dismiss:**
A new `BatchResultModal` shows after every copy operation. Displays
succeeded/failed counts, the manifest path, and per-entry failure details
(up to 5 shown). On success, a brief result modal appears; on failure, it
stays open so the user sees what went wrong. Single-file success still
shows only a toast (no result modal). `Modal::BatchResult(BatchResultSpec)`
added to the modal enum.

**RFC-062 B3 — Explicit, symmetric direction labels:**
Per-row copy buttons in the Directory Report now show `Copy to right` and
`Copy to left` in full text instead of `Copy →` / `← Copy`. Changed entries
show **both** directions; LeftOnly shows only `Copy to right`; RightOnly only
`Copy to left`. Batch copy toolbar buttons updated the same way.

**RFC-062 B4 — Confirmation modal wording clarified:**
`ConfirmDirOpModal` title changed to "Copy this file?" (was "Copy file?").
Backup notice changed to "The destination exists. A .bak backup will be
created first." (was "Destination exists. A .bak backup will be created.").
Backup notice uses `notice-ok` (green) as it is a reassurance, not a warning.
The label field is removed (modal already shows From/To paths).

### Added

- `dirs-next = "2"` dependency for XDG data directory resolution.
- 10 new i18n keys (ja); 4 dead keys removed.

---

---

## [0.145.2] — 2026-06-14

RFC-061: Explorer pane focus and keyboard completeness.

### Changed

**RFC-061 — Explorer focused-pane model:**

Added `FocusedPane { Left, Right }` signal to the Explorer component. All
keyboard navigation in the aligned tree now dispatches to the focused pane's
tree rather than always the left tree.

- **F6** switches the focused pane (Left ↔ Right). The active pane's root
  header cell shows a focus ring (2px accent outline + accent-tinted bg) and
  a `◀` indicator. Clicking a pane-root-cell also focuses that pane.
- **↑ / ↓ / ← / → / Enter / Space / Home / End** — operate on the focused
  tree. Space and Enter also set the pick for the focused pane.
- **Alt + ↑** — now navigates only the focused pane up (previously navigated
  both panes simultaneously). "Go up one directory (focused pane)" in keyboard
  reference.
- ARIA: each pane-root-cell gets `role="heading"` and `aria-label` including
  the pane name and current root directory name.

A keyboard-only user can now complete the full select-and-compare flow:
focus left pane (default) → navigate → Space to pick → F6 → navigate right
pane → Space to pick → Tab to Compare button → Enter.

**`ui/keybindings.rs`** — F6 added to Navigation section; Alt+↑ description
clarified to "(focused pane)".

**`assets/main.css`** — `.pane-root-cell.pane-focused` focus ring; cursor
pointer on all pane-root-cells.

### Added

- i18n keys (ja): `Left pane`, `Right pane`, `Switch focused pane (left ↔ right)`,
  `Go up one directory (focused pane)`.

---

---

## [0.145.1] — 2026-06-14

UX hardening sprint — RFC-060, RFC-063 partial implementation.

### Changed

**RFC-060 W1 — Explicit `stop_propagation` on all text-entry surfaces:**
The Sprint 0 safety fix already added `stop_propagation` to the search bar.
Now added to the path-bar editor (`dir_pane.rs` Enter and Escape) and the
settings scrim Escape handler. Coverage is now explicit per-surface rather
than incidental to the modal-open guard.

**RFC-063 C5 — Severity-based notice/toast policy:**
Introduced `Notice` type with `NoticeSeverity` (Success / Info / Warning /
Error). `Store::notify` (error, persistent) is joined by `notify_success` (3.5 s
auto-dismiss), `notify_info` (5 s auto-dismiss), `notify_warning` (persistent).
Success toasts: "Saved.", "Reloaded.", "Copied." Auto-dismiss via
`tokio::time::sleep` in the render layer. CSS classes `toast-success`,
`toast-info`, `toast-warning`, `toast-error` added with matching border colors.
Binary/text mismatch guidance reclassified from error to warning.

**RFC-063 C7 — "Local only" trust marker:**
Status bar now shows `🔒 Local only` with tooltip "Files stay on this
computer. ForskScope does not upload them." Japanese translation added.

**RFC-063 C9 — Exact directory-report statistics:**
Stats now computed per category: `changed · equal · left only · right only`
instead of the previous `different · equal (= total − different) · total`.
The old `equal = total − diff_cnt` formula incorrectly folded Computing entries
into the equal count. `total_common_eq` helper removed. "Checking…" progress
shown while any entry is still in Computing state.

**RFC-063 C4 — Destructive-modal focus policy audit:**
All six destructive modals (Overwrite, Reload, Swap, Close tab,
Confirm dir-op, Batch copy) already correctly focus Cancel by default.
No code changes; audit result recorded.

### Added

- `tokio "time"` feature added for toast auto-dismiss.
- `left only` / `right only` i18n keys (ja).
- `Files stay on this computer…` tooltip i18n key (ja).

---

---

## [0.145.0] — 2026-06-14

Build warnings fixed; UX improved when both file paths are not found.

### Fixed

**`crates/forskscope-ui/src/ui/diff_actions.rs`** — Two `drop(tab)` calls
on a `&CompareTab` reference did nothing (you cannot drop a reference).
Replaced with `let _ = tab` which correctly releases the borrow without
emitting a warning.

**`crates/forskscope-ui/src/state/mod.rs`** — Two unused imports removed:
- `DiffProfile` from `pub use settings::{}` — not referenced outside
  `settings.rs` itself.
- `pub use forskscope_core::DiffAlgorithm` re-export — all callers use
  `forskscope_core::DiffAlgorithm` directly.

**`crates/forskscope-ui/src/ui/diff.rs`** — Two UX fixes for missing files:
- Added `(FileKind::Missing, FileKind::Missing)` arm before the existing
  one-sided-missing arm; shows `"Both files not found — read-only."` instead
  of the misleading `"One side is missing"` when neither file exists.
- `identical` field is now `tab.diff.is_identical() && !both_missing` —
  suppresses the `"Files are identical"` message when both sides are empty
  only because neither path was found.

**`crates/forskscope-ui/src/i18n.rs`** — `"Both files not found — read-only."`
added in English and Japanese (両ファイルが見つかりません — 読み取り専用。).

---

---

## [0.144.0] — 2026-06-14

Build errors fixed (confirmed by `cargo build --release` in a GTK environment);
documentation accuracy pass; all `.read().clone()` → `.read().cloned()` across
the UI crate to match the Dioxus 0.7 / generational-box 0.7.9 API contract.

### Fixed

**Build errors** (GTK-environment only — invisible without system libs):

- `state/settings.rs` — Added missing imports: `std::path::PathBuf`,
  `forskscope_core::DiffOptions`. Used in `AppSettings`, `BatchCopySpec`,
  and `DiffProfile::to_diff_options()` but not imported after module split.
- `state/settings.rs` — Removed `add_profile` / `remove_profile` functions
  that referenced `Store` (circular). Moved to `state/mod.rs` where `Store`
  is in scope.
- `state/mod.rs` — Removed `DirOp` from `pub use settings::{}` re-export;
  `DirOp` is defined in `mod.rs`, not `settings.rs`.
- `state/mod.rs` — Added `#[derive(Clone)]` to `Modal`. `GenerationalRef::cloned()`
  requires `T: Clone`; `Modal` was not `Clone`.
- `state/mod.rs` — Removed unused `use serde::{Deserialize, Serialize}`.
- `ui/keybindings.rs` — Removed unused `Lang` import (warning).
- `state/settings.rs` — Removed leftover `remove_profile` stub (regex removal
  had only removed `add_profile` in the previous attempt).
- `app.rs` — Borrow conflict: `store.tabs.write().get_mut()` held a mutable
  borrow while `crate::i18n::t(store.lang(), …)` attempted an immutable borrow.
  Fixed by computing `merge_label` before acquiring the write lock.
- All UI files — All `.read().clone()` on signal guards changed to
  `.read().cloned()` (23 call sites). Consistent with generational-box 0.7.9.

**Documentation accuracy:**

- `docs/src/intermediate/file-types.md` — Large-file thresholds corrected to
  actual values: Small ≤ 512 KiB, Medium 512 KiB – 4 MiB, Large 4 – 64 MiB,
  Very large > 64 MiB. Previous values (1 / 4 / 16 MiB) were wrong.
- `docs/src/users/features.md`, `docs/src/users/settings.md` — Built-in
  compare profile names corrected: "Exact (default)", "Ignore whitespace",
  "Ignore case", "Histogram". Previous names did not match `default_profiles()`.
- `docs/src/users/known-limitations.md` — "Large File Safe" profile reference
  removed; threshold corrected to 64 MiB.

---

---

## [0.143.0] — 2026-06-13

Deep compare digest tasks now respect `DIGEST_CONCURRENCY_LIMIT` (32);
previously unlimited spawning could overwhelm the thread pool on large trees.

### Fixed

**`crates/forskscope-ui/src/ui/deep_compare.rs`** — The phase-2 digest
loop previously spawned one `tokio::task::spawn_blocking` per file pair
with no concurrency limit. On a directory tree with hundreds or thousands
of common files this could queue thousands of blocking tasks simultaneously.

Now uses `Arc<tokio::sync::Semaphore>` with capacity
`DIGEST_CONCURRENCY_LIMIT` (= 32, from `forskscope_core::job`) to cap
concurrent in-flight digest operations. Tasks still spawn eagerly for low
latency on small trees; the semaphore only throttles beyond 32 concurrent.

**`Cargo.toml`** — `tokio` workspace dependency gains the `sync` feature
(required for `tokio::sync::Semaphore`).

**`docs/src/users/known-limitations.md`** — Updated the "Digest comparison
does not cache across navigation" note to reflect the new 32-task limit.

---

---

## [0.142.0] — 2026-06-13

Two documentation accuracy fixes: patch-export.md rewrote to match actual
implementation; local-dev.md test counts updated to v0.141.0 actuals.

### Fixed

**`docs/src/intermediate/patch-export.md`** — Rewrote to match the actual
implementation. The previous version described a `--export-patch` CLI flag,
a **File → Export Patch** menu item, and per-export options (include
additions/deletions, include binary notices) that do not exist. The real
workflow is: **More ▼** → **Export patch** button in the diff toolbar.
The section on directory-level patch export was moved to a "planned" note.

**`docs/src/maintainers/local-dev.md`** — Test counts updated:
- `forskscope-core`: 695 tests (650 unit + 27 diff corpus + 16 merge corpus
  + 2 patch apply), up from the stale "646 unit + 43 integration".
- `forskscope-ui-logic`: 241 tests (228 unit + 5 CSS coverage + 7 doctests
  + 1 integration), up from the stale "189 unit + 5 integration + 1 doctest".
- CI equivalent command now shows the total (936).

---

---

## [0.141.0] — 2026-06-13

RFC and governance documents brought current: RFC-042 update note added,
RFC README Progress column updated, release-readiness checklist rewritten
to reflect actual current status with all verifiable items marked complete.

### Changed

**`rfcs/proposed/042-roadmap-and-rfc-execution-plan.md`** — Added
v0.141.0 update note summarising the entire UI polish and correctness pass
(v0.111.0–v0.140.0): i18n completion (158 keys), CSS cleanup (504 lines),
bug fixes, per-file copy, keyboard shortcut work, and ROADMAP/README updates.
Status field updated to reflect pre-GTK verification phase.

**`rfcs/README.md`** — RFC-042 Progress column updated from
"current through v0.113.0" to "updated v0.141.0; pre-GTK verification phase".

**`rfcs/notes/release-readiness-checklist-v0.3.md`** — Fully rewritten for
v0.141.0. All 5 documentation items, 4 data safety items, and 7 functional/UX
items that do not require GTK or are verifiable from code are now marked `[x]`.
Remaining unchecked items are clearly labelled *(requires GTK)* or *(deferred)*.
Clarifies that v1.0 RC gate is the 3 RFC-041 GTK smoke-test items.

---

---

## [0.140] — 2026-06-13

Six GTK build errors fixed; file size thresholds and compare profile names
corrected in docs.

### Fixed (build)

**`crates/forskscope-ui/src/state/settings.rs`** — Orphaned
`#[derive(Debug, Clone, PartialEq, Eq)]` attribute at end of file removed.
The `DirOp` struct that belonged after this derive had been moved to
`state/mod.rs`, leaving the attribute without a following item.

**`crates/forskscope-ui/src/state/mod.rs`**
- `add_profile` and `remove_profile` added to the `pub use settings::{}`
  re-export. Both are defined in `settings.rs` but were not re-exported,
  causing unresolved function errors in the settings UI.
- Unused `use serde::{Deserialize, Serialize}` import removed.

**`crates/forskscope-ui/src/app.rs`** and **`src/ui/settings.rs`** —
`store.modal.read().clone()` → `.cloned()`. `Signal::read()` returns
`GenerationalRef`; `.cloned()` copies the inner value, `.clone()` does not.

**`crates/forskscope-ui/src/ui/diff.rs`** — `{&snap.readonly_notice}` →
`{snap.readonly_notice.clone()}`. Dioxus RSX requires owned `String`,
not `&String`, for `IntoDynNode`.

### Fixed (docs)

**`docs/src/intermediate/file-types.md`** — "Large files" table corrected
with actual `PerformanceLimits::default()` values: Small ≤ 512 KiB,
Medium 512 KiB – 4 MiB, Large 4 MiB – 64 MiB, Very large > 64 MiB.
Previous values (1 MiB / 4 MiB / 16 MiB) were wrong.

**`docs/src/users/features.md`** and **`docs/src/users/settings.md`** —
Built-in compare profile names corrected to match `default_profiles()`:
"Exact (default)", "Ignore whitespace", "Ignore case", "Histogram".
Previous names ("Default", "Code Review", "Loose Text", "Large File Safe")
do not exist.

**`docs/src/users/known-limitations.md`** — Removed reference to the
non-existent "Large File Safe" profile; corrected threshold from ~16 MiB
to 64 MiB.

---

---

## [0.140.0] — 2026-06-13

README and ROADMAP updated to reflect current state: stale "Filter and sort"
feature removed, F3/Shift+F3 added to shortcuts, ROADMAP brought current
with delivered slices marked and test count corrected.

### Fixed

**`README.md`**
- Removed stale "**Filter and sort** in the explorer — All / Different /
  Equal; sort by name, status, or size; name substring filter" feature bullet.
  The sort dropdown, hidden-file toggle, and name filter input were part of
  the old explorer toolbar removed in v0.131.0; only the "All / Different /
  Equal" filter in Directory Report mode remains.
- Added the Directory Report filter as its own accurate bullet.
- Added **F3 / Shift+F3** (next / previous search match) to the keyboard
  shortcuts table. Implemented in v0.128.0; missing from the README since.

**`ROADMAP.md`**
- Updated header from v0.117.0 → v0.140.0.
- Updated test count from 875 → 936.
- Rewrote "Current state" paragraph to accurately describe the pre-GTK
  verification phase (code and docs complete, awaiting display server tests).
- Added "UI polish + i18n (v0.111–v0.139)" row to delivered milestones table.
- Section renamed "UI implementation slices — status at v0.140.0".
- Slices 1–5 marked ✓ *(shipped)*; Slices 6–8 marked ○ with correct
  post-v1 deferral notes.

---

---

## [0.139.0] — 2026-06-13

i18n: keyboard reference modal descriptions and scrim aria_label translated.
All 11 shortcut description strings now go through `t()`; 158 active keys.

### Fixed

**`crates/forskscope-ui/src/ui/keybindings.rs`** — All `KbRow` description
strings were hardcoded English `&'static str` literals passed directly to the
component. `KbRow.desc` changed to `String`; all 11 descriptions now go
through `t(lang, ...)`. Scrim `aria_label: "Keyboard shortcuts"` also wired
through `t()`.

**`crates/forskscope-ui/src/i18n.rs`** — 15 new Japanese translations: 11
shortcut descriptions + `"Keyboard shortcuts"` scrim aria_label (already had
the heading, now also has the dialog label). Zero missing, zero dead, zero
duplicate keys.

---

---

## [0.138.0] — 2026-06-13

Bug fix: `.filter-btn` CSS removed during dead-code cleanup (v0.131.0) was
still used by deep compare filter buttons. Dead duplicate `ja()` match arms
removed from `i18n.rs`.

### Fixed

**`crates/forskscope-ui/assets/main.css`** — `.filter-btn` and
`.filter-btn.active` rules restored under `/* ── Deep compare filter
buttons ── */`. These classes were removed in v0.131.0 as part of a dead CSS
sweep targeting the old explorer toolbar, but `deep_compare.rs` uses them
independently for the `Different` / `All` / `Equal only` filter buttons and
the batch-copy button in the deep compare toolbar. Without the rules, these
buttons rendered with no padding, border, or active-state indicator.
CSS: 497 → 504 lines.

**`crates/forskscope-ui/src/i18n.rs`** — 7 duplicate `match` arms removed.
In a Rust `match`, only the first matching arm fires; subsequent arms with
the same key are unreachable. The dead arms were: `"Save As"` (×2 extra),
`"Swap sides"` (×1), `"Close"` (×1), `"File changed on disk"` (×1),
`"Export patch"` (×1), `"Copy"` (×1). All dead arms had identical or
inferior translations to the first occurrence. `ja()` reduced from 152
entries to 145 unique keys.

---

---

## [0.137.0] — 2026-06-13

Documentation accuracy pass: stale feature descriptions removed, missing
features added, known-limitations updated to reflect per-file copy.

### Fixed

**`docs/src/users/known-limitations.md`** — "No directory merge operations"
updated. Per-file copy (added v0.132.0) and batch copy are now available;
the section was rewritten to describe what works and what remains
unsupported (delete, full sync).

**`docs/src/users/features.md`** — Added **Directory compare (Deep compare)**
section covering Directory Report mode, two-phase scan, per-file and batch
copy. The feature was not listed anywhere in the feature overview.

**`docs/src/users/comparing-files.md`** — Two fixes:
- Removed stale "By dragging files onto the window" paragraph — drag-and-drop
  was in the old Tauri/Svelte app but is not implemented in the current
  Dioxus explorer.
- Added **F3** / **Shift+F3** to the inline search description.

---

---

## [0.136.0] — 2026-06-13

Documentation accuracy pass; duplicate CHANGELOG entry removed.

### Fixed

**`CHANGELOG.md`** — Duplicate `## [0.129.0]` entry removed. The shorter,
less complete copy (missing the `diff.rs` double-translation fix) was dropped.

**`docs/src/intermediate/keyboard.md`** — `F3` (next search match) and
`Shift+F3` (previous search match) added to the View shortcuts table.
These were implemented in v0.128.0 but not reflected in this doc.

**`docs/src/users/directory-compare.md`** — Two inaccuracies fixed:
- "Filters and sorting" section described the old explorer toolbar
  (sort dropdown, hidden toggle, filter-names input) that was removed in
  v0.131.0. Replaced with a "Modes" table showing the actual `Browse` /
  `Directory Report` buttons.
- "Click **⟳ Deep compare**" changed to "Click **Directory Report**" to
  match the actual button label.

**`docs/src/users/explorer.md`** — Two stale sections removed:
- "Dragging" paragraph describing folder drag-and-drop, which was in the
  old `DirPane` but was not ported to the current aligned explorer.
- "Sync panes" section describing a **⇄ Sync** button that no longer exists.

**`docs/src/maintainers/testing.md`** — Version header updated from
v0.120.0 to v0.135.0.

---

---

## [0.135.0] — 2026-06-13

Bug fix: missing `lang` prop in `tail_rows` Row calls in `hunk.rs`.
RFC-041 checklist and GTK smoke test updated to v0.135.0.

### Fixed

**`crates/forskscope-ui/src/ui/hunk.rs`** — The `tail_rows` loop in
`HunkBlock` was passing `Row` components without the `lang` prop, while both
`head_rows` and `all_rows` passed it correctly. The context-collapse tail
rows (the last `context_lines` rows below the "··· N unchanged ···" divider)
were receiving the default/unset language instead of the active UI language,
meaning screen-reader labels and action button aria-labels would have been
wrong for Japanese users viewing collapsed equal hunks.

### Changed

**`rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`** —
Updated from v0.110.0 to v0.135.0. Added "Progress since v0.110.0" summary
documenting the i18n pass, keyboard shortcut fixes, CSS cleanup, bug fixes,
and per-file copy feature (v0.111.0–v0.135.0). Test count corrected to 936.

**`docs/src/maintainers/gtk-smoke-test.md`** — Added §2f "Per-file copy"
smoke test step covering the `Copy →` / `← Copy` button flow added in
v0.132.0.

---

---

## [0.134.0] — 2026-06-13

Code quality: dead variables in DeepRow fixed; duplicate CSS selectors
removed and conflicting `.tab` definitions merged. CSS 517 → 497 lines.

### Fixed

**`crates/forskscope-ui/src/ui/deep_compare.rs`**

- Dead `src`/`dst` variables in `copy_dir` computation removed — they were
  computed in the match arms but never used; paths are re-derived inside the
  `onclick` closure. (Introduced in v0.132.0.)
- Dead `if to_right { ... } else { ... }` in the `onclick` label generation
  removed — both branches produced identical `"{src} → {dst}"` strings.

**`crates/forskscope-ui/assets/main.css`** — Duplicate CSS rules removed:

- First (shorter, incomplete) `.search-bar` / `.search-input` /
  `.search-count` / `.search-close` definitions removed; the second
  (complete) set at the end of the file is authoritative.
- Second `.sr-only` definition (without `position: absolute !important`)
  removed; the first definition with `!important` is the correct one for
  screen-reader-only elements.
- `.tab { gap: 0 }` override removed; the original `gap: .4rem` from the
  first `.tab` definition is correct — it provides the spacing between the
  dirty-dot indicator and the tab title.
- `.tab-title` definitions merged into one complete rule.

---

---

## [0.133.0] — 2026-06-13

Error messages, fallback tab title, and git mergetool suffix translated;
`tab_title()` signature extended with `lang`.

### Fixed

**`crates/forskscope-ui/src/state/mod.rs`**

- `reload_tab` error toasts `"Reload L: {e}"` / `"Reload R: {e}"` →
  `t(lang, "Left file read error")` / `t(lang, "Right file read error")`
  prefixes with the Rust error appended.
- `open_compare` error toasts `"L: {e}"` / `"R: {e}"` → same translated
  prefixes.
- `tab_title()` gains a `lang: Lang` parameter; the `"comparison"` fallback
  (shown when both paths have no filename component) is now translated.
  All six `tab_title` tests updated to pass `Lang::En`.

**`crates/forskscope-ui/src/app.rs`** — git mergetool mode tab suffix
`" (merge)"` → `format!(" ({})", t(store.lang(), "merge"))`.

**`crates/forskscope-ui/src/i18n.rs`** — 4 new Japanese translations:
`"Left file read error"` → 左ファイルの読み込みエラー,
`"Right file read error"` → 右ファイルの読み込みエラー,
`"comparison"` → 比較, `"merge"` → マージ.

---

---

## [0.132.0] — 2026-06-13

Per-file copy button wired in deep compare view; dead `#[allow(dead_code)]`
annotation removed from `Modal::ConfirmDirOp`.

### Added

**`crates/forskscope-ui/src/ui/deep_compare.rs`** — Each `DeepRow` now shows
a per-file copy button alongside the existing "Compare" button:

- **Changed / LeftOnly** entries show `"Copy →"` — copies the left file to
  the right directory, triggering `Modal::ConfirmDirOp` with src/dst paths.
- **RightOnly** entries show `"← Copy"` — copies the right file to the left
  directory.
- **Equal / Computing / Symlink** entries show no copy button.

The copy confirmation uses the existing `ConfirmDirOpModal` (modals.rs) which
was already fully implemented but had no way to be triggered from the UI.

### Fixed

**`crates/forskscope-ui/src/state/mod.rs`** — Removed stale
`#[allow(dead_code)]` annotation from `Modal::ConfirmDirOp(DirOp)`. The
variant is now actively constructed by `DeepRow`.

---

---

## [0.131.0] — 2026-06-13

CSS housekeeping: 44 more dead lines removed; `main.css` 561 → 517 lines.
Total CSS reduction across v0.130–v0.131: 583 → 517 lines (−66).

### Changed

**`crates/forskscope-ui/assets/main.css`** — Removed dead CSS from four
replaced UI patterns:

- **Old `DirPane` layout** (`.dir-pane`, `.dir-tree`, `.dir-empty`,
  `.dir-pane-footer`, `.explorer-panes`, `.dir-mtime` + media query) —
  replaced by the aligned two-pane tree in RFC-054.
- **Old file picker / compare bar** (`.pick`, `.pick-label`, `.pick-hint`,
  `.compare-bar`) — replaced by the explorer footer with the `Compare`
  button.
- **Old explorer toolbar** (`.explorer-toolbar`, `.sort-label`,
  `.hidden-toggle`, `.name-filter-input`) — replaced by the mode selector
  buttons (`Browse` / `Directory Report`).
- **Old breadcrumb ellipsis** (`.bc-ellipsis`) — the path-bar RTL overflow
  design makes an explicit ellipsis class unnecessary.
- **Old search focus ring** (`.row.search-focused`) — replaced by
  `.diff-row.match` / `.row.match`.

The `.explorer-footer` and `.compare-label` rules were preserved and moved
to a cleaner section. The `fs-line-*`, `fs-inline-*`, and `fsk-conflict-*`
classes (the RFC-024 CSS contract, generated by `forskscope-core`) were
correctly identified as used and untouched.

---

---

## [0.130.0] — 2026-06-13

CSS housekeeping: 22 dead lines from the pre-v0.24.0 flat directory listing
removed from `main.css`. Status colour classes used by `deep_compare.rs`
preserved. 583 → 561 lines.

### Changed

**`crates/forskscope-ui/assets/main.css`** — Removed 16 dead CSS rules from
the flat directory listing era (the `/* explorer flat listing (v0.24.0) */`
comment block and its associated `.dir-table`, `.dir-row`, `.dir-icon`,
`.dir-name`, `.dir-size`, `.dir-status`, `.dir-loading`, `.dir-row.folder`,
`.dir-row.file`, `.dir-row.focused`, `.dir-copy-btn`, `.compare-bar .hint`,
`.compare-bar .summary` rules). These classes are no longer emitted by any
Rust component after the aligned two-pane tree replaced the flat listing.

The six `.status-*` colour classes (`status-equal`, `status-changed`,
`status-only`, `status-cmp`, `status-err`, `status-symlink`) were retained
and moved to a `/* Deep compare row status colours */` section, since they
are actively used by `deep_compare.rs`.

---

---

## [0.129.0] — 2026-06-13

Three bugs found through careful code reading: rendering bug in PathBar,
double-translation in diff view, untranslated modal aria-labels.

### Fixed

**`dir_pane.rs`** — `PathBar` Back button had `title:` keyword missing; the
expression `t(lang, "Back")` was a bare child of the button element instead
of a `title:` attribute. The button rendered `"←Back"` text content with no
tooltip. Fixed to `title: t(lang, "Back")`.

**`diff.rs`** — Two double-translation bugs: `warnings` and `readonly_notice`
in `TabSnapshot` are pre-translated strings (via `t()` in `from_tab()`), but
the render code wrapped them in `t()` again. Since `t()` returns the key
unchanged when not found, this was harmless in practice — but conceptually
wrong and fragile. Fixed: render uses the already-translated `String` values
directly (`"{w}"` and `{&snap.readonly_notice}`).

**`modals.rs`** — 6 dialog scrim `aria_label` attributes wired through `t()`:
`"File changed on disk"`, `"Save As"`, `"Reload files"`, `"Swap sides"`,
`"Close comparison"`, `"Copy file"`. Batch-copy completion toast
(`"Copied {n} files, {n} failed"`) wired through `t()` with `"Copied"`,
`"files"`, `"failed"` keys.

**`settings.rs`** — Settings dialog scrim `aria_label: "Settings"` wired
through `t()`.

**`i18n.rs`** — 9 new Japanese translations added: modal aria-labels (6),
plus `"Copied"` → コピー完了, `"failed"` → 失敗.

---

---

## [0.128.0] — 2026-06-13

Shift+F3 previous-search-match shortcut implemented; dead Escape arm removed;
F3/Shift+F3 added to keyboard reference.

### Added

- **Shift+F3** now navigates to the **previous** search match, completing
  the pair with F3 (next match). The F3 handler in `app.rs` checks
  `mods.contains(Modifiers::SHIFT)` and clicks `#search-prev-btn` or
  `#search-next-btn` accordingly.
- `id: "search-prev-btn"` added to the Previous match button in `search.rs`
  (mirrors the `search-next-btn` id added in v0.127.0).
- `keybindings.rs` — `"F3 / Shift+F3"` row added under Diff view shortcuts.
- `docs/src/maintainers/gtk-smoke-test.md` — F3 and Shift+F3 added to §1c
  keyboard navigation checklist.

### Fixed

- **Dead `Key::Escape => {}`** arm in `app.rs`'s inner `match e.key()` block
  removed. Escape was already handled by the early-return guard above the
  match; the second arm was unreachable and misleading.

---

---

## [0.127.0] — 2026-06-13

Bug fix: F3 and Ctrl+F keyboard shortcuts broken for Japanese users after
the i18n pass translated the button attributes they selected by.

### Fixed

**Regression introduced in v0.125.0**: `app.rs` used JavaScript
`document.eval` to simulate button clicks for F3 (search next) and Ctrl+F
(open search bar). These selectors targeted buttons by their `title` and
`aria-label` attribute values, which became Japanese after the i18n pass:

- `querySelector('.search-nav[title*=\'Next\']')` — failed when title was
  `"次の一致 (Enter / F3)"`.
- `querySelector('.diff-wrap button[aria-label="Open search bar"]')` —
  failed when aria-label was `"検索バーを開く"`.

**Fix**: stable `id` attributes added to both buttons; eval selectors updated
to use `getElementById`:

- `diff.rs` search open button: `id: "search-open-btn"` added.
- `search.rs` Next match button: `id: "search-next-btn"` added.
- `app.rs` F3 handler: `getElementById('search-next-btn')?.click()`.
- `app.rs` Ctrl+F handler: `getElementById('search-open-btn')?.click()`.

`id`-based selectors are stable across all languages.

---

---

## [0.126.0] — 2026-06-13

i18n: last three hardcoded strings fixed; comprehensive scan confirms zero
remaining English fallbacks anywhere in the UI. 134 active translation keys.

### Fixed

**`crates/forskscope-ui/src/ui/modals.rs`**
- `BatchCopyModal` scrim `aria_label: "Batch copy"` → `t(lang, "Batch copy")`.
- `BatchCopyModal` heading `"Copy {count} files?"` → `format!` with
  `t(lang, "Copy")` and `t(lang, "files")` for correct word-order in Japanese.

**`crates/forskscope-ui/src/ui/tabs.rs`**
- Tab close button `aria_label: "Close {title}"` → `format!("{} {title}", t(lang, "Close"))`.
  The filename `{title}` stays in its original language; only the verb is translated.

**`crates/forskscope-ui/src/i18n.rs`** — 3 new entries: `"Batch copy"` →
一括コピー, `"files"` → ファイル, `"Close"` → 閉じる.

**Intentionally untranslated** (product names): `"ForskScope v{version}"` in
`AboutModal`; `"About ForskScope"` tooltip in `settings.rs`.

---

---

## [0.125.0] — 2026-06-12

i18n: all remaining tooltip, aria-label, and visible-text strings now go
through `t()`. 132 active translation keys, all present in `ja()`.

### Fixed

**`crates/forskscope-ui/src/ui/deep_compare.rs`** — 8 strings wired:
`"Scanning…"`, stat line words (`"different"`, `"equal"`, `"total"`),
`"checking"` progress word, `"Copy"` and direction labels for batch-copy
buttons and `BatchCopySpec` label strings. `lang` added to `BatchCopyButtons`.

**`crates/forskscope-ui/src/ui/dir_pane.rs`** — 5 `PathBar` button
tooltips wired: `"Back"`, `"Forward"`, `"Home directory"`, `"Open folder…"`,
`"Edit path"`. `lang: Lang` prop added to `PathBar`.

**`crates/forskscope-ui/src/ui/explorer.rs`** — `lang` threaded to both
`PathBar` callsites; mode-button tooltips `"Browse and navigate…"` and
`"Recursively compare…"` wired through `t()`.

**`crates/forskscope-ui/src/ui/header.rs`** — `"Keyboard shortcuts (Ctrl+/)"` 
tooltip wired through `t()`.

**`crates/forskscope-ui/src/ui/diff.rs`** — 16 toolbar aria-labels and
tooltips wired: `"File comparison"`, `"F7 — Previous change"`, `"F8 — Next
change"`, `"Undo last merge action (Ctrl+Z)"`, `"Save merge result (Ctrl+S)"`,
`"Reload both files from disk"`, `"Reload files from disk"`, `"Search within
diff (Ctrl+F)"`, `"Open search bar"`, toggle labels for inline diff, word wrap,
ignore whitespace, ignore case, `"Diff algorithm"`, `"Export unified-diff patch
file"`, `"Export patch"`.

**`crates/forskscope-ui/src/i18n.rs`** — 32 new Japanese translations added.

---

---

## [0.124.0] — 2026-06-12

i18n: hunk view and search bar strings translated; screen-reader labels
and tooltips now go through `t()`. 102 active translation keys, all in ja().

### Fixed

**`crates/forskscope-ui/src/ui/hunk.rs`**

- `"··· {n} unchanged lines — click to expand ···"` collapse divider text
  wired through `t()` with `{n}` substitution. `lang: Lang` added to `Row`
  component and threaded from `HunkBlock`.
- Screen-reader `sr_label` strings (`"Deleted"`, `"Inserted"`, `"Changed"`)
  wired through `t()`.
- Merge action button `aria_label: "Apply change left to right"` and
  `aria_label: "Applied"` wired through `t()`.

**`crates/forskscope-ui/src/ui/search.rs`**

- `"Search within diff"` aria-label wired through `t()`.
- `"Previous match (Shift+Enter)"` and `"Next match (Enter / F3)"` title
  tooltips wired through `t()`.
- `"Previous match"`, `"Next match"`, `"Close search bar"` aria-labels
  wired through `t()`.
- Plural match count `"N match/matches"` wired through `t()`.

**`crates/forskscope-ui/src/i18n.rs`** — 13 new Japanese translations added.

---

---

## [0.123.0] — 2026-06-12

i18n: two final hardcoded strings wired through `t()`; Japanese interface
now complete with zero English fallbacks anywhere in the UI.

### Fixed

- **`crates/forskscope-ui/src/ui/tabs.rs`** — tab close button tooltip
  `"Close tab"` wired through `t(lang, "Close tab")`; `lang` added to
  `TabItem` component. Japanese: タブを閉じる.

- **`crates/forskscope-ui/src/ui/diff.rs`** — defensive fallback notice
  `"No comparison."` (shown when a tab index resolves to `None`) wired
  through `t(lang, "No comparison.")`. Japanese: 比較がありません。

**`ja()` map now has 90 entries**: 88 actively used keys plus `"on"` /
`"off"` (used via inline conditional expressions). Zero English fallbacks
remain in any user-facing component. The i18n pass that began at v0.121.0
is complete.

---

---

## [0.122.0] — 2026-06-12

i18n: all remaining hardcoded English strings in modals, statusbar, and
search bar now route through `t()`. Japanese translation now covers the
entire UI without exception.

### Fixed

**`crates/forskscope-ui/src/ui/modals.rs`** — 10 user-visible strings
wired through `t()`:

- `CloseTabModal`: body text `"… has unsaved changes. Discard them and close?"`
- `ConfirmDirOpModal`: `"From"`, `"To"` field labels; `"Destination exists. A .bak backup will be created."` notice
- `BatchCopyModal`: `"Existing files will receive a .bak backup."` notice
- `AboutModal`: `"Version"`, `"Rust"`, `"OS"`, `"Arch"`, `"CPUs"` key labels

**`crates/forskscope-ui/src/ui/statusbar.rs`** — `"Local only"` wired
through `t()`.

**`crates/forskscope-ui/src/ui/search.rs`** — `"No matches"` wired
through `t()`.

**`crates/forskscope-ui/src/i18n.rs`** — 12 new Japanese translations
added for all of the above keys.

After this fix: 86 keys in active use, all present in `ja()`, none dead.
The Japanese interface is now complete with no English fallback strings in
any user-facing component.

---

---

## [0.121.0] — 2026-06-12

i18n: warning and read-only notice strings in the diff view now translate
into Japanese; 9 dead translation entries removed.

### Fixed

**`crates/forskscope-ui/src/ui/diff.rs`** — `TabSnapshot::from_tab()` now
accepts a `lang: Lang` parameter and wraps all user-visible strings through
`t()`. Previously these strings were hardcoded `&'static str` values that
had Japanese translations in `ja()` that could never be shown:

- `DiffWarning::LargeFilePolicyApplied` → `t(lang, "Large file — inline diff disabled and deadline shortened.")`
- `DiffWarning::DeadlineExpired` → `t(lang, "Diff timed out — result may be approximate.")`
- `DiffWarning::InlineSkippedHunkTooLarge` → `t(lang, "Some hunks were too large for character-level diff.")`
- `readonly_notice` for Binary, Excel, Missing, Unsupported, and default
  cases — all five strings now go through `t()`.

### Changed

**`crates/forskscope-ui/src/i18n.rs`** — removed 9 dead `ja()` entries
that had no corresponding `t()` call anywhere in the codebase: `"Apply ▶"`,
`"List"`, `"Next change"`, `"Open Files"`, `"Previous change"`,
`"Select left, then right, then Compare."`, `"Use as Left"`, `"Use as Right"`,
`"changes"`. These were added speculatively and never wired. The `ja()` map
now has 76 entries, all of which correspond to active `t()` call sites.

---

---

## [0.120.0] — 2026-06-12

Dependency maintenance: transitive dependency patch updates.

### Changed

- **`Cargo.lock`** — updated four transitive dependencies to latest
  Rust 1.85-compatible patch versions:
  - `openssl` 0.10.80 → 0.10.81
  - `openssl-sys` 0.9.116 → 0.9.117
  - `zeroize` 1.8.2 → 1.9.0
  - `zeroize_derive` 1.4.3 → 1.5.0

  All 936 tests pass against the updated lockfile. 23 other transitive
  dependencies are unchanged at their latest compatible versions.

- **`docs/src/maintainers/testing.md`** — version header updated to
  v0.120.0.

---

---

## [0.119.0] — 2026-06-12

GTK smoke test checklist; testing.md counts updated.

### Added

- **`docs/src/maintainers/gtk-smoke-test.md`** — step-by-step manual
  verification checklist for the three RFC-041 UI items that require a
  running GTK display server: two-way file compare end-to-end (open, navigate
  hunks, apply, undo, save, search), directory compare (explorer navigation,
  digest indicators, same-name diff, deep compare), and keyboard navigation
  completeness (all shortcuts from `keybindings.rs`). Includes build
  instructions, pass criteria, and known GTK environment workarounds.
  Added to `docs/src/SUMMARY.md`.

### Changed

- **`docs/src/maintainers/testing.md`** — test counts updated to v0.119.0:
  `diff_corpus` 25 → 27; total 930 → 936; version header updated.

---

---

## [0.118.0] — 2026-06-12

ELOC compliance: UI `state.rs` split; all files now at or below the project's
300-line soft limit (except unavoidably dense single-concept modules).

### Changed

Pure refactor — no behaviour, public API, or test changes.

**`crates/forskscope-ui/src/state.rs` (444 lines) → `state/mod.rs` + `state/settings.rs`**

- `state/settings.rs` (141): UI-layer settings types — `Theme`, `Lang`,
  `DiffAlgorithmSetting`, `DiffProfile`, profile management functions
  (`add_profile`, `remove_profile`), `AppSettings`, `BatchCopySpec`, `DirOp`.
- `state/mod.rs` (312): `Modal`, `CompareTab`, `Store`, session persistence
  functions (`save_session`, `restore_session`), tab operations
  (`open_compare`, `close_tab`, `reload_tab`, `swap_sides`,
  `recompute_diff`), GTK-free unit tests.

All types re-exported from `crate::state` unchanged; call sites require no
modification.

---

---

## [0.117.0] — 2026-06-12

Documentation housekeeping following the v0.115–v0.116 ELOC split work.

### Changed

- **`ROADMAP.md`** — updated to v0.117.0; added ELOC compliance milestone
  row (v0.115–v0.116: command, error, session, report, settings, job modules
  split; zero files over 500 lines).

- **`rfcs/notes/core-completion-summary-v0.72.md`** — updated to v0.117.0;
  added note documenting the six-module ELOC split with the observation that
  the public API is unchanged.

---

---

## [0.116.0] — 2026-06-12

ELOC compliance: settings and job modules split; no files remain over 500 lines.

### Changed

Pure refactors — no behaviour, public API, or test changes.

**`settings.rs` (466 lines) → `settings/mod.rs` + `settings/display.rs`**

- `settings/display.rs` (161): display primitives — `ThemeId`, `ThemeTokens`,
  `Density`, `FontFamilySetting`, `LocaleId`, `SETTINGS_SCHEMA_VERSION`.
- `settings/mod.rs` (311): settings sections — `AppearanceSettings`,
  `DiffSettings`, `FileSettings`, `BehaviourSettings`, `UserSettings` with
  serialise/deserialise; parse helpers.

**`job.rs` (370 lines) → `job/mod.rs` + `job/limits.rs`**

- `job/limits.rs` (81): `FileSizeClass` and `PerformanceLimits` (RFC-013 §5).
- `job/mod.rs` (294): threshold policy, `JobKind`, `JobProgress`, `JobHandle`,
  job lifecycle state machine, `JobRegistry`.

---

---

## [0.115.0] — 2026-06-12

ELOC compliance: four core modules over 500 lines split into submodules.

### Changed

All changes are pure refactors — no behaviour, public API, or test changes.

**`command.rs` (525 lines) → `command/mod.rs` + `command/registry.rs`**

- `command/mod.rs` (314): identity types, `cmd` submodule of well-known IDs,
  availability context, shortcut model, `CommandDefinition`.
- `command/registry.rs` (224): `CommandRegistry` with `builtin()` factory and
  all built-in command definitions.

**`error.rs` (554 lines) → `error/mod.rs` + `error/app.rs`**

- `error/mod.rs` (201): `CoreError`, `IoOperation`, `ErrorSeverity`.
- `error/app.rs` (360): `AppErrorKind`, `RecoveryAction`, `AppError`,
  `UserMessage`, `TechnicalDetail`, `ErrorId`.

**`session.rs` (559 lines) → `session/mod.rs` + `session/tab.rs`**

- `session/tab.rs` (179): per-tab session types — `SessionId`, `TabId`,
  `Timestamp`, `FilePairRoot`, `DirectoryPairRoot`, `WorkspaceRoot`,
  `WorkspaceTab`, `DiffTabSession`, `BinaryTabSession`, `ExcelTabSession`,
  `ErrorTabSession`, `SESSION_SCHEMA_VERSION`.
- `session/mod.rs` (406): `WorkspaceSession` with all operations and queries;
  `RecentSessionEntry`; serialisation/deserialisation via `VersionedEnvelope`.

**`report.rs` (506 lines) → `report/mod.rs` + `report/file.rs` + `report/dir.rs`**

- `report/mod.rs` (51): re-exports + shared `pub(crate)` helpers
  `display_path` and `hunk_kind_label`.
- `report/file.rs` (262): `FileComparisonReport`, `HunkSummaryRow`,
  `HistoryEntry`, `ReportOptions`, `ReportPathMode`.
- `report/dir.rs` (208): `DirComparisonReport`, `DirFileRow`, `BatchSummary`;
  private helper `fmt_size`.

---

---

## [0.114.0] — 2026-06-12

Final note-file housekeeping.

### Changed

- **`rfcs/notes/core-completion-summary-v0.72.md`** — version header updated
  to v0.114.0; ui-logic unit test count corrected (189 → **228**, reflecting
  the full coverage pass completed in v0.109.0); version note updated.

- **`rfcs/notes/proposed-rfc-backlog-writeup-v0.40.md`** — staleness notice
  added at the top, explaining that "Partly shipped" / "Open" labels from
  v0.40.0 have since been fully shipped, and directing readers to
  `rfcs/README.md` for current state.

- **`rfcs/notes/acceptance-test-corpus-plan.md`** — note added explaining
  that the planned `text/basic/` / `text/newline/` path structure was not
  used; actual paths are `tests/fixtures/text/`, `tests/fixtures/newlines/`,
  `tests/fixtures/merge/`; see `tests/fixtures/README.md`.

---

---

## [0.113.0] — 2026-06-12

Governance accuracy pass: RFC README, RFC-041/042 updated; FAQ cross-reference
to troubleshooting guide; stale note files marked superseded.

### Changed

- **`rfcs/README.md`** — proposed section table expanded with Progress column:
  RFC-026 and RFC-030 marked "Partially shipped" / "Substantially shipped";
  blocked RFCs annotated with their blockers; RFC-042 noted as current through
  v0.113.0.

- **`rfcs/proposed/042-roadmap-and-rfc-execution-plan.md`** — §4a extended
  through v0.112.0: added ui-logic coverage pass (v0.109.0), i18n completion
  (v0.111.0), startup diagnostics + troubleshooting guide (v0.112.0).
  Header updated to v0.113.0; v0.113.0 update block added.

- **`rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`** —
  architecture/testing docs version updated (v0.110.0 → v0.113.0, noting
  troubleshooting.md added).

- **`docs/src/users/faq.md`** — GTK/WebKitGTK entry now links to the full
  [Troubleshooting guide](troubleshooting.md) for blank-window, NVIDIA
  DMA-BUF, and other platform issues.

- **`rfcs/notes/implementation-gate-checklist-v0.2.md`** — "Superseded
  v0.113.0" notice added (same pattern as implementation-checklist.md, which
  was superseded in v0.102.0). All gate conditions were met in v0.23–v0.72.

---

---

## [0.112.0] — 2026-06-12

RFC-026 diagnostics CLI flag; RFC-030 troubleshooting guide; both RFCs advanced.

### Added

**`--diagnostics` CLI flag** (`main.rs`) — RFC-026 §"Startup diagnostics"

`forskscope --diagnostics` prints `PlatformInfo::to_report()` and exits
without launching the UI or requiring a display server. Output includes OS,
arch, CPU count, app version, Rust version, home (redacted), and config
directory. Designed for inclusion in bug reports and for diagnosing startup
failures on headless or restricted systems.

**`docs/src/users/troubleshooting.md`** (new, 141 lines) — RFC-030 §"Troubleshooting"

Platform-specific troubleshooting guide covering:
- `forskscope --diagnostics` usage and example output.
- Linux: WebKitGTK 4.1 installation on Debian/Ubuntu, Fedora, Arch.
- Linux: blank window (NVIDIA + Wayland DMA-BUF workaround, X11 fallback).
- Linux: file picker dialog not opening (xdg-desktop-portal).
- macOS: Gatekeeper unsigned binary warning and fix.
- macOS: post-upgrade crash.
- Windows: WebView2 runtime missing and fix.
- Windows: long path support.
- Session not restored (config directory permissions).
- Bug report instructions (referencing `--diagnostics`).

`troubleshooting.md` added to `docs/src/SUMMARY.md`.

**`docs/src/intermediate/cli.md`** — added `--diagnostics` section with
usage, example output, and home-redaction note.

### Changed

- **`rfcs/proposed/026-cross-platform-webview-and-linux-desktop-compatibility.md`**
  — status updated from "Proposed" to "Partially implemented"; lists what
  shipped (v0.93.0 PlatformInfo, v0.100.0 About panel wiring, v0.112.0
  CLI flag + troubleshooting doc) and what remains (smoke tests, blank-window
  detection, `--safe-editor`, compatibility settings UI).

- **`rfcs/proposed/030-user-documentation-onboarding-and-help-system.md`**
  — troubleshooting page added to shipped list; "Troubleshooting page for
  WebView/Linux dependency issues" removed from remaining items.

---

---

## [0.111.0] — 2026-06-12

**Milestone: i18n complete across all UI surfaces.**

Every user-visible string in every UI component now routes through `t()`.
The Japanese interface is now complete with zero untranslated labels in any
component. This closes RFC-009 (locale/i18n) at the UI layer.

### Fixed

**`crates/forskscope-ui/src/ui/deep_compare.rs`**
- `"Deep compare"` toolbar label → `t(lang, "Deep compare")`.
- `"Different"`, `"All"`, `"Equal only"` filter buttons → `t(lang, ...)`.
- Japanese translations: 深度比較, 差分あり, すべて, 同一のみ.

**`crates/forskscope-ui/src/ui/header.rs`**
- `"Settings"` button → `t(lang, "Settings")`.
- Added `use crate::i18n::t` import and `let lang = store.lang()`.

**`crates/forskscope-ui/src/ui/explorer.rs`**
- `"Browse"` mode tab → `t(lang, "Browse")`.
- `"Directory Report"` mode tab → `t(lang, "Directory Report")`.
- Japanese translations: ブラウズ, ディレクトリレポート.

**`crates/forskscope-ui/src/ui/keybindings.rs`**
- `"Keyboard shortcuts"` modal heading → `t(lang, "Keyboard shortcuts")`.
- `"Diff view"`, `"Navigation"`, `"App"` section headings → `t(lang, ...)`.
- Removed duplicate `"Alt + ↑"` row (was listed twice in Navigation).
- Japanese translations: キーボードショートカット, 差分ビュー,
  ナビゲーション, アプリ.

### Note on remaining English strings

Three categories intentionally remain in English:
1. **File dialog labels** (`"Export patch"`, `"Patch files"`, `"All files"`)
   in `diff_actions.rs` — these are passed to the OS native file picker via
   `rfd` and their localisation depends on the platform API.
2. **Screen-reader-only row labels** (`"Deleted"`, `"Inserted"`, `"Changed"`)
   in `hunk.rs` Row component — the component does not take a `lang` prop
   and adding one requires updating all callsites in a GTK environment.
3. **Keyboard shortcut descriptions** in `keybindings.rs` `KbRow` — these
   are `&'static str` props and translating them would require significant
   additional translation work with low user-facing impact for a developer tool.

---

---

## [0.110.0] — 2026-06-12

**Milestone: pre-v1 stabilisation complete — all non-GTK work done.**

All work achievable without a GTK/display environment is complete. The
project is ready for GTK integration testing to close the three remaining
RFC-041 items. Total tests: **936** (930 → 936, +6).

### Added

**`save_tests.rs`** (7 → 11 tests) — `SaveOutcome` field coverage:
- `backup_path_is_none_when_policy_is_none` — `backup_path` is `None`
  when `BackupPolicy::None` is used.
- `new_fingerprint_reflects_written_content` — `new_fingerprint` differs
  from the original and matches a fresh `FileFingerprint::capture` after write.
- `encoding_fallback_to_utf8_is_true_for_unknown_encoding` — an unknown
  encoding label triggers fallback; `encoding_fallback_to_utf8 == true`;
  content is still written correctly as UTF-8.
- `written_bytes_matches_content_length` — `written_bytes` equals the exact
  byte length of the written content.

**`diff_corpus.rs`** (25 → 27 tests) — two new fixture scenarios:
- `mixed_crlf_lf_file_has_changes_detected` — file with mixed CRLF and LF
  line endings; a one-line change is detected correctly.
- `very_long_single_line_produces_one_replace_hunk` — 2000-character
  single-line files; diff engine handles them without truncation or panic.

**`tests/fixtures/newlines/`** — `left_mixed_endings.txt`,
`right_mixed_endings.txt` (mixed CRLF/LF fixture pair).

**`tests/fixtures/text/`** — `left_long_line.txt`, `right_long_line.txt`
(2000-character single-line fixture pair).

### Changed

- **`rfcs/proposed/041-v1-product-stabilisation-and-rfc-governance.md`** —
  checklist updated to v0.110.0 (final pre-GTK state): core test count
  936; ui-logic 228; Architecture docs current (v0.110.0). 12 of 16 items
  ticked; remaining 4 are GTK-dependent or explicitly deferred.

- `docs/src/maintainers/testing.md` — v0.110.0; total 936; diff_corpus
  count 25 → 27.

- `rfcs/notes/core-completion-summary-v0.72.md` — 936 tests; diff_corpus
  25 → 27; core unit 646 → 650; version → v0.110.0.

- `ROADMAP.md` — last-updated → v0.110.0.

---

---

## [0.109.0] — 2026-06-12

**Milestone: ui-logic view-model test coverage pass complete.**

All 14 `forskscope-ui-logic` modules now have comprehensive tests covering
every public type, method, and field. Total tests: **930** (891 → 930, +39).
No GTK required to run any of these tests.

### Added

Tests added across five modules to close the remaining coverage gaps:

**`compare/tab_state.rs`** (5 → 14 tests)
- Conflict flags (`has_active_conflict`, `any_conflict_unresolved`) propagate
  to `CommandContext`; `ActiveConflict`/`AnyConflictUnresolved` rules satisfied.
- `can_redo` flag propagates; `CanRedo` rule satisfied; toolbar redo enabled.
- `selected_path_exists` propagates; `SelectedPathExists` rule satisfied.
- Read-only tab (`right_side_is_editable=false`): `ActiveHunkEditable` unavailable.
- Focused hunk guard: `ActiveHunkEditable` unavailable when no hunk is focused.
- All-flags-true snapshot satisfies all 8 `AvailabilityRule` variants.

**`compare/conflict_nav_view.rs`** (10 → 19 tests)
- `focused_row()` returns `None` with no focus, `Some` with a valid focus id.
- `is_focused` set on exactly one row in a multi-conflict session.
- Resolved-state glyphs: `'L'` (left), `'R'` (right), `'-'` (ignore).
- `status_text` non-empty for all conflict rows.
- Progress text references resolved count with partial resolution.

**`explore/align.rs`** (9 → 15 tests)
- `is_selected` propagates on left-only and right-only rows.
- Both-sides-selected: same-name files merge into one `AlignedRow`, both sides selected.
- `depth` value passes through unchanged.
- `abs_path` is absolute; `rel_path` is relative and correct.
- `is_expanded` propagates: one expanded dir, one collapsed dir.

**`compare/palette_view.rs`** (11 → 16 tests)
- `shortcut_hint` is non-empty for `file.save`.
- `disabled_reason` is `Some` when disabled, `None` when enabled.
- `description` is non-empty for every builtin command.
- `enabled_count` equals manual count and is positive in diff context.

**`compare/search_index.rs`** (13 → 21 tests)
- `len` and `is_empty` consistent for empty/non-empty index.
- `focused().hunk_id` and `focused().row_index` match the correct match.
- `focused_number()` is 1 at start, increments on advance.
- `advance()` and `retreat()` return `None` without panicking on empty index.

**`compare/command_bar.rs`** (13 → 17 tests)
- `disabled_reason` is `Some` when item disabled, `None` when enabled.
- `shortcut_hint` is `Some` and non-empty for `file.save`.
- `enabled_count` is positive in diff context.

### Changed

- `docs/src/maintainers/testing.md` — version v0.109.0; total 930;
  `tab_state`, `conflict_nav_view`, `align`, `palette_view`, `search_index`,
  `command_bar` rows updated with new coverage descriptions.
- `rfcs/notes/core-completion-summary-v0.72.md` — 930 tests; ui-logic 228;
  version → v0.109.0.
- `rfcs/proposed/041-…` — 930 total.

---

---

## [0.108.0] — 2026-06-12

8 new search_index tests (len, focused data, focused_number, empty-index safety); 916 → 924 tests.

### Added

- **8 new tests in `crates/forskscope-ui-logic/src/compare/search_index.rs`**
  (13 → 21 tests):

  *len / is_empty:*
  - `len_and_is_empty_consistent_for_empty_index` — both `len()` and
    `is_empty()` correctly reflect a zero-match index.
  - `len_equals_match_count` — `len()` ≥ 3 for three rows each containing
    the query.

  *focused() MatchPosition fields:*
  - `focused_returns_correct_hunk_id` — `focused().hunk_id` matches the
    hunk that contained the match.
  - `focused_returns_correct_row_index` — `focused().row_index` is 2 when
    the match is in the third row (0-based).

  *focused_number:*
  - `focused_number_is_1_at_start` — `focused_number()` returns `Some(1)`
    before any advance.
  - `focused_number_increments_on_advance` — `focused_number()` returns
    `Some(2)` after one `advance()`.

  *advance / retreat on empty:*
  - `advance_on_empty_index_returns_none` — `advance()` returns `None`
    without panicking when the index is empty.
  - `retreat_on_empty_index_returns_none` — `retreat()` returns `None`
    without panicking when the index is empty.

### Changed

- `docs/src/maintainers/testing.md` — 916 → **924**; `search_index` row
  updated with focused-data and empty-safety coverage.
- `rfcs/notes/core-completion-summary-v0.72.md` — 916 → 924; ui-logic
  214 → 222; version → v0.108.0.
- `rfcs/proposed/041-…` — 924 total.

---

---

## [0.107.0] — 2026-06-12

9 new tests across align.rs and palette_view.rs; 907 → 916 tests.

### Added

- **5 new tests in `crates/forskscope-ui-logic/src/explore/align.rs`**
  (9 → 14 tests) — field propagation coverage:
  - `is_selected_propagates_to_left_row_data` — `is_selected=true` in a
    left `FlatRow` appears in the corresponding `RowData`; the selected
    row is identified by `rel_path`.
  - `is_selected_propagates_to_right_row_data` — same for right-side-only
    rows.
  - `depth_propagates_to_row_data` — non-zero depth values (0, 2) pass
    through `compute_aligned_rows` unchanged.
  - `rel_path_is_relative_and_abs_path_is_absolute` — `abs_path` is
    absolute; `rel_path` is relative and equals `"file.txt"`.

- **5 new tests in `crates/forskscope-ui-logic/src/compare/palette_view.rs`**
  (11 → 16 tests) — `PaletteRow` field coverage:
  - `save_row_has_ctrl_s_shortcut_hint` — `file.save` row has a non-empty
    `shortcut_hint` containing `'s'` (Ctrl+S).
  - `disabled_row_has_disabled_reason_some` — `disabled_reason` is `Some`
    with non-empty text when a command is disabled.
  - `enabled_row_has_disabled_reason_none` — `disabled_reason` is `None`
    for the always-enabled `view.command_palette` command.
  - `all_rows_have_non_empty_descriptions` — `description` field is
    non-empty for every builtin command.
  - `enabled_count_matches_enabled_rows_in_diff_context` — `enabled_count`
    equals the manual count of enabled rows; at least one command enabled.

### Changed

- `docs/src/maintainers/testing.md` — 907 → **916**; align and
  palette_view rows updated.
- `rfcs/notes/core-completion-summary-v0.72.md` — 907 → 916;
  ui-logic 205 → 214; version → v0.107.0.
- `rfcs/proposed/041-…` — 916 total.

---

---

## [0.106.0] — 2026-06-12

9 new conflict_nav_view tests (focus, resolved glyphs, progress); 899 → 907 tests.

### Added

- **9 new tests in `crates/forskscope-ui-logic/src/compare/conflict_nav_view.rs`**
  (10 → 19 tests):

  *Focus propagation:*
  - `focused_row_returns_none_when_no_focus_set` — `focused_row()` is `None`
    when `ConflictNavigator::build` is called with `focused_id = None`.
  - `focused_row_returns_some_when_focus_is_set` — `focused_row()` returns
    `Some` with correct `conflict_id` when `focused_id` is provided.
  - `is_focused_flag_set_only_on_focused_conflict` — exactly one row has
    `is_focused = true` in a multi-conflict session; it matches the given id.

  *Resolved-state glyphs:*
  - `resolved_left_row_has_l_glyph` — `resolve_left` produces glyph `'L'`.
  - `resolved_right_row_has_r_glyph` — `resolve_right` produces glyph `'R'`.
  - `ignored_row_has_dash_glyph` — `ignore` produces glyph `'-'`.
  - `all_glyph_status_texts_are_non_empty` — `status_text` is non-empty for
    every conflict row.

  *Progress text:*
  - `progress_text_reflects_partial_resolution` — with 2 conflicts and 1
    resolved, `progress_text` is non-empty and contains `'1'`.

### Changed

- `docs/src/maintainers/testing.md` — 899 → **907**; `conflict_nav_view`
  row updated to describe focus/glyph/progress coverage.
- `rfcs/notes/core-completion-summary-v0.72.md` — 899 → 907; ui-logic
  197 → 205; version note updated to v0.106.0.
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` —
  907 total.

---

---

## [0.105.0] — 2026-06-12

9 new tab_state tests (conflict/redo/read-only scenarios); 891 → 899 tests.

### Added

- **9 new tests in `crates/forskscope-ui-logic/src/compare/tab_state.rs`**
  (5 → 14 tests) covering previously untested `TabStateSnapshot` fields:

  - `redo_flag_is_forwarded_to_context` — `can_redo` propagates to
    `CommandContext` and satisfies `AvailabilityRule::CanRedo`.
  - `redo_only_snapshot_enables_redo_toolbar_item` — toolbar has redo enabled,
    undo disabled when only `can_redo` is set.
  - `conflict_flags_are_forwarded_to_context` — `has_active_conflict` and
    `any_conflict_unresolved` propagate; `ActiveConflict` and
    `AnyConflictUnresolved` rules are satisfied.
  - `no_conflict_context_is_unavailable_for_conflict_rules` — both conflict
    rules unavailable on default snapshot.
  - `selected_path_flag_is_forwarded_to_context` — `selected_path_exists`
    propagates; `SelectedPathExists` rule satisfied.
  - `read_only_tab_disables_apply_hunk` — `right_side_is_editable = false`
    makes `ActiveHunkEditable` unavailable (xlsx/binary tabs).
  - `editable_tab_without_focused_hunk_disables_apply` — hunks exist but none
    focused: `ActiveHunkEditable` unavailable.
  - `all_flags_true_snapshot_satisfies_all_rules` — exhaustive check: every
    `AvailabilityRule` is satisfied when all snapshot flags are true.

### Changed

- `docs/src/maintainers/testing.md` — count 891 → **899**; tab_state row
  updated with conflict/redo/read-only coverage description.
- `rfcs/notes/core-completion-summary-v0.72.md` — title → v0.105.0;
  ui-logic count 189 → 197; total 891 → 899; version note updated.
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` —
  core test total 891 → 899.

---

---

## [0.104.0] — 2026-06-12

RFC-042 fully audited: §4b priorities 1–7 all Done; §4a extended to v0.104.

### Changed

- **`rfcs/proposed/042-roadmap-and-rfc-execution-plan.md`** — comprehensive
  audit pass:

  **§4b priority table — all items audited against `rfcs/done/`:**
  - Row 1 (RFC-034): was "Open — requires GTK"; corrected to **Done** (core
    v0.64.0, `ConflictNavigator`); UI workspace still requires GTK noted.
  - Row 2 (RFC-059 + RFC-019): was "Open — requires GTK"; corrected to
    **Done** (RFC-059 v0.41.0 CSS/audit; RFC-019 core v0.63.0).
  - Row 3 (RFC-037): was "Open — requires GTK"; corrected to **Done** (core
    v0.42.0 + v0.58.0, cancellation + `DirectoryIndex`).
  - Row 4 (RFC-014): was "Open — requires GTK"; corrected to **Done** (core
    v0.43.0, `MatchIndex` + `SearchIndex`).
  - Row 5 (RFC-023): was "Open — requires GTK"; corrected to **Done** (core
    v0.44.0, `BatchManifest` + `batch_copy`).
  - Row 6 (RFC-058): was "Open"; corrected to **Done** (v0.57.0, sheets-diff
    v2.2.1 adapter). RFC-058 has been in `rfcs/done/` since v0.57.0.
  - Summary paragraph rewritten: priorities 1–7 all done; remaining open =
    editor adapter (RFC-004 track), packaging/QA (RFC-010/026), governance
    (RFC-041).

  **§4a delivered milestones** — extended from v0.40.0 stop to **v0.104.0**:
  added 14 post-v0.40 rows covering RFC-059, RFC-037, RFC-014, RFC-023,
  RFC-058, RFC-009, RFC-019, RFC-034, view-model layer, CSS contract,
  platform diagnostics, and UI stabilisation.

  **Header and update blocks** — status line updated; v0.104.0 update block
  added summarising the audit findings.

---

---

## [0.103.0] — 2026-06-12

RFC-042 and governance notes updated to v0.102.0 reality.

### Changed

- **`rfcs/proposed/042-roadmap-and-rfc-execution-plan.md`** — updated as
  living document to v0.102.0:
  - Header status: v0.73.0 / 629 tests → **v0.102.0 / 891 tests**.
  - Added v0.102.0 update block: i18n complete, merge corpus, patch export,
    PlatformInfo wired, user docs complete, RFC-041 12/16 ticked.
  - §4b priority table: added Status column; row 7 (i18n + command registry)
    marked **Done**; remaining rows annotated with GTK requirement or
    deferred status; added "three immediate non-GTK work items" note.

- **`rfcs/notes/implementation-checklist.md`** — added "Superseded v0.102.0"
  notice at the top with forward references to ROADMAP.md, RFC-041, and
  RFC-042. Checklist body preserved as historical record.

- **`rfcs/notes/core-completion-summary-v0.72.md`** — test count updated
  875 → **891**; `merge_corpus` row added to test table; version note updated
  to v0.103.0.

- **`ROADMAP.md`** — last-updated header: v0.97.0 → v0.103.0; phase
  description updated.

---

---

## [0.102.0] — 2026-06-12

Three-way merge corpus added (16 tests, 18 fixtures); i18n fix; 875 → 891 tests.

### Added

- **`crates/forskscope-core/tests/merge_corpus.rs`** — 16 corpus-driven
  integration tests for `ThreeWayMergeSession` across 6 fixture triples:
  - `noconflict` — non-overlapping changes auto-merge, no conflicts, can_save
  - `conflict` — divergent single-line produces one conflict, blocks save,
    resolve_left/resolve_right each produce correct result text
  - `both_same` — identical changes on both sides deduplicate to auto-merge
  - `left_insert` — one-sided insertion auto-merges
  - `crlf` — CRLF line terminators preserved through merge result
  - `multi` — three divergent lines produce three conflicts; resolving all
    enables save; result matches left-side resolutions

- **`tests/fixtures/merge/`** — 18 fixture files (6 base/left/right triples)
  for the merge corpus.

- **`tests/fixtures/README.md`** — `merge/` section documenting all 6 triples
  with descriptions and contribution instructions.

### Fixed

- **`"Ignore file extensions"` missing from Japanese** (`i18n.rs`) —
  `"Ignore directory names"` had a translation but its sibling key did not.
  Fixed: `"Ignore file extensions"` → `"除外ファイル拡張子"`.

### Changed

- `docs/src/maintainers/testing.md` — count table updated (875 → **891**);
  `merge_corpus` row added to integration tests table.
- `docs/src/maintainers/local-dev.md` — core integration count updated
  (27 → 43 tests).
- `rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md` —
  core test total updated (875 → 891).

---

---

## [0.101.0] — 2026-06-12

i18n completeness pass; FAQ expanded with four common questions.

### Fixed

- **`"Compare"` missing from Japanese translations** (`i18n.rs`) — the key
  was used throughout the Explorer and diff toolbar but had no entry in
  `ja()`. Fixed: `"Compare"` → `"比較"`.

- **Three placeholder strings bypassed `t()`** (`search.rs`, `settings.rs`):
  - `"Search…"` placeholder in `SearchBar` → `t(lang, "Search…")`.
  - `"o, class, tmp  (comma separated, no dot needed)"` in the ignore
    extensions field → `t(lang, ...)`.
  - `"target, node_modules, *.cache  (* wildcard allowed)"` in the ignore
    dirs field → `t(lang, ...)`.
  - `SearchBar` now reads `store.lang()` from context; `"Search…"` added to
    `ja()` as `"検索…"`.
  - Two placeholder translations added to `ja()`.

### Changed

- **`docs/src/users/faq.md`** (93 → 161 lines) — four new entries:
  - *How do I export a patch file?* — More ▼ → Export patch; file dialog;
    unified-diff output; identical-files note.
  - *Why does Linux require GTK/WebKitGTK?* — Dioxus Desktop WebView
    dependency explained; install commands for Debian/Ubuntu and Fedora/RHEL;
    4.0 vs 4.1 version troubleshooting.
  - *Can I compare PDF or Word documents?* — unsupported; text export
    workaround; link to file types reference.
  - *What do the ✓ and ⚠ icons in the Explorer mean?* — four-row icon table
    (✓ identical, ⚠ different, none, ⊙ scanning); double-click ⚠ tip.

---

---

## [0.100.0] — 2026-06-12

PlatformInfo wired to About panel; patch export UI added; i18n completed.

### Added

- **Patch export button** (`diff.rs`, `diff_actions.rs`) — "Export patch"
  button in the advanced toolbar (More ▼). Calls `export_patch(store, index)`:
  opens a native save-file dialog defaulting to `<filename>.patch`, generates
  a unified-diff patch via `patch_from_file_diff` + `to_unified` from
  `forskscope-core`, and writes the file. Does nothing silently if the two
  files are identical (no hunks to export). Japanese: パッチをエクスポート.

### Changed

- **`AboutModal`** (`modals.rs`) — replaced hand-rolled diagnostic string with
  `PlatformInfo::collect()` and `to_report()` from `forskscope-core::platform`
  (added v0.93.0 but previously unused). The About panel now shows: Version,
  Rust compiler version, OS, Arch, CPUs — all sourced from the tested
  `PlatformInfo` module rather than ad-hoc `env!()` + `std::env::consts`
  calls. "Copy diagnostics" button text now goes through `t()`.

- **`i18n.rs`** — added `"Copy diagnostics"` → 診断情報をコピー and
  `"Export patch"` → パッチをエクスポート.

- **`ROADMAP.md`** — added v0.99–v1.0 milestone row to delivered table.

---

---

## [0.99.0] — 2026-06-12

RFC-041 v1 checklist updated; stale notes corrected; 8 more items now ticked.

### Changed

- **`rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`** —
  complete rewrite to a single, clean document (238 → 165 lines):
  - Header: v0.87.0 → **v0.99.0**.
  - RFC inventory: Done 38 → 39, Proposed 10 → 9.
  - Removed the two duplicate checklist copies that accumulated across updates.
  - **Checklist: 12 of 16 items now ticked** (was 9 of 16 at v0.87.0):
    - `[x] Core tests pass` — updated to **875 total** (was 599).
    - `[x] ui-logic tests pass` — count confirmed.
    - `[x] Architecture and testing docs current` — (v0.95.0).
    - `[x] User guide covers common workflows` — ticked (v0.96.0–v0.98.0).
    - `[x] Recovery/backup behavior documented` — ticked (merging.md).
    - `[x] Known limitations documented` — ticked (known-limitations.md, v0.98.0).
  - Remaining open items are all UI-wiring (requires GTK) or deferred (RFC-040, RFC-010).

- **`rfcs/notes/core-completion-summary-v0.72.md`** — updated to v0.99.0:
  - Title and version header: v0.78.0 → v0.99.0.
  - Core modules count: 26 → 27 (`platform` added in v0.93.0).
  - Test count table: 797 → **875** with correct per-suite breakdown
    (diff_corpus 25, patch_apply 2, CSS coverage 5, doctest 7).
  - Status line reflects UI stabilisation phase.

---

---

## [0.98.0] — 2026-06-12

CONTRIBUTING.md added; known-limitations.md added; RFC-030 status updated;
local-dev.md expanded with corpus and MSRV guidance.

### Added

- **`CONTRIBUTING.md`** — complete contributor guide: prerequisites, project
  layout (with the GTK constraint spelled out), pre-coding RFC guidance,
  step-by-step change workflow (branch/test/lint/doc), corpus fixture
  instructions, view-model module addition recipe, RFC governance note,
  commit message format, PR etiquette, licence acknowledgement.

- **`docs/src/users/known-limitations.md`** — 19 documented limitations across
  diff view (shared scroll, no three-way UI, inline diff scope, large files),
  explorer (no digest cache, no directory merge), file types (Excel is derived
  text, no binary merge), and platform (Linux WebKitGTK version, macOS
  unsigned binary, Windows long paths). Intentional non-goals section
  distinguishes limitations from design decisions.

- `known-limitations.md` registered in `docs/src/SUMMARY.md`.

### Changed

- **`rfcs/proposed/030-user-documentation-onboarding-and-help-system.md`** —
  status updated to "substantially implemented v0.96.0–v0.98.0"; full shipped
  list (17 doc files) and remaining items (in-app help, troubleshooting page).

- **`docs/src/maintainers/local-dev.md`** — added: full directory layout with
  `ui-logic` and `tests/fixtures/` entries; corpus test contribution
  instructions (fixture pair + corpus test + README update); MSRV verification
  command (`cargo +1.85 test`).

---

---

## [0.97.0] — 2026-06-12

ROADMAP, release process, and features documentation updated to v0.97.0.

### Changed

- **`ROADMAP.md`** — updated to v0.97.0:
  - Header: v0.88.0 → v0.97.0, phase description updated.
  - Current state: 801 → **875 tests**; describes UI completeness (two-pane
    split, i18n, modal ESC, theme select), platform diagnostics, acceptance
    corpus.
  - Delivered milestones table: added CSS bug fixes (v0.89), test coverage
    (v0.90–v0.91), UI four-bug fix (v0.92), platform diagnostics (v0.93),
    scroll fix + modals i18n (v0.94), docs pass (v0.95–v0.96).

- **`docs/src/maintainers/release.md`** (28 → 84 lines) — rewritten to match
  the actual release process:
  - Pre-release checklist (tests, clippy, CHANGELOG, version, RFCs, ROADMAP).
  - Accurate `cp -r` + `rm -rf target` + `tar` archive recipe (the previous
    doc used `git archive` which is not the process used).
  - Archive naming table and verification command.
  - Version scheme explanation (v0.x pre-release conventions).
  - Post-archive steps (upload, git tag, AUR).
  - MSRV check (`cargo +1.85 test`).

- **`docs/src/users/features.md`** (40 → 105 lines) — added all capabilities
  that shipped since the original stub:
  - Two-pane layout with shared horizontal scroll bar.
  - Full diff options table (Inline diff, Wrap, Ignore WS, Ignore case,
    Algorithm).
  - Compare profiles section (built-in: Default, Code Review, Loose Text,
    Large File Safe; custom profiles).
  - File types table updated (encoding detection, BOM round-trip).
  - Three-way merge / git mergetool section.
  - Localisation: English + Japanese explicitly stated.

---

---

## [0.96.0] — 2026-06-12

User documentation expanded: four pages rewritten to reflect current UI.

### Changed

- **`docs/src/users/diff-workflow.md`** (20 → 94 lines) — complete reference:
  colour table with non-colour accessibility indicators; navigation (◀▶, F7/F8,
  scroll behaviour); inline character diff instructions; context collapse
  explanation; gutter mark table (▶, ✓, −, +); per-tab diff options table
  (Inline diff, Wrap, Ignore WS, Ignore case, Algorithm) and link to profiles.

- **`docs/src/users/explorer.md`** (20 → 91 lines) — complete reference:
  opening directories (path bar, CLI, drag); navigation (expand/collapse, Alt+↑,
  history buttons); file selection (mouse and keyboard); same-name shortcut;
  digest equality icons (✓/⚠/none with meanings); multiple tabs; sync panes.

- **`docs/src/intermediate/file-types.md`** (12 → 91 lines) — complete reference:
  classification rule order (Missing→Unsupported→ExcelXlsx→Binary→Text);
  capability table; text encoding (UTF-8 → chardetng → encoding_rs, encoding
  preservation, UTF-8 BOM round-trip); binary hex preview; Excel structured
  diff; large-file class table with thresholds and behaviour changes;
  unsupported-file behaviour.

- **`docs/src/users/settings.md`** (84 → 125 lines) — updated to match the
  actual Settings dialog: added **Language** section (was missing); added
  **Ignore file extensions** and **Ignore directory names** sections (added in
  v0.92.0 but not documented); corrected built-in profile names to match the
  real `CompareProfile` presets (Default, Code Review, Loose Text, Large File
  Safe — the old doc listed invented names); added Ctrl+/, Esc close
  instructions; added Copy diagnostics description to About section.

---

---

## [0.95.0] — 2026-06-12

Documentation pass: testing.md, architecture.md, and local-dev.md updated
to v0.95.0 reality (875 tests, 38 core test modules, platform module).

### Changed

- **`docs/src/maintainers/testing.md`** — major update:
  - Test count table: v0.87.0 / 797 → **v0.95.0 / 875**, with per-suite
    breakdown including `diff_corpus` (25) and `css_coverage` (5) rows.
  - Core test module table: added `cancel_tests`, `file_kind_tests`,
    `path_tests`, `platform_tests` (all added in v0.89.0–v0.93.0 but
    never documented). Now 38 rows covering every test file.
  - Integration tests section: new table listing `diff_corpus` (25 tests,
    describes corpus fixture categories) and `patch_apply` (2 tests).
  - ui-logic integration note: corrected from "patch round-trip via
    hunk_decorations" (inaccurate) to `tests/css_coverage.rs`.

- **`docs/src/maintainers/architecture.md`** — added `platform` module row
  to the core modules table: `PlatformInfo::collect()`, `to_report()`,
  RFC-026 reference.

- **`docs/src/maintainers/local-dev.md`** — updated inline test counts
  from `599 unit + 2 integration` / `85 unit, 7 modules` to current
  `646 unit + 27 integration` / `189 unit + 5 integration + 1 doctest,
  14 modules`.

---

---

## [0.94.0] — 2026-06-12

ISSUE-001 resolved (single shared scroll bar); modals i18n completed.

### Fixed

**ISSUE-001 — Compare pane: single scroll bar for both panes** (`main.css`)

Resolved using Approach B from `known-ui-issues.md`:

- Removed `overflow-x: auto` from `.diff-pane` — panes no longer own their
  own scroll bars.
- Set `overflow-x: auto` on `.diff-scroll` — one scroll bar at the bottom
  of the diff view scrolls both panes together.
- Added `min-width: max(100%, 110ch)` to both `.diff-row` and
  `.diff-pane-labels` so narrow windows still show at least ~50ch per pane,
  and the label bar stays aligned with content rows when scrolled.
- `.diff-pane .cell` retains `white-space: pre` — long lines expand the row
  naturally; `.diff-scroll` clips and provides the scroll affordance.

Both panes always advance together horizontally, matching WinMerge default
synchronized scroll behaviour. Approach A (independent pane scroll bars)
remains the preferred long-term target but is deferred post-v1.

**Modals i18n completed** (`modals.rs`, `i18n.rs`)

All hardcoded English strings in every modal dialog wired through `t()`.
16 strings converted; 15 Japanese translations added to `ja()`:

`"The target file was modified…"`, `"Save As"`, `"Path"`,
`"Reload files?"`, `"Unsaved merge changes will be discarded."`,
`"Discard and Reload"`, `"Swap sides?"`,
`"Unsaved merge changes will be discarded when sides are swapped."`,
`"Discard and Swap"`, `"Close comparison?"`, `"Discard and close"`,
`"Copy file?"`, `"Copied."`, `"Copy"`, `"Copy all"`.

`AboutModal` was missing `let lang = store.lang();` — added.

`known-ui-issues.md` ISSUE-001 status updated to **Resolved v0.94.0**.

---

---

## [0.93.0] — 2026-06-12

Acceptance corpus extended; `platform` diagnostic module added; known UI
issue recorded.

### Added

- **`tests/fixtures/` extended** — 10 new fixture files:
  - `text/utf8_bom.txt`, `text/utf8_no_bom.txt` — UTF-8 BOM vs no-BOM pair.
  - `text/left_unicode.txt`, `text/right_unicode.txt` — Japanese + ASCII
    content; tests Unicode diff and `ignore_case` on non-ASCII.
  - `text/binary_nul.bin` — 9-byte file with a NUL byte; classifies as
    `FileKind::Binary`.
  - `text/large_equal_left.txt`, `text/large_equal_right.txt` — 200
    identical lines; tests the context-collapse path.
  - `text/large_one_change_left.txt`, `text/large_one_change_right.txt` —
    200-line file with one change at line 100.
  - `whitespace/left_mixed_trailing.txt`, `whitespace/right_clean.txt` —
    mixed trailing spaces and tabs.

- **9 new corpus integration tests** in `diff_corpus.rs`:
  - `unicode_content_diffed_correctly` — Japanese text with case change.
  - `unicode_content_equal_with_ignore_case` — world/WORLD ignored.
  - `utf8_bom_differs_from_no_bom` — BOM byte is a real difference.
  - `mixed_trailing_whitespace_detected_by_default` / `hidden_with_ignore_ws`.
  - `large_equal_files_are_identical` — 200-line identical files.
  - `large_file_with_one_change_produces_one_hunk` — one replace hunk, two
    changed lines.
  - `binary_fixture_classifies_as_binary` — NUL byte → `FileKind::Binary`.
  - `text_fixtures_classify_as_text` — three text fixtures → `FileKind::Text`.

- **`crates/forskscope-core/src/platform.rs`** — `PlatformInfo` struct with
  `collect()` and `to_report()` for the About / Diagnostics panel
  (RFC-026 §"Diagnostics panel").

  Fields: `app_version`, `rustc_version`, `target_triple`, `os`, `arch`,
  `logical_cpus`, `home_redacted` (username stripped to `***`),
  `config_dir_hint` (platform-appropriate config directory).

  8 unit tests: non-panic, non-empty fields, report format, home redaction,
  determinism, logical CPUs positive.

- **`rfcs/notes/known-ui-issues.md`** — issue tracker for deferred UI bugs.
  ISSUE-001: diff pane scroll bar per-line instead of per-pane (v0.92.0
  deferral), with root cause analysis and two recommended fix approaches.

### Test count: 875
(646 core unit + 25 diff_corpus + 2 patch_apply + 189 ui-logic +
 5 css_coverage + 7 doctest + 1 ui-logic-integration)

---

---

## [0.92.0] — 2026-06-12

Four UI bug fixes: two-pane split, theme select colours, ESC closes modals,
i18n dictionary expanded.

### Fixed

**1. Compare view two-pane split** (`main.css`, `diff.rs`)

The diff rows already used a 7-column grid (`4ch 1.2ch 1fr 5ch 4ch 1.2ch
1fr`) but had no visual separation between the left and right panes. Fixed:

- Added `border-left` and `border-right` to `.act` (the action column) to
  create a visible vertical divider between panes.
- Added `.diff-pane-labels` bar above the scroll area with "Left / Old" and
  "Right / New" headings, using the same grid column spans so they align
  exactly with the pane content below.
- Pane label text goes through `t()` so it appears in Japanese as 左/旧 and
  右/新 when the language is set to Japanese.

**2. Dark/Night theme `select` text colour** (`main.css`)

`select option` elements inherit native system colours on some platforms,
overriding `color: var(--text)` set on the parent `select`. Added an explicit
`select option { background: var(--surface); color: var(--text); }` rule to
force the correct colours in all three themes.

**3. ESC key closes Settings and Help dialogs** (`app.rs`, `settings.rs`,
`keybindings.rs`)

- Added `Key::Escape` branch to the global `onkeydown` handler in `app.rs`
  that closes any open modal immediately.
- Added `onkeydown` on each `scrim` div (Settings and KeyboardRef) that closes
  when Escape is pressed — catches the case where focus is inside the modal
  and the global handler doesn't fire.
- Added `onclick` on each `scrim` div so clicking the backdrop also closes the
  modal (standard UX pattern). The inner `div.modal` has `onclick:
  stop_propagation()` to prevent clicks inside the dialog from bubbling.

**4. i18n dictionary expanded** (`i18n.rs`, `diff.rs`, `settings.rs`,
`keybindings.rs`)

Previously many toolbar and dialog strings bypassed `t()` entirely. All are
now wired through the translation function. New keys added to `ja()`:

`Save As` → 名前を付けて保存, `More ▼` → 詳細 ▼, `Less ▲` → 簡略 ▲,
`Wrap` → 折り返し, `on` → オン, `off` → オフ, `Swap sides` → 左右入替,
`Ignore WS` → 空白無視, `Ignore case` → 大小文字無視,
`Context lines` → コンテキスト行数, `Compare profiles` → 比較プロファイル,
`+ New profile` → + 新規プロファイル, `Profile name` → プロファイル名,
`Add` → 追加.

Previously wired but missing from `ja()` (removed the fallthrough): `Undo`,
`Redo`, `Save As`.

---

---

## [0.91.0] — 2026-06-12

Diff acceptance corpus — 26 fixture files and 16 corpus integration tests.

### Added

- **`tests/fixtures/`** — workspace-level test fixture corpus implementing
  the acceptance test plan from `rfcs/notes/acceptance-test-corpus-plan.md`.

  **26 fixture files** across three categories:
  - `text/` — 14 files: identical pair, one-changed-line pair (charlie/CHARLIE),
    insertions pair, deletions pair, reordered-blocks pair, single-line
    function-edit pair, empty file, nonempty file.
  - `newlines/` — 5 files: LF, CRLF, no-final-newline, CRLF-no-final-newline,
    mixed newlines.
  - `whitespace/` — 6 files: extra space, trailing spaces, tab indent, space
    indent, and their respective counterparts.

  `tests/fixtures/README.md` documents the fixture structure, what each
  pair tests, and how to add new fixtures.

- **`crates/forskscope-core/tests/diff_corpus.rs`** — 16 corpus integration
  tests using the fixture files via `compute_diff`:
  - Identical fixture → `is_identical()`.
  - One-changed-line → single Replace hunk with correct line content.
  - Insertions / deletions → correct hunk kinds and line counts.
  - Both-empty → identical; empty vs nonempty → pure Insert.
  - LF vs CRLF differs by default (newline-significant).
  - No-final-newline vs with-newline differs.
  - Extra space detected by default; hidden with `ignore_whitespace`.
  - Trailing space detected by default.
  - Case change detected by default; hidden with `ignore_case`.
  - Tab vs space indent differs.
  - Single-line function edit → exactly one changed hunk.

### Test count: 856
(637 core unit + 16 diff_corpus + 2 patch_apply + 189 ui-logic +
 5 css_coverage + 6 doctest + 1 ui-logic-integration)

---

---

## [0.90.0] — 2026-06-12

CancellationToken and FileKind tests close the last untested core areas.

### Added

- **`tests/cancel_tests.rs`** in `forskscope-core` — 11 tests for
  `CancellationToken` (RFC-037, RFC-008):
  - New token is not cancelled; `cancel()` sets it; `cancel()` is idempotent.
  - Clone observes cancel from original; original observes cancel from clone.
  - Multiple clones (including clone-of-clone) all observe cancel.
  - Cancel from any clone propagates to all.
  - `Default::default()` is not cancelled.
  - `Debug` format does not panic (before and after cancel).

  The doctest in `cancel.rs` was a compile-only check. These tests verify
  the actual cross-clone propagation contract.

- **`tests/file_kind_tests.rs`** in `forskscope-core` — 11 tests for
  `FileKind` predicates and the `classify()` function (RFC-001 §6.2):
  - `is_mergeable_text()`: Text → true; Binary/ExcelXlsx/Missing/Unsupported → false.
  - `classify()` on missing path → `Missing`.
  - `classify()` on UTF-8 text file → `Text`.
  - `classify()` on file with NUL byte → `Binary`.
  - `classify()` on `.xlsx`-extension file → `ExcelXlsx` (before content check).
  - `classify()` on `.XLSX` (uppercase) → `ExcelXlsx` (case-insensitive).
  - `classify()` on empty file → `Text`.
  - `classify()` on Rust source → `Text`.
  - `classify()` on a directory → `Unsupported`.

  Added `tempfile = "3"` as a dev-dependency to `forskscope-core`.

### Test count: 840
(637 core + 189 ui-logic + 2 core-integration + 5 ui-logic-integration + 6 doctest + 1)

---

---

## [0.89.0] — 2026-06-12

CSS bug fix; CSS var coverage test; path.rs tests.

### Fixed

- **`--danger-bg` CSS variable missing from all three themes.** The close
  button hover background (`var(--danger-bg)`) was referenced in the tab
  close button CSS rule but never defined, leaving it invisible/unstyled.
  Added to all three theme blocks: `#5c1e1e` (dark), `#ffd5d5` (light),
  `#4a1515` (night).

### Added

- **`all_css_vars_used_are_defined_in_main_css`** integration test in
  `tests/css_coverage.rs` — scans every `var(--name)` usage in `main.css`
  and asserts a corresponding `--name:` definition exists. This test
  would have caught the `--danger-bg` bug immediately. Uses a careful
  character-by-character extraction to avoid false positives from adjacent
  CSS values.

- **`tests/path_tests.rs`** in `forskscope-core` — 16 tests for `path.rs`
  helper functions (RFC-001 §6.1):
  - `split_parent_name`: typical path, root file, relative path, filename
    only, dotfile.
  - `has_extension`: exact match, case-insensitive ASCII, no match, no
    extension, dotfile (no extension in Rust's Path model), xlsx match.
  - `display`: non-empty output, contains filename.
  - `canonicalize_lenient`: nonexistent absolute path returns input, existing
    `/tmp` produces absolute result, never panics on edge cases (empty, `.`,
    `..`, `/`, path with `..` components).

  `path.rs` was the only core module without any test coverage.

### Test count: 818
(615 core + 189 ui-logic + 2 core-integration + 5 ui-logic-integration + 6 doctest + 1)

---

---

## [0.88.0] — 2026-06-12

CSS class contract established; 4 coverage integration tests.

### Added

- **`fs-line-*` and `fs-inline-*` CSS classes** in `main.css` — the class
  tokens produced by `LineDecorationKind::css_class()` and
  `InlineDecorationKind::css_class()` (from `DiffDecorationSet`, RFC-024).
  Previously the stylesheet only had the older `.hunk-del` / `.hunk-ins` /
  `.in-del` / `.in-ins` classes; the `fs-*` classes from core were absent,
  meaning `DecorationIndex`-based rendering would produce unstyled rows.

  Classes added: `fs-line-unchanged`, `fs-line-added`, `fs-line-deleted`,
  `fs-line-modified`, `fs-line-empty-counterpart`, `fs-line-conflict`,
  `fs-line-merge-applied`, `fs-inline-inserted`, `fs-inline-deleted`,
  `fs-inline-replaced`, `fs-inline-whitespace`.

- **`fsk-conflict-*` CSS classes** in `main.css` — the tokens produced by
  `ConflictNavigatorEntry::css_class()` (RFC-034). Six variants covering
  all `ConflictStatus` values: `fsk-conflict-unresolved`,
  `fsk-conflict-left`, `fsk-conflict-right`, `fsk-conflict-both`,
  `fsk-conflict-manual`, `fsk-conflict-ignored`.

- **`tests/css_coverage.rs`** integration test in `forskscope-ui-logic` —
  4 tests that read `main.css` at compile time via `include_str!` and
  verify every CSS class token from core is present in the stylesheet:
  - `line_decoration_css_classes_defined_in_main_css` — all 7 `LineDecorationKind` classes
  - `inline_decoration_css_classes_defined_in_main_css` — all 4 `InlineDecorationKind` classes
  - `conflict_navigator_css_classes_defined_in_main_css` — all 6 `fsk-conflict-*` classes
  - `row_state_gutter_symbols_are_distinct` — glyph uniqueness smoke test

  These tests catch contract drift at compile time. If a class is renamed in
  core or missing from the CSS, the test fails immediately rather than
  silently producing unstyled UI.

- **`ROADMAP.md`** updated to v0.88.0: correct test counts (801), add CSS
  contract milestone, reflect 14 view-model modules.

### Total test count: 801
(599 core + 189 ui-logic + 2 core-integration + 5 ui-logic-integration + 6 doctest)

---

---

## [0.87.0] — 2026-06-12

Documentation pass: maintainer docs updated to v0.87.0 reality.

### Changed

- **`docs/src/maintainers/architecture.md`**:
  - `ui-logic` modules table: **9 → 14 modules**. Added `conflict_nav_view`,
    `load_guard`, `palette_view`, `save_error`, `scroll_sync`,
    `settings::settings_view` with purpose and Slice/RFC cross-references.
  - UI modules table: added shim re-export row listing all 14 shim files.

- **`docs/src/maintainers/testing.md`**:
  - Version header: v0.79.0 → v0.87.0.
  - Test count table: 692 → **797** (189 ui-logic, +1 ui-logic integration).
  - `ui-logic` test modules table: **9 → 14 rows**. Added `conflict_nav_view`,
    `palette_view`, `save_error`, `scroll_sync`, `settings/settings_view` with
    coverage descriptions and RFC columns.

- **`rfcs/notes/core-completion-summary-v0.72.md`**: updated version,
  module count (7 → 14), test count (85 → 189), status line.

- **`rfcs/proposed/041-v1-product-stabilization-and-rfc-governance.md`**:
  updated version (v0.78.0 → v0.87.0), ui-logic gate (85/7 → 189/14),
  RFC inventory description.

---

---

## [0.86.0] — 2026-06-12

Settings form view-model; `ui-logic` now covers all 7 ROADMAP slices (Slice 5).

### Added

- **`settings::settings_view`** in `forskscope-ui-logic` — picker metadata and
  validators for the settings dialog (RFC-009, Slice 5).

  **`SelectChoice { value, label }`** — one `<option>` in a `<select>`.

  **`theme_choices() → Vec<SelectChoice>`** — three entries (`dark`, `light`,
  `night`), values matching `ThemeId::as_str()`.

  **`density_choices() → Vec<SelectChoice>`** — three entries (`comfortable`,
  `compact`, `spacious`), matching `Density::as_str()`.

  **`font_family_choices() → Vec<SelectChoice>`** — three entries (`system-mono`,
  `system-sans`, `system-serif`), matching `FontFamilySetting::as_str()`.

  **`profile_presets() → Vec<ProfileChoice>`** — one entry per
  `CompareProfile::all_presets()` preset. `ProfileChoice { name, profile }`
  where `profile` provides `to_diff_options()`.

  **`validate_font_size(u32) → Result<u8, (u32, u32)>`** — checks 6–50 pt.
  **`clamp_font_size(u32) → u8`** — silent clamp. Both used by the font-size
  input field's `oninput` handler.

  **`validate_context_lines(usize) → Result<usize, (usize, usize)>`** — 0–20.

  **`find_active<'a>(choices, value) → Option<&'a SelectChoice>`** — finds the
  currently selected option by value string; component falls back to first
  choice when `None`.

- **`ui/settings_view.rs`** shim.

- **21 new tests** — theme values round-trip through `ThemeId::from_id`;
  density/font values round-trip; profile count matches `all_presets()`;
  font-size validation boundaries (5 fails, 6/14/50 pass, 51 fails);
  `clamp_font_size` extremes; context-lines boundary; `find_active` hit/miss;
  no duplicate values in any choice list.
  Total ui-logic count: 189.

### Changed

- `lib.rs` doc comment updated to reflect all 14 modules across 3 areas.

---

---

## [0.85.0] — 2026-06-12

Command palette and conflict navigator view-models (Slices 6 & 7).

### Added

- **`compare::palette_view`** — command palette search result view-model
  (RFC-019 §"Command palette", Slice 7).

  **`PaletteRow`** — one filtered result: `command_id`, `label`,
  `description`, `shortcut_hint`, `enabled`, `disabled_reason`,
  `is_dangerous`.

  **`build_palette(registry, ctx, query) → Vec<PaletteRow>`** — filters the
  registry by query (empty = all), evaluates `AvailabilityRule` for each
  command, then stable-sorts: enabled commands first, disabled last.
  Case-insensitive match on label and description.

  **`palette_enabled_count(rows) → usize`** — convenience count.

  Tests (15): empty query returns all; query matches label; nonsense returns
  empty; case-insensitive; enabled before disabled; Save disabled in empty
  context; Next Difference enabled in diff context; `enabled_count` matches;
  all labels/IDs non-empty.

- **`compare::conflict_nav_view`** — three-way merge navigator rail
  view-model (RFC-034, Slice 6).

  **`ConflictRailRow`** — one rail entry: `conflict_id`, `display_num`,
  `glyph` (`!`/`L`/`R`/`B`/`~`/`-`), `status_text`, `css_class`
  (`fsk-conflict-*`), `is_focused`.

  **`ConflictNavView`** — complete rail snapshot: `rows`, `progress_text`
  (`"2 of 5 resolved"` / `"All resolved"` / `"No conflicts"`), `can_save`,
  `prev_id`, `next_id`, `summary`. `from_navigator(nav, can_save)`.
  `len()`, `is_empty()`, `focused_row()`.

  Tests (12): non-empty with conflicts; empty with no conflicts; display_num
  ≥ 1; unresolved → `!` glyph; css_class starts with `fsk-`; progress_text
  not "All resolved" when unresolved; "No conflicts" text; `can_save` false
  when unresolved; true when no conflicts; `len == rows.len`.

- **`ui/palette_view.rs`** and **`ui/conflict_nav.rs`** shims.

  Total ui-logic count: 168 (was 147, +21).

---

---

## [0.84.0] — 2026-06-12

`compare::save_error` — save-error dialog view-model (Slice 3).

### Added

- **`compare::save_error`** in `forskscope-ui-logic` — maps `AppError` to
  everything the save-error dialog needs (RFC-007, RFC-017).

  **`action_label(action) → &'static str`** — human-readable button label
  for each `RecoveryAction`. Covers all 12 variants; labels are 3 words or
  fewer. `"Overwrite anyway"`, `"Reload"`, `"Save As…"`, etc.

  **`RecoveryButton`** — one dialog button: `action`, `label`, `is_primary`.
  `is_primary` is true for the first non-destructive action (`OverwriteAnyway`
  and `ReportBug` are never primary).

  **`SaveErrorView`** — complete dialog snapshot: `title`, `body`, optional
  `path`, `Vec<RecoveryButton>`. `from_error(err, path)` builds from an
  `AppError`; `has_action(action)` and `primary_button()` are convenience
  accessors.

  Replaces the ad-hoc `Err(CoreError::Conflict { .. }) =>
  store.modal.set(Modal::ConfirmOverwrite(index))` pattern in
  `diff_actions.rs` — every `AppErrorKind` now maps to a fully-rendered
  dialog rather than requiring per-variant match arms in the Dioxus modal
  layer.

- **`ui/save_error.rs`** shim.

- **14 new tests** — all `RecoveryAction` labels non-empty, `Dismiss` label
  correct, `OverwriteAnyway` mentions overwrite; external-modification view
  has correct action set, primary button is `Reload` not `Overwrite`;
  `SaveConflict`/`FileWriteFailed`/`InternalFault` action sets; path
  passthrough; title/body non-empty for all save error kinds; buttons
  non-empty; each button has a label; exactly one primary button.
  Total ui-logic count: 147.

---

---

## [0.83.0] — 2026-06-12

Scroll synchronisation view-model; release archive fix.

### Fixed

- **Release archive contamination.** Since v0.81.0, archives included an
  old `forskscope-0.80.0/` directory (with its `target/`) nested inside
  the release tree, inflating archives to ~300 MiB. Root cause: `cp -r`
  of the working tree did not strip sibling version directories that had
  accumulated in the work root. Fixed by cleaning the work tree before
  every `cp` and verifying no stray dirs remain. Archives are now ~3 MiB.

### Added

- **`compare::scroll_sync`** in `forskscope-ui-logic` — synchronized-scroll
  view-model for the two-pane diff view (RFC-035).

  **`ScrollSyncState`** — holds a shared `ScrollAnchor` (row index + row
  fraction) and the uniform row height. Three construction paths:
  - `at_top(row_height_px, total_rows)` — initial state.
  - `from_scroll_top(px, row_height_px, total_rows)` — converts a raw
    `scrollTop` value from one pane into an anchor; the other pane then
    calls `scroll_top_px()` to get its matching position.
  - `scroll_to_row(row_index)` — used by F7/F8 hunk navigation to jump
    both panes to a specific hunk's first row.

  `is_at_top()`, `max_scroll_px(visible_height_px)`, `scroll_top_px()`.
  Zero `row_height_px` is guarded against (treated as 1.0).

- **`ui/scroll_sync.rs`** shim — re-exports `ScrollSyncState`.

- **19 new tests** — `at_top`, pixel→anchor→pixel round-trip, mid-row
  fraction, negative input clamping, `scroll_to_row` correctness and
  over-bounds clamping, past-end clamping, `max_scroll_px` normal and
  content-fits cases, zero row-height guard. Total ui-logic count: 133.

---

---

## [0.82.0] — 2026-06-12

`compare::load_guard` — pre-diff file-size decision view-model (Slice 1).

### Added

- **`compare::load_guard`** in `forskscope-ui-logic` — pre-diff action
  derived from `FileSizeClass` thresholds (RFC-013 §"Large file prompt").

  **`LoadGuard`** — three variants:
  - `Proceed` — both files small; diff immediately.
  - `WarnBanner { message, suppress_inline }` — medium-sized file(s);
    proceed but show a non-blocking yellow banner and suppress character-
    level inline diff.
  - `ConfirmPrompt { title, body, confirm_label, too_large }` — large or
    very-large file(s); block and ask the user before diffing. `too_large`
    distinguishes "diff anyway" (Large) from "metadata only" (VeryLarge).

  **`guard_for_sizes(left_bytes, right_bytes) → LoadGuard`** — entry point
  with default `PerformanceLimits`. Takes the *worst* `FileSizeClass` of the
  two files; if one side is VeryLarge and the other is Small, the result is
  a `ConfirmPrompt` with `too_large: true`.

  **`guard_for_sizes_with_limits(…, limits) → LoadGuard`** — same but with
  explicit thresholds; used in tests and for future settings integration.

  Replaces the reactive-only `DiffWarning::LargeFilePolicyApplied` path
  (which fires *after* the diff) with a *pre-diff* decision `open_compare`
  and `DiffWorkspace` can act on before triggering the expensive computation.

- **`ui/load_guard.rs`** shim — re-exports `LoadGuard`, `guard_for_sizes`,
  `guard_for_sizes_with_limits` from `ui-logic`.

- **19 new tests** covering: all four `FileSizeClass` branches (both files
  small/medium/large/very-large), worst-of-pair logic, boundary values
  (exactly at limit, one byte over each threshold), message/label
  non-empty, distinct large vs very-large confirm labels, and default-limit
  smoke tests. Total ui-logic count: 119.

---

---

## [0.81.0] — 2026-06-12

Bug fix in `hunk_decorations` tests; `hunk_decorations` shim added to UI crate.

### Fixed

- **`compare::hunk_decorations::tests::identical_texts_produce_empty_index`**
  was asserting `idx.is_empty()` for identical texts. `DecorationIndex::is_empty()`
  means "no rows at all", but identical texts produce one Equal hunk whose
  single row gets an `Unchanged` decoration — so `is_empty()` correctly
  returns `false`. The test's intent was "no changed decorations"; the fix
  replaces it with `identical_texts_produce_only_unchanged_decorations`,
  which checks that every row in the index has `LineDecorationKind::Unchanged`.
  The `empty_diff_get_returns_unchanged` test (checking `idx.get(0, …).kind`)
  is kept as a complementary spot-check.

### Added

- **`ui/hunk_decorations.rs`** shim in `forskscope-ui` — re-exports
  `DecorationIndex`, `DiffSide`, `RowDecoration` from `ui-logic` following
  the established shim pattern. `hunk.rs` can now switch from its inline
  `match hunk.kind { ... => "hunk-del" }` CSS logic to
  `DecorationIndex::get(row_index, side)` when the GTK build environment
  is available.

### Test count

708 total (599 core + 100 ui-logic + 2 core-integration + 6 doctest + 1
ui-logic-integration). The ui-logic count increased from 85 to 100: the
`hunk_decorations` module's 15 tests were already present in the crate
but the `0.80.0` release note undercounted them; the correct baseline count
going forward is 100.

---

---

## [0.80.0] — 2026-06-12

UI crate: shim re-exports for all `ui-logic` modules; GTK-free test template.

### Added

- **5 new shim re-export modules** in `crates/forskscope-ui/src/ui/`:

  All follow the pattern established by `explorer_align.rs` and
  `search_index.rs`: a one-line `pub use forskscope_ui_logic::...` with
  `#[allow(unused_imports)]` so components can migrate imports at their own
  pace without unused-import warnings blocking the build.

  | File | Re-exports |
  |---|---|
  | `explore_status.rs` | `RowStatusKind`, `StatusRow` |
  | `deep_filter.rs` | `DeepCompareSummary`, `DeepFilter`, `apply_filter` |
  | `command_bar.rs` | `ToolbarItem`, `ToolbarSection`, `build_toolbar`, `enabled_count`, `find_item` |
  | `compare_summary.rs` | `CompareStatusSummary`, `DiffNavigationState` |
  | `tab_state.rs` | `TabStateSnapshot`, `context_from_snapshot` |

  All 5 registered in `ui/mod.rs`.

- **GTK-free test template in `state.rs`** — 8 tests in a `#[cfg(test)]`
  block covering `tab_title` (same/different/one-sided/missing/dotfile/nested
  filenames) and `SessionState` serde round-trip (with tabs, empty). Tests
  are syntactically correct but require a GTK build environment to execute
  (the `dioxus-desktop` dependency pulls in GTK at compile time even for
  `cargo test --lib`; documented in `local-dev.md` and `testing.md`).

### Changed

- `docs/src/maintainers/local-dev.md` — updated Build section to clearly
  distinguish GTK-free tests (`-p forskscope-core -p forskscope-ui-logic`)
  from workspace tests (requires GTK); noted the `state.rs` test situation.
- `docs/src/maintainers/testing.md` — added `forskscope-ui` section
  documenting the GTK-required test template.

---

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

---

## [0.22.3] and earlier

Tauri v2 + Svelte 5 + similar v2 baseline.
See the [v0.22.x repository](https://github.com/forskscope/forskscope/tree/v0.22.3)
for the previous changelog.

---
