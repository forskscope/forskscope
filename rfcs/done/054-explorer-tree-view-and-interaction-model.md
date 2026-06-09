# RFC 054: Explorer Tree-View and Interaction Model

**Status.** Implemented (v0.36.0)

**Tracks.** Explorer workspace redesign. Replaces the flat two-pane
directory list (RFC-005) with an expandable tree view and revises the
selection / compare-launch interaction model.

**Touches.** `crates/forskscope-ui-dioxus/src/ui/dir_pane.rs`,
`crates/forskscope-ui-dioxus/src/ui/explorer.rs`, the
`dioxus-swdir-tree` dependency, and the explorer-related CSS in
`assets/main.css`.

## Summary

The current explorer renders each pane as a flat list of the current
directory's immediate entries. Navigating into a subdirectory replaces
the whole list. This RFC replaces that flat list with an expandable
**tree view** built on `dioxus-swdir-tree` (`DirectoryTreeView` with its
scan driver), and revises the click model so that:

- **single click** selects an item and updates the label beside the
  **Compare** button;
- **double click** starts a comparison when the selected item has a
  counterpart on the opposite pane — either the *same item* (same
  relative path) or a *same-name file* — and otherwise expands a
  directory or does nothing for a file with no counterpart.

It also defines **same-name row alignment**: rows whose name matches an
entry on the opposite pane are positioned so the two panes line up
visually, making "what changed between these trees" legible at a glance.

This RFC also resolves a defect: the **Explorer tab button does not
return to the explorer workspace** in the current build.

## Motivation

A flat list forces users to drill in and out one directory at a time,
losing context about the surrounding tree. WinMerge-class tools show a
directory comparison as a single navigable tree where both sides scroll
together and same-name items align. `dioxus-swdir-tree` already provides
a scan-driven `DirectoryTreeView`, so the tree itself is not new code to
design from scratch — the design work is in the interaction model and the
two-pane alignment layered on top of it.

## Goals

- Render each explorer pane as an expandable directory tree via
  `dioxus-swdir-tree`.
- Single click selects; the selection drives the Compare-button label so
  the user always knows what "Compare" will act on.
- Double click launches a comparison when a counterpart exists on the
  other pane (same relative path, or same file name), expands a
  directory, or is a no-op for a counterpart-less file.
- Align rows that have a same-name counterpart on the opposite pane so
  the two trees read as a single side-by-side comparison.
- Fix the Explorer tab so it reliably switches back to the explorer
  workspace.
- Preserve existing digest equality indicators (✓ / ⚠ / one-sided).

## Non-Goals

- Recursive directory *merge* or synchronization (deferred; see the
  non-goals policy and future directory-sync RFCs).
- Replacing the separate deep-compare report view (RFC-037); the tree
  view is for interactive browsing, not the flat recursive report.
- Lazy infinite-depth scanning policy tuning beyond what
  `dioxus-swdir-tree` provides by default; performance limits for very
  large trees are tracked separately.

## External Design

### Pane layout

```text
┌─ LEFT / OLD ─────────────────┐   ┌─ RIGHT / NEW ────────────────┐
│ breadcrumb path (RFC-055)    │   │ breadcrumb path (RFC-055)    │
├──────────────────────────────┤   ├──────────────────────────────┤
│ ▸ 📁 components        ⚠      │   │ ▸ 📁 components        ⚠      │
│ ▾ 📁 core              ✓      │   │ ▾ 📁 core              ✓      │
│     📄 engine.rs       ⚠      │   │     📄 engine.rs       ⚠      │
│     📄 model.rs        ✓      │   │     📄 model.rs        ✓      │
│ 📄 main.rs             ⚠      │   │ 📄 main.rs             ⚠      │
│ 📄 old_only.rs   (left only) │   │ ░░░░░░░░░░░ (alignment gap)  │
│ ░░░░░░░░░░░ (alignment gap)  │   │ 📄 new_only.rs  (right only) │
└──────────────────────────────┘   └──────────────────────────────┘
 [Compare]  selected: core/engine.rs ↔ core/engine.rs
```

### Selection model

- A pane holds at most one **selected node** (a `RowId` keyed by the
  node's relative path within the pane root).
- Single click sets the selected node for that pane.
- The Compare-button label reflects the combined selection:
  - both sides selected → `left/rel ↔ right/rel`;
  - one side selected with a same-name counterpart resolvable on the
    other side → show the resolved pair;
  - otherwise → `select a file on each side`.

### Double-click compare resolution

On double click of node `N` in pane `P`:

1. If `N` is a directory → toggle expand/collapse (tree behavior).
2. Else (`N` is a file), resolve a counterpart on the opposite pane:
   1. **Same item**: a node at the *same relative path* exists on the
      opposite pane → compare `N` against it.
   2. **Same name**: a file with the *same file name* exists in the
      opposite pane's *currently visible scope* → compare against it.
   3. Otherwise → no comparison opens; surface a brief status hint
      ("no counterpart on the other side").

Rule (1) takes priority over (2) when both apply.

### Same-name row alignment

When both panes share a common parent scope, rows are ordered so that
entries present on both sides occupy the **same vertical row index** in
each pane. An entry present on only one side reserves an alignment gap
(rendered as a muted placeholder row) on the opposite pane. Alignment is
computed per expanded directory level, not globally, so expansion of one
subtree does not reflow unrelated siblings.

Open design question: alignment within *expanded* subtrees vs. only at
the root level. The conservative starting point is **root-level
alignment with per-level alignment inside expanded directories**, matching
the digest-icon scope already in place.

### Defect: Explorer tab does not switch back

The Explorer tab must set the active-workspace signal to the explorer
(no active comparison tab). The fix is verified by: opening a diff tab,
clicking Explorer, and confirming the explorer workspace renders and the
diff tab remains available for re-selection.

## Alternatives Considered

- **Keep the flat list, add a separate tree report.** Rejected: two
  different directory UIs increase surface area and contradict the
  "less is more" principle. The deep-compare report (RFC-037) already
  covers the flat recursive case; interactive browsing should be the
  tree.
- **Global cross-pane alignment of the entire recursive tree.** Rejected
  for v1: expensive and visually unstable as subtrees expand. Per-level
  alignment is simpler and matches how users reason about one directory
  level at a time.

## Testing

- Selection state: single click updates selection; the resolved compare
  pair is computed correctly for same-path and same-name cases.
- Double-click resolution priority: same-path beats same-name.
- Counterpart-less file double click is a no-op with a status hint.
- Explorer tab switches back to the explorer workspace from a diff tab.
- Alignment: a left-only file produces an alignment gap on the right at
  the same row index (unit-testable on the alignment computation, which
  should live in a pure function independent of the Dioxus tree widget).

## Open Questions

- Exact alignment behavior inside deeply nested expanded subtrees.
- Whether the tree's scan driver should pre-expand the first level or
  start fully collapsed (proposed default: first level expanded, matching
  current immediate-entry behavior).
