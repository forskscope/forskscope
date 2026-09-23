# Review Request 096 — Handoff 026: F82 status vocabulary, RFC-063 delivery audit

Handoff: `dev-record/handoffs/026-f82-shared-status-vocabulary.md`
Commit: `1a44a2a` (pushed to `main`). CI run `34085948885`: green.

This handoff has two independent parts. Part A is an implementation
(F82); Part B is a pure audit with no code changes, per §5's explicit
instruction.

---

## Part A — F82: one shared status vocabulary

### §6's three falsifications, run for real

#### 1 — every concept renders the same glyph in both views

Falsified by changing one arm of `RowStatusKind::status_glyph` (Error →
`StatusGlyph::Different` instead of `Unreadable`):

```
thread 'explore::status::tests::explorer_and_deep_compare_render_overlapping_concepts_identically'
panicked:
assertion `left == right` failed: Error (Explorer) and Unreadable (Deep
Compare) must render the same concept
  left: Different
 right: Unreadable
```

This is the test that "does not compare the shared table to itself" —
it compares what `RowStatusKind` (Explorer) and `RecStatus` (Deep
Compare) each resolve to, for all six overlapping concepts. I also
falsified the Explorer-only path separately, by making
`RowStatusKind::glyph()` special-case `Error` back to a hardcoded `'!'`
instead of delegating:

```
thread 'explore::status::tests::row_status_kind_delegates_to_status_glyph_for_every_variant'
panicked: assertion `left == right` failed
  left: '!'
 right: '⊘'
```

#### 2 — removing either view's use of the shared table fails a test

Falsified by reverting Deep Compare's `status_glyph` helper (the
function `DeepRow` actually calls) to its old local hardcoded match:

```
thread 'ui::view::deep_compare::tests::status_glyph_matches_the_shared_vocabulary_for_every_status'
panicked: assertion `left == right` failed: Changed glyph must match the
shared table
  left: '⚠'
 right: '≠'

thread 'ui::view::deep_compare::tests::deep_compare_adopts_the_target_glyphs_not_its_old_ones'
panicked: assertion `left == right` failed
  left: '✓'
 right: '='
```

This is the F75 guard the handoff named directly: before this change,
Deep Compare had zero `forskscope-ui-logic` dependency for its status
glyphs at all — every value was a local, duplicated match. These two
tests fail specifically because `DeepRow`'s glyph now comes from calling
`forskscope_ui_logic::StatusGlyph::for_rec_status`, not because the
shared table changed.

#### 3 — every glyph still carries its accessible label

Falsified by making `StatusGlyph::Symlink`'s `aria_label()` return `""`:

```
thread 'explore::status::tests::all_status_glyph_aria_labels_are_non_empty'
panicked: Symlink must have a non-empty aria label
```

All three restored and green afterward; ran the full workspace suite
after each restoration, not just the one test that caught it.

### Design decisions disclosed

**`StatusGlyph` is a third, presentation-only type — `RowStatusKind` and
`RecStatus` are untouched and unmerged**, per §3's explicit instruction.
Each Explorer/Deep Compare status maps into one of eight `StatusGlyph`
variants (`Equal`, `Different`, `Computing`, `Unreadable`, `LeftOnly`,
`RightOnly`, `NotCompared`, `Symlink`); the two concepts each view alone
can produce (`NotCompared`, `Symlink`) still go through this table rather
than staying local, so there is exactly one definition of every glyph.

**Unreadable adopts Deep Compare's glyph (`⊘`) and CSS class
(`status-unreadable`); LeftOnly/RightOnly adopt Explorer's classes
(`status-left-only`/`status-right-only`) over Deep Compare's merged
`status-only`.** The handoff's own §2 table only specified the *Unreadable*
swap directly ("Deep Compare's wins here"). For LeftOnly/RightOnly it
said the two already agree glyph-wise and didn't name a CSS-class
divergence — but "one shared table" means one `css_class()` per concept,
and Deep Compare's `status-only` (one class covering both directions,
distinguished only by glyph) and Explorer's two distinct classes were
never actually the same value. I picked Explorer's more specific pair
because it costs nothing (the two rules were already visually identical
— `color: var(--muted)` — in both views) and is strictly more
information for anyone who later wants to style the two directions
differently, where merging them back down later would have been a real
information loss. Changed in `12-view-directory-report.css`:
`.status-only` → `.status-left-only, .status-right-only` (same rule
body); Deep Compare's own component code required no change here since
it already called through the shared table for these two.

