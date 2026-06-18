# RFC-073 — Bridge Minimisation (Phase 5)

**Status.** Implemented (v0.161.0)
**Tracks.** `crates/forskscope-ui/src/ui/bridge/`.
**Touches.** All 14 files under `ui/bridge/`; `ui.rs`.

## Summary

Audited all 14 bridge files. 12 had zero usages. 2 had active call sites
but added no vocabulary or isolation value. All 14 removed; `ui/bridge/`
and `ui/bridge.rs` deleted; `pub mod bridge` removed from `ui.rs`.

---

## Audit result

| File | Usages | Decision | Reason |
|---|---|---|---|
| `command_bar.rs` | 0 | Removed | Unused |
| `compare_summary.rs` | 0 | Removed | Unused |
| `conflict_nav.rs` | 0 | Removed | Unused |
| `deep_filter.rs` | 0 | Removed | Unused |
| `explore_status.rs` | 0 | Removed | Unused |
| `explorer_align.rs` | 1 | Removed | Call site migrated to direct `forskscope_ui_logic::compute_aligned_rows` |
| `hunk_decorations.rs` | 0 | Removed | Unused |
| `load_guard.rs` | 0 | Removed | Unused |
| `palette_view.rs` | 0 | Removed | Unused |
| `save_error.rs` | 0 | Removed | Unused |
| `scroll_sync.rs` | 0 | Removed | Unused |
| `search_index.rs` | 2 | Removed | Call sites migrated to direct `forskscope_ui_logic::MatchIndex` |
| `settings_view.rs` | 0 | Removed | Unused |
| `tab_state.rs` | 0 | Removed | Unused |

---

## Result

The `ui/bridge/` layer no longer exists. The four-layer hierarchy is now:

```
ui/
  component/   — reusable visual primitives (RFC-072)
  layout/      — persistent app shell
  view/        — main workspaces
  overlay/     — modals and safety guards
```

UI phases 4 and 5 are complete. RFC-072 and RFC-073 close the structural
redesign started in RFC-071.
