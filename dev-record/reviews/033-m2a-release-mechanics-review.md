# RFC-074 M2-A release-mechanics review

**Review date:** 2026-08-02
**Request:** `dev-record/review-requests/029-m2a-release-mechanics.md`
**Baseline:** `896f2c6` (`release: align trigger and gates for v1 stabilization (F19-F22, N5-N6)`)
**Governing documents:** RFC-074 milestone M2-A;
`rfcs/handoffs/074-v1-release-stabilization-program/m2a-release-mechanics-handoff.md`
**Review mode:** Independent verification against the repository and the GitHub
API; no implementation changes made.

## 1. Verdict

**Conditionally Approved.**

The implementation conforms to the handoff exactly. Every specified decision was
followed, the two decisions left to the implementer were made well, and the
documentation work exceeds what was asked in one place. All gates reproduce
independently.

One mandatory correction is required, and it is **a defect in the handoff's
specification rather than in this implementation** — the implementer followed
what was written, and their own test case is what exposed it. See C1.

One evidence gap remains open that neither the implementer nor this review could
close in the available environment. See N1.

The slice changes no product behaviour, cuts no release, and closes no audit
blocker. B2, B3, and B4 remain open; v1/public release stays **No-Go**.

## 2. Mandatory correction

### C1 — The fail-closed guard does not catch an empty CHANGELOG section

`test -s release-notes.md` tests byte length. A CHANGELOG section that exists as
a heading with no body extracts to a single newline — one byte — which passes.

Verified against the committed CHANGELOG:

```text
ver=0.165.1  ->  1 byte
test -s      ->  PASSES
grep -q '[^[:space:]]'  ->  NO NON-WHITESPACE CONTENT
```

The consequence is worse than "notes are thin". Because the compare link is
appended *after* the guard, a heading-only section composes to a blank line plus
`**Full Changelog**: …` — which is **precisely the bare compare link that F22
exists to eliminate**. The original defect is reproduced, in a narrower case,
inside the change that fixes it.

This is reachable in normal operation: the post-release bump opens an empty
`## [X.Y.Z]` section by design, and `version-sync` asserts only that the heading
exists, never that it has content. Tagging before entries are written hits it.

**Required change** — test for content, not bytes:

```sh
grep -q '[^[:space:]]' release-notes.md || { echo "::error::CHANGELOG section for ${GITHUB_REF_NAME} is empty"; exit 1; }
```

Either replace `test -s` or add this after it. **Required evidence:** re-run the
four existing extraction cases plus a fifth — a heading-only section must now
fail the guard — with recorded output.

**Attribution:** the handoff §4.1 specified `test -s` verbatim, and the
implementer used it as written. The request's own testing recorded
`ver=0.165.1 -> 1 byte, test -s passes` and reasoned it was correct for the
current in-development state, which it is; what neither the spec nor that
reading covered is the same state existing *at tag time*. The handoff is being
amended.

**Timing:** this code first executes at M2-B's release cut. The correction must
land before that cut. It does not require re-opening anything else in the slice.

## 3. Non-blocking findings

### N1 — `release.yml`'s YAML validity is still unproven

The request's reasoning that the green CI run demonstrates the file parses is
**overstated**. GitHub evaluates a workflow when an event matches its triggers.
`ci.yml` running green proves `ci.yml` parses. `release.yml` triggers only on
tag push, and no tag has been pushed since it was edited, so nothing has parsed
it. `gh api .../actions/workflows` reports it `active`, but registration is not
validation.

This review could not close the gap either: no `python3-yaml`, `ruby`,
`js-yaml`, `yq`, or `actionlint` is available in this environment, and `pip` is
absent.

The first thing that exercises the file is M2-B's release cut, where a parse
error means no release at all. That is the slice's own thesis — configuration
credited as working without being exercised — recurring one level up, in the
tooling used to verify the fix.

**Registered as F23:** add `actionlint` to CI so every workflow file is parsed
and linted on every push. It is a single binary with a maintained action, it
closes this permanently rather than per-change, and it would have caught the
original `v`-prefixed trigger mismatch as a policy check.

## 4. Answers to the requested review focus

### 4.1 Does the checkout ordering actually prevent the artifact wipe?

**Yes, and the surrounding steps are also safe.** Verified order: `checkout` →
compose notes → `download-artifact` → create release. `actions/checkout` cleans
the workspace, so it must precede the download, and it does. Two further
interactions checked, neither problematic:

- `download-artifact` does not clean, so `release-notes.md` written in the prior
  step survives to `body_path`.
- the `files:` globs are all `forskscope-v*`, so `release-notes.md` cannot be
  swept into the release as an asset.

The explanatory comment left in the workflow is the right call — the failure
mode is silent and reordering looks harmless.

