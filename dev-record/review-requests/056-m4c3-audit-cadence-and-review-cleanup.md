# Review Request: M4-C3 — Audit Cadence (F55) and Review-057/058 Cleanup

**Date:** 2026-08-13
**Reviewer stance:** F55's design decision and its falsifiability evidence are the main focus
**Repository baseline:** `3493361` (F55's implementation); reviews 057/058's corrections were already committed at `a179394`, before this handoff was issued
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c3-audit-cadence-and-cleanup-handoff.md`

## 1. Implementation summary

F50's lockfile slice (`2f956b7`) landed before this handoff existed, per its
own §1 ("not part of this handoff"). Reviews 057 and 058's small
corrections (N1, N2, §5.2, §5.3, §4.3) were also already committed
(`a179394`) in response to their respective reviews, before this handoff's
§3/§4 formally re-scoped them as M4-C3 register items — §2 below notes
this so it isn't mistaken for scope creep or a missed item.

This slice's actual new work is **F55**: `cargo audit`'s cadence.
Implemented as designed and owner-approved (`3493361`), then verified with
a real, observed falsifiability demonstration (§5) rather than asserted.

## 2. Addressed items

- **F55** — see §4 and §5 below.
- **Review 058 N1** (`release.yml`'s redundant source-archive glob) —
  already fixed in `a179394`, confirmed still present at this baseline.
- **Review 058 N2** (`PKGBUILD`'s `sha256sums=('SKIP')`) — already
  addressed in `a179394`; see §6 for the choice made and why.
- **Review 058 §5.2** (patch export's versioning note) — already added to
  `patch.rs`'s module doc in `a179394`.
- **Review 058 §5.3** (F16's method sentence) — already added to
  `ROADMAP.md` in `a179394`.
- **Review 057 §4.3** (rolling-label note) — already added to
  `matrix-plan.md` in `3ad0ce7`, before this handoff.

## 3. Changed files

`.github/workflows/audit.yml` (new), `.github/workflows/ci.yml`
(`cargo audit` and its install step removed from the per-push job),
`ROADMAP.md` (F55 marked resolved), `docs/src/maintainers/testing.md`
(new "`cargo audit`'s cadence" section).

`release.yml` is **unchanged** — confirmed via `git diff`; its preflight
`cargo audit` step still hard-blocks release tags exactly as before.

## 4. F55 §2.3 — the judgment call, and what makes a new advisory impossible to miss

**Decision: removed `cargo audit` from `ci.yml`'s per-push job entirely**,
rather than keeping it alongside the new scheduled run.

Reasoning, following the handoff's own framing rather than defaulting to
"add a check, keep the old one too": the per-push block's only benefit was
speed of notice, and the new daily scheduled run now supplies that benefit
on its own terms — noticed within a day, as a tracked finding rather than
an obstacle blocking whatever unrelated work happens to be in flight. The
cost the per-push block carried (non-deterministic CI for every
contributor — F50's exact failure mode) doesn't disappear by adding a
second check; it only disappears by removing the first one. Keeping both
would mean paying the non-determinism cost for a benefit already covered
elsewhere.

`cargo xtask audit-deps` was deliberately **not** moved — it checks this
repo's own dependency-path shape against `.cargo/audit.toml`'s reviewed
exceptions, which `Cargo.lock` alone determines. It has none of
`cargo audit`'s live-database flakiness, so there was no reason to move it
off the deterministic per-push gate where it belongs.

**What makes a new advisory impossible to miss:** GitHub's default
behavior emails the repository's watchers when a scheduled workflow run
fails. No additional notification plumbing (issue creation, Slack webhook,
etc.) was added — for a repository this size, the built-in mechanism is
the same one that already covers every other scheduled/automated failure
here, and adding a second, redundant notification path would be
complexity without a demonstrated gap. If this project's structure changes
(more contributors, notification settings that don't route to whoever
needs to see it), that assumption should be revisited — flagging this as
the one place §2.3's answer depends on an assumption about who's watching,
rather than a fact independent of team size.

## 5. Falsifiability demonstration, with observed output

Per M4-B's standing standard and this handoff's explicit requirement,
demonstrated both outcomes using `workflow_dispatch` rather than waiting a
day or for a real new advisory:

**Pass, against `main`:** `gh workflow run "Scheduled Security Advisory
Audit" --ref main` → run
[`31699200117`](https://github.com/forskscope/forskscope/actions/runs/31699200117)
→ `completed success`.

**Fail, against a throwaway branch reproducing F50's exact advisory:**

```text
$ git checkout -b demo/f55-audit-fail
$ cargo update -p webbrowser --precise 1.2.1
    Downgrading webbrowser v1.2.4 -> v1.2.1
$ git add Cargo.lock && git commit -m "DO NOT MERGE - F55 falsifiability demo..."
$ git push -u origin demo/f55-audit-fail
$ gh workflow run "Scheduled Security Advisory Audit" --ref demo/f55-audit-fail
```

Run
[`31699489630`](https://github.com/forskscope/forskscope/actions/runs/31699489630)
→ `completed failure`, with the exact advisory reproduced in the log:

```text
Crate:     webbrowser
Version:   1.2.1
Title:     Unix `BROWSER` handling allows browser argument injection
Date:      2026-07-29
ID:        RUSTSEC-2026-0257
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0257
Solution:  Upgrade to >=1.2.2
```

**Cleanup, verified:**

```text
$ git checkout main
$ git branch -D demo/f55-audit-fail
$ git push origin --delete demo/f55-audit-fail
$ grep -A1 'name = "webbrowser"' Cargo.lock
name = "webbrowser"
version = "1.2.4"
$ cargo audit; echo "exit=$?"
... exit=0, RUSTSEC-2026-0257 absent
```

`main`'s dependency graph was never modified by this demonstration — only
the throwaway branch, deleted after both runs completed. No dependency
change landed on `main` in this slice, matching the handoff's constraint
that F50's slice is the only place a dependency moves.

## 6. N2's choice

Recorded rationale, not a real checksum: no real, tagged release exists
yet to hash the fetched tarball against (`source=` points at
`archive/refs/tags/$pkgver.tar.gz`, which only resolves once `$pkgver` is
an actual pushed tag) — any hash written now would be wrong the moment a
real tag is cut. `PKGBUILD` now carries a comment explaining why `SKIP` is
accepted today, and `release.md`'s checklist gained a new step: refresh
`sha256sums` via `updpkgsums` once the tag from step 2 exists, so it does
not stay `SKIP` indefinitely once a real release exists to hash.

## 7. Difference from the handoff or RFC-074

None. §2.3 (keep, remove, or reconsider the per-push block) was the one
judgment call the handoff explicitly left to me, with the "impossible to
miss" constraint — answered in §4 above.

## 8. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass — 1094, unchanged
cargo xtask i18n                                                 pass — 227 keys, unchanged
mdbook build docs                                                pass
git diff --check                                                 pass
actionlint .github/workflows/*.yml (installed locally, v1.7.12, matching ci.yml's pin) pass
```

CI run
[`31699015617`](https://github.com/forskscope/forskscope/actions/runs/31699015617)
on `3493361`: full success — `ci.yml`'s "Test & Lint" job passes clean
with `cargo audit` removed, confirming nothing else in that job implicitly
depended on it (e.g. via the `cargo-audit` binary install step's caching
side effects).

## 9. Unresolved issues and known limitations

- §4's flagged assumption: the "impossible to miss" answer relies on
  GitHub's default scheduled-workflow-failure email reaching whoever needs
  to act on it. Correct for this repository's current structure; worth
  re-examining if that changes.
- The Windows floor / `MaxVersionTested` question from review 057 §4.2 is
  resolved by the owner as F49b, already recorded in `ROADMAP.md` — no
  action needed from this slice.
- Gate C's remaining precondition (RFC-078 §7 of the handoff) —
  `matrix-plan.md` §4's remaining fields, particularly the Linux support
  baseline — is still open and outside this slice's scope.

## 10. Requested review focus

1. §2.3's removal decision (§4) — whether the per-push/scheduled split
   lands in the right place, or whether `cargo audit` should have stayed
   in `ci.yml` for pull requests specifically (untested by this slice's
   demonstration, which only exercised branch pushes via
   `workflow_dispatch`) even if removed from ordinary `push` runs.
2. Whether GitHub's default email notification is a sufficient "loud"
   mechanism, or whether this project's scale already warrants something
   more explicit (an auto-filed issue, for instance).
3. The falsifiability demonstration's method (§5) — whether reproducing
   F50's exact advisory on a throwaway branch is convincing evidence, or
   whether a different failure mode should also have been exercised.
