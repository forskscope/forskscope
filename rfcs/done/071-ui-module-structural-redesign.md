# RFC-071 — UI Module Structural Redesign (Rust 2024 Hierarchy)

**Status.** Implemented (v0.152.0–v0.158.0)
**Tracks.** `crates/forskscope-ui/src/` module structure.
**Touches.** Every `.rs` file under `crates/forskscope-ui/src/`.

## Summary

Restructures the `forskscope-ui` crate from a flat file layout into a
four-layer Rust 2024 module hierarchy, splits every file that exceeded the
300 ELOC soft limit, and eliminates all `mod.rs` files in favour of
sibling-file module declarations.

The work shipped across seven releases (v0.152.0–v0.158.0) with no
user-visible behaviour changes except one bug fix discovered during the
process (see §Bug fixes).

---

## Motivation

After the Tauri → Dioxus migration the `crates/forskscope-ui/src/ui/`
directory was a flat list of 27 files with no grouping. Several files
exceeded 700 ELOC. Navigating the codebase required reading prose
descriptions rather than relying on module structure. The architect review
(v0.152.0) defined a four-layer layout and a five-phase migration plan.

---

## Design

### Layer map

```
ui.rs
ui/
  layout/     Persistent app shell: header, tab bar, status bar
  view/       Main user-facing workspaces: Explorer, Diff, Settings, …
  overlay/    Modals, keyboard help, safety guards
  bridge/     Thin re-export adapters from forskscope-ui-logic
```

### Module style

Rust 2018/2024 sibling-file style throughout (`foo.rs` + `foo/` coexist;
no `mod.rs` anywhere under `forskscope-ui` or `forskscope-ui-logic`).

### ELOC soft limit

300 ELOC per file (soft); 500 ELOC (strongly recommended split).

### Test placement

Per-module `tests.rs` files alongside each source module they test,
declared with `#[cfg(test)] mod tests`. A single monolithic `tests.rs`
at crate root is not used.

---

## Phases shipped

### Phase 1 — Establish hierarchy (v0.152.0)

- `ui/mod.rs` replaced by `ui.rs` with four `pub mod` declarations.
- 27 files moved into `layout/`, `view/`, `overlay/`, `bridge/`.
- `state/mod.rs` → `state.rs`.
- Backward-compatible `pub use` re-exports added with `TODO(v0.153)` tags.

### Import migration (v0.153.0)

- All `crate::ui::X` paths updated to explicit `crate::ui::{layer}::X` paths.
- `TODO(v0.153)` re-exports removed from `ui.rs`.
- `forskscope-ui-logic` `mod.rs` files converted to sibling files.

### Phase 2 — Split large files (v0.154.0–v0.156.0)

| File | Before | After | Subfiles |
|---|---|---|---|
| `view/explorer.rs` | 704 ELOC | 251 | `tree`, `compact`, `filter`, `footer` |
| `view/diff.rs` | 339 ELOC | 211 | `toolbar` |
| `view/settings.rs` | 310 ELOC | 46 | `modal`, `profile` |

### Phase 2 continued — Split state and overlays (v0.157.0–v0.158.0)

| File | Before | After | Subfiles |
|---|---|---|---|
| `state.rs` | 427 ELOC | 75 | `types`, `tab`, `compare`, `session`, `profile` |
| `overlay/modals.rs` | 303 ELOC | 12 | `file`, `tab`, `copy`, `about` |

---

## Bug fixes discovered during restructuring

- **Loading state never transitions to diff view** (v0.158.0). `open_compare`
  called `spawn(...)` which scopes the task to the calling component. When
  `ExplorerFooter` unmounts (Explorer replaced by DiffWorkspace), Dioxus
  cancels the in-flight load task. Fixed: `spawn_forever(...)` at both call
  sites in `state/compare.rs`.

- **"Copy value hoisted" runtime warning** (v0.158.0). `Store` signals were
  created in `App`'s scope but written from `ScopeId(0)` (root) via
  `spawn_forever`. Fixed: `Signal::new_in_scope(value, ScopeId::ROOT)` for
  all nine `Store` fields.

- **`binary_cache` not cleared on directory navigation** (v0.152.0 audit).

- **Filter loop bypassing cache** (v0.152.0 audit).

---

## Result

No file in `forskscope-ui` exceeds the 300 ELOC soft limit. No `mod.rs`
remains in `forskscope-ui` or `forskscope-ui-logic`. Tests live in
per-module `tests.rs` sibling files. The `bridge/` layer is explicit and
bounded. Phases 4 (component extraction) and 5 (bridge minimisation) are
deferred to RFC-072 and RFC-073.

---

## Open questions

None. Phases 4 and 5 are tracked by RFC-072 and RFC-073 respectively.
