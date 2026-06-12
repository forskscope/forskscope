# Known UI Issues

Issues identified during manual testing that are deferred for a near-future
release. Each entry records the symptom, root cause analysis, and the
recommended fix approach.

---

## ISSUE-001 — Compare pane: scroll bar operates per-line, not per-pane

**Reported:** v0.92.0 (2026-06-12)  
**Resolved:** v0.94.0 — Approach B implemented (shared single scroll bar).  
**Component:** `crates/forskscope-ui/src/ui/hunk.rs`, `assets/main.css`

### Symptom

In the diff compare view, each `.diff-pane` element (`left`/`right`) has
`overflow-x: auto` set directly on it. Because `.diff-pane` is an inline
flex container that wraps a single line of content (gutter + mark + cell),
the scroll bar appears per-line rather than once for the whole pane column.
Users see many tiny scroll bars, one per row, instead of a single scroll bar
at the bottom of each pane.

### Root cause

The `.diff-pane` element was intended to be a column container scrolling
independently, but the current DOM nests `.diff-pane` *inside* `.diff-row` —
one `.diff-pane` per row, not one `.diff-pane` per column. There is no
persistent left-column or right-column wrapper element that spans all rows.

### Recommended fix

Restructure the DOM so each column is a single scrollable container that
holds all rows, rather than having one `.diff-pane` per row.

Approach A — Column wrappers around all rows (preferred):

```
.diff-columns (display:flex, height: fill)
  .diff-col.left  (flex:1, overflow-x:auto, overflow-y:hidden)
    for each row: .diff-col-row (display:flex)
      .pane-gutter | .diff-mark | .cell
  .diff-act-col (flex:0 0 5ch, overflow:hidden)
    for each row: .diff-act-row
      [apply button or ✓]
  .diff-col.right (flex:1, overflow-x:auto, overflow-y:hidden)
    for each row: .diff-col-row (display:flex)
      .pane-gutter | .diff-mark | .cell
```

This requires the component to emit separate left-column, act-column, and
right-column elements rather than complete rows. Row alignment is maintained
by giving every `.diff-col-row` the same fixed `height: var(--line-h)`.

Approach B — CSS `overflow-x: auto` on the outer `.diff-scroll` with
`overflow-x: clip` on each `.diff-pane`:

This is simpler but reintroduces shared scrolling — both panes scroll
together horizontally, which is acceptable WinMerge behaviour. If independent
pane scrolling is not required, this approach restores a single scroll bar at
the bottom of the diff view.

### Acceptance criterion

A single horizontal scroll bar appears at the bottom of the left pane and a
single horizontal scroll bar at the bottom of the right pane (Approach A),
OR a single scroll bar appears at the bottom of the entire diff view and both
panes scroll together (Approach B — acceptable fallback for v1).

---
