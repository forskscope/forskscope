# RFC-074 M4-C4 Developer Handoff: Evidence Layout and Matrix Freeze

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md), with [RFC-078](../../proposed/078-platform-runtime-acceptance.md)
**Milestone.** M4-C4 — the last slice of M4; Gate C is assessed after it
**Register items.** F56, plus `matrix-plan.md`'s remaining fields (all now answered)
**Baseline.** `main` at `1235083`

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend the RFC first, then
update this handoff to match.

## 1. Summary

Every owner question is answered. This slice applies those answers, fixes the
evidence layout they exposed, and freezes the plan — after which **M4 closes**
and I assess Gate C.

It is small. Nothing here is a judgement call except where §4 says so.

## 2. F56 — restructure the evidence layout

RFC-078 §"Durable evidence layout" hard-codes:

```text
docs/src/maintainers/release-evidence/
  vX.Y.Z-rcN/
    README.md, artifacts.md, matrix-plan.md, advisories.md, <platform>.md …
```

Three things are wrong with it:

1. **The `v` prefix** — this project's tags are unprefixed (`0.166.0`), and
   `release.md` says so explicitly because the release workflow's trigger only
   matches that form.
2. **The `-rcN` component** — nothing in Gate D requires it. Gate D requires
   artifacts built by the release workflow from a known commit, and every result
   naming the artifact digest it tested. This project already produces a
   **draft** release whose tag may be re-cut while in draft, so the draft *is*
   the candidate. A second versioning scheme buys nothing.
3. **Naming a directory before the cut pre-commits a version level** before its
   content exists — the rule `release.md` removed at F21.

**Restructure into standing documents and per-cut records:**

```text
docs/src/maintainers/release-evidence/
  matrix-plan.md      # standing: hosts, cases, executors — freezes now
  advisories.md       # standing: dispositions, policy, upgrade triggers
  <tag>/              # per-cut: created at the cut, named for the tag actually cut
    README.md
    artifacts.md      # digests
    linux-wayland.md, linux-x11.md, windows-11.md, windows-10.md, macos-aarch64.md
```

Move the two existing files out of `0.167.0-rc1/` and remove that directory.
Amend RFC-078's layout section to match, and state the reasoning briefly so the
`-rcN` form is not reintroduced.

**Sweep for references** to `0.167.0-rc1` — reviews 056/057/058 cite those
paths. Those are dated historical records; **do not rewrite them**, consistent
with how `AtomicSaveStrategy` was handled in the archive. Fix only current-state
documents.

## 3. The owner's answers, to be applied

- **Supported platforms: Windows, macOS, Linux — unqualified.** There is no
  per-distribution Linux floor, and the matrix plan should not imply one. The
  *test* hosts remain `ubuntu-latest` / `windows-latest` / `macos-latest` plus
  manual passes; support breadth and test hosts are different things and the
  plan should say so in a sentence.
- **macOS floor: 13.0**, already enforced by `Info.plist` and
  `MACOSX_DEPLOYMENT_TARGET`. Settled; no change.
- **Windows: `MinVersion` stays 1809, `MaxVersionTested` unchanged** pending M5
  evidence (F49b). Settled; no change.
- **Executors:** CI rows are executed by GitHub Actions; the two manual rows
  (Linux Wayland, Windows 11) by the owner. Fill these in — this was a
  bureaucratic field, not a decision.

## 4. F44's standing has changed — record it

Because Linux is supported unqualified, **F44 is a defect on a supported
platform**, not a limitation of an out-of-scope distribution. The published
Linux binary does not start on libxdo-4 distributions; the fix is merged
upstream and awaiting a `dioxus-desktop` release.

The plan must state the consequence plainly: **if that release has not landed
when M5 runs, F44 is a Go/No-Go input, not a footnote.** Linux P01 cannot pass
on such a distribution, and "we only tested Ubuntu" does not satisfy a claim of
Linux support.

This is the one item here with judgement in it: decide how the plan represents a
case that is expected to fail for a known, externally-blocked reason, without
either hiding it or pre-declaring a No-Go. Say what you chose and why.

## 5. Two things to carry forward

- **F46 stays unverifiable** under current resourcing — no macOS manual host,
  and CI cannot observe Gatekeeper because a checkout carries no quarantine
  attribute. Already recorded in RFC-078; make sure `matrix-plan.md` names it as
  an open Gate D input rather than a passing row. One person opening the DMG on
  any Mac once would close it.
- **Review 060's residual** — `audit.yml`'s `push`/`pull_request` trigger has
  never fired. F44's eventual `dioxus-desktop` bump is a `Cargo.lock` change and
  is the natural first exercise. Note in the plan that that slice's review
  should record whether the audit workflow fired.

## 6. Freeze

Once §2–§4 land, mark `matrix-plan.md` **frozen**, with the date and what frozen
means here: hosts, cases and executors are fixed, and changing them after M5
begins invalidates the evidence gathered under the previous plan.

## 7. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed.
- No product behaviour changes. This slice is documentation and file layout.
- Do not gather platform evidence. That is M5.
- Do not rewrite dated historical records (§2).

## 8. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F56, the plan's remaining fields, the freeze);
3. changed, moved and deleted files;
4. **how the plan represents F44's expected Linux failure** (§4), with reasoning
   — the review's main focus;
5. confirmation that the `0.167.0-rc1` sweep touched only current-state
   documents;
6. any difference from this handoff, RFC-074, or RFC-078;
7. executed gates with observed output, including `mdbook build docs` given the
   file moves;
8. unresolved issues and known limitations;
9. requested review focus.

## 9. After this slice

**M4 closes.** I assess Gate C against RFC-074's criteria — full documented
gates, docs/RFC status synchronized, advisory dispositions recorded,
`matrix-plan.md` frozen — and the release-core candidate becomes eligible for
M5's platform QA, or does not.
