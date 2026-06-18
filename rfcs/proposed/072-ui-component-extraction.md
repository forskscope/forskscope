# RFC-072 — UI Component Extraction (Phase 4)

**Status.** Proposed
**Tracks.** `crates/forskscope-ui/src/ui/` — reusable visual primitives.
**Touches.** `ui/view/`, `ui/overlay/`, new `ui/component/`.

## Summary

Extract reusable visual primitives from view and overlay files into a
dedicated `ui/component/` layer. Only components that meet a proven-usage
threshold qualify.

This is Phase 4 of the UI structural redesign defined in RFC-071.

---

## Acceptance criteria

A component qualifies for extraction when:

1. Used by **at least two views**, or **one view plus one overlay**.
2. Props are UI-oriented and plain — no business logic, no signal writes.
3. The extracted file does not import from `crate::state` directly.

---

## Shipped in v0.160.0

### `component/notice.rs` — `Notice` + `NoticeKind`

Extracted from `ui/view/diff.rs` (4 uses) and `ui/overlay/modals/copy.rs`
(4 uses). Normalises `notice`, `notice-ok`, `notice-warn`, and `notice-err`
CSS classes into a typed `NoticeKind` enum. Warning and error variants
automatically carry `role="alert"`.

---

## Deferred candidates

| Candidate | Reason deferred |
|---|---|
| `EmptyState` | Single use (`explorer/tree.rs`) — does not meet threshold |
| `PathLabel` | Inline `<code class="path-display">` — not a component boundary |
| `IconButton` | Every toolbar button has a unique handler; wrapper adds indirection without reducing duplication |

These may be revisited if usage grows.

---

## Folder layout after Phase 4

```
ui/
  component/
    notice.rs
  component.rs          ← pub mod declarations
```
