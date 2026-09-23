# M5 re-verification review — 0.167.1

**Request:** `dev-record/review-requests/069-m5-0167-1-verification.md`
**Review date:** 2026-08-17
**Candidate:** `0.167.1`, source commit `f65c74b`
**Review mode:** Independent verification of the required F69 flip, and a programmatic test of the §3 fix's scoping rather than a check of its enumeration.

## 1. Verdict

**Approved. The loop is closed.**

F61, F68, F69, F72 and F73 are confirmed against published artifacts and
correctly marked resolved. **Gate D's un-waivable blocker count drops from three
to one: F44 alone**, and it is the one nobody here can act on.

Both mid-flight findings were handled correctly, and §3's scoping — which is the
one claim in this request that could have hidden something — holds under an
independent test.

## 2. Verified

| Check | Result |
|---|---|
| **F69's required flip** — Windows P11, `0.167.0` → `0.167.1` | **`failure` → `success`** (runs `31938755692` → `32002971000`) |
| Register: F61, F68, F69, F72, F73 | All marked resolved — correct, now that verification exists |
| Digests | Match what I recorded at publication |
| §3's fix scoping — no sibling instance | **Confirmed programmatically** (§3) |
| New `0.167.1/gate-d-input-list.md`, `0.167.0`'s left intact | Confirmed |

The F69 flip is the specific evidence I required and withheld approval for in
review 070. It is now in hand: the same check, same platform, failing on the old
candidate and passing on the new one. That is as clean as this kind of
confirmation gets.

## 3. §3's scoping tested, not taken on the enumeration

Your Q1 asks whether narrow scoping risks missing a sibling. You checked by
enumeration; I tested it by construction — mapping every `launch()` call site to
its enclosing function and intersecting that with every function that isolates
`XDG_CONFIG_HOME`:

```text
functions isolating XDG_CONFIG_HOME:  p06, p07, p08, p12
launch() calls inside one of them:    lines 1075, 1088 — both inside your own
                                      explanatory comment block; the actual call
                                      is subprocess.Popen(..., env=env)
```

**No sibling instance exists.** Every real `launch()` call sits outside any
env-isolating function, and all four isolating functions use the raw `Popen`
pattern. Your diagnosis and its scope are both correct.

Worth noting what the bug actually was, because it is nastier than "a missing
parameter": the affected launch ran against **the CI runner's real
`XDG_CONFIG_HOME`**, so the sub-test was reading and writing the runner's own
config while believing it was isolated. It failed loudly here, but the same
shape could equally have produced a false *pass* by finding state a previous run
left behind. That it surfaced as a failure was luck, not design.

## 4. Answers to the other requested focus

### 4.1 Flipping macOS P07's assertions — right, and there is a general rule in it

Flipping in place was correct. A fresh fixture would have discarded continuity —
same fixture, same run history — for no gain, and the assertions were the only
thing that had become wrong.

But the episode is worth generalising, because it will recur. That case
**encoded a known defect's exact shape as the expected, passing outcome**. The
consequence is that it *failed when the product was fixed* — the check was
inverted relative to what anyone wants from it, and it would have silently
passed again the day the defect returned.

The practice to adopt: **assert correct behaviour and let the case fail while
the defect is open**, recording the failure as evidence. That is exactly what
M5-A did for F44 — recorded Fail, un-waivable, cause named — and it is why
F44's status has never needed inverting. A case that passes because the product
is broken is not evidence of anything.

You caught this yourself the moment the fix landed, which is the outcome that
matters. Recording the rule stops the next person building the same trap.

### 4.2 Per-candidate input list — right pattern

Correct, and consistent with F56's split: the standing documents (`matrix-plan.md`,
`advisories.md`) live above the candidate directories; per-candidate records live
inside them. Leaving `0.167.0`'s list untouched as a record of what was true for
*that* candidate is the same principle as not rewriting the archive.

Amending `0.167.0`'s document in place would have made it a moving record of the
present rather than a fixed record of a past decision point, and Gate E will
want to read both.

### 4.3 Does this close the loop? — yes

For F61, F68, F69, F72, F73: yes, fully. Nothing further is expected before
those are settled.

**F72's single-platform confirmation is acceptable.** Only macOS P07 drives
Back/Forward, and the shared mechanism is covered by a `dir_pane.rs` unit test
confirmed to fail before the fix. Unit test for the mechanism plus one platform
for the integration is proportionate for a navigation-history defect — I would
ask for more only if it were data-affecting.

**§8's "targeted 18-run dispatch, not a full re-execution" is the right call**
and is correctly disclosed. Re-running every case against a patch release whose
diff is four bug fixes would spend a great deal of CI to re-confirm things the
same Rust source already established. The disclosure is what makes the
assumption inspectable.

## 5. Notable quality observations

- Confirming the tag's ancestry contained all four fix commits **before**
  dispatching anything. A verification run against a candidate that silently
  lacked a fix would have produced a confidently wrong result.
- Treating Linux P12's failure as suspect *because* Windows and macOS passed
  immediately, rather than assuming a Linux-specific regression — the correct
  inference from a platform-agnostic Rust fix, and it led straight to the real
  cause.
- Adding a temporary diagnostic to locate the file rather than reasoning about
  where it should have been.
- Fixing the internally inconsistent leftover comment block found while editing
  §4's fixture, instead of leaving it because it was not what you were there for.
- Leaving `0.167.0`'s input list intact.

## 6. Where this leaves Gate D

**One un-waivable blocker: F44.** Everything else on the list is an input rather
than a blocker.

Gate D still cannot pass, and the matrix is still not formally complete —
`linux-wayland` and F45's Windows sub-case remain outstanding, and F70 keeps
Windows P07 failing. But the character of the remaining work has changed: **every
blocker this project could act on has been acted on.** What is left is one
upstream release, two owner-executed rows, and one owner check that would settle
F70 in minutes.

## 7. Recommended next action

1. **Nothing for the dev team from this request.** The loop is closed.
2. **Owner:** F70's Windows Explorer check remains the highest-value open item —
   it would let Windows P07 complete, and Windows is the only row where the F73
   fix is unverified because the case cannot reach it.
3. **Someone, periodically:** `cargo update -p dioxus-desktop --dry-run`.
   Nothing here will notice the release that unblocks F44.
