# M4-C3 review — audit cadence (F55) and review-057/058 cleanup

**Review date:** 2026-08-13
**Request:** `dev-record/review-requests/056-m4c3-audit-cadence-and-review-cleanup.md`
**Baseline:** `3493361`, with `a179394` and `3ad0ce7` carrying the earlier corrections
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c3-audit-cadence-and-cleanup-handoff.md`
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved**, with one refinement worth taking (N1, §4) that answers your own
question 1 better than either option you weighed.

F55 is implemented as designed, and the falsifiability demonstration is the
strongest one this program has seen — you reproduced the *actual* advisory that
motivated the finding, on a throwaway branch, and showed both outcomes.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `audit.yml` — daily cron + `workflow_dispatch` | Confirmed |
| `cargo audit` removed from `ci.yml`'s per-push job | Confirmed |
| `cargo xtask audit-deps` **kept** on the per-push gate | Confirmed — correct, see §3 |
| `release.yml` preflight `cargo audit` unchanged | Confirmed — lines 43–44, 59 still hard-block |
| Pass run `31699200117` on `main` | `success` |
| Fail run `31699489630` on `demo/f55-audit-fail` | `failure` |
| Demo branch deleted from origin | Confirmed — no `demo/f55` refs remain |
| `Cargo.lock` on `main` at `webbrowser 1.2.4` | Confirmed |
| N1 — stale `forskscope-v*.tar.gz` glob | Gone |
| N2 — `PKGBUILD` rationale + `release.md` `updpkgsums` step | Both present |
| 058 §5.2 patch note, §5.3 F16 sentence, 057 §4.3 rolling-label note | All present |
| `fmt`, `clippy --all-targets`, `test --workspace` (1094) | Pass |

## 3. Two judgements that were right and easy to get wrong

**Keeping `cargo xtask audit-deps` on the per-push gate.** The finding was
about `cargo audit`'s mutable external input; `audit-deps` checks this
repository's own dependency-path shape against a checked-in policy and is fully
determined by `Cargo.lock`. Moving both because they sound alike would have
removed a deterministic gate for no reason. You separated them on the actual
property that mattered.

**Removing the per-push block rather than keeping both.** Your reasoning is
correct and worth restating: the cost of the per-push block is
non-determinism, and that cost does not shrink by adding a second check —
only by removing the first. Keeping both would have paid the cost for a benefit
already covered.

## 4. N1 — the refinement, and the answer to your question 1

You asked whether `cargo audit` should have stayed for pull requests. Neither
"keep it on PRs" nor "remove it everywhere" is quite right. **Scope it to
dependency-changing commits.**

The removal loses something real: `cargo audit` also caught **our own** changes
introducing a known-vulnerable crate. F55's argument does not cover that case at
all — a commit that changes `Cargo.lock` and pulls in a vulnerable dependency is
bad *because of the change*, not because the database mutated underneath us.

The concrete instance is uncomfortable: **F50's own fix was a `Cargo.lock`
change.** Under the new arrangement, a future security bump of exactly that
shape gets no CI audit on the commit that makes it — the gate that would confirm
the fix worked does not run on the fix. It lands, and the daily run confirms it
up to 24 hours later.

The fix keeps both properties:

```yaml
on:
  pull_request:
    paths: ['Cargo.lock', '**/Cargo.toml']
  push:
    paths: ['Cargo.lock', '**/Cargo.toml']
```

- Unrelated pushes — F50's exact case, no dependency change — never run it, so
  the non-determinism you removed stays removed.
- Dependency-changing commits are audited immediately, where a failure is
  attributable to the change in front of you.
- The daily run still catches database mutation on unchanged code.

One wrinkle to handle: a `paths`-filtered job reports as skipped, which can
block merges if it is a required status check. Note how you handle it.

**Non-blocking.** What shipped is a strict improvement on what was there, and
this is an addition rather than a correction.

## 5. Answers to the other requested focus

### 5.1 GitHub's default email — sufficient now, and you flagged the right dependency

Sufficient, and I would not add issue-filing plumbing today. Your reasoning is
sound: it is the same mechanism already covering this repository's other
automation, and a second notification path is complexity without a demonstrated
gap.

What makes this answer acceptable is that you named the assumption it rests on —
that the default routes to someone who will act — rather than presenting it as a
property of the design. That is the difference between an accepted risk and an
unexamined one. The `audit.yml` comment says it too ("if that default ever
proves insufficient, add one rather than assuming silence means safe"), which is
where the next person will actually look.

One thing worth knowing about the failure mode, not requiring action: GitHub
disables scheduled workflows in repositories with no activity for 60 days, and
notifies the owner when it does. A quiet repository is exactly when a silently
disabled advisory check matters most. Worth a line in the comment.

### 5.2 The demonstration method — convincing, and better than what I asked for

Reproducing F50's *actual* advisory by pinning `webbrowser` back to 1.2.1 is
stronger evidence than a synthetic failure would have been: it proves the
workflow detects the specific class of thing it exists to detect, on the exact
input that motivated it. Using `workflow_dispatch` against a throwaway ref, then
deleting the branch and verifying `main`'s lockfile untouched, is the right
hygiene.

No second failure mode needed. The one that matters — a vulnerable dependency
present, advisory in the database — is exercised end to end.

## 6. Notable quality observations

- Separating `cargo audit` from `audit-deps` on the property that actually
  differs, rather than on their similar names.
- Declining to add notification plumbing, and naming the assumption instead.
- Choosing a recorded rationale over a fabricated checksum for N2, with the
  reason being *correct*: no tag exists yet to hash, so any value written now
  would be wrong at the first real cut — and then adding a `release.md` step so
  it does not stay `SKIP` once one exists.
- Putting F55's whole rationale in `audit.yml`'s header, where someone
  re-adding a per-push audit will read it before doing so.
- Flagging that the corrections predated the handoff, so it reads as sequencing
  rather than scope creep.

## 7. Recommended next action

1. **N1** (§4) — the lockfile-scoped trigger, plus §5.1's line about scheduled
   workflows being disabled after 60 days of inactivity. Small.
2. **Owner** — `matrix-plan.md` §4's remaining fields, the Linux support
   baseline in particular. That is the last thing gating Gate C.
3. Once both land, **M4 closes** and Gate C is assessable.
