# RFC-074 M4-B Developer Handoff: Gate Integrity

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M4-B — gate integrity, after M4-A (residual correctness), before M4-C (truth reconciliation)
**Register items.** F6, F18, F24, F34, F36, F42
**Baseline.** `main` at `8a99c77`

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend RFC-074 first, then
update this handoff to match.

## 1. Summary

**This slice is the theme of the entire stabilization program.**

The single most repeated finding in this project is *a green gate credited with
more than it measures*. Six instances so far, none found by the gate that was
supposed to catch them:

| Gate | What it was credited with | What it actually measured |
|---|---|---|
| `version-sync` | version integrity | not published tags — 26 commits drifted past `0.164.0` |
| release workflow | producing releases | nothing; the `v`-prefix trigger had never once fired |
| release notes | notes from the CHANGELOG | a compare link; every release would have shipped empty |
| `css_coverage` | the compare view is correct | not layout — F32 shipped misaligned in two releases |
| `i18n` | all user strings translated | not strings that never reach `t()` (F39) |
| F38 permission test | umask correctness | nothing under CI's own umask (F41) |

Every one was found by a human looking at an artifact or a rendered screen.
M4-B is where that stops being the only detection mechanism.

Gate C requires "full documented gates." A gate that cannot fail is not a gate,
so this slice is a precondition for Gate C rather than housekeeping.

This slice changes no product behaviour and closes no audit blocker. B4 remains
open; v1/public release stays **No-Go**.

## 2. Scope

In scope: F6, F18, F24, F34, F36, F42.

Not in scope:

- **M4-C** — F7, F9, F11, F12, F16, F25/F25b, F31, F37, F39, F43, and RFC-074's
  advisories N1–N6. F39 in particular is *adjacent* (the i18n gate is blind) but
  its remediation is a wording-versus-translation decision, which belongs with
  the other truth work.
- **F44** — waiting on a `dioxus-desktop` release.
- **F45, F46** — M5; they need real Windows and macOS hosts.
- **F47** — post-v1.

## 3. The standard for this slice

Every item here must be **demonstrated failing before it is accepted as
working.** Not argued, not inspected — run.

This is the same standard applied to F23/F41 in review 053, where the mutation
that mattered showed a check passing under CI's umask and failing only under
`077`. For each item below, the review request must contain the observed output
of the check *failing* on a deliberately broken input, and the revert.

If you cannot make a check fail, you have not established that it works. Say so
rather than substituting inspection — that judgement was the most valuable thing
in review 050.

## 4. Items

### 4.1 F42 — the gates added by F23/F41 can silently degrade

Closest to the theme, and the smallest. Two guards:

1. **`actionlint`'s shellcheck pass runs only if a `shellcheck` binary is on
   PATH.** Absent, it does not warn or fail — it skips the rule and exits 0.
   Verified by experiment in review 053. `ubuntu-latest` ships shellcheck today;
   nothing here depends on that staying true, and if it stops being true,
   `release.yml`'s eleven `run:` blocks stop being checked with CI still green.
   Make the absence fail loudly.
2. **The F41 umask step's `"permissions"` substring filter** silently covers
   nothing if its one load-bearing test is renamed — `cargo test` exits 0 when a
   filter matches no tests. Make the step fail if it stops matching the F38
   regression test. Note the trap: `-- --exact` with an unknown name also exits
   0, so any solution must assert on what actually ran.

### 4.2 F24 — the CHANGELOG guard fires after the tag exists

The empty-CHANGELOG-section guard runs in the release workflow's **last** job,
after the source archive and all three platform builds. By then the tag exists,
so recovery means a re-cut. The condition is detectable at preflight from the
repository alone.

Extend `cargo xtask version-sync`'s **release mode only** (the form taking a tag
argument) to require non-whitespace content in that version's section. **Dev mode
must keep accepting the empty section** that the post-release bump opens — the
tree is in exactly that state right now, so getting this wrong turns every
subsequent commit red.

Both directions need demonstrating: release mode rejects an empty section, dev
mode accepts one.

### 4.3 F6 — `clippy --all-targets` fails on test-target lints

The mandatory gate is `cargo clippy --workspace -- -D warnings`, which does not
cover test targets. `--all-targets` currently fails.

