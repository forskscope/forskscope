# Review Request 079: F75(b) part 1 — delete four obsolete `ui-logic` view-models

**Governing task.** `dev-record/handoffs/010-f75b-delete-four-obsolete-view-models.md`
**Register.** F75(b), part 1. The four wirings are a separate handoff (§6).
**Baseline.** `main` at `770bd19` (docs: review 080 - Japanese Symlink label reworded)
**Commit.** `d69c83b`

## No falsification — a deletion has none, and the handoff says so

Per §7: *"There is no falsification for a deletion... Nothing consumed this code, so nothing can be shown to break."* What I ran instead, as the three required checks:

## 1. Test-count delta

| Crate | Before | After | Delta |
|---|---|---|---|
| `forskscope-core` | 697 | 697 | 0 |
| `forskscope-ui-logic` | 257 | 199 | **−58** |
| `forskscope-ui` (lib) | 76 | 76 | 0 |
| `forskscope-ui` (bin) | 76 | 76 | 0 |

Confirmed the "before" figure directly: `git stash` back to the pre-deletion tree, ran `cargo test -p forskscope-ui-logic --lib`, got 257; popped the stash, re-ran, got 199.

**One correction to my own arithmetic, caught before reporting it**: `grep -c '#\[test\]'` across the four deleted files gave 17+13+14+15 = 59, matching the handoff's "roughly 59." The real delta is 58. `command_bar.rs`'s own module doc comment contains the literal text `` Works in a `#[test]` without a display server. `` — a false-positive grep hit, not an actual test attribute. 16+13+14+15 = 58, matching the observed 257→199 exactly. Checked rather than reported the grep count as-is.

No other crate moved.

## 2. `grep` for every deleted public item

```
ToolbarSection, ToolbarItem, build_toolbar, TabStateSnapshot,
context_from_snapshot, ScrollSyncState, CompareStatusSummary,
DiffNavigationState
```

Zero matches across `crates/` for any of the eight, and zero matches for the four module paths (`compare::command_bar`, `compare::tab_state`, `compare::scroll_sync`, `compare::summary`) outside the deletions themselves. One near-miss worth naming: `scroll_sync` also substring-matches `install_hscroll_sync` in `crates/forskscope-ui/src/ui/view/diff.rs` — checked directly and confirmed it's the *unrelated* horizontal-sync mechanism §2 of the handoff itself names as scroll_sync's replacement, not a reference to the deleted module.

## 3. RFCs annotated

**Four files, not three** — the handoff's §5b groups RFC-003 and RFC-006 under one bullet ("RFC-003 / RFC-006 — the `CompareStatusSummary` view-model is removed") since both reference the workspace/diff-summary concern; I annotated both rather than picking one, so a reader who opens either finds the note. Stating the count plainly since §4 says "Three done/ RFCs."

- **`rfcs/done/035-scroll-sync-line-mapping-and-diff-decoration-engine.md`**:
  > **Note (2026-08-27).** The `ScrollAnchor` view-model (`forskscope-ui-logic::compare::scroll_sync`) was removed as obsolete (F75(b), handoff 010): the shipped UI achieves vertical sync with a single scroll container plus horizontal sync via `install_hscroll_sync` (mirroring `scrollLeft` between panes), so the two-independently-scrolling-panes model this view-model presumed is superseded, not outstanding.

- **`rfcs/done/019-command-shortcut-palette-accessibility.md`**:
  > **Note (2026-08-27).** The `ToolbarSection`/`ToolbarItem` view-model (`forskscope-ui-logic::compare::command_bar`) was removed as obsolete (F75(b), handoff 010): the toolbar is built directly in `ui/view/diff/toolbar.rs`, nothing adopted the parallel representation. The command palette's own view-model (`palette_view`) is unaffected and remains deferred.

- **`rfcs/done/003-dioxus-application-shell.md`** and **`rfcs/done/006-diff-merge-workspace.md`** (identical sentence in both):
  > **Note (2026-08-27).** The `CompareStatusSummary`/`DiffNavigationState` view-model (`forskscope-ui-logic::compare::summary`) was removed as obsolete (F75(b), handoff 010): change counts and tab titles are derived directly at their use sites (`diff.rs`, `state/tab.rs::tab_title`), both jobs the view-model existed to do.

Each inserted immediately after the file's `**Status.**` line — the first thing a reader sees, matching where the handoff says a future reader "looks for the scroll-sync view-model."

## 4. The dangling prose reference (§3)

`palette_view.rs`'s module doc called itself *"the search-filtered complement to `command_bar`"*. Reworded to describe what it does without naming the deleted module, and pointed at where the toolbar actually lives now:

> This produces the dynamic, search-filtered command list for the palette. The fixed toolbar is a separate, unrelated presentation built directly in `ui/view/diff/toolbar.rs` (handoff 010, F75(b) part 1 — the toolbar's own former view-model, `command_bar`, was deleted as obsolete; see RFC-019's note).

## 5. Scope discipline

- The four KEEPs (`palette_view`, `conflict_nav_view`, `save_error`, `load_guard`) — untouched, confirmed by the diff touching only `compare.rs`'s `mod` lines, `lib.rs`'s re-exports, `palette_view.rs`'s doc comment, and the four deleted files themselves.
- No allowlist gate added — correctly deferred to F75(b)'s last step, after the four KEEPs are wired.
- `forskscope-ui` — untouched (confirmed nothing there referenced any deleted item, per §3 above).
- `ROADMAP.md` — not touched.
- No test ported to another module to preserve a count.

## 6. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (clean — no dead-code or unused-import residue), `cargo test --workspace` (deltas above), `cargo xtask css --check`, `cargo xtask version-sync`, `cargo xtask i18n` (236 keys, unchanged), `cargo xtask rfc-sync` (unaffected — no `rfcs/proposed/` or `ROADMAP.md` changes), `git diff --check`. Pushed as `d69c83b`; CI dispatched on push.
