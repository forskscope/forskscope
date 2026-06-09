# RFC 034: Conflict Resolution Workspace

**Status.** Proposed

## Status
Proposed. Originally sketched in RFC package v0.4; rewritten for the v0.41+
line to target the three-way merge **model that actually shipped** in
v0.40.0 (`forskscope-core::merge::ThreeWayMergeSession`, RFC-033). This
revision replaces the earlier stub's invented types (`TextRange`,
`BothOrder`, a separate resolution-transaction enum) with the real core
API, so the design is directly implementable.

## Summary

Define the Dioxus UI workspace for resolving the conflicts produced by the
three-way merge model. The workspace surfaces base / left / right / result,
lets the user navigate and resolve conflicts with mouse or keyboard, shows a
single canonical conflict state, and prevents an unsafe save while any
conflict remains unresolved.

This RFC is **UI only**. All merge truth — conflict records, resolution
operations, undo/redo, result text, and the save-block predicate — already
exists in core (RFC-033) and is *not* reimplemented here. The workspace is a
projection of, and a controller for, `ThreeWayMergeSession`.

## Motivation

Three-way merge without a good conflict workflow is dangerous. RFC-033
delivered a correct, tested model; it is currently unreachable from the GUI.
This RFC connects it: a calm, keyboard-first surface where the user
understands what changed on each side, picks a resolution, optionally edits
the result, and knows whether saving is safe.

The design is bound by the project's own rules:

- **Less is more** (UI/UX principle). The default is a calm four-region view
  with one focused conflict and a compact action bar — not every pane,
  counter, and control at once. Advanced options sit behind disclosure.
- **D-002 — one canonical conflict state.** Editor decorations must derive
  from `ConflictStatus` in core. The UI must never maintain an independent
  notion of "resolved."
- **D-008 — editor is not product truth.** Manual edits flow back to core
  via `resolve_manual(id, text)`; the rendered DOM is never read as the
  merge result.
- **D-005 / D-015 — safe, non-destructive, never hide a state.** Save is
  blocked while unresolved; unresolved conflicts are always visible.

## Scope

### In scope
- A `ThreeWayTab` (or three-way variant of the existing tab) hosting the
  workspace.
- Four-region layout (base, left, right, result) with a conflict-focused
  view and a conflict navigator.
- Per-conflict resolution actions mapped 1:1 onto the core API.
- Manual-edit affordance for the result region routed through
  `resolve_manual`.
- Keyboard navigation and shortcuts; accessibility semantics.
- Save gating via `can_save()`; integration with existing save dialogs
  (RFC-007) and the dirty-close guard.

### Out of scope (deferred)
- The three-way **open** dialog and file-triad selection (base/left/right
  picking) — small, but belongs with the explorer/open flow; may be a thin
  follow-up. This RFC assumes a `ThreeWayMergeSession` already exists for a
  tab.
- Editor *operation* model (RFC-032). Manual edit here is whole-region text
  captured on commit, not a typed-operation stream. When RFC-032 lands,
  manual edit can upgrade to operations without changing this UI's contract.
- Marker-based conflict-file export (a future opt-in; see RFC-033).
- VCS-assisted base discovery (RFC-038).

## Core API this UI consumes (already shipped)

```rust
ThreeWayMergeSession::from_texts(base, left, right) -> Self
  .conflicts() -> &[MergeConflict]          // each: id, base/left/right lines, status, manual
  .conflict(id) -> Option<&MergeConflict>
  .stats() -> ThreeWayStats                 // regions_total, auto_merged, conflicts_total, conflicts_unresolved
  .unresolved_count() -> usize
  .is_fully_resolved() -> bool
  .can_save() -> bool                       // false while any conflict unresolved
  .can_undo() / .can_redo() -> bool
  .is_dirty() -> bool
  .resolve_left(id) / .resolve_right(id) / .resolve_both(id)
  .resolve_manual(id, &str) / .ignore(id) / .reset(id) -> Result<()>
  .undo() / .redo() -> Result<ConflictId>   // returns the affected conflict
  .mark_saved()
  .result_text() -> String                  // canonical merged text for save
```

