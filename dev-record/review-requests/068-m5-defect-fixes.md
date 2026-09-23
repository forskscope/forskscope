# Review Request 068: M5 defect fixes — F73/F68, F72, F69

**Governing RFC.** [RFC-074](../../rfcs/proposed/074-v1-release-stabilization-program.md), clearing inputs recorded under [RFC-078](../../rfcs/proposed/078-platform-runtime-acceptance.md)
**Governing handoff.** [`rfcs/handoffs/078-platform-runtime-acceptance/m5-defect-fixes-handoff.md`](../../rfcs/handoffs/078-platform-runtime-acceptance/m5-defect-fixes-handoff.md)
**Register items.** F73 + F68 (one fix), F72, F69
**Baseline.** `main` at `0eeaee9`
**Scope.** Product work only, per the handoff's own framing — no evidence
file is edited, no matrix row is re-run here. Three separate commits
(`22a50ac`, `a4dd9d8`, `4efa5af`), one per fix, matching the handoff's
requirement that F73 "stays separately reviewable" from the other two and
"must not be reviewed under the attention a convenience fix gets."

## 1. F73's fix, both parts, and evidence the F68 half is closed

**Root cause, exactly as review 068 §5 established it:** `DeepRow` was
never given `left_root`/`right_root` — unlike `BatchCopyButtons` one call
site up, which receives them correctly — and substituted
`store.settings.read().last_left_dir`/`last_right_dir` (Explorer's
remembered pane directory) instead, both for the per-row copy buttons'
src/dst and the per-row Compare button's targets.

**Part one — the roots are now props.** `DeepRow(entry, lang, left_root:
PathBuf, right_root: PathBuf)`, passed by `DeepCompareView` the same way
`BatchCopyButtons` already was. The Compare button and both copy buttons'
src/dst computation is extracted into two plain functions —
`left_then_right`/`right_then_left` — that take the roots as parameters
and have no `Store` access at all, so there is no settings state left for
a future change to accidentally reach back into.