**Found and removed while in the file: an orphaned `.status-err` CSS
rule** in `12-view-directory-report.css`, left over from an earlier
`status-err`→`status-unreadable` rename (F79) that never deleted the old
selector. Confirmed unreferenced by any Rust code before removing it.

**`cargo xtask css`**, not hand-editing: `assets/main.css` regenerated
after every split-source change; `cargo xtask css --check` and the new
`css_coverage` test (below) both green.

**Added a new `css_coverage` test** (`status_glyph_css_classes_defined_in_main_css`)
asserting `main.css` defines all eight `StatusGlyph` CSS classes — the
same shape this test file already used for conflict-navigator classes,
extended to cover the new shared table so a class typo or a forgotten
CSS update fails a test rather than shipping an unstyled status silently.

### Scope

Touched exactly what §7 named: `ui-logic/src/explore/status.rs`,
`ui/src/ui/view/deep_compare.rs`, the new shared table, CSS classes
(they did consolidate, per above), tests — plus `ui-logic/src/lib.rs`
(re-exporting `StatusGlyph`) and `ui-logic/tests/css_coverage.rs` (the
new coverage test). Did not touch `RowStatusKind`'s or `RecStatus`'s
variant sets, and did not implement anything from RFC-080's tiered
comparison.

### Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core / 123 ui-lib / 123 ui-bin /
204 ui-logic / 5 css_coverage, all green — +2 ui-lib tests, +4 ui-logic
tests, +1 css_coverage test over the pre-handoff baseline), `cargo xtask
css --check` (main.css regenerated via `cargo xtask css`, not
hand-edited), `version-sync`, `i18n` (244 keys, unchanged), `rfc-sync`,
`audit-deps`, `git diff --check`, `mdbook build docs` — all green.
`cargo audit`: exit 0, the same 14 pre-existing warnings as the last
several reviews (no dependency added).

---

## Part B — RFC-063 delivery audit

Pure audit per §5: **no code was changed for this part.** Every verdict
below is read from the current tree, with file:line evidence. C2 and C5
were already spot-checked by the architect and confirmed shipped
(`--control-h`/`--row-h` density variables; `ui/component/notice.rs`'s
severity model) — not re-derived here, just listed for completeness.

