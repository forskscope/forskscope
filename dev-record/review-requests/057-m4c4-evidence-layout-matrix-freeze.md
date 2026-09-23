# Review Request: M4-C4 — Evidence Layout Restructure and Matrix Freeze

**Date:** 2026-08-13
**Reviewer stance:** how F44's expected Linux failure is represented is the main focus, per the handoff
**Repository baseline:** `b7960bc`
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c4-matrix-freeze-handoff.md`, RFC-078

## 1. Implementation summary

One commit (`b7960bc`) covering the whole slice — file moves, RFC-078
amendment, and the plan's content updates are one coherent change, not
independently reviewable pieces the way per-item M4-C2/C3 work was.

- F56: restructured the evidence layout, moved the two existing documents
  out of the placeholder `0.167.0-rc1/` directory, amended RFC-078 to
  match.
- Applied every owner answer from handoff §3.
- Recorded F44's changed standing and the one judgment call it requires
  (handoff §4).
- Carried forward F46's Gate D framing and the review-060 residual note
  (handoff §5).
- Froze `matrix-plan.md` (handoff §6).

## 2. Addressed items

### F56 — evidence layout

Moved `docs/src/maintainers/release-evidence/0.167.0-rc1/{matrix-plan.md,advisories.md}`
to `docs/src/maintainers/release-evidence/{matrix-plan.md,advisories.md}` as
standing documents (`git mv`, confirmed as renames in the commit, not
delete+recreate). Removed the now-empty `0.167.0-rc1/` directory. Amended
RFC-078's "Durable evidence layout" section: replaced the hard-coded
`vX.Y.Z-rcN/` scheme with standing docs (`matrix-plan.md`, `advisories.md`)
plus `<tag>/` per-cut records, and stated the three reasons inline (no `v`
prefix in this project's tags, no Gate D requirement for an RC number given
the draft-release mechanism already provides a candidate, and pre-naming a
directory pre-commits a version level before content exists) so the form
isn't reintroduced by a future reader who doesn't know why it was removed.

### The owner's answers (handoff §3) — all applied

- **Supported platforms unqualified.** New `matrix-plan.md` §1a states
  Windows/macOS/Linux support has no per-distribution floor, distinct from
  which hosts test it (`ubuntu-latest`/`windows-latest`/`macos-latest` CI
  plus the two manual passes).
- **macOS floor 13.0** — settled, no change; `matrix-plan.md`'s row table
  and RFC-078's macOS platform-specific section both updated from
  "provisional pending owner confirmation" to confirmed.
- **Windows `MinVersion`/`MaxVersionTested` unchanged** — settled per F49b;
  same treatment in both documents.
- **Executors filled in** — CI rows: GitHub Actions. Manual rows
  (`linux-wayland` in full, `windows-11`'s F45 prerequisite sub-case): the
  owner. Host-access status resolved as a direct consequence (the owner
  being the named executor implies the owner's own access), not asked as a
  separate question — noted explicitly in `matrix-plan.md` §4 Q3 as a
  judgment call about how to close that field, in case it should have been
  asked instead of inferred.

## 3. F44's changed standing — the review's main focus

See `matrix-plan.md` §3, and the full reasoning below since this is the
one item the handoff explicitly flagged as requiring judgment rather than
mechanical application.

**Why it changes:** F44 (published Linux artifact fails to start on
libxdo-4 distributions — Arch/CachyOS-family) was previously discussable
as "the artifact is Debian/Ubuntu-family, and maybe that's the actual
supported baseline." The owner's unqualified-support answer removes that
escape: there is no per-distribution floor, so a libxdo-4 distribution is
a supported platform like any other. "We only tested Ubuntu" no longer
satisfies a claim of Linux support, and P01 cannot pass on such a host
while F44 is open.

**How the plan represents it, chosen to satisfy three constraints at
once — visible, not hidden; consequential, not a footnote; and not a
pre-declared verdict:**

1. **If the upstream `dioxus-desktop` fix has landed by the time M5 runs
   P01** on `linux-wayland`/`linux-x11`: no special handling. P01 runs in
   full against the fixed artifact, Required as already planned.
2. **If it has not landed:** P01 on a libxdo-4 host is still run and
   recorded as **Fail** — not silently skipped by testing only a
   Debian/Ubuntu-family host, not marked Waived (RFC-078's own Waiver
   policy already forbids waiving "inability to launch on a claimed
   supported platform," and this is exactly that), and not omitted from
   the evidence file. The record states the known cause, the upstream fix
   status, and the tracking link (F44) — an *expected* failure in the
   sense that it isn't a new discovery, explicitly not a *waived* one in
   the sense that matters for a release decision.
3. **What the plan deliberately does not do:** decide in advance that this
   failure blocks the release. That's Gate D's call, made with full
   context at the time, not something this planning slice pre-empts by
   asserting a verdict before the evidence (or the upstream release timing)
   exists. The plan's job is narrower and mechanical: make sure the
   failure, if it happens, is visible and correctly attributed rather than
   laundered into a green row by testing a convenient host.

I chose "run and record as Fail, let Gate D weigh it" over the two
tempting alternatives — pre-declaring a No-Go now (overstepping what this
planning slice is positioned to decide, and needlessly rigid if the
upstream release timing is close), or silently narrowing the test host to
avoid ever exercising the failure (which is the exact "green matrix that
doesn't mean what it appears to" failure mode RFC-078 exists to prevent).

## 4. Changed, moved, and deleted files

**Moved (git rename):**
`docs/src/maintainers/release-evidence/0.167.0-rc1/matrix-plan.md` →
`docs/src/maintainers/release-evidence/matrix-plan.md`;
`docs/src/maintainers/release-evidence/0.167.0-rc1/advisories.md` →
`docs/src/maintainers/release-evidence/advisories.md`.

**Deleted:** `docs/src/maintainers/release-evidence/0.167.0-rc1/` (empty
after the two moves).

**Changed:** `rfcs/proposed/078-platform-runtime-acceptance.md` (layout
section rewritten, Windows/macOS platform-specific F49 notes updated from
provisional to confirmed, new Linux platform-specific note on unqualified
support and F44); `ROADMAP.md` (F56 marked resolved, F44's entry appended
with its changed standing — F44 itself stays open, not resolved, since the
underlying bug isn't fixed yet).

Both moved documents were also substantially edited in place: removed all
placeholder-directory language (no longer "TBD"/"provisional"/pending
questions — every field either has a settled value or points at §4's
resolved-questions record), added §1a (support vs. test-host distinction),
rewrote §3's F44 treatment per §3 above, converted §4 from open questions
to a resolved-questions record, and changed the document header from
"structurally complete, not yet frozen" to "FROZEN as of 2026-08-13" with
what frozen means stated explicitly (handoff §6).

## 5. `0.167.0-rc1` sweep — confirmed current-state-only

`grep -rln "0\.167\.0-rc1"` across the repo (excluding `target/`,
`docs/book/`) now returns exactly two files, both dated historical
records left untouched per the handoff's explicit instruction:

- `ROADMAP.md`'s F7 entry — describes what was done and where *at the
  time* (2026-08-11); the path it names was correct when F7 was resolved.
- `rfcs/handoffs/074-v1-release-stabilization-program/m4c4-matrix-freeze-handoff.md`
  itself — the handoff's own problem statement, a directive document I
  don't edit.

`ROADMAP.md`'s F56 entry (the problem statement written when F56 was
registered) also mentions the old path once, in describing the problem —
left as-is; only my own "Resolved" addendum was added, matching how every
other resolved F-item in this project appends rather than rewrites.

## 6. Difference from the handoff, RFC-074, or RFC-078

One judgment call beyond §3's F44 decision, flagged for review: handoff §3
says host-access status "was a bureaucratic field, not a decision" once
executors are named, and I resolved it by inference (owner-executed implies
owner's-own-access) rather than treating it as a still-open question.
`matrix-plan.md` §4 Q3 states this inference explicitly rather than
silently closing the field, in case it should have been asked rather than
inferred.

## 7. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass — 1094, unchanged
cargo xtask i18n                                                 pass — 227 keys, unchanged
mdbook build docs                                                pass, no broken-reference/anchor warnings despite the file moves
git diff --check                                                 pass
```