`ConflictStatus` = `Unresolved | ResolvedLeft | ResolvedRight |
ResolvedBoth | ResolvedManual | Ignored`.

The UI adds no merge logic. Every state badge, navigator entry, and save
gate reads these methods.

## External Design

### Workspace layout

```text
+--------------------------------------------------------------------------------+
| Three-Way Merge: config.toml        12 conflicts · 8 auto-merged · 4 unresolved |
+----------------+----------------+----------------+----------------+------------+
| Base           | Left           | Right          | Result         | Conflicts  |
| read-only      | read-only      | read-only      | editable       | navigator  |
| (ancestor)     | (variant A)    | (variant B)    | (merged)       | (toggle)   |
|                |                |                |                | [!] #1  L42|
|  120  common   |  120  A's text |  120  B's text |  120  ??       | [/] #2  L88|
|                |                |                |                | [!] #3 L120|
|                |                |                |                | [~] #4  L15|
+----------------+----------------+----------------+----------------+------------+
| Conflict 3/12 · both edited · unresolved                                       |
| [Use Left] [Use Right] [Use Both v] [Edit] [Ignore] [Reopen]   < Prev  Next >  |
+--------------------------------------------------------------------------------+
| Ready · Local only · Unresolved conflicts block save · Ctrl+S disabled         |
+--------------------------------------------------------------------------------+
```

Calm-by-default rules:
- The conflict **navigator** is a collapsible right rail, not always
  expanded. On narrow widths it becomes a dropdown above the action bar.
- The four content regions are equal-width by default; **base may be
  collapsed** to reclaim width (RFC-033 allows this) but the unresolved
  count must remain visible when it is.
- Only the **focused conflict's** region is emphasized; auto-merged regions
  render as ordinary context.

### Compact / narrow layout

Stack to a single scrollable column showing, for the focused conflict only:
Base -> Left -> Right -> Result blocks, with the action bar pinned at the
bottom. Side identity is shown by label + accent, never by position alone.

```text
+------------------------------+
| 3-Way: config.toml   3/12    |
+------------------------------+
| Conflict #3  unresolved      |
| Base:  common text           |
| Left:  A's text              |
| Right: B's text              |
| Result: ??                   |
+------------------------------+
| [L] [R] [Both v] [Edit][Skip]|
| < Prev   1 left   Next >     |
+------------------------------+
```

### Conflict navigator

One row per conflict, in document order, each showing a status glyph **plus
text** (color is never the sole cue, see Accessibility):

| Glyph | Text | Status |
|---|---|---|
| `!` | unresolved | `Unresolved` |
| `L` | left | `ResolvedLeft` |
| `R` | right | `ResolvedRight` |
| `B` | both | `ResolvedBoth` |
| `~` | manual | `ResolvedManual` |
| `-` | ignored | `Ignored` |

Footer: `8 resolved / 4 unresolved` from `stats()`. Clicking a row focuses
that conflict. Auto-merged (non-conflict) regions are **not** listed — they
need no action — but the header's "8 auto-merged" count acknowledges them.

### Resolution actions (1:1 with core)

| Button | Core call | Result |
|---|---|---|
| Use Left | `resolve_left(id)` | result region = left content |
| Use Right | `resolve_right(id)` | result region = right content |
| Use Both v -> L then R | `resolve_both(id)` | left then right |
| Use Both v -> R then L | *(see note)* | right then left |
| Edit... | enter manual edit, then `resolve_manual(id, text)` on commit | custom |
| Ignore / Skip | `ignore(id)` | result region = base |
| Reopen | `reset(id)` | back to `Unresolved` |

Note on "R then L": core currently exposes `resolve_both` as **left then
right** only. The UI ships the L-then-R action against the existing API; the
R-then-L variant is presented only if/when core adds it (a small additive
method `resolve_both_order(id, order)` — recorded as a core follow-up, not a
blocker). The UI must not fake R-then-L by reading/concatenating DOM text,
since that would violate D-008.

### Manual edit affordance

- "Edit..." makes the **result region** for the focused conflict editable
  (a textarea seeded with the current resolved content, or empty for an
  unresolved conflict).
