# RFC-074 R0 stabilization-baseline review

**Review date:** 2026-08-01
**Request:** `dev-record/review-requests/028-r0-stabilization-baseline.md`
**Stated baseline:** `899497f`; **actual repository HEAD at review:** `d9ec16e`
(`release: post-release version bump to 0.166.0`)
**Governing documents:** RFC-074 milestone R0;
`rfcs/handoffs/074-v1-release-stabilization-program/r0-stabilization-baseline-handoff.md`
**Review scope:** R0's eight tasks, the addendum's release execution and F17
remediation, evidence accuracy, and lifecycle/documentation conformance.
**Review mode:** Independent verification against the repository and the
GitHub API; no implementation changes made.

## 1. Verdict

**Conditionally Approved.**

R0 is substantively complete and correctly implemented. Every task in the
handoff was executed, every gate claim in the request was independently
reproduced, and all four artifact digests match the GitHub API exactly. The
milestone achieved its purpose: it reconciled the release baseline and it
proved the release pipeline by running it, which immediately surfaced a real
Windows build failure that configuration review had not and could not have
found.

The conditions are six documentation-currency findings and one process
correction. **None blocks publishing the `0.165.0` draft release** — see §3 for
why they should land in `0.166.0` rather than trigger another re-cut.

R0 closes no audit blocker. B2, B3, and B4 remain open; the v1/public release
decision remains **No-Go**.

## 2. Blocking findings

None.

## 3. Non-blocking findings

All six are documentation currency. The `0.165.0` artifacts are already built
and digested, so correcting them now would make the published source tarball
diverge from its recorded digest. They should land in `0.166.0`; re-cutting a
tag for documentation currency would be disproportionate.

### N1 — ROADMAP progress record contradicts the delivered state

`ROADMAP.md:73-79` still reads "R0 is approved and is the active milestone" and
"R0 and M2–M6 remain outstanding." R0 is complete and released. `ROADMAP.md:3`
still says "post-v0.164.0 stabilization baseline", and `ROADMAP.md:218` labels
the UI slice section "status at v0.164.0". The roadmap is the planning
baseline, so its progress record carrying a superseded state is the same defect
class R0 was created to remove.

### N2 — Re-release policy is invoked but not documented in the repository

The addendum justifies the tag re-cut with "same version may be re-cut before
publication." That policy exists only in the superseded v0.164.0 handoff bundle
(`testing-and-gates.md`, Part B). `docs/src/maintainers/release.md` contains no
immutability or re-release section at all.

This is the most consequential of the six. A policy that governs a destructive,
irreversible action must live where the releaser reads it. §4.7 below proposes
the wording.

### N3 — Threat model audit history omits the RFC-075 integrity fix

The audit-history table gained four correctly re-attributed `v0.165.0` rows,
but has no row for RFC-075. RFC-075's own "Security and safety impact" section
states it "prevents integrity failures in which content from one user-selected
path pair is displayed or saved under another tab identity" — a security-
relevant change shipping in this release. Meanwhile the `v0.148.0` row still
asserts "Stale-tab guard prevents write to closed tab", which audit finding B1
established was insufficient. Add a `v0.165.0` row recording the token guard
and superseding that claim.

### N4 — Threat model section heading missed during the version update

`threat-model.md:3` was correctly updated to `v0.165.0`, but `threat-model.md:29`
still reads `## Data flows and controls (v0.164.0)`.

### N5 — `git_lines` doc comment misdescribes its contract

The comment states it returns "trimmed non-empty stdout lines"; the
implementation neither trims nor filters empties. Behaviour is correct today
because `git tag` output contains neither, so this is a comment defect, not a
logic defect. Either trim and filter, or describe what the code does.

### N6 — "already published" message overstates what the check knows

`check_version_not_already_published` triggers on any local tag. A tag created
locally and never pushed is not published. The check's behaviour is right — a
local tag is still a version collision — but the wording asserts more than the
evidence supports. Prefer "version {version} is already tagged; bump it".

### Candidate F18 — `xtask` is outside the formatting gate