| Item | Verdict | Evidence |
|---|---|---|
| **C1** — Empty/first-run states | **Partly shipped** | The "no compare picks" hint ships, and better than the RFC's own spec: `ui/view/explorer/footer.rs`'s `ExplorerFooter` shows progressive guidance ("Choose a file or folder on each side" → "...on the right" → both names), not just a static line. `ui/view/explorer/tree.rs:122-133` renders an `explorer-empty` state (icon + "Compare files or folders" + hint + "🔒 Nothing leaves this computer.") when the aligned tree has zero rows — covers the general "nothing to show" case, including first launch. **Gaps against the literal spec:** no distinct "this folder is empty, go up a level" message for a single empty directory (the general empty-state text talks about choosing folders, which reads oddly if folders are already chosen and just empty); no persisted "seen it once" flag anywhere in `state/settings.rs` — grepped for `first_run`/`seen_onboarding`/`dismissed`, zero hits — so `explorer-empty` reappears every time the tree is empty, not once-and-dismissed-forever as C1 specifies. |
| **C2** — Control density | **Shipped** (architect spot-check, not re-audited) | `--control-h`, `--row-h` density variables. |
| **C3** — Labeled vs icon-only actions | **Partly shipped** | Write actions are labeled: `deep_compare.rs:368,386` render "Copy to right"/"Copy to left" as visible button text (not just title); `hunk.rs:322-326` renders "▶" + a visible `{t(lang, "Use")}` span for the apply button; the diff toolbar's Undo/Redo/Swap sides all carry visible text (`toolbar.rs:45-46,119,127`). **Gap:** `PathBar`'s navigation icons (Back/Forward/Up/Home/Open folder, `dir_pane.rs:159-168`) have `title` but no `aria_label` — the downscope's own stated condition ("an accessible label") is unmet; a screen reader falls back to announcing the raw glyph character content. Keyboard focus *is* covered globally (`button:focus-visible` in `02-layout-shell.css:42` applies to every `<button>`, these included). The help modal documents Alt+↑ (up) and "Back/Forward buttons" (`overlay/keybindings.rs:42-43`) but not Home or the folder picker. |
| **C4** — Destructive-modal focus policy | **Shipped** | Checked every `Modal::Confirm*` dialog in `overlay/modals/file.rs` (Overwrite, Save As overwrite, diff-option/encoding/reload/swap discard) and `overlay/modals/copy.rs` (single copy, batch copy) and `overlay/modals/tab.rs` (close dirty tab) — all nine consistently `autofocus: true` on Cancel, never the destructive action, with named verbs ("Overwrite", "Discard and Change", "Discard and Swap", "Discard and Reload", "Copy file", "Copy all"), never "OK". Copy modals explicitly state the backup policy ("A .bak backup will be created first" / "Existing files will receive a .bak backup", `copy.rs:23,86`). `SaveErrorModal`'s `autofocus: button.is_primary` (`file.rs:284`) is a recovery-after-failure dialog, not a pre-action destructive confirmation, and is out of C4's stated scope. |
| **C5** — Severity-based notices | **Shipped** (architect spot-check, not re-audited) | `ui/component/notice.rs`. |
| **C6** — Plain-language settings | **Partly shipped** | Progressive disclosure ships (`settings/modal.rs:19`, `show_advanced` hidden by default, explicitly commented "RFC-063 C6"); algorithm names live under the toolbar's Advanced disclosure panel (`diff/toolbar.rs:95` section start, Myers/Patience/Histogram at 181-183), not in the main UI. **Gap:** the settings modal has only two sections — "Appearance" and "Advanced" (`settings/modal.rs:52,121`) — no third "Compare behaviour" plain-language section. Every compare-related setting (context lines, ignore patterns, compare profiles) lives inside the single Advanced block; none is elevated to an always-visible, plain-labeled tier the way the RFC specifies. |
| **C7** — "Local only" trust marker | **Partly shipped** | `statusbar.rs:63-64` renders exactly `🔒 Local only` with the exact tooltip text from the RFC ("Files stay on this computer. ForskScope does not upload them."), unconditionally. **Gaps:** no narrow-layout variant (`🔒 Local`, shortened) — the statusbar always renders the full string regardless of width; no first-run appearance of the longer statement (C1 not shipped, so this half of C7 has no home to appear in either). |
| **C8** — Identical-files presentation | **Verified — nothing built** | `ui/view/diff.rs:143-144`: `if snap.identical { Notice { kind: NoticeKind::Ok, "Files are identical" } }` renders unconditionally *above* the diff columns (`diff.rs:151` on), which render unconditionally regardless of `identical`. No "Show content" button, no content-hiding path anywhere in this file. Matches the RFC's own description of the kept (rejected-reversion) behavior exactly. |
| **C9** — Directory-report exactness | **Shipped** | `ui-logic/src/explore/deep_filter.rs:74-94`: `equal`, `different`, `computing` are each an independent `.filter(...).count()` over `RecStatus`, never derived as `total - other`. `is_different` (line 120) deliberately excludes `Unreadable` from the different count, matching the RFC's stated concern about folding non-verdict states into a category. |
| **C10** — Friendly error mapping | **Partly shipped** | `diff_actions.rs:386-393`'s `handle_result` — the main save-error path — now routes every `Err` through `AppError::from_core` + `SaveErrorView`, not a raw string (this was migrated since the RFC was written; a stale doc comment at `diff_actions.rs:321` and `file.rs:268` still describes the *old* `store.notify(e.to_string())` arm as "established" — noted, not fixed, since a comment edit is still a code change and this is an audit). **Still open:** `overlay/modals/recovery.rs:142` and `:252` (settings-recovery and session-recovery "reset and back up original" failure handlers) both call `store.notify(e.to_string())` directly — the exact raw-OS-error-string pattern C10 names. |

**No code was changed to produce this table** — every row is read from
the tree as it stands on `1a44a2a`. Scheduling what (if anything) to do
about the partial items is the architect's, per §5's explicit "report
and stop."
