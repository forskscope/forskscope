# Developer Handoff 010 — F75(b), part 1: delete four obsolete view-models

**From:** architect
**Date:** 2026-08-27
**Register:** F75(b). The four **wirings** are not in this handoff — see §6.
**Gate:** Not a Gate D blocker.

---

## 1. Task title

Delete `command_bar`, `tab_state`, `scroll_sync` and `summary` from
`forskscope-ui-logic`, with their tests and re-exports, and record why in the
RFCs that produced them.

## 2. Purpose

F75 found nine `ui-logic` modules with no consumer anywhere. `explore/status` was
wired (handoff 007). Of the remaining eight, I decided four **wire** and four
**delete** on 2026-08-27; this handoff is the four deletions.

These four are **obsolete, not pending.** Each describes a job the product now
does another way:

| Module | Why it is obsolete |
|---|---|
| `command_bar` | A real toolbar ships at `ui/view/diff/toolbar.rs`. `ToolbarSection`/`ToolbarItem` is a parallel representation nothing adopted. |
| `tab_state` | Exists only to derive `CommandContext` for `build_toolbar`. Dies with it. |
| `scroll_sync` | The UI uses **one** vertical scroll container (`.diff-scroll`) plus `install_hscroll_sync` mirroring `scrollLeft`. RFC-035's `ScrollAnchor` presumes **two independently scrolling panes**. The architecture diverged. |
| `summary` | Change counts are computed inline in `diff.rs` (`changes: ids.len()`); tab titles by `state/tab.rs::tab_title`. Both of its jobs are already done. |

**Precedent:** F48 deleted two layers once *"no product justification [was] found
for wiring them through"* rather than keeping them against a someday. This is the
same call, four times.

## 3. Dependencies — checked, and there are none

I verified before writing this, because a wrong answer here breaks the build:

- **`CommandContext` lives in `forskscope-core`** (`core/src/command.rs:219`), not
  in `tab_state`. `palette_view`, which is a **KEEP**, therefore survives.
- `conflict_nav_view`'s `summary` field is its own `NavigatorSummary`, unrelated
  to the `summary` module.
- Every reference to the eight deleted public items outside their own files is a
  re-export line in `lib.rs`.

**One prose reference will dangle**, and it is the only edit needed outside the
deletions: `palette_view.rs:7` calls itself *"the search-filtered complement to
`command_bar`"*. Reword it to describe what it does without naming a module that
no longer exists.

## 4. Change scope

- **Delete:** `compare/command_bar.rs`, `compare/tab_state.rs`,
  `compare/scroll_sync.rs`, `compare/summary.rs`
- `compare.rs` — their `mod` declarations
- `lib.rs` — their `pub use` re-exports
- `compare/palette_view.rs` — the dangling doc reference (§3)
- Three `done/` RFCs — one sentence each (§5b)

## 5. Required implementation

### 5a. The deletions

Modules, their `mod` lines, their re-exports, and their tests — roughly 59 tests
go with them, which is correct and not a regression. **Do not port a test to
another module to preserve a count.**

### 5b. Record it where the next reader will look

A future reader finds RFC-035 marked *Implemented*, looks for the scroll-sync
view-model, and finds nothing. **Add one sentence to each `done/` RFC whose
view-model this removes**, saying what was removed and why:

- **RFC-035** — the `ScrollAnchor` view-model is removed; the shipped UI achieves
  vertical sync with a single scroll container and horizontal sync via
  `install_hscroll_sync`, so the two-pane anchor model is superseded rather than
  outstanding.
- **RFC-019** — the `ToolbarSection`/`ToolbarItem` view-model is removed; the
  toolbar is built directly in `ui/view/diff/toolbar.rs`. **Say nothing about the
  command palette**, which is still deferred and whose view-model is kept.
- **RFC-003 / RFC-006** — the `CompareStatusSummary` view-model is removed; change
  counts and tab titles are derived at their use sites.

Keep each to a sentence with the date. These are historical records; you are
annotating them, not rewriting them.

## 6. Explicit non-change scope

- **Do not wire anything.** `palette_view`, `conflict_nav_view`, `save_error` and
  `load_guard` are **KEEP** and stay exactly as they are. Their wirings are
  separate handoffs; two of them (palette, conflict workspace) are post-v1 per
  `ROADMAP.md:51`.
- **Do not add the no-allowlist gate.** It cannot land until the four KEEPs are
  wired — before that it would need the baseline F75 argues against. It is the
  **last** step of F75(b), not this one.
- **Do not touch `forskscope-ui`.** Nothing there references these.
- Do not edit `ROADMAP.md`.

## 7. Required tests

**There is no falsification for a deletion, and I would rather say so than invent
ceremony.** Nothing consumed this code, so nothing can be shown to break.

What must be true instead:

1. `cargo test --workspace` passes, with `forskscope-ui-logic`'s count down by
   exactly the deleted modules' tests and **every other crate unchanged**.
2. `cargo clippy --workspace --all-targets -- -D warnings` is clean — in
   particular no dead-code or unused-import warnings left behind by a partial
   removal.
3. `grep` for each deleted public item across `crates/` returns nothing.

If any of those surprises you, stop: it means something did depend on this and
my §3 analysis was wrong.

## 8. Acceptance criteria

- The four files, their `mod` lines and their re-exports are gone.
- No dangling reference remains, prose included.
- The three RFCs carry their note.
- Gates green: `fmt`, `clippy --workspace --all-targets -- -D warnings`,
  `test --workspace`, `xtask css --check`, `xtask version-sync`, `xtask i18n`,
  `xtask rfc-sync`, `git diff --check`.

## 9. Prohibited shortcuts

- **Do not delete a KEEP module** because it is also unwired. Four go; four stay.
- **Do not leave a `pub use` behind** pointing at nothing — it will not compile,
  which is fine, but a half-removal that *does* compile (module gone, re-export
  gone, doc reference dangling) is the failure mode to avoid.
- **Do not preserve test counts** by moving tests elsewhere.

## 10. Required review-request format

Short. State plainly:

- the test-count delta per crate, and that no other crate moved;
- that `grep` finds no surviving reference to any deleted item;
- which RFCs you annotated and the sentence you used.
