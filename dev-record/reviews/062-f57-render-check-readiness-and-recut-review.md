# F57 review — render-check readiness and the 0.167.0 re-cut

**Review date:** 2026-08-15
**Request:** `dev-record/review-requests/058-f57-render-check-readiness-and-0.167.0-recut.md`
**Baseline:** `cb6f5b6`, tagged `0.167.0`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f57-render-check-readiness-and-recut-handoff.md`
**Review mode:** Independent verification, including re-downloading a draft artifact to check its digest. No implementation changes made.

## 1. Verdict

**Approved.** The readiness fix is correct, all three demonstrations hold, and
the re-cut followed `release.md` exactly.

**`0.167.0` exists as a draft with three artifacts and verified digests. M5 can
begin.**

Your §9 observation is escalated as **F58** (§4.3) — it is a real gap, not
expected behaviour to leave alone, though not for quite the reason you gave.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| Run `31847082187` — pre-fix, on-demand | **failure**, original message |
| Run `31847446602` — post-fix | **success** |
| Run `31847749205` — post-fix, F32 injected | **failure**, misalignment detected |
| Release run `31847956550` | All five jobs **success**, including Linux |
| Tag `0.167.0` → commit | `cb6f5b6` — confirmed |
| Draft state | `isDraft: true`, three assets |
| **Linux artifact digest** | **`e17baa26…bf09` — matches your record exactly** |
| `cargo test --workspace` | 1095 (+1, the row-count pinning test) |
| Row-count test pins 7 against `compute_diff` | Confirmed, with the derivation in-comment |
| `wait_for_ready` polls landmark, frame **and** row counts to a deadline | Confirmed |

Two things I got wrong on the way, recorded so the numbers above are trusted for
the right reasons: `git rev-parse 0.167.0` returns the *annotated tag object*
(`dcfc21b`), not the commit — `^{commit}` gives `cb6f5b6` as you stated. And an
unauthenticated fetch of a draft asset returns a 9-byte "Not Found" whose hash
is meaningless; `gh release download` gives the real artifact, and it matches.

## 3. The sequencing deserves singling out

Committing the on-demand entry point **before** the readiness fix, so
demonstration (1) ran genuinely pre-fix code on a real runner, is the difference
between evidence and a reconstruction. Most implementations would have fixed
first and then reproduced the failure by reverting — which proves the revert
works, not that the original defect behaved as diagnosed.

Same instinct in running all three locally before touching CI, to catch a broken
patch script cheaply.

## 4. Answers to the requested review focus

### 4.1 Exact row count is the right strictness — because you pinned it

"At least N rows" would have been the weaker choice, and the reason is precise:
a partially rendered tree can satisfy `>= N` on its way to the full count, so
the looser condition reintroduces exactly the race it was meant to close, just
with a smaller window. Exact match cannot be satisfied early.

The usual objection to exactness is brittleness under fixture change — and you
removed it by pinning the count in `diff_corpus.rs` against `compute_diff`'s own
output. A future fixture change now breaks a *test*, loudly, at the point of
change, instead of silently loosening a CI check nobody re-reads. That is what
makes strict safe here, and it is why the answer would be different if the
number were only a constant in the script.

Deriving the seven rows in-comment rather than reading them off one observed run
is the same discipline.

### 4.2 F32's literal defect is the right demonstration — and one branch is still unexercised

Reintroducing the actual historical defect is more convincing than a synthetic
one, for the reason you gave in review 056 and which still holds: it proves the
check detects the specific thing it exists for.

One caveat carried forward, not new to this slice: **only the child-count
assertion has ever fired.** My review 055 N1 noted the geometry branch —
`x`-origin comparison — has never been observed failing, and demonstration (3)
again shows only child-count messages. I tried twice then to trigger it with CSS
mutations and could not produce a shift. It remains defence-in-depth of unproven
effectiveness.

Not something to fix here. But now that a dispatchable entry point exists, the
cost of settling it has dropped a lot — `inject_f32_defect` proves the harness
can inject a defect and observe the result, and a second injection mode for a
geometry-only shift would reuse all of it. Worth doing whenever that file is
next open.

### 4.3 `version-sync` — escalate, and the reason is not the one you gave

**Your reading of the mechanism is right; the conclusion that it is expected and
should be left alone is not.**

First, a correction on the current state: `version-sync` **passes** on `cb6f5b6`
right now — I ran it. The rule is "the version is tagged but *not at HEAD*", and
HEAD is now the tagged commit. The red CI run you saw was ordering, not steady
state: CI fired ~3 seconds after the push, while the tag still pointed at
`2948008`; the re-tag came after. A re-run today would pass.

So `main` is not red now. **It goes red on the next commit**, and stays red for
as long as the candidate window lasts — which, because M5 is the first milestone
to make that window long, is new. Every previous release published within
minutes, so the window never existed in practice.

That matters more than a red build: this project has already established that a
red `main` blocks unrelated work and creates pressure to soften checks (F50,
F55). A window measured in days or weeks would apply that pressure continuously.

**The check is correct and must not be weakened.** A commit after the tag builds
as `0.167.0` while differing from the tagged `0.167.0` — the R0 defect exactly.
`release.md` also explicitly rejects making the check key on draft state, and
that reasoning still holds.

**Bumping the workspace version now is also wrong**, and worth stating because
it is the obvious move: if `main` went to `0.167.1`, a re-cut of `0.167.0` would
have to tag a commit declaring `0.167.1`, which release-mode `version-sync`
would refuse. We would lose the ability to re-cut — the thing this slice just
needed.

So the gap is in the **process**, not the check: `release.md` documents the
post-release bump but says nothing about the tagged-but-unpublished window.
Registered as **F58** with the shape of the answer — freeze `main` for the
window, or land work on a branch and merge after publish-or-abandon — but the
policy is the owner's to set, and it is not urgent until someone needs to commit.

Escalating rather than leaving it is right because the next person to hit it
will be under time pressure with a red build, which is the worst moment to
design a policy.

## 5. Notable quality observations

- The commit ordering (§3).
- Pinning the row count as a *fact about the fixture* rather than a constant
  copied from an observation.
- Using `git push origin --delete` rather than `gh release delete --cleanup-tag`,
  citing `release.md`'s own caution about that flag's scope.
- A re-cut note that says the check was at fault, not the product, and that
  macOS and Windows had already passed — accurate, and it will stop a future
  reader inferring a product defect from a re-cut.
- `reintroduce_f32_defect.py` exiting 1 rather than silently no-op'ing if
  `hunk.rs`'s shape has changed. A demonstration helper that quietly does
  nothing would make a broken check look healthy.

## 6. Recommended next action

1. **Owner: `0.167.0` is a draft and ready.** Do **not** publish yet — M5 runs
   against these exact artifacts, and publishing makes the version immutable
   before the evidence exists.
2. **M5.** The frozen `matrix-plan.md` governs; evidence goes under
   `release-evidence/0.167.0/` with the digests above. F44 will fail Linux P01
   un-waivably; that is known and accepted.
3. **F58** — set the candidate-window policy before anyone needs to commit to
   `main`.
4. F55's geometry branch (§4.2) whenever `render_check.py` is next open.
