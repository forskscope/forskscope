# RFC 061: Explorer Pane Focus and Keyboard Completeness

**Status.** Proposed — partially implemented in v0.145.2
**Tracks.** Explorer keyboard accessibility; left/right pane focus model;
making the stated keyboard-only path real for the Explorer's core
select-and-compare loop.
**Touches.** `crates/forskscope-ui/src/ui/explorer.rs` (keydown handler, pane
state), `crates/forskscope-ui/src/ui/dir_pane.rs` (tree row focus/aria),
`crates/forskscope-ui-logic` (any pure focus-resolution logic + tests),
`crates/forskscope-ui/assets/main.css` (pane focus ring).

## Summary

The Explorer presents two directory panes (left/right) as an aligned two-column
tree. Keyboard events on the aligned tree are currently dispatched only to the
**left** tree (`handle_key(&tree_l.read(), ...)` and events sent to `tree_l`).
The right pane has no keyboard path: a keyboard-only user cannot move focus into
it, expand its nodes, select a right-side compare pick, or navigate it.

This breaks a stated product guarantee. The handoff and RFC-019 claim all
primary workflows are keyboard-completable, but the Explorer select-and-compare
loop — pick a file on the left, pick a file on the right, Compare — cannot be
completed with the keyboard because only one side is reachable.

This RFC defines a **focused-pane model**: an explicit notion of which pane has
keyboard focus, a key to switch between panes, a visible focus indicator, and
assistive-technology announcement of pane identity. It is the Explorer half of
the keyboard-completeness work; the diff/merge half is already keyboard-complete
(F7/F8/Enter/Ctrl+Z etc.).

## Motivation

The UI/UX architect source review classified this as P0-3 (High/Critical):
the keyboard-only claim is materially false for the Explorer. For a tool whose
identity is "serious Unix/Linux workstation with first-class keyboard support,"
a half-keyboardable primary surface is a credibility gap, not a polish item.

It also has an accessibility dimension: a screen-reader user navigating by
keyboard has no way to operate the right pane at all.

## Design

### Focused-pane state

Add `focused_pane: Signal<Pane>` to the Explorer where `Pane = Left | Right`.
The aligned-tree keydown handler dispatches to `tree_l` or `tree_r` based on
`focused_pane`. All existing left-pane key behaviour is preserved when
`focused_pane == Left`; the same behaviour becomes available for the right pane
when `focused_pane == Right`.

### Pane switching

- **Tab / Shift+Tab** or **F6** switches the focused pane. (Tab is the
  conventional "move between regions" key; F6 is the conventional "cycle panes"
  key in desktop apps. Decide one as primary in the open questions; support at
  least one.)
- Switching focus must move the visible focus ring and announce the newly
  focused pane to assistive technology.

### Visible focus indicator

The focused pane shows a focus ring on its column (e.g. an outline on the
focused pane's `pane-root-cell` and tree column, using `var(--focus)`). The
unfocused pane is visually quieter. This must be visible in all three themes.

### Keyboard operations per focused pane

When a pane is focused, these operate on that pane (mirroring today's left-only
behaviour):

- Up/Down: move row focus
- Right/Left: expand/collapse (or move to parent/child)
- Enter: open directory as new root / open same-name file compare
- Space: select this row as that pane's compare pick
- Home/End: first/last row
- Alt+Up: navigate that pane to its parent (see RFC's relationship to the
  per-pane vs coupled Alt+Up question below)

### ARIA / assistive technology

- Each pane column is a labelled region: "Left / Old pane", "Right / New pane".
- Tree rows expose `aria-selected` (pick state) and `aria-expanded` (directory
  state).
- The focused pane is announced on switch.

This extends the row-semantics work already owned by RFC-019; this RFC adds the
pane-level focus layer specifically.

## Relationship to Alt+Up coupling

The UI/UX architect review (handoff P1-6, source Q8) also flagged that `Alt+↑`
currently navigates **both** panes simultaneously, which is surprising. With a
focused-pane model in place, the natural resolution is:

- `Alt+↑` navigates the **focused pane only** (the new default).
- A coupled "move both" action, if desired, becomes an explicit separate
  shortcut (e.g. `Ctrl+Alt+↑`) or an explicit "link folders" toggle.

This RFC adopts per-pane `Alt+↑` as the default once `focused_pane` exists.
The coupled behaviour is downgraded to an optional explicit action; whether to
keep it at all is an open question.

## Non-goals

- This RFC does not redesign the tree's visual row layout (owned by RFC-054).
- It does not add the "Open folder" hover/focus row action (clarity work,
  belongs to RFC-063).
- It does not implement the modal focus-trap (RFC-060 cross-reference).

## Acceptance criteria

- A keyboard-only user can: focus the left pane, pick a file; switch to the
  right pane, pick a file; trigger Compare — without using the mouse.
- The focused pane is visually indicated in all three themes.
- Pane identity and row selected/expanded state are exposed to assistive
  technology.
- `Alt+↑` affects only the focused pane by default.
- Pure focus-resolution logic (which pane, which row) is unit-tested in
  `forskscope-ui-logic`.

## Cross-references

- RFC-019 — command/shortcut/accessibility (owns shortcut map and row ARIA).
- RFC-054 — Explorer tree-view and interaction model (owns visual rows).
- RFC-059 — recorded H2/H3 "Explorer keyboard completeness" as deferred; this
  RFC is where that deferred work lands.
- RFC-060 — global keyboard scope (focus tracking shares mechanism).

## Open questions

- Primary pane-switch key: Tab vs F6. Tab is more discoverable; F6 avoids
  colliding with any future in-tree Tab use. Recommend F6 primary, Tab
  secondary — confirm during implementation.
- Keep coupled `Alt+↑` (move both panes) at all, or drop it? It has a narrow
  use (sibling directory trees) but adds surface. Lean toward dropping unless a
  user need is demonstrated.