The request raises this in its own review question 3 and it is a real gap.
`xtask` is deliberately not a workspace member (DEC-005), so `cargo fmt --check`
never sees `xtask/src/main.rs`, and its committed form has drifted from current
rustfmt output in pre-existing, unrelated places. That drift is what nearly
pushed this change over the 500-ELOC hard threshold. Recommend adding
`cargo fmt --manifest-path xtask/Cargo.toml --check` to CI and the release
preflight, then a one-time formatting pass as its own reviewed change. Register
as F18 against M4; do not fold it into `0.166.0` implementation work.

## 4. Answers to the review questions

### 4.1 Is the §1a precondition satisfied?

**Yes, verified.** Run `30690928136`, event `push`, head `f04f5cad` — the
pre-R0 HEAD — conclusion `success`. The backlog was pushed and observed green
standalone, before any R0 edit, exactly as the handoff required.

### 4.2 Is the published-tag check correct across all four cases?

**Yes.** Reading the implementation, the decision table is:

| Condition | Result | Correct |
|---|---|---|
| `git tag --list` errors | `SKIPPED` + reason | ✓ |
| tag list empty | `SKIPPED` ("shallow clone?") | ✓ |
| version not among tags | silent pass | ✓ |
| version is a tag, `--points-at HEAD` includes it | pass | ✓ |
| version is a tag, HEAD elsewhere | `fail` | ✓ |
| `--points-at` errors | `SKIPPED` | ✓ |
| release mode (`expected_version.is_some()`) | not invoked | ✓ |

`--points-at` is the right primitive: it handles annotated and lightweight tags
without separate SHA resolution. The invariant holds forward — after tagging
`0.166.0`, the tagged commit passes and the next commit fails until bumped,
which is precisely the intended forcing function.

### 4.3 Is 498 ELOC an acceptable landing point?

**Yes.** Independently measured at 498 non-blank, non-comment lines (575 raw),
under the 500 hard threshold. More importantly, the implementer hit 521, stopped,
diagnosed the cause, and reduced it by removing unrelated churn rather than by
splitting the file opportunistically — which is exactly what the handoff asked
for. The rustfmt-scope diagnosis is a genuine finding and is registered as F18
above.

### 4.4 Is the CHANGELOG entry accurate?

**Yes.** It is well weighted: the MSRV change and the XLSX fail-closed decision
lead the Security section rather than being buried, the RFC-075 fix is described
in terms of the user-visible failure it prevents, and the `app-json-settings`
note correctly attributes the defect upstream without either minimising it or
implying ForskScope shipped a Windows bug. Verified that `git diff 0.164.0 HEAD --
CHANGELOG.md` contains no deletions, so all prior entries are byte-identical
(M-006 satisfied).

### 4.5 Is the F3 threat-model correction accurate?

**Yes.** It states the mechanism (UI serializes its own structs rather than the
core `VersionedEnvelope`), the consequence (`#[serde(default)]` collapses
future-schema and corrupt files into silent defaults), and the tracking
(B2/RFC-076) without characterising it as an exploitable vulnerability. That is
the correct register for a local-file trust issue.

### 4.6 Were other `v`-prefixed tag instructions missed?

**No.** An independent repository-wide search returns only descriptive prose —
the ROADMAP rationale, the CHANGELOG describing the fix, and the handoff
describing the defect. No instruction survives. `release.md` now documents the
unprefixed form and explains why.

### 4.7 Was deleting and re-pushing the tag the right call?

**Yes, and it was the better of the two options** — but the policy must be
written down.

No GitHub Release existed at any point for the first push: the workflow failed
at the Windows job, `Create GitHub Release` requires all four platform jobs, and
the manually created release was deleted. So nothing was ever published or
consumable. The alternative — shipping `0.165.1` — would have left `0.165.0`
permanently tagged at a commit whose Windows build does not compile, with no
artifacts, which is a worse historical record than a re-cut of an unpublished
tag.

The risk that makes tag mutation normally unacceptable is that consumers who
already fetched retain a divergent tag. Here there were no consumers and no
published release. Verified current state is consistent: annotated tag object
`bd2dc989` peels to `aab3f629`, and local and `origin` agree exactly.

The problem is that "before publication" was doing decisive work while being
undefined and undocumented. Recommended wording for `release.md`:

> A version is **published** once its GitHub Release is out of draft state.
> Before that point the tag may be re-cut: delete the remote tag, re-tag the
> corrected commit, and record the re-cut in the release's CHANGELOG entry.
> After that point the version is immutable — supersede it with a new patch
> version. Never re-cut a tag whose release has left draft, even to fix a
> broken build.

