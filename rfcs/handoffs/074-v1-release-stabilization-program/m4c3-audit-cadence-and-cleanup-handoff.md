# RFC-074 M4-C3 Developer Handoff: Audit Cadence and Review Cleanup

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M4-C3 — after F50's lockfile slice, before Gate C is assessed
**Register items.** F55 (owner-approved), plus the small corrections from reviews 057 and 058
**Baseline.** `main`, after F50's slice lands

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend RFC-074 first, then
update this handoff to match.

## 1. Order

**F50's lockfile slice goes first and is not part of this handoff.** It is
`cargo update -p webbrowser` alone, with `cargo audit` exit 0 and an unchanged
test count as its whole evidence (review 057 §3.3). Do not fold any item below
into it — a minimal-diff security bump stays minimal.

Everything here follows once `main` is green.

## 2. F55 — `cargo audit`'s cadence (owner-approved)

**The problem, from review 059.** `cargo audit` reads a mutable external
database from a blocking per-push gate, so a green CI result is a property of
the commit *and the clock*, not the commit. F50 demonstrated it: two runs
minutes apart on unrelated commits, one green one red, no dependency change
between them.

The second-order effect is the one that matters: a blocking per-push gate
rewards making CI green *fast*, and the fastest path is `audit.toml`'s ignore
list — exactly what the disposition process (reachability, owner, review date,
upgrade trigger) exists to prevent. The cadence pushes toward the behaviour the
policy forbids.

**The owner approved this shape on 2026-08-13:**

1. **Release preflight keeps the hard block.** It runs against a tag, so it is
   deterministic, and a release must never ship a known-vulnerable dependency.
   Do not weaken this.
2. **Add a scheduled run against `main`** — daily is the intent — so a new
   advisory is noticed within a day and gets a tracked response on its own
   terms rather than as an obstacle to unrelated work.
3. **Reconsider the per-push block** on that basis. The benefit it provides
   (speed of notice) is what (2) now supplies; the cost is non-deterministic CI
   for every contributor.

Item 3 is a judgement call left to you, with one constraint: **whichever way you
go, a new advisory must be impossible to miss.** If you keep the per-push block,
say why the ignore-list pressure is acceptable. If you remove it, the scheduled
run's failure must be loud — a failing scheduled workflow that nobody watches is
worse than the flakiness it replaced. Say which mechanism makes it loud.

**Falsifiability, per M4-B's standing standard:** demonstrate the scheduled
workflow actually runs and actually fails on a real advisory condition. A
`workflow_dispatch` trigger alongside the schedule makes this testable without
waiting a day; use it to show a failing run and a passing one.

## 3. Corrections from review 058

- **N1** — `.github/workflows/release.yml`'s `Create release` step still lists
  `forskscope-v*.tar.gz`, the removed source archive's glob. It is not broken
  (it now also matches the Linux tarball) but it is a redundant superset that
  will silently attach anything matching that pattern later. Delete the line.
- **N2** — `packaging/linux/PKGBUILD` keeps `sha256sums=('SKIP')`, which cost
  little when the source was a local file the maintainer placed by hand and
  costs more now that F43 made it a network fetch — the case where GitHub's
  tarball instability actually bites. Either set a real `sha256sums` refreshed
  per release (`updpkgsums`), or record in the file why `SKIP` is accepted for a
  PKGBUILD users build themselves. Leaving it unstated is the only unacceptable
  option.
- **§5.2** — add a one-line note to `patch`'s module docs recording that patch
  export needs no schema envelope for the same reason batch manifests do not
  (no read path in this codebase; unified diff is a fixed external format), so
  the next auditor does not re-derive F31's reasoning.
- **§5.3** — add to `ROADMAP.md`'s F16 entry the sentence that makes the audit
  repeatable: *audited against the UI crate specifically, because core-complete
  does not imply user-reachable.* That is F54's lesson in one line.

## 4. Correction from review 057

- **§4.3** — `matrix-plan.md` names rolling runner labels
  (`macos-latest`, `windows-latest`, `ubuntu-latest`). When GitHub advances one,
  that row's runtime changes with no commit here, so evidence recorded under the
  label is not reproducible from the plan — which is what RFC-078 §118 exists to
  prevent. Record that each evidence file must capture the **resolved image
  version** at execution time. macOS matters most: it is the only row with no
  manual pass behind it.

## 5. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed. F50's slice is the only
  place a dependency moves, and it is not this slice.
- Release preflight's `cargo audit` must still hard-block (§2.1). If your §2.3
  choice changes any workflow's gating, show the release path is unaffected.
- Existing gates must keep passing, including M4-B's.

## 6. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F55, and reviews 057/058's corrections);
3. changed files;
4. **F55's §2.3 decision and its reason, plus what makes a new advisory
   impossible to miss** — the review's main focus;
5. **the falsifiability demonstration for the scheduled run** (§2), with
   observed output of it failing and passing;
6. **N2's choice** — real checksum or recorded rationale;
7. any difference from this handoff or from RFC-074;
8. executed gates with observed output;
9. unresolved issues and known limitations;
10. requested review focus.

## 7. After this slice

Gate C becomes assessable once this lands and the owner has answered
`matrix-plan.md` §4's remaining fields — the Linux support baseline in
particular, which decides whether F44 is a documented limitation or a schedule
dependency for M5.