- On **commit** (explicit "Done"/Ctrl+Enter, or blur with confirmation),
  the UI calls `resolve_manual(id, &edited_text)`. Status becomes
  `ResolvedManual`.
- On **cancel** (Escape), no core call is made; status is unchanged. An
  uncommitted edit never affects `result_text()` or `can_save()`.
- The editor is a plain controlled textarea (no contenteditable on rendered
  diff rows). This is the model-backed boundary the migration roadmap
  requires; when RFC-032 lands, the commit path can emit typed operations
  instead of whole-text replace, transparently to the rest of this UI.

### Save and close behaviour

- **Save (Ctrl+S):** disabled while `can_save() == false`. The status bar
  states *why* ("Unresolved conflicts block save"). When all conflicts are
  resolved/ignored, Save writes `result_text()` through the existing
  save-safety path (RFC-007: atomic write, backup, external-change check),
  then calls `mark_saved()`.
- **Save As:** allowed at any time for a *resolved* session. While
  unresolved, Save As is offered only as an explicit "Save As Conflict
  File..." if/when marker export exists (deferred); until then it follows the
  same `can_save()` gate.
- **Dirty close:** reuses the existing `ConfirmClose` modal, driven by
  `is_dirty()`. Discard / Save / Cancel.

## Internal Design (Dioxus)

### State ownership

Follows the established pattern (`Store` of `Signal`s; the session model is
the truth; UI signals hold only transient view state).

```text
forskscope-core (truth)        ThreeWayMergeSession (per tab)
forskscope-ui-dioxus (view)    ThreeWayTab + transient UI signals
```

Proposed additions to `state.rs`:

```rust
/// A three-way merge tab. Parallels `CompareTab` but owns a
/// `ThreeWayMergeSession` instead of a two-way `MergeSession`.
pub struct ThreeWayTab {
    pub title: String,
    pub base_path:  Option<PathBuf>,
    pub left_path:  Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub session: ThreeWayMergeSession,   // core truth
    pub focused_conflict: Option<ConflictId>,
    pub base_collapsed: bool,
    pub navigator_open: bool,
    pub editing: Option<ConflictId>,     // Some(id) while manual edit open
    pub edit_buffer: String,             // transient textarea contents
}
```

Two integration choices for the tab list — decide at implementation:
- (a) extend the existing `tabs: Signal<Vec<CompareTab>>` into an enum
  `Tab { TwoWay(CompareTab), ThreeWay(ThreeWayTab) }`; or
- (b) keep a parallel `three_way_tabs` list.
(a) is cleaner for the shared tab bar / dirty markers and is the
recommendation; (b) is lower-risk if the tab bar code is fragile. This RFC
does not mandate one.

`Modal` gains no new *blocking* variant for resolution itself (resolution is
inline). Manual-edit commit-on-blur may reuse a small confirm, or commit
directly; implementer's choice. Save/close reuse existing modals.

### Component tree

```text
ThreeWayWorkspace
├─ ThreeWayHeader        (title, stats summary, base-collapse toggle)
├─ ThreeWayRegions
│  ├─ RegionPane(Base)        read-only
│  ├─ RegionPane(Left)        read-only
│  ├─ RegionPane(Right)       read-only
│  └─ ResultPane              read-only OR textarea while editing
├─ ConflictNavigator     (collapsible rail / dropdown)
└─ ConflictActionBar     (resolution buttons + prev/next)
```

Region rows for the focused conflict are aligned by region (same vertical
band), mirroring the existing two-pane row-alignment work (CHANGELOG
v0.38.0). Auto-merged regions render as plain context.

### Command flow (example: Use Left)

```text
click "Use Left"
  -> session.resolve_left(focused_id)        // core mutates + logs undo
  -> on Ok: re-read stats(), conflicts(), can_save()  // signals refresh
  -> advance focus to next unresolved (optional, behind a setting)
  -> on Err (stale id): toast, refresh navigator
```

Undo/redo (Ctrl+Z / Ctrl+Y or Ctrl+Shift+Z) call `session.undo()/redo()`,
which return the affected `ConflictId`; the UI focuses that conflict so the
user sees what changed.

## Keyboard Interaction

