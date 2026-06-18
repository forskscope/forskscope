# RFC 068: Explorer Unaligned (Compact) View Mode

**Status.** Implemented (v0.151.0)
**Tracks.** An optional view mode that removes the empty spacer rows from the
two-pane Explorer so each pane packs its entries independently, for users who
prefer density over cross-pane row alignment. Bound to a user setting.
**Touches.** `crates/forskscope-ui/src/ui/explorer.rs` (row generation:
`compute_aligned_rows` and a new unaligned path), `state/settings.rs` (the
mode setting), `assets/main.css`, i18n.

## Summary

The Explorer currently aligns same-name entries across the two panes, inserting
**spacer rows** where an entry exists on one side but not the other. This
alignment is structural: row N on the left corresponds to row N on the right,
which is what makes same-row comparison and vertical scroll-sync meaningful.

The maintainer has requested an option to **remove the empty spacer rows** so
each pane packs its entries independently. This RFC specifies that as an explicit
**view mode** (not the default), because it changes the core mental model: with
spacers removed, row N on the left no longer corresponds to row N on the right.

## Motivation

In directories where many entries are one-sided (e.g. comparing two loosely
related trees), the aligned view is mostly spacer rows — a lot of vertical
whitespace to scroll past. Packing each pane independently is denser and, for
some workflows, easier to scan.

## The trade-off (recorded explicitly)

This is a genuine design tension, captured here so it is a conscious choice:

| | Aligned (default) | Unaligned (this RFC) |
|---|---|---|
| Row N ↔ Row N | Corresponds across panes | Does **not** correspond |
| Spacer rows | Present (whitespace where one side missing) | None (dense) |
| Vertical scroll-sync | Meaningful (same-name rows track) | Not meaningful (panes independent) |
| Same-name digest icons | Read across a row | Must be matched by name, not row |
| Best for | Comparing similar trees | Scanning dissimilar/large trees densely |

Because unaligned mode breaks row correspondence, **vertical scroll-sync is
disabled in this mode** (each pane scrolls independently), and the same-name
status indicator must be attached to the entry by name rather than implied by
row position.

## Design

### Setting

A view-mode setting, e.g. **Explorer layout: Aligned (default) / Compact**,
persisted with other settings. (Name TBD — "Compact" vs "Unaligned" vs
"Independent panes"; pick the clearest user-facing term.)

### Row generation

- **Aligned mode (default):** unchanged — `compute_aligned_rows` produces the
  paired rows with spacers. (Note: the Explorer owns both tree signals so this
  operates on a consistent snapshot; that invariant is preserved.)
- **Compact mode:** each pane generates its own row list from its own tree with
  no spacers. The two lists are independent and may differ in length.

### Status indicators in compact mode

Same-name equality (✓ / ⚠) can still be shown, but it must be computed by
name-keyed lookup (does an entry with this name exist on the other side, and is
it equal?) rather than by reading the adjacent row. The digest map is already
name-keyed, so this is a rendering change, not a new computation.

### Scroll behaviour

- Aligned mode: vertical scroll-sync as today.
- Compact mode: independent vertical scroll per pane. (Horizontal scroll is
  per-pane in both modes once RFC-064 lands.)

### Keyboard / focus

The focused-pane model (RFC-061) already makes each pane independently
navigable, so compact mode needs no new keyboard design — F6 switches panes,
arrows move within the focused pane's (now independent) list.

## Relationship to filtering (RFC-067)

In aligned mode, filters preserve pairs (hide both sides of an identical pair).
In compact mode, each pane filters independently (a name-pattern hides
non-matching rows per pane). The filter code must branch on the active mode.

## Non-goals

- This RFC does not remove or change the aligned default. Aligned remains the
  default and the recommended mode for comparing similar trees.
- It does not add a third layout (e.g. single merged list). Two modes only.

## Acceptance criteria

- A setting switches the Explorer between Aligned (default) and Compact.
- In Compact mode there are no spacer rows; each pane packs independently.
- In Compact mode vertical scroll is independent per pane; same-name status is
  still shown (name-keyed).
- Switching modes does not lose the current directories or picks.
- Aligned mode behaviour is byte-for-byte unchanged from today.

## Cross-references

- RFC-061 — focused-pane model (makes per-pane navigation already work).
- RFC-064 — per-pane horizontal scroll (compact mode relies on per-pane scroll).
- RFC-067 — filters (semantics branch on mode).
- Memory/architecture note: "Explorer must own both tree signals for
  `compute_aligned_rows()` to operate on a consistent snapshot" — preserved; the
  compact path also reads both signals from the same owner.

## Open questions

- User-facing name for the mode: "Compact" / "Unaligned" / "Independent panes".
- Should compact mode still attempt *any* alignment (e.g. sort both panes the
  same way so same-name entries often line up by coincidence), or make no
  alignment promise at all? Recommend: no promise, just sort each pane
  consistently (name order) so it is predictable.
