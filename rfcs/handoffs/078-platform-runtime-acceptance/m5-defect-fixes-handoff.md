# M5 Defect Fixes Handoff: F73/F68, F72, F69

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md), clearing inputs recorded under [RFC-078](../../proposed/078-platform-runtime-acceptance.md)
**Register items.** F73 + F68 (one fix), F72, F69
**Baseline.** `main` at `0eeaee9`
**Standing.** **F73 is an un-waivable Gate D blocker.** The others are Gate D inputs found by the same milestone.

This is **product work, not evidence work.** No evidence file is edited, no
matrix row is re-run here. Those follow a new candidate (§6).

## 1. Why these three together

A new candidate is needed regardless — F61's fix is already on `main` and not in
`0.167.0`. Cutting one candidate that also carries these three means M5's
re-runs cover every known fix at once rather than one per cut.

They stay **separately reviewable** inside the slice: F73 is a blocker and must
not be reviewed under the attention a convenience fix gets.

**Do not add anything else.** F67, F52, F53, F54 and the other open register
items are not in this slice.

## 2. F73 + F68 — one root cause, and the fix has two parts

**Verified in review 068 §5.** The cause is that `DeepRow` is never given the
roots it needs:

```rust
pub fn DeepCompareView(left_root: PathBuf, right_root: PathBuf, lang: Lang)  // :25   has them
    BatchCopyButtons { entries, left_root: …, right_root: … }                // :157  ← receives them
fn DeepRow(entry: RecEntry, lang: Lang)                                      // :183  ← does not
    let has_left_root  = store.settings.read().last_left_dir.is_some();      // :203  ← substitutes
    let has_right_root = store.settings.read().last_right_dir.is_some();     // :204
    if has_left_root && has_right_root && copy_left_to_right { … }           // :225  ← gates on the substitute
```

`BatchCopyButtons` one line above already does it correctly, which is why the
batch path was verified sound and the per-row path was not.

**Part one — pass the roots in.** Give `DeepRow` `left_root`/`right_root` as
props, matching `BatchCopyButtons`, and use them for the copy source and
destination and for the per-row Compare action (`:215` has the same
substitution).

**Part two — remove the gating, and do not skip this.** Once the roots arrive as
props they are always present, so `has_left_root`/`has_right_root` become
meaningless. **That gating is F68's entire mechanism**: it is why the buttons
vanish when `remember_explorer_dirs` is off. Passing the roots without deleting
the gate fixes F73 and leaves F68 exactly as it is — the easy half-fix, and the
one a reviewer might not catch because F73's symptom disappears.

**Required evidence:**

- a test that **fails before the fix**, asserting a per-row copy lands under the
  compare root — not under `last_right_dir`. The failure mode is a wrong
  *destination*, so asserting "a file was written" is not enough; assert *where*;
- the F68 half: per-row copy buttons render with `remember_explorer_dirs` **off**;
- `BatchCopyButtons` still correct — it was already right, and this change
  touches the file it lives in.

## 3. F72 — Back destroys that pane's Forward history

`explorer.rs:383` (and `:392` for the right pane):

```rust
on_back: move |_| { let p = left_hist.write().back();
                    if let Some(p) = p { …navigate_to(p, true, store, left_hist, left_dir); } },
```

`NavHistory::back()` only moves the index, leaving `entries` intact — correct.
But `navigate_to` then **unconditionally pushes**, and `push`'s duplicate guard
(`entries.last() == path`) does not save it, because `entries.last()` is still
the not-yet-truncated forward entry. So `push` truncates and re-appends,
destroying the forward entry.

Both panes have the same shape. Fix at whichever level is right — a
non-pushing navigation path, or `navigate_to` learning not to push when it is
replaying history — and say which you chose and why. A flag threaded through
every call site is the option most likely to rot; weigh that.

**Required evidence:** a test that fails before the fix — navigate A → B, press
Back, assert Forward is available and returns to B.

## 4. F69 — `autofocus` does not focus a destructive modal on WebView2

Every destructive modal relies on the HTML `autofocus` attribute for its
Cancel-equivalent button. **On WebKitGTK that works** — verified in review 067
§2 on a real desktop, focus lands on Cancel. **On WebView2 it does not**: focus
stays on the background control that opened the modal.

So the app is not doing anything invalid; it is relying on a behaviour one
engine does not provide for a dynamically mounted element. The fix is therefore
**an explicit focus call after mount**, not a corrected attribute.

Apply it to the whole pattern, not one modal. All seven were confirmed to share
it: `OverwriteModal`, `BatchCopyModal`, `ConfirmDirOpModal`, `ReloadModal`,
`SwapModal`, `ConfirmDiffOptionChangeModal`, `ConfirmSaveAsOverwriteModal`.

**Required evidence:** the Windows M5 harness's P11 item-2 check going from Fail
to Pass on real CI — that check already exists and already detects this. Confirm
Linux and macOS P11 still pass, since they pass today and the change touches
their path too.

**If an explicit focus call proves unreliable on WebView2 as well, report rather
than working around it.** A focus that cannot be placed into a destructive modal
is an accessibility finding in its own right, and worth more than a fix that
merely satisfies the check.

## 5. Constraints

- `0.165.0`, `0.166.0`, `0.167.0` are published and immutable.
- No dependency added, removed, or version-changed — `dioxus-desktop` included.
  F44 is upstream and not addressed here.
- **No evidence file is edited.** `docs/src/maintainers/release-evidence/**` is
  a record of what `0.167.0` did; it does not change because `main` changed.
- `matrix-plan.md` stays frozen.
- Every fix needs a test that fails before it. F73's especially: a
  wrong-destination defect that a test cannot distinguish from a right-destination
  one is not covered.

## 6. After this slice

1. **A new candidate** carrying F61, F73/F68, F72 and F69.
2. **Re-run the affected M5 rows** against it — P12 (F61), P07 (F73, and F70's
   Windows blocker permitting), P11 (F69). Not the whole matrix.
3. Gate D still cannot pass: **F44 remains**, upstream and un-waivable.

## 7. Required review-request content

1. **F73's fix, both parts** (§2), and evidence the F68 half is closed —
   the review's main focus;
2. F72's chosen level and why;
3. F69's applied pattern and the P11 check flipping on real CI;
4. the failing-before tests for all three;
5. changed files;
6. any product defect found along the way, registered not fixed;
7. executed gates;
8. unresolved issues;
9. requested review focus.
