# F23/F41 — workflow linting and umask gate coverage review

**Review date:** 2026-08-08
**Request:** `dev-record/review-requests/050-f23-f41-workflow-linting-and-umask-gate.md`
**Baseline:** `0573de5`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f23-workflow-linting-handoff.md`
**Review mode:** Independent verification, including the falsifiability demonstration you could not run. No implementation changes made.

## 1. Verdict

**Approved.** F23 and F41 both stand as `**Resolved.**`

§7.2's gap is closed — I ran the demonstration myself (§3.1). One new finding,
non-blocking, registered as **F42** (§4).

**M2's cut is unblocked.** That is the practical consequence: the last item
before the release is done, and the cut is now the owner's call.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check`, `clippy --workspace -D warnings` | Pass |
| `cargo test --workspace` | Pass — 1094, unchanged |
| `cargo xtask version-sync` | Pass — `v0.165.1` |
| CI run `31247867953` | `success` on `0573de5` |
| **Pinned sha256 in `ci.yml` matches the published artifact** | **Confirmed independently** — downloaded `actionlint_1.7.12_linux_amd64.tar.gz`, `sha256sum -c` → `OK` |
| `actionlint` clean against both current workflows | Confirmed — zero findings |
| Steps placed before apt/toolchain install | Confirmed |
| `umask 077` step in its own `run:` shell | Confirmed — no leak into later steps |

The checksum deserves singling out. Until now `8aca8db9…` was an unverified
literal in a workflow file — a supply-chain assertion nobody had checked. It is
correct. That is verified, not assumed, and this review is where that is
recorded.

## 3. Answers to the requested review focus

### 3.1 §7.2's gap — closed, by me, in review

Not acceptable to leave open, so I did not leave it open. `curl` +
`sha256sum -c` + `tar` succeeded here where your sandbox refused, so I ran the
demonstration the handoff asked for. Three mutants against `release.yml` and
`ci.yml`, each reverted immediately:

**Mutant 1 — bad indentation** (`preflight:`'s `name:` over-indented):

```text
.github/workflows/release.yml:15:2: could not parse as YAML: did not find expected key [syntax-check]
exit=1
```

**Mutant 2 — `runs-on: ubunut-latest`,** a plausible typo:

```text
.github/workflows/release.yml:18:14: label "ubunut-latest" is unknown. available labels are … [runner-label]
exit=1
```

Mutant 2 is the more valuable of the two and was not something the handoff
asked for. An unknown runner label is not a syntax error — the workflow parses
fine, and GitHub simply queues the job forever waiting for a runner that will
never exist. That is a release-blocking failure with *no* error message, and it
is exactly the class F23 was registered against. `actionlint` catches it.

**Mutant 3 — an unquoted expansion in a `run:` block:**

```text
.github/workflows/ci.yml:109:9: shellcheck reported issue in this script: SC2086:info:3:6: … [shellcheck]
exit=1
```

So the bundled shellcheck pass is genuinely active, and your §4.3 "zero
findings" is a real result rather than a check that silently did nothing —
locally. In CI it is not established; see §4.

Working tree confirmed clean afterward (`git status --short .github/ crates/`
empty). The verified binary is at `.git-exclude/tmp/f23/actionlint` if you want
it; it is gitignored scratch.

**On how you handled not being able to do this.** Correctly, and I want to be
specific about why. You could have presented the clean CI run as though it
were the demonstration — it superficially resembles one — and instead you wrote
"this is a real gap against §5's requirement, not a substitute for it." That
distinction is the whole job. You also stopped after the second sandbox denial
rather than trying a third variant, which is what your instructions say. The
mitigations you substituted (manual shellcheck over every `run:` block, `cat -A`
on the inserted YAML) were the right shape too: they cover today's content,
which is what was available to you.

### 3.2 The `"permissions"` substring convention — your doubt is right, the remedy isn't a macro

You asked whether it needs stronger enforcement "given F41 exists precisely
because an existing check silently stopped covering what it was credited for."
That is the correct instinct and the correct reason, and it applies to your own
step: if the F38 test is ever renamed, the filter matches only the incidental
`copy_io_hints_check_permissions`, the step passes green, and the coverage is
gone with no signal.

But an attribute macro or a naming lint is disproportionate to a two-test
filter, and it puts the enforcement somewhere nobody reads it. The property to
want is narrower:

> The step must fail if its filter stops matching the F38 regression test.

The mechanism is yours. Be aware of the trap while choosing: `cargo test` with a
filter matching **nothing** exits **0**, and so does `-- --exact` with an
unknown name — libtest has no "fail if the filter selected no tests" mode. So
any solution has to assert on what actually ran, not merely run it.

Non-blocking, and folded into F42 with the §4 finding, because they are the same
defect in the same file.

### 3.3 One commit — correct

Yes, unambiguously. 38 lines, one file, and the commit message explains both
halves and why they are together. Splitting would have produced two reviews of
one diff.

The register move (F23 "before M2's release cut" → `M2`) is fine — the phrasing
was descriptive, not load-bearing, and F23 is resolved now regardless.

## 4. New finding — F42: both new checks can silently degrade

The shellcheck half of the lint runs **only if a `shellcheck` binary is on
PATH**. When it is absent, `actionlint` does not warn, does not note it, and
does not fail — it just skips the rule. Verified by experiment: the identical
Mutant 3 that produced `SC2086` above exits **0** when `shellcheck` is removed
from PATH.

```text
mutant 3, shellcheck on PATH     → SC2086 reported, exit=1
mutant 3, shellcheck off PATH    → no output,       exit=0
```

`ubuntu-latest` ships shellcheck today. The point is that nothing in this
repository depends on that being true, and nothing would tell you if it stopped
being true. GitHub has removed preinstalled tooling from runner images before.
If it goes, `release.yml`'s eleven `run:` blocks quietly stop being checked and
CI stays green — F23's defect, reinstalled one layer beneath F23's fix.

**This also bounds what your §4.3 evidence proves.** A clean `actionlint` run in
CI is equally consistent with "shellcheck ran and found nothing" and "shellcheck
never ran," and nothing in the run output distinguishes them. Your *manual*
shellcheck pass is what actually supports "zero findings" — that evidence holds,
and it covers today's content. What is unsupported is that CI will catch
tomorrow's. Since the durable gate is the entire point of F23, that is worth
separating cleanly rather than letting the CI run appear to cover both.

Registered as **F42** at M4, covering both guards:

- assert `shellcheck` is present before running `actionlint`, so its absence
  fails loudly instead of narrowing the check silently;
- assert the F41 filter still matches the F38 regression test (§3.2).

Both are a line each, both convert a silent degradation into a loud one, and
both are in `ci.yml`. **Not blocking the cut** — the release-blocking rules
(syntax, expressions, runner labels) run regardless of shellcheck, and Mutants 1
and 2 prove they work. F42 hardens a layer that is currently working.

## 5. Notable quality observations

- Running a manual shellcheck pass over every extracted `run:` block *because*
  you could not run the tool, rather than claiming the tool's result. That is
  substituting evidence you can get for evidence you cannot, and saying which
  is which.
- Reading the sha256 from the release's own published `checksums.txt` rather
  than computing it from whatever you happened to download. Those are different
  things, and only one of them is a check.
- Naming the incidental second filter match and calling it harmless, instead of
  reporting "2 tests pass" and leaving me to discover the second one.
- Asking whether your own new check has F41's disease. It does, mildly, and you
  found that before I did.

## 6. Recommended next action

1. **Nothing blocks the cut.** M2's remaining exit criterion is verification at
   a real release cut, which is the owner's action, not yours. RFC-076's schema
   change is expected to promote it to `0.166.0`.
2. **F42** rides with M4 alongside F40. Not before the cut.
3. After the cut lands and M2 closes, **M4** opens: Gate C, advisory
   dispositions, `matrix-plan.md`, and the accumulated register (F6–F9, F13,
   F16, F18, F24, F25/F25b, F31, F34–F37, F39, F40, F42).
