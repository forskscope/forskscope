# RFC 064: Compare View — Per-Pane Horizontal Scroll and All-Different Coloring

**Status.** Implemented (v0.147.0)
**Tracks.** Two defects in the text diff workspace: (1) horizontal overflow on a
pane cannot be revealed by the current single shared scrollbar; (2) when no lines
match between the two files, no diff coloring is drawn.
**Touches.** `crates/forskscope-ui/src/ui/diff.rs`, `hunk.rs`,
`crates/forskscope-ui/assets/main.css` (`.diff-scroll`, `.diff-pane`,
`.diff-row`), and the diff-row class assignment logic.

## Summary

Two reported defects in the Compare workspace. They are unrelated in mechanism
but both live in the diff rendering path, so they are addressed together.

The first defect requires reversing a previously intentional design decision —
the single shared horizontal scrollbar ("Approach B", ISSUE-001) — in favour of
**per-pane horizontal scrollbars ("Approach A")**. That reversal and its
consequence for scroll synchronization are the substantive content of this RFC.
The maintainer has decided to proceed with Approach A.

## Defect 1 — Horizontal overflow cannot be revealed

### Current behaviour

The diff view uses one shared horizontal scrollbar owned by a `.diff-scroll`
wrapper that contains both panes. Both panes advance together when the row
overflows. The CSS comment records this as the deliberate "Approach B" from
ISSUE-001: a unified single-grid layout that makes horizontal scroll-sync
structural rather than coordinated by code.

### Problem

When the **left** pane has a long line, the shared scrollbar sits at the bottom
of the combined row. In practice the scrollbar is positioned such that it is
covered by / clipped against the parent window edge, so the user cannot drag it
to reveal the left pane's overflow. The content exists but is unreachable.

### Decision: switch to Approach A (per-pane horizontal scrollbars)

This reverses the intentional Approach B decision. The rationale recorded here:

- The shared-scrollbar model couples the two panes' horizontal positions, which
  is elegant for files whose long lines coincide but actively harmful when only
  one side has a long line — the side without overflow wastes the scroll range
  and the side with overflow may be unreachable (the reported bug).
- Per-pane scrollbars let each pane reveal its own overflow independently. This
  is the conventional behaviour of WinMerge, Meld, and VS Code's diff view.

### Consequence: horizontal scroll-sync becomes optional, not structural

Under Approach B, horizontal scroll-sync was free (one scrollbar). Under
Approach A, each pane scrolls horizontally on its own. Vertical scroll-sync is
unaffected (it is row-aligned and remains structural).

The question is whether to *also* synchronize horizontal scrolling between panes
in code (so dragging one pane's horizontal bar moves the other). Options:

- **A1 — Independent horizontal scroll (no sync).** Simplest. Each pane reveals
  its own overflow. The panes can show different horizontal offsets.
- **A2 — Synchronized horizontal scroll via scroll-event listeners.** Preserves
  the "both move together" feel of Approach B while fixing reachability. More
  code; requires a scroll-event handler that mirrors offset between panes and
  guards against feedback loops.

Recommendation: **A1 first** (fixes the bug with the least machinery and matches
WinMerge/Meld), with A2 as a possible follow-up if users miss coupled
horizontal motion. Record the decision in the implementation.

### CSS shape

```
.diff-pane.left, .diff-pane.right {
  overflow-x: auto;     /* each pane owns its horizontal scrollbar */
}
/* .diff-scroll wrapper no longer owns the shared scrollbar */
```

Vertical layout, row height alignment, and the centre act column are unchanged.

## Defect 2 — No coloring when all lines differ

### Current behaviour

Diff rows are coloured by kind: equal (none), delete (red), insert (green),
replace (amber), with `+`/`−`/`~` gutter marks. The colouring is applied per
hunk based on the diff algorithm's classification.

### Problem

When the two files share **no** matching lines (e.g. completely different
content, or a one-line file vs another one-line file with different text), the
diff may be represented in a way that renders without any per-line colour — the
rows appear plain, as if equal. The user cannot see that every line differs.

### Likely cause (to confirm in implementation)

When there is no common subsequence, the diff is a single large replace (or a
delete-block + insert-block) spanning the whole file. The row-class assignment
likely has a path where a whole-file replace, or the absence of any `equal`
anchor, falls through to the default (uncoloured) class. The fix is to ensure
every non-equal line receives its delete/insert/replace class regardless of hunk
size or whether any `equal` anchor exists.

### Fix

Audit the row-class assignment in `hunk.rs` / `diff.rs` so that:

- Every line in a delete hunk → delete class + `−` mark.
- Every line in an insert hunk → insert class + `+` mark.
- Every line in a replace hunk → replace class + `~` mark.
- This holds when the hunk spans the entire file and when there are zero `equal`
  lines anywhere.

Add a regression test at the `forskscope-ui-logic` level (pure row-model
construction) for the "no common lines" case: two inputs with no shared line
must produce a row model where every row is classed non-equal.

## Non-goals

- This RFC does not add a minimap or overview ruler (separate future work).
- This RFC does not change vertical scroll-sync (remains structural).
- A2 (coded horizontal scroll-sync) is optional and may be deferred.

## Acceptance criteria

- A long line in either pane can be fully revealed by that pane's own horizontal
  scrollbar.
- Vertical scrolling keeps both panes row-aligned.
- Two files with no matching lines render with every row coloured by its
  delete/insert/replace kind and the correct gutter mark.
- A regression test covers the all-different case.

## Cross-references

- ISSUE-001 / prior "Approach B" decision — this RFC reverses it; the reversal
  rationale is recorded above.
- `known-limitations.md` "Horizontal scroll is shared between both panes" — to be
  updated when this ships.
- RFC-025 (visual diff rendering model) — row-class assignment lives here.

## Open questions

- A1 vs A2 (independent vs coded-synchronized horizontal scroll). Recommend A1
  first; confirm during implementation whether coupled motion is missed.
