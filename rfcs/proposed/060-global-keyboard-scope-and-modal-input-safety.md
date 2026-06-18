# RFC 060: Global Keyboard Scope and Modal/Input Safety

**Status.** Proposed
**Tracks.** Keyboard event ownership; modal and text-input safety; prevention
of accidental merge/save/close actions.
**Touches.** `crates/forskscope-ui/src/app.rs` (root `onkeydown`),
`crates/forskscope-ui/src/ui/search.rs`, modal components in
`crates/forskscope-ui/src/ui/modals.rs` and `settings.rs`, plus UI-logic
regression tests.

## Summary

The application has a single global `onkeydown` handler on the app-root
element that owns all keyboard shortcuts (F7/F8 hunk navigation, Enter to apply
a hunk, Ctrl+S save, Ctrl+W close, Ctrl+Z/Y undo/redo, Ctrl+F search, Ctrl+/
help). Because the handler is global, keypresses can fire app-level actions
even when the user's attention is somewhere that should own the key — inside a
modal dialog, or while typing in a text field.

The v0.145.0 Sprint 0 safety patch fixed the two highest-severity instances
(modal-open guard in the root handler; `stop_propagation` in the search input).
This RFC ratifies that fix as a **policy**, defines the complete rule for
keyboard-event ownership, and specifies the remaining work and regression tests
so the class of bug cannot reappear as new shortcuts or new input surfaces are
added.

## Motivation

A diff/merge tool writes files. A keypress that silently applies a merge hunk
or saves a file while the user believes they are doing something else (typing a
search query, confirming a dialog) is a trust-destroying, potentially
data-losing event. The UI/UX architect source review classified two such bugs
as P0:

- **P0-1:** the root handler closed modals on Escape but continued to act on
  the active tab for every other key, so `Ctrl+S` behind an overwrite dialog,
  `Ctrl+W` behind Settings, or `Enter` behind any dialog fired on the tab
  behind the modal.
- **P0-2:** the search input handled `Enter` (advance match) but did not stop
  propagation, so the same keypress bubbled to the root and applied the focused
  merge hunk.

Both were fixed in v0.145.0. This RFC exists so the fix is not an ad-hoc patch
but a stated invariant with tests, because the global-handler architecture
makes regressions easy to introduce.

## Policy: keyboard-event ownership

The rule, in priority order, evaluated by the root handler on every keydown:

1. **Escape always belongs to the nearest dismissable surface.** If a modal is
   open, Escape closes it and the event stops. If a search bar is open (no
   modal), Escape closes the search bar. Otherwise Escape cancels any transient
   state.
2. **While any modal is open, the root handler ignores every non-Escape key.**
   Modal content owns its own keys. Global shortcuts (save, close, apply, undo)
   must not fire on the workspace behind the modal.
3. **While a text-entry surface is focused, the root handler must not fire
   character or Enter shortcuts.** Text-entry surfaces are: the search input,
   the path-bar editor, the Save-As path input, and the profile-name input.
   These surfaces stop propagation for the keys they consume (at minimum Enter,
   Escape, and their navigation keys).
4. **Otherwise, the active workspace owns the shortcut.** Requires an active
   tab; if none, the handler returns.

## Current state (shipped in v0.145.0)

- Rule 1 and Rule 2 are implemented in `app.rs`: `modal_open` is computed once;
  Escape closes the modal; all other keys early-return while a modal is open.
- Rule 3 is implemented for the **search input** (`search.rs` calls
  `e.stop_propagation()` on Enter and Escape). The path-bar editor and Save-As
  input are inside surfaces where the root handler already returns early (the
  path bar is only present in Explorer, where no tab is active; Save-As is
  inside a modal), so they are covered transitively today — but that coverage
  is incidental, not explicit.

## Proposed work (remaining)

### W1 — Make Rule 3 explicit, not incidental

Add `stop_propagation` for consumed keys to every text-entry surface directly,
so coverage does not depend on the surface happening to live somewhere the root
handler ignores. This protects against future refactors (e.g. a path editor
added to the diff toolbar, or an inline rename field in the tree).

### W2 — Target-type guard as defence in depth

Add a helper that returns early from the root handler when the event originates
from an `input`, `textarea`, `select`, `button`, or `contenteditable` element.
Dioxus 0.7 does not expose the DOM target type ergonomically on
`Event<KeyboardData>`; investigate whether a focused-element check via a small
`eval` shim or a tracked "text-entry focused" signal is the cleaner mechanism.
If neither is clean, W1 (per-surface `stop_propagation`) is sufficient and W2
may be withdrawn — record the decision here.

### W3 — Regression tests

Because the bug class is behavioural and the handler is GUI-coupled, add
UI-logic-level tests that assert the *decision*, not the rendering:

- Given a modal is open, a synthesized `Ctrl+S` / `Ctrl+W` / `Enter` produces
  no save / no close / no hunk-apply.
- Given search is focused, `Enter` advances the search index and produces no
  merge transaction on the active session.

These require factoring the handler's decision logic into a pure function
(e.g. `fn resolve_key_action(modal_open, search_focused, key, mods) -> Option<Action>`)
that the GUI handler calls and the tests can call directly. That refactor is
part of this RFC.

## Non-goals

- This RFC does not add a full modal focus-trap (Tab cycling within the modal);
  that is accessibility work owned by RFC-061's accessibility section and the
  existing RFC-019 (command/shortcut/accessibility). It is cross-referenced
  there.
- This RFC does not change which shortcuts exist, only when they are allowed to
  fire.

## Acceptance criteria

- The root handler ignores all non-Escape shortcuts while any modal is open.
- Every text-entry surface stops propagation for the keys it consumes.
- A pure decision function exists and is unit-tested for the modal-open and
  search-focused cases.
- No keypress in a text field or behind a modal can produce a merge, save, or
  tab-close.

## Cross-references

- RFC-019 (command/shortcut/accessibility) owns the shortcut map itself.
- RFC-061 (Explorer pane focus) shares the focus-tracking concern.
- The v0.145.0 CHANGELOG records the Sprint 0 partial implementation.

## Open questions

- Is a tracked "text-entry focused" signal (set on focus/blur of known inputs)
  cleaner than a per-surface `stop_propagation`, given Dioxus 0.7's event API?
  W1 is the safe baseline; W2 is the nicer-if-feasible alternative.