**Part two — the gate is gone, not narrowed.** The old
`has_left_root && has_right_root && copy_left_to_right` condition (and its
`copy_right_to_left` twin) is replaced by `copy_left_to_right`/
`copy_right_to_left` alone — extracted into `can_copy_left_to_right`/
`can_copy_right_to_left`, which take only `RecStatus`. This is F68's entire
mechanism (the handoff's own words) — the buttons vanished when
`remember_explorer_dirs` left `last_left_dir`/`last_right_dir` as `None`;
with the roots always present as props, that condition can never be false
again, so removing it rather than reworking it is the actual fix, not a
simplification of a still-present bug.

**`BatchCopyButtons` is untouched** — it already received the roots
correctly, and this change only touched the two functions that didn't.

### Evidence

Five new tests in `deep_compare.rs`:

- `left_then_right_joins_the_relative_path_onto_each_actual_root` /
  `right_then_left_swaps_source_and_destination` — assert the exact
  resulting paths, not merely "a path was returned." The handoff was
  explicit that a wrong-*destination* defect needs a test that checks
  *where*, not just that a copy happened.
- `copy_targets_do_not_depend_on_any_remembered_explorer_directory` — a
  `last_right_dir`-shaped sentinel path is constructed and asserted
  *unequal* to the real result, naming the exact condition that produced
  F73's silent wrong-location write.
- `copy_to_right_is_available_for_left_only_and_changed_entries` /
  `copy_to_left_is_available_for_right_only_and_changed_entries` — every
  `RecStatus` variant checked against `can_copy_left_to_right`/
  `can_copy_right_to_left`, closing the F68 half: the only thing gating
  button visibility now is status, and a `Some`/`None` `last_left_dir`
  cannot appear in either function's signature to gate on.

**Confirmed to fail before the fix**, per the handoff's explicit
requirement: temporarily reverted `left_then_right` to hardcode a
different directory in place of `right_root` (simulating the old
substitution), ran the suite, watched
`left_then_right_joins_the_relative_path_onto_each_actual_root` and
`copy_targets_do_not_depend_on_any_remembered_explorer_directory` fail
with the exact wrong-destination assertion, then restored the fix and
confirmed all five pass again.

## 2. F72's chosen level, and why

**Level chosen: a non-pushing navigation path, not a flag.** The handoff
flagged "a flag threaded through every call site" as the option most
likely to rot, and there are ten call sites total (four Back/Forward, six
fresh-navigation), so a boolean parameter would have touched all ten for
a distinction only four of them need.

Instead: the shared work (settings-remember, persist, `current_dir.set`,
scroll-to-top) is now a private `apply_navigation`. `navigate_to` (push +
apply) keeps its exact existing signature and behavior for the six
fresh-navigation call sites — **untouched**, not merely unchanged in
effect. A new `navigate_to_from_history` (apply only) replaces it at
exactly the four Back/Forward call sites in `explorer.rs`.

**One structural side effect, disclosed rather than buried:**
`apply_navigation`'s scroll-to-top eval moved from `spawn` to
`spawn_forever`. `spawn` requires a "current scope" that only real event
dispatch provides — calling `apply_navigation` directly (as the new test
does, and as any future test of this function would need to) panicked
inside `dioxus-core` before this change. `spawn_forever` is already this
codebase's established way around exactly this constraint
(`state/compare.rs`, `state.rs`'s own `Store` signals). No user-visible
behavior change: the eval is one-shot and has no real dependency on the
calling scope's lifetime either way.

### Evidence

A new `dir_pane.rs` test, `back_then_forward_returns_to_the_page_that_was_left`,
drives the real `navigate_to`/`navigate_to_from_history` functions through
`with_test_store` (not a reimplementation of `NavHistory`'s own logic,
which was never the bug): navigate A → B, Back, assert Forward is
available and returns to B.

**Confirmed to fail before the fix:** temporarily called `navigate_to`
(the pushing variant) for the Back step instead of the new
`navigate_to_from_history`, ran the test, watched it fail with "F72: Back
must not destroy the Forward entry," restored the fix.

## 3. F69's applied pattern

**Pattern, not a fix to one modal:** a shared `focus_autofocus_button`
(`modals.rs`) attached via `onmounted` on each modal's outer `.scrim` div,
re-asserting focus via `document.querySelector('.scrim [autofocus]')?.focus()`
— the same attribute every one of these modals already carries on its
Cancel-equivalent button, so no renaming was needed anywhere. Attached to
the `.scrim` div rather than the button itself because by the time an
element's `onmounted` fires its own subtree (the button included) is
already in the DOM — the same shape `app.rs`'s existing `onmounted` +
`eval("...#app-root...focus()")` pattern already established for the
identical "an attribute-based focus doesn't apply itself" problem at the
app-root level, not a new technique introduced for this fix.

Applied to all seven modals review 067 §2 confirmed share the pattern:
`OverwriteModal`, `ConfirmSaveAsOverwriteModal`,
`ConfirmDiffOptionChangeModal`, `ReloadModal`, `SwapModal` (`file.rs`),
`ConfirmDirOpModal`, `BatchCopyModal` (`copy.rs`).

**Not applied to `CloseTabModal`** (`tab.rs`) — it carries the identical
`autofocus: true`-on-Cancel pattern (confirmed by reading it) but was not
in review 067's confirmed set of seven, and the handoff names those seven
explicitly. Flagging this for the reviewer rather than silently expanding
scope beyond what was named — see §8.

### Evidence

**Local, not the required evidence:** re-ran the existing Linux M5 harness's
P11 check (`packaging/evidence/linux_harness.py p11`) against the freshly
built binary in this sandbox (which has real GUI capability) — still
**Pass**, confirming the explicit focus call doesn't conflict with or
regress WebKitGTK's already-working native `autofocus` behavior.

**The required evidence — the Windows M5 harness's P11 item-2 check going
from Fail to Pass on real CI — is not attempted here.** It needs a new
candidate build, which the handoff's §6 places explicitly after this
slice, and this slice's own §5 constraint says no evidence file is edited
here. Flagged in §8 as the one piece of required evidence this request
cannot supply on its own.

## 4. Failing-before tests for all three

Covered inline in §1/§2 above (F69 has no comparable unit test — see §8;
its evidence is real-CI-only, as the handoff's own required-evidence list
for it states).

## 5. Changed files

- `crates/forskscope-ui/src/ui/view/deep_compare.rs` (F73/F68, commit `22a50ac`)
- `crates/forskscope-ui/src/ui/view/dir_pane.rs`,
  `crates/forskscope-ui/src/ui/view/explorer.rs` (F72, commit `a4dd9d8`)
- `crates/forskscope-ui/src/ui/overlay/modals.rs`,
  `crates/forskscope-ui/src/ui/overlay/modals/copy.rs`,
  `crates/forskscope-ui/src/ui/overlay/modals/file.rs` (F69, commit `4efa5af`)

## 6. Product defects found along the way

None beyond the three this slice was scoped to fix. `CloseTabModal`'s
shared `autofocus` pattern (§3) is a related *observation*, not a new
defect confirmed on WebView2 — nobody has verified whether it exhibits
F69's failure, only that it carries the same attribute.

## 7. Executed gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`, `cargo xtask i18n` (227 UI keys,
unaffected — no new translation keys added, all `t(lang, ...)` calls
touched were pre-existing), `git diff --check`. Full local `cargo test
--workspace`: 1109 passed, 0 failed, 1 ignored, across all workspace
crates (`forskscope-ui`'s lib and bin targets each report the six new
`deep_compare.rs` tests and the one new `dir_pane.rs` test, since both
targets share the same `src/`). Full CI (fmt, clippy, test, i18n,
actionlint) confirmed green on `4efa5af` via
[`gh run view`](https://github.com/forskscope/forskscope/actions/runs/31994579230) —
not assumed from the push alone.

## 8. Unresolved issues

- **F69's required evidence (the Windows P11 check flipping Fail→Pass on
  real CI) cannot be supplied by this request** — it needs a new
  candidate build, which is explicitly §6's post-slice step, not this
  one's. This request's own evidence for F69 is code-level (the pattern
  applied identically to all seven, matching review 067's confirmed set)
  plus a local Linux non-regression check, not the CI confirmation the
  handoff asks for.
- **`CloseTabModal` shares F69's exact pattern but is outside this
  handoff's named scope** (§3) — worth the reviewer's judgment on whether
  it should be folded into this fix now (its risk profile is identical)
  or left for a follow-up once F69 itself is CI-confirmed.
- **If the explicit focus call proves unreliable on WebView2 too**, the
  handoff asked that this be reported rather than worked around — that
  question can only be answered once the new-candidate CI re-run happens
  (§6), not from this slice alone.

## 9. Requested review focus

1. **Is F73/F68's split into `left_then_right`/`right_then_left` +
   `can_copy_left_to_right`/`can_copy_right_to_left` the right level of
   extraction** — enough to make the fix genuinely unit-testable without
   over-restructuring `DeepRow` beyond what F73 needed?
2. **Is `spawn` → `spawn_forever` in `apply_navigation` (F72) an
   acceptable incidental change**, or does it deserve its own scrutiny
   given it wasn't named in the handoff?
3. **Should `CloseTabModal` be folded into F69's fix now** (§3/§8), given
   it visibly shares the same `autofocus: true`-on-Cancel shape, or held
   for a separate pass once Windows CI confirms F69's fix actually works?
4. **Is the F69 evidence gap (§3/§8) acceptable for this request to be
   approved**, with the real CI confirmation tracked as an explicit
   follow-up per handoff §6, or does approval need to wait for that
   confirmation first?
