# RFC 069: Explorer Layout — Compare Action, Targets Label, and Tab Header/Footer

**Status.** Proposed
**Tracks.** Repositioning the Compare button and the label that shows its current
targets; rethinking the Explorer tab's header and footer regions in light of the
new filter bar (RFC-067) and view-mode control (RFC-068).
**Touches.** `crates/forskscope-ui/src/ui/explorer.rs` (header/footer layout,
Compare action placement), `assets/main.css`, i18n. Layout-only; no change to
comparison behaviour.

## Summary

As the Explorer gains a filter bar (RFC-067), a view-mode control (RFC-068), and
the focused-pane model (RFC-061, shipped), its header and footer regions are
accumulating controls without a deliberate layout. This RFC steps back and
defines where things go: the **Compare** action and its **targets label** (what
will be compared), and the overall **header/footer** composition of the Explorer
tab.

This is a layout/clarity RFC, not a behaviour change. It is deliberately
sequenced *after* RFC-067 and RFC-068 because those add the controls this RFC
must place.

## Motivation

The reported needs:

- "Position of Compare button and its targets label" — the Compare action and a
  clear statement of *what* it will compare need a sensible, discoverable home.
- "Tab page header / footer rethink (with consideration on the two above)" — the
  header/footer should be designed as a whole once the filter bar and view-mode
  control exist, rather than bolting each control on independently.

## Current state (to confirm during design)

The Explorer tab today has, roughly: a path bar per pane, the pane-root focus
bar (RFC-061), the two trees, and a footer with the Compare action and
copy/sync actions. Controls have been added incrementally. Before redesigning,
the implementation should inventory every control currently in the Explorer
header and footer so nothing is lost.

## Design direction

### Targets label (what will be compared)

When the user has made picks (left pick and/or right pick, via click or Space),
a **targets label** states exactly what Compare will act on, e.g.:

```
Compare:  left/path/main.rs  ↔  right/path/main.rs
```

When picks are incomplete, the label guides the next action ("Pick a file on the
right to compare"), consistent with the empty-state guidance work (RFC-063 C1).
The label removes ambiguity about which two things Compare will open — important
now that picks can be set by keyboard (Space) and mouse independently per pane.

### Compare action placement

The Compare button sits adjacent to the targets label so the action and its
subject are visually connected (button + "this is what it will do"). Candidate
locations: a dedicated action row in the footer, or a pinned bar directly under
the trees. Decide during design with the targets label as a unit — they move
together.

### Header/footer composition

Define the Explorer tab's regions as a deliberate stack, accounting for all
controls:

```
┌─ Explorer tab ─────────────────────────────────────────────┐
│ [optional] Filter bar (RFC-067)                            │  header
│ Path bar (L) │ Path bar (R)                                │
│ Pane-root focus bar (L) │ (R)   (RFC-061)                  │
├────────────────────────────────────────────────────────────┤
│ Tree (L)            │ Tree (R)                             │  body
├────────────────────────────────────────────────────────────┤
│ Targets label + Compare action                             │  footer
│ Copy/sync actions │ view-mode toggle (RFC-068) │ size unit │
└────────────────────────────────────────────────────────────┘
```

The exact arrangement is the design work of this RFC; the principle is that
header carries *input/navigation* controls (filter, paths, focus) and footer
carries *action* controls (compare, copy, view mode), with the targets label
making the pending action explicit. Keep it calm (D-001): group related
controls, avoid a dense toolbar, use progressive disclosure where a control is
advanced.

## Non-goals

- No change to what Compare does, how picks are made, or the comparison itself.
- No change to the diff workspace header/footer (this is the Explorer tab only).
- Not a visual reskin — layout/placement and labelling only.

## Acceptance criteria

- The targets label clearly states what Compare will open, and guides the user
  when picks are incomplete.
- The Compare action is visually connected to its targets label.
- Every control previously in the Explorer header/footer is still present and
  has a deliberate place.
- The layout remains calm and uncluttered with the filter bar and view-mode
  control present.

## Cross-references

- RFC-061 — pane-root focus bar (already in the header).
- RFC-063 C1 — empty/guidance states (targets label borrows this tone).
- RFC-067 — filter bar (header control to place).
- RFC-068 — view-mode toggle (footer control to place).
- D-001 — calm default layout.

## Open questions

- Footer single-row vs two-row, given the number of action controls. Decide once
  RFC-067/068 controls are concrete.
- Whether the targets label is always visible (showing "no picks yet") or only
  appears once a pick exists. Lean toward always visible with guidance text, for
  discoverability.