### 4.2 Is the literal-prefix match sufficiently verified for CI's awk?

**Yes.** The script uses only `BEGIN`, string concatenation, `index()`, `next`,
`exit`, and a bracket regex — all POSIX awk, no GNU extensions. It behaves
identically under `gawk`, `mawk`, and BusyBox awk, so `ubuntu-latest`'s awk
holds no surprises. The four local cases are adequate; the adversarial
`0X165X0` case is the one that matters and it is correct.

Independently reproduced: `0.165.0` extracts 106 lines with no leaked headings,
and `0X165X0` yields zero lines.

### 4.3 Is `git tag --sort=-creatordate` acceptable for the previous tag?

**Yes.** Verified it resolves `0.165.0 → 0.164.0` and `0.164.0 → 0.163.0`. The
awk that walks to the entry after the current tag is correct, and the empty
`PREV_TAG` case is guarded so a first-ever or oldest tag omits the link rather
than emitting a malformed one.

The trade-off is worth recording rather than changing. Creation-date ordering
answers "what shipped immediately before this", which is the right question, and
it handles a late hotfix on an older line correctly where `-v:refname` would
not. Its weakness is that a re-cut rewrites a tag's creation date — as R0's
`0.165.0` re-cut did — so a future re-cut of an *older* tag could misorder it.
That is a narrow case, it degrades only a convenience link, and the alternative
has a worse failure mode. Accepted as chosen.

### 4.4 Does `release.md` match §4.3's intent?

**Yes, and the asymmetry paragraph improves on the handoff.** The blockquote was
reproduced as specified. The tagged-versus-published explanation goes beyond it
by stating *why* the two lines differ and explicitly warning against "fixing"
`version-sync` to key on draft state — which is exactly the well-intentioned
change a future reader would otherwise make. The `--cleanup-tag` caution also
adds recovery guidance that was not requested. Both are accepted as
improvements.

### 4.5 Is CI-parse-as-YAML-validation acceptable?

**No** — see N1. The reasoning does not hold, the gap is real, and it is
correctly identified as the slice's own weak point. It is non-blocking because
nothing depends on that file until M2-B's cut, and F23 closes it structurally.
Raising it as a question rather than asserting the file was validated is the
right handling.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo xtask version-sync` | Pass — `v0.165.1` |
| `cargo xtask css --check` | Pass |
| `cargo xtask i18n` | Pass — 203 keys |
| `cargo xtask audit-deps` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — 8 suites, 943 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30738340551` | `success` on `896f2c60` |
| Checkout precedes `download-artifact` | Confirmed in file |
| `generate_release_notes` removed; `body_path` used | Confirmed |
| Extraction: `0.165.0` | 106 lines, no leaked headings |
| Extraction: `0X165X0` | 0 lines — literal match confirmed |
| Extraction: `0.165.1` | 1 byte — **passes `test -s`** (C1) |
| `PREV_TAG` for `0.165.0` / `0.164.0` | `0.164.0` / `0.163.0` |
| `release.md` — publication/immutability, level rule, publish command, caution | All present |
| `threat-model.md` heading | `(v0.165.0)` |
| `threat-model.md` RFC-075 row | Present; supersedes v0.148.0 without rewriting it |
| `xtask` N5 comment / N6 message | Both corrected |
| `xtask/src/main.rs` ELOC | 498, unchanged, under threshold |
| `release.yml` YAML parse | **Not validated** — no parser available (N1) |

## 6. Notable quality observations

- The threat-model supersession is exemplary: the `v0.148.0` row is left intact
  as a record of what was believed then, and the new row explicitly supersedes
  its claim. That is the correct way to keep a security history honest.
- The request distinguishes what the handoff specified from what the implementer
  chose, and flags its own weakest evidence as a question rather than asserting
  it. Both make this review substantially faster and more trustworthy.
- No scope creep: F18 untouched, no `workflow_dispatch` publish trigger added,
  no release cut to "prove" the change, `xtask` held at 498 ELOC.

## 7. Recommended next action

1. Apply C1 with its fifth test case; submit as a short follow-up rather than a
   full re-review cycle.
2. Register F23 (`actionlint` in CI) against M4, alongside F18.
3. Amend the M2-A handoff §4.1 and §5 to specify the content-based guard, so the
   defective `test -s` is not inherited by any future handoff that copies it.
4. Then begin M2-B (RFC-076) against its existing handoff, whose design-review
   pause after the first patch still applies. Its persistence adapters must never
   install legacy persisted IDs as runtime `CompareTabId` values.

M2-A's end-to-end evidence — a real draft release whose body is the composed
CHANGELOG section — still arrives at M2-B's cut. Both C1 and N1 must be settled
before that cut, because that cut is the first time any of this code runs.
