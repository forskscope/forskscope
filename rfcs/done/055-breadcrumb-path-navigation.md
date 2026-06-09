# RFC 055: Breadcrumb Path Navigation

**Status.** Implemented (v0.36.0)

**Tracks.** Explorer path-bar navigation. Replaces the "to upper
directory" button with a clickable path breadcrumb.

**Touches.** `crates/forskscope-ui-dioxus/src/ui/dir_pane.rs` (the path
bar), explorer CSS in `assets/main.css`.

## Summary

Each explorer pane currently shows the current directory as text plus a
dedicated **Up** (`↑`) button to move to the parent. This RFC removes the
Up button and instead renders the path as a sequence of **clickable
segments** — the pattern used by GNOME Files (Nautilus) and most modern
file managers. Clicking a segment sets that ancestor directory as the
pane root; the trailing segment is the current root and is not a link.

## Motivation

A single Up button only moves one level at a time. To jump from
`/home/me/project/src/core/diff` back to `/home/me/project` takes four
clicks. A breadcrumb makes every ancestor reachable in one click and also
shows the full location at a glance, which the current truncated path
text does not communicate as clearly. Removing the Up button also reduces
the control count in the path bar, consistent with the "less is more"
UI principle.

## Goals

- Render the pane's root path as clickable ancestor segments.
- Clicking a segment re-roots the pane at that ancestor.
- Remove the dedicated Up button.
- Keep the existing back/forward history buttons (RFC-021 / nav history);
  re-rooting via breadcrumb pushes onto the same history stack.
- Keep keyboard navigation: `Alt+↑` continues to move up one level even
  though the button is gone (the shortcut is independent of the button).
- Degrade gracefully for very deep paths (truncate the middle, never the
  current segment).

## Non-Goals

- An editable path text box (typing a path). The existing path input
  field, if retained, is orthogonal; this RFC concerns the breadcrumb
  display only.
- Breadcrumb drop-downs showing sibling directories (a Nautilus feature).
  Out of scope for v1; can be a later enhancement.

## External Design

### Breadcrumb rendering

```text
 / home / me / project / src / core ▸
 └┬┘ └┬─┘ └┬┘ └──┬───┘ └┬─┘ └─┬──┘
  link link link  link  link  current (not a link)
```

- Each segment except the last is a button that re-roots the pane at the
  cumulative path up to and including that segment.
- The leading `/` (POSIX) or drive root (Windows) is itself a clickable
  segment that re-roots at the filesystem root / drive.
- The current (trailing) segment is rendered emphasized and is not
  clickable.

### Deep-path truncation

When the breadcrumb would overflow the pane width, collapse the middle
into an ellipsis segment:

```text
 / … / project / src / core
```

The ellipsis is itself clickable and, when activated, expands to show the
hidden ancestors (or, minimally for v1, re-roots one level up from the
first visible segment). The first segment (root) and the last two
segments are always shown.

### Interaction with history and Alt+↑

- Clicking a breadcrumb segment is a navigation event: it pushes the new
  root onto the per-pane history stack (same mechanism as choosing a
  directory), so Back returns to the previous root.
- `Alt+↑` remains bound and re-roots at the parent of the current root,
  also pushing history. The keyboard path and the breadcrumb path share
  one navigation function.

## Alternatives Considered

- **Keep the Up button and add the breadcrumb.** Rejected: redundant
  controls; the breadcrumb subsumes the Up button (clicking the
  second-to-last segment is exactly "go up one level").
- **Editable path text only (no breadcrumb).** Rejected: typing a path is
  slower for the common "jump to a visible ancestor" case and is less
  discoverable.

## Testing

- The navigation function that maps a clicked segment index to a target
  path is a pure function and is unit-tested: given a root path and a
  clicked segment index, it returns the correct cumulative ancestor path
  on both POSIX and Windows-style inputs.
- Re-rooting pushes history such that Back restores the prior root.
- Truncation preserves the root and the final two segments.

## Open Questions

- Exact behavior of the ellipsis segment when activated (expand in place
  vs. re-root). Proposed v1 default: re-root at the first hidden ancestor.
- Whether to retain the free-text path input alongside the breadcrumb or
  remove it. Proposed: retain it, since it serves the "paste a path"
  workflow the breadcrumb cannot.
