# RFC-073 — Bridge Minimisation (Phase 5)

**Status.** Proposed
**Tracks.** `crates/forskscope-ui/src/ui/bridge/`.
**Touches.** All 14 files under `ui/bridge/`.

## Summary

Audit every file in `ui/bridge/` and either justify its existence with an
explicit comment or remove it. Bridge files are thin re-export adapters from
`forskscope-ui-logic`; they should not persist as permanent clutter.

This is Phase 5 of the UI structural redesign defined in RFC-071.

---

## Current bridge inventory

| File | Re-exports from | Justified? |
|---|---|---|
| `command_bar.rs` | `ui-logic/compare/command_bar` | Review |
| `compare_summary.rs` | `ui-logic/compare/summary` | Review |
| `conflict_nav.rs` | `ui-logic/compare/conflict_nav_view` | Review |
| `deep_filter.rs` | `ui-logic/explore/deep_filter` | Review |
| `explore_status.rs` | `ui-logic/explore/status` | Review |
| `explorer_align.rs` | `ui-logic/explore/align` | Review |
| `hunk_decorations.rs` | `ui-logic/compare/hunk_decorations` | Review |
| `load_guard.rs` | `ui-logic/compare/load_guard` | Review |
| `palette_view.rs` | `ui-logic/compare/palette_view` | Review |
| `save_error.rs` | `ui-logic/compare/save_error` | Review |
| `scroll_sync.rs` | `ui-logic/compare/scroll_sync` | Review |
| `search_index.rs` | `ui-logic/compare/search_index` | Review |
| `settings_view.rs` | `ui-logic/settings/settings_view` | Review |
| `tab_state.rs` | `ui-logic/compare/tab_state` | Review |

---

## Acceptance criteria

1. Every remaining bridge module has a doc comment explaining why it exists.
2. Bridge files that only re-export a single stable type with no vocabulary
   added are removed; the call sites import from `ui-logic` directly.
3. No new bridge files are added without a documented reason.

---

## Open questions

- Which bridge files are temporary (awaiting `ui-logic` API stabilisation)
  vs. permanent adapters? Decided per-file during implementation.
