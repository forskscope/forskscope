# RFC 067: Explorer Name-Pattern Filter and Filter Checkboxes

**Status.** Proposed
**Tracks.** A view filter for the Explorer to cope with directories containing
very many files: a name-pattern text filter plus a small set of high-value
filter checkboxes (e.g. hide binary, hide identical).
**Touches.** `crates/forskscope-ui/src/ui/explorer.rs` (filter bar + row
filtering), `dir_pane.rs` (row visibility), `state/settings.rs` (if filter
state persists), `assets/main.css` (filter bar styling), i18n.

## Summary

In directories with hundreds or thousands of entries, scanning the two-pane
tree for the files of interest is slow. This RFC adds a **filter bar** to the
Explorer: a name-pattern text input that narrows visible rows live, plus a few
checkboxes for common filters. The bar is compact and unobtrusive, consistent
with the "less is more" / calm-default principle (D-001) — it does not become a
panel of a dozen toggles.

## Motivation

The reported need: "filter shown items by name pattern to optimize view in a
directory where there are too many files." A name filter is the highest-value,
lowest-risk Explorer ergonomic improvement for power users working in large
trees. The companion idea — filter checkboxes such as "non-binary only" — pairs
naturally with the binary policy (RFC-066).

## Design

### Filter bar

A single compact row above the panes (or within the existing path-bar area),
containing:

- A **name-pattern** text input. Matching is live, case-insensitive, substring
  by default; glob (`*`, `?`) support is an open question (see below).
- A small number of **filter checkboxes** (see below).
- A clear (`✕`) affordance to reset the filter.

The bar follows the calm-default principle: empty/neutral by default, no
visual noise when unused. Consider making it toggleable (a filter icon that
reveals the bar) if always-visible feels heavy — decide in implementation.

### Filter checkboxes (initial set)

Start with the two highest-value filters; do **not** ship a large grid (D-001):

| Filter | Effect |
|---|---|
| **Hide binary** | Hide rows classified as binary (composes with RFC-066). |
| **Hide identical** | Hide rows whose digest is byte-identical on both sides (the ✓ rows). |

Additional filters (hide one-sided, hide directories, etc.) are deferred until a
demonstrated need; the framework should make adding one cheap, but the default
set stays minimal.

### Filtering semantics with the aligned two-pane model

The Explorer aligns same-name entries across panes with spacer rows. Filtering
must preserve alignment in the default (aligned) view:

- A name-pattern match is evaluated per entry; an aligned pair is shown if
  **either** side matches (so a renamed-on-one-side file isn't hidden because
  only one side matches). Confirm this "either side" rule in implementation.
- "Hide identical" removes the aligned pair entirely (both sides), keeping
  alignment for the remaining rows.
- Interaction with RFC-068 (unaligned/compact view) is noted there: in the
  unaligned view each pane filters independently.

### Persistence

Open question whether filter state persists across launches or resets per
session. Name-pattern almost certainly should *not* persist (it is a transient
search). The checkboxes *might* persist as preferences. Recommend: name-pattern
is session-only; checkboxes are session-only initially, promote to persisted
settings only if users ask.

## Non-goals

- Not a full query language; substring (+ maybe glob) only.
- Not a content search (that is the in-diff search, RFC-014). This filters the
  file *list* by name/metadata, not file contents.
- Not a large grid of filter toggles (D-001) — two to start.

## Acceptance criteria

- Typing in the name filter narrows visible Explorer rows live, case-insensitive.
- "Hide binary" and "Hide identical" checkboxes filter rows correctly while
  preserving cross-pane alignment in the aligned view.
- Clearing the filter restores the full list.
- The filter bar is unobtrusive when unused.

## Cross-references

- RFC-066 — binary policy (the "hide binary" filter composes with it).
- RFC-068 — unaligned/compact view (filtering semantics differ there).
- RFC-020 — directory listing/sorting/filtering/metadata model (core filter
  hooks, if any, live here).
- D-001 — calm default layout (keep the bar minimal).

## Open questions

- Glob (`*`, `?`) vs plain substring for the name pattern. Substring is simplest
  and usually enough; glob is more powerful but needs clear semantics. Recommend
  substring first.
- Always-visible filter bar vs reveal-on-demand (filter icon). Lean toward
  reveal-on-demand to honour the calm default.
- Persist checkbox state or not. Recommend session-only initially.
