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
3. The extracted file does not import from `state/` directly.

---

## Candidate components

| File | Component(s) | Current locations |
|---|---|---|
| `component/notice.rs` | `Notice`, `WarningBanner` | `diff.rs`, `modals/file.rs`, `modals/copy.rs` |
| `component/empty_state.rs` | `EmptyState` | `explorer/tree.rs`, future dir compare |
| `component/path_label.rs` | `PathLabel` | `diff.rs` header, `explorer.rs` path bars |
| `component/icon_button.rs` | `IconButton` | toolbar, path bars, modal actions |

---

## Non-candidates (do not extract)

- `HunkBlock` — too tightly coupled to diff merge model.
- `TreeRow` — belongs with `dir_pane` tree logic.
- `FilterBar` — single-use, specific to Explorer.

---

## Folder layout after Phase 4

```
ui/
  component/
    empty_state.rs
    icon_button.rs
    notice.rs
    path_label.rs
  component.rs          ← pub mod declarations
```

---

## Open questions

None blocking start. Extraction is mechanical once candidates are confirmed
against the acceptance criteria above.