Wired in the existing `onkeydown` handler (see `app.rs`), active when a
three-way tab is focused:

| Key | Action |
|---|---|
| F8 / F7 | Next / previous conflict (focus move) |
| Ctrl+Down / Ctrl+Up | Next / previous **unresolved** conflict |
| Alt+Left | Use Left for focused conflict |
| Alt+Right | Use Right for focused conflict |
| Alt+B | Use Both (L then R) |
| Alt+E | Edit result (enter manual edit) |
| Alt+I | Ignore |
| Alt+O | Reopen / reset |
| Enter | Commit manual edit (when editing) |
| Escape | Cancel manual edit; else close navigator/menu |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo resolution |
| Ctrl+S | Save (only when `can_save()`) |
| Ctrl+G | Toggle conflict navigator |

These extend, and must not collide with, the existing global/diff shortcuts
(F7/F8, Ctrl+S, Ctrl+F, Ctrl+/). The keyboard reference modal
(`KeyboardRefModal`) gains a "Three-Way Merge" section.

## Accessibility Requirements

- Every action and navigation step is keyboard reachable; no mouse-only
  path (acceptance criterion below).
- Conflict status uses glyph **and** text label, never color alone
  (D-002 alignment; see Conflict navigator).
- The four regions are landmark/labelled regions: "Base (ancestor)",
  "Left (variant A)", "Right (variant B)", "Result (merged, editable)".
- Action buttons carry explicit labels, e.g. *"Use left for conflict 3 of
  12"*.
- The conflict navigator is a list with each row labelled by conflict
  number, line range, and status text.
- Focus is visible in all themes; manual-edit textarea traps nothing but
  restores focus to the action bar on commit/cancel.
- Status-bar save-block reason is also exposed via an `aria-live` polite
  region so screen-reader users learn why Save is disabled.

## Acceptance Criteria

- A three-way session's conflicts are listed, navigable, and individually
  resolvable using only the keyboard.
- Each resolution action maps to exactly one core call; the UI holds no
  independent resolved/unresolved state (D-002).
- Manual edits are committed to core via `resolve_manual`; the rendered DOM
  is never read back as the merge result (D-008).
- Save is disabled while `can_save() == false`, with a visible reason; when
  enabled, it writes `result_text()` through the RFC-007 save path and calls
  `mark_saved()`.
- Undo/redo revert/redo resolution changes and refocus the affected
  conflict.
- Closing a dirty three-way tab triggers the existing unsaved-changes guard.
- Conflict status is communicated without relying on color.

## Dependencies

- RFC 033 — Three-Way Merge Model (**shipped, v0.40.0**) — the model this UI
  drives.
- RFC 003 — Dioxus Application Shell, State Runtime, Workspace Model
  (**shipped**) — tab/workspace host, `Store`, `Modal`.
- RFC 007 — Save, Session, and File Safety (**shipped**) — save path,
  external-change detection, dirty-close guard.
- RFC 015 — Undo/Redo Transaction Log — undo/redo semantics (the three-way
  log lives in core already).
- RFC 019 — Command/Shortcut Palette and Accessibility (proposed) — shared
  shortcut registry and a11y conventions.
- RFC 032 — Text Editing Operation Model (proposed) — *optional* upgrade
  path for manual edit; not required for v1 of this workspace.

## Open Questions

- **Tab integration:** enum-of-tabs (a) vs parallel list (b). Recommend (a);
  confirm against the current tab-bar component's tolerance for change.
- **Auto-advance:** after resolving a conflict, should focus jump to the
  next unresolved one automatically? Propose: yes, behind a Behavior setting
  defaulting on.
- **R-then-L "Use Both":** add `resolve_both_order(id, order)` to core, or
  ship L-then-R only for v1? Propose: ship L-then-R; file the core follow-up.
- **Three-way open flow:** fold base selection into the explorer/open
  dialog now, or as an immediate follow-up RFC? Propose: follow-up, to keep
  this RFC focused on resolution.
- **Result region while unresolved:** show base, left, an empty placeholder,
  or a "choose a resolution" prompt? Propose: a neutral placeholder line
  with the prompt, since unresolved conflicts contribute nothing to
  `result_text()`.