## 5. Process assessment of the addendum

### 5.1 The manual release creation, and my share of it

Creating a GitHub Release directly via `gh release create` when CI had failed
bypassed the release mechanism, and under §11.3 it is the "ad hoc workaround
that hides the underlying issue" pattern. The owner identified it, it was
reverted, the underlying defect was then fixed properly upstream, and the
implementer disclosed it unprompted in the review request. Disclosure of a
reverted misstep is the behaviour the process is built to produce, and it is
recorded here as a process note rather than a finding against the work.

The handoff shares responsibility. Its §8 instructed "if the corrected workflow
still does not fire on the pushed tag, stop and report" — it specified
behaviour for *the workflow not firing* and said nothing about *the workflow
firing and a platform job failing*, which is what actually happened. That gap
is mine. Future release-bearing handoffs must cover partial-failure explicitly:
a red platform job is a stop-and-report condition, and releases are created only
by CI, never by hand.

### 5.2 `gh release delete --cleanup-tag`

The flag deletes the remote tag as well as the release. The tag was recovered
from the intact local tag and re-pushed. Worth carrying into the release
documentation as an explicit caution, since the failure mode is silent and the
flag reads as release-scoped.

### 5.3 Review sequencing

The request was written to be reviewed before tagging; the owner approved and
the release proceeded first. Release approval is the owner's authority, so this
is not a deviation — but it means this review is post-hoc with respect to the
tag, the workflow run, and the draft release. Those are verified as facts about
what happened, not gates I cleared in advance. Stating this plainly so the
evidence chain is not later read as pre-approval.

### 5.4 F17 handling

The remediation is exemplary and worth recording as the pattern: reproduce
locally against the target platform, confirm the latest published version is
also affected so the trivial fix is ruled out, identify that the fix must be
upstream, report it, verify the fix by the same reproduction, then re-run the
full gate suite before re-cutting. The upstream fix also resolved two unrelated
latent failures.

## 6. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo xtask version-sync` | Pass — `v0.166.0` |
| `cargo xtask css --check` | Pass — `main.css` up to date |
| `cargo xtask i18n` | Pass — 203 keys covered |
| `cargo xtask audit-deps` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — 943 (643+27+16+2+241+6+7+1) |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| Version in all six enforced locations | `0.166.0` consistently |
| `xtask/src/main.rs` ELOC | 498, under the 500 hard threshold |
| CHANGELOG prior entries unchanged vs `0.164.0` | Pass — no deletions in diff |
| Pre-R0 CI run `30690928136` | `success` on `f04f5cad`, event `push` |
| Release run `30702192142` | `success` on `aab3f629`; all six jobs green |
| Draft release `0.165.0` | Exists, `isDraft: true`, four assets |
| Artifact digests vs request | All four match the GitHub API exactly |
| Tag `0.165.0` local vs `origin` | Identical; object `bd2dc989` peels to `aab3f629` |
| `v`-prefixed tag instructions remaining | None outside descriptive prose |
| F1, F2, F3, F14, F15 corrections | All verified in place |
| `app-json-settings` in `Cargo.lock` | `2.4.1` |

## 7. Missing evidence

- No runtime execution of any `0.165.0` artifact on any platform. Build and
  packaging success is not runtime acceptance; that remains RFC-078/M5.
- The Windows artifact is verified to compile and package, not to run.
- The four platform jobs' internal logs were not inspected beyond job
  conclusions.

## 8. Recommended next action

1. **Owner decision — publish the `0.165.0` draft.** All gates pass, all six
   jobs are green, and the digests are verified. Nothing in this review blocks
   publication.
2. Land N1–N6 in `0.166.0` as a small documentation-currency change, with N2
   (the re-release policy in `release.md`) as the priority item.
3. Register F18 against M4 and do not fold it into implementation work.
4. Amend RFC-074 to record R0 complete, and add the partial-failure and
   CI-only-release rules to the release-bearing handoff pattern.
5. Begin M2 (RFC-076). Its persistence adapters must never install legacy
   persisted IDs as runtime `CompareTabId` values.

Do not read R0's completion as movement on the v1 decision. B2, B3, and B4 are
untouched.