CI run
[`31703130411`](https://github.com/forskscope/forskscope/actions/runs/31703130411)
on `b7960bc`: full success.

No dependency added, removed, or version-changed. No product behavior
changed — this slice is documentation and file layout only, confirmed by
the unchanged test count and a clean `cargo build --workspace`.

## 8. Unresolved issues and known limitations

- §6's host-access inference — flagged above, not re-asked as a blocking
  question since the handoff called it bureaucratic, but noted in case
  that reading was wrong.
- F44 itself remains genuinely open (gated on an upstream `dioxus-desktop`
  release, not on anything this slice could fix) — only its *standing*
  changed, not its status.
- RFC-078's own status line stays "Proposed," correctly — the matrix is
  planned and frozen, not executed; M5 is what changes that.

## 9. Requested review focus

1. §3's F44 representation — whether "run and record as Fail, let Gate D
   weigh it" is the right shape, or whether the plan should go further
   (e.g., naming in advance what evidence would make Gate D lean Go versus
   No-Go on this specific input).
2. Whether the host-access inference (§6) needed to be a separate question
   rather than a closed field.
3. Whether `matrix-plan.md`'s freeze declaration (§4/handoff §6) states
   "what frozen means" with enough precision to actually stop a future
   slice from quietly amending a row without recognizing that as
   invalidating prior evidence.
