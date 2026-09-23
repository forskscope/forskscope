# M5 defect fixes review — F73/F68, F72, F69

**Request:** `dev-record/review-requests/068-m5-defect-fixes.md`
**Review date:** 2026-08-16
**Governing document:** `rfcs/handoffs/078-platform-runtime-acceptance/m5-defect-fixes-handoff.md`
**Review mode:** Independent verification of each fix at source, including both halves of F73/F68.

## 1. Verdict

**Approved**, with one required addition: **`CloseTabModal` must be folded into
F69** (§4.3). It is destructive, carries the identical pattern, and leaving it
out means a modal that discards unsaved merge work keeps the defect.

**F73's fix is correct and complete, both parts.** That clears the un-waivable
Gate D blocker this project can act on — pending confirmation against a
candidate.

All four items are correctly still **open** in the register (§3). That is the
right discipline and I want to say so before anything else.

## 2. Verified

| Check | Result |
|---|---|
| `DeepRow` receives the roots as props | Confirmed — `DeepRow(entry, lang, left_root: PathBuf, right_root: PathBuf)` (`:196`) |
| The `has_left_root && has_right_root` gate is **gone**, not narrowed | Confirmed — `can_copy_left_to_right(entry.status)` (`:216`) |
| No settings substitution remains in the copy path | Confirmed — every surviving `last_*_dir` mention is a comment explaining the old defect |
| `left_then_right` / `right_then_left` join onto the real roots | Confirmed at source (`:282`, `:291`) |
| F72: all four history call sites switched | Confirmed — 4 uses of `navigate_to_from_history` at `explorer.rs:383,384,392,393` |
| F69: pattern applied across the modal files | Confirmed — `modals.rs`, `copy.rs`, `file.rs` |
| `cargo test --workspace` | **1109**, matching your count |
| `clippy --workspace --all-targets -D warnings`, `fmt --check` | Pass |

## 3. Leaving all four open in the register was right

None of F68, F69, F72, F73 is marked Resolved. That is correct and not obvious:
the code is fixed on `main`, and the temptation to close them is real. But none
is verified against a candidate, and F69's required evidence does not exist yet.

This is the discipline whose absence caused review 065 — where F61 was marked
Resolved while the defect persisted. Getting it right unprompted matters.

## 4. Answers to the requested focus

### 4.1 The extraction level is right

Pure functions taking the roots as parameters, with **no `Store` access at
all**, is the correct level — not over-restructuring.

The property that matters is not "the code was tidied" but that
`copy_targets_do_not_depend_on_any_remembered_explorer_directory` is now
*expressible*. A function with no `AppSettings` in its signature cannot reach
back into settings, so F73's defect class is closed structurally rather than by
a corrected call site that a future edit could undo.

Same for `can_copy_left_to_right(status)` taking only `RecStatus`: F68 cannot
recur, because there is no longer a parameter through which a `remember` setting
could gate a button.

### 4.2 `spawn` → `spawn_forever` — acceptable, with one note

Acceptable. The justification holds: `spawn` requires a current scope that only
real event dispatch provides, `apply_navigation` now needs to be callable
outside it, and `spawn_forever` is established elsewhere in this codebase for
exactly that constraint.

The note worth recording rather than acting on: `spawn_forever` means the task
is not cancelled if the component unmounts mid-flight. For a one-shot
scroll-to-top eval that completes immediately, negligible — but the reason it is
negligible is a property of *this* eval, not of `spawn_forever`, so a comment at
the call site would stop a future change inheriting the assumption silently.

Flagging it rather than burying it was right; it was a real change not named in
the handoff.

### 4.3 `CloseTabModal` — fold it in now

**Yes.** I read it:

```rust
h2 { "Close comparison?" }
p  { "… has unsaved changes. Discard them and close?" }
button { autofocus: true, … "Cancel" }
button { onclick: … close_tab(…) … "Discard and close" }
```

It is destructive in the most direct sense — it discards unsaved merge work —
and it carries the same `autofocus: true`-on-Cancel pattern.

Review 067's "seven" was an enumeration of what that row happened to confirm
while investigating F69, **not a considered scope boundary**, and my handoff
repeated the number from that enumeration. Neither is a reason to leave a
destructive modal defective on WebView2.

You were right to flag rather than silently expand scope. The answer is include
it, and the general rule is worth stating: **when a fix addresses a pattern, its
scope is the pattern, not the instances that happened to be enumerated.**

### 4.4 The F69 evidence gap — approve now, keep F69 open

Approve. Blocking on the Windows CI confirmation would invert the sequence the
handoff itself set: §6 places the candidate *after* this slice, so requiring
pre-candidate CI evidence would deadlock.

Two conditions, both of which you have already met or stated:

- **F69 stays open** until the Windows P11 item-2 check flips Fail → Pass on
  real CI. Done — it is open.
- **If the explicit focus call also proves unreliable on WebView2, report it.**
  A focus that cannot be placed into a destructive modal is an accessibility
  finding worth more than a fix that satisfies a check.

Your local Linux non-regression run is the right supporting evidence for what it
covers — that the explicit call does not conflict with WebKitGTK's already-working
native behaviour — and you labelled it as *not* the required evidence, which is
the distinction that matters.

## 5. Notable quality observations

- Removing the gate rather than reworking it, and saying why removal *is* the
  fix rather than a simplification of a still-present bug. That is the half the
  handoff warned would be easy to miss, and it was not missed.
- Constructing a `last_right_dir`-shaped sentinel and asserting the result is
  *unequal* to it — testing the specific wrong-destination condition rather than
  merely that a path came back.
- Choosing a separate non-pushing function over a boolean threaded through ten
  call sites, with the count given as the reason. Six of ten call sites remain
  literally untouched, which is a real property, not a claim.
- Disclosing `spawn_forever` as a structural side effect when nothing would have
  surfaced it.
- Flagging `CloseTabModal` instead of either silently expanding scope or
  silently leaving a known gap.

## 6. Recommended next action

1. **Fold `CloseTabModal` into F69** (§4.3). Small, and it should ride with this
   slice rather than a follow-up.
2. **Cut a new candidate** carrying F61, F73/F68, F72, F69.
3. **Re-run the affected M5 rows** against it — P12 (F61), P07 (F73; Windows
   still blocked by F70), P11 (F69, the required evidence). Not the whole matrix.
4. Mark F68/F72/F73 resolved **only** once verified against that candidate;
   F69 once Windows CI confirms.
5. **F44 remains upstream** — after this, the only blocker nobody here can act
   on.