RFC-074's advisory N6 asks for "both mandatory and stronger all-target clippy
results" to be recorded. Two acceptable outcomes:

1. Fix the test-target lints and add `--all-targets` to the gate. Preferred if
   the fixes are mechanical.
2. If any lint is a genuine false positive or would damage test clarity, allow
   it **narrowly and in place** with a comment giving the reason, then add the
   stronger gate.

What is not acceptable is leaving the stronger form unrun and unrecorded, which
is where it has sat since M1.

### 4.4 F18 — `xtask` is outside `cargo fmt --check`

`xtask` is not a workspace member (DEC-005), so it escapes the format gate and
`xtask/src/main.rs` has drifted from current rustfmt output. The drift nearly
pushed R0's addition past the 500-ELOC threshold, so this is not purely
cosmetic — it distorts a measurement the project uses.

Bring `xtask` under the format gate without making it a workspace member.
Re-verify the ELOC figure afterwards and report both numbers.

### 4.5 F34 — nothing looks at the rendered application

**The most valuable item in this slice, and the least prescribed.**

F32 shipped misaligned in two releases with every gate green, and was found only
when a human took a screenshot. `0.166.0` then shipped with no artifact having
been launched by anyone, and F44 followed.

Add a rendering check to CI or release preflight. Per review 044, build **one**
fixture producing all three label-bearing hunk kinds — Replace, Insert, and pure
Delete — in a single file pair, rather than promoting the two ad-hoc demo
fixtures.

Design decisions left to you, with reasons required:

- **What is asserted.** A screenshot alone is not a check unless something
  examines it. Options include geometry assertions via AT-SPI (cheapest, and
  the F32 defect was positional), an image comparison against a committed
  reference (catches more, brittle across renderer versions), or a DOM/computed-
  style assertion. F32 was found by *looking*; make the machine do the looking.
- **Where it runs.** CI needs a display; the release preflight may be the more
  honest home given the cost.
- **How it fails.** State what a failure looks like and prove it — reintroduce
  the F32 `.sr-only` placement and show the check going red.

If a full check proves too costly, an artifact-launch smoke test that merely
proves the binary starts and renders a window is still worth more than nothing —
that alone would have caught F44. Say clearly which you built.

### 4.6 F36 — decide the `Store` testability question

`Store` cannot be constructed in a test (`Signal::new_in_scope` needs a live
Dioxus scope), so integration seams that touch it rest on runtime evidence
alone. Five occurrences now: RFC-076 patch 4's startup wiring, its C1 CLI-mode
fix, patch 6's recovery queue, RFC-077's Save As default-path regression, and
F40's toolbar guard.

**This is a decision, not an implementation.** Decide deliberately whether a
`VirtualDom`-backed harness is worth introducing, rather than re-deciding it per
patch under deadline. Either answer is acceptable; record it where the next
patch will find it.

If the answer is no, say what carries the weight instead — the pattern of
"pure predicate extracted, unit-tested, component left untested" that F35 and
F40 both used is a real answer, and worth stating as policy rather than
rediscovering.

## 5. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed. `dioxus-desktop` stays
  put until F44's upstream fix is released.
- No product behaviour changes. This slice changes gates, not the app. If a new
  gate exposes a product defect, **register it and report — do not fix it here.**
- Dev-mode gates must keep passing on the current tree, which has an open empty
  CHANGELOG section (§4.2).
- No real user paths, host names, or secrets in workflow files or evidence.

## 6. Sequencing suggestion

Not binding, but F42 and F24 are small and self-contained; F34 is the one that
may need iteration. Landing the small ones first keeps a long F34 investigation
from blocking everything else.

## 7. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F6, F18, F24, F34, F36, F42);
3. changed files;
4. **the demonstrated-failure evidence required by §3, per item, with observed
   output** — this is the review's main focus, and an item without it will be
   sent back regardless of how correct the implementation looks;
5. **F34's design decisions and their reasons** (§4.5), and clearly which of
   the two levels you built;
6. **F36's decision and where it is recorded** (§4.6);
7. any difference from this handoff or from RFC-074;
8. executed gates with observed output, including the new ones;
9. anything a new gate surfaced that you registered rather than fixed;
10. unresolved issues and known limitations;
11. requested review focus.
