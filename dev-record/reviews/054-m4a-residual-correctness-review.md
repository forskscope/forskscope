# M4-A residual correctness review — F40, F8, F35, F10

**Review date:** 2026-08-11
**Request:** `dev-record/review-requests/051-m4a-residual-correctness-f40-f8-f35-f10.md`
**Baseline:** `db8a95b`, over `8213945`, `b8139e2`, `383ebf8`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4a-residual-correctness-handoff.md`
**Review mode:** Independent verification, including the environment test you could not run. No implementation changes made.

## 1. Verdict

**Approved with one mandatory correction (C1, §3).**

F40, F8 and F35 are complete and correct. F10's production-side fix is correct
and I closed its evidence gap myself (§5.4) — but the same run showed that one
of F10's three *new* tests carries the exact environmental assumption F10 exists
to remove. Small fix, but it must land before F10 stands as resolved.

M4-A is otherwise done. B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check`, `clippy --workspace -D warnings` | Pass |
| `cargo test --workspace` | Pass — **1105**, matching your count |
| `cargo xtask i18n` | Pass — **227** keys |
| `cargo xtask version-sync` | Pass — `v0.166.1` |
| CI run `31460472535` | `success` on `db8a95b` |
| F40 — all three toolbar controls routed through the guard | Confirmed — 3 call sites, no unguarded `recompute_diff` left outside `state/tab.rs` |
| F40 — `is_dirty()` cannot go silently false | Confirmed — guard reads `merge.is_dirty()` before any mutation |
| F8 — `new_fingerprint` is consumed | Confirmed — `diff_actions.rs:309`, feeds `TargetExpectation::MustMatch` |
| F8 — `FileFingerprint.digest` has no reader in the save path | Confirmed — `check_external_state` compares `len` and mtime only; the only `.digest` readers are in `dir/index.rs` |
| F35 — predicate | Confirmed — `kind == Replace && has_content`, pure and directly tested |

## 3. Mandatory correction

### C1 — F10's new negative test reproduces the defect F10 fixes

`ancestor_has_git_is_false_with_no_dotgit_anywhere_in_the_fixture`
(`vcs_tests.rs:293`) builds a fixture under `std::env::temp_dir()` and asserts
`!ancestor_has_git(&dir)` unconditionally. If the OS temp directory sits inside
a repository, that assertion is false — which is precisely the premise of F10.

Observed, running the suite with `TMPDIR` pointed inside this repository:

```text
test ancestor_has_git_is_false_with_no_dotgit_anywhere_in_the_fixture ... FAILED
  assertion failed: !ancestor_has_git(&dir)   (vcs_tests.rs:299)

test detect_returns_none_outside_any_repo ... ok
  skipping detect_returns_none_outside_any_repo: … sits inside an enclosing
  Git repo — environment confound, not a `detect()` defect
```

The two tests you set out to fix behave perfectly. The third, added as part of
the fix, is the one that breaks.

Your comment on it is honest — *"This cannot prove the real system temp root is
clean"* — so you saw the limitation. The gap is that the comment describes it
while the assertion still depends on it.

**Fix:** guard it the same way the other two are now guarded, and skip loudly.
Using `ancestor_has_git` to decide whether its own negative case is testable is
mildly circular, but acceptable here because the two positive tests establish
independently that the walk detects `.git` correctly — and it is strictly better
than an assertion that fails on a legitimate machine. If you prefer something
non-circular, assert a *transition* instead (create `.git` under a fixture
subtree and require the result to flip), which holds regardless of what encloses
the temp directory.

Nothing else blocks. Everything else in this slice stands.

## 4. Answers to the requested review focus

### 4.1 F40 — confirm-over-preserve is the right call, and the argument is stronger than "hunk identity is unstable"

**Verified at source.** `DIFF_COUNTER` is a `static AtomicU64`, `diff_id =
DIFF_COUNTER.fetch_add(1, …)` runs on **every** `compute_diff`
(`engine.rs:20,132`), and `HunkId` is `hash(diff_id, ordinal, left_range,
right_range, kind)` (`model.rs:167`). So every hunk in the workspace gets a new
identity on any recompute — including one that changes nothing at all.

That is a materially stronger finding than the handoff assumed. I wrote that
"hunk identity changes when the diff options change"; in fact `HunkId` is not a
stable identifier across recomputes *at any time*. Design (a) was therefore never
a matter of handling an options-change edge case — it requires a content- or
position-based rebasing rule, which is its own design. Choosing (b) was correct,
and you established why rather than asserting it.

**On rule 4's status: yes, it needs a tracked item, not just a note.** A "Not
met" line with no owner is how a gap becomes permanent — and RFC-015 is marked
Implemented, so a reader has every reason to trust its rules. Registered as
**F47**, scoped as what it actually is: a hunk-identity design question
(stable identity across recompute, or a rebasing rule for the transaction log),
sitting alongside RFC-015's other open items — the history panel and crash
recovery. Post-v1, not M4. Your dated note stays; it now points somewhere.

### 4.2 F8 — your correction is accurate, and it exposes a register duplication

Both halves check out: `new_fingerprint` is genuinely consumed, and the `digest`
field genuinely has no reader in the conflict path. Outcome 3 with a correction
is the right characterisation, and refusing to "fix" it by wiring up a consumer
nobody asked for was right.

One thing to record beyond what you wrote: **F8 and RFC-074's N1 are the same
finding under two labels**, both at M4. RFC-074 §"Advisory dispositions" already
says *"N1 — digest not used for save conflict checks | M4 | Either use the digest
when metadata is inconclusive, or document the same-size/same-mtime
limitation."* That is F8's remediation verbatim.

Two trackers for one item at one milestone is how something gets closed twice or
not at all. F8 now points at N1 as the authority and carries no independent
remediation. Handled in the register; no action for you.

### 4.3 F35 — RFC-019 is the right home, and you were right to flag it

You followed the handoff and said why you doubted it. That is exactly the
behaviour I want, and the doubt is correct: RFC-061 is explorer pane focus and
keyboard completeness; RFC-019 owns row ARIA, which is what this decision is
about. My handoff sent you to the wrong RFC.

Move it to RFC-019, leaving a one-line pointer in RFC-061 since the decision was
surfaced by RFC-061's AT-SPI work. Fold into C1's commit.

The decision itself is right. A bare "Changed" with nothing after it is noise,
and per-row labels on rows that *do* have content are genuinely useful when
navigating a multi-line change. The before/after you documented — four
announcements on the left, one plus three silent rows on the right — is the
correct outcome.

### 4.4 F10 — the evidence gap is closed; I ran it

Your sandbox declined the `TMPDIR`-into-a-repo run, and you stopped rather than
retrying variants, which is right. It succeeded here, and the skip path works
end to end:

```text
skipping detect_returns_none_outside_any_repo: … sits inside an enclosing Git
repo — environment confound, not a `detect()` defect
```

Both "outside repo" tests take the skip branch and report why. So the answer to
your question is that the gap does not need closing — it is closed, by
observation, recorded here.

Deliberately not reusing `detect()` for the precondition was the right call, and
your reason is the correct one: a shared implementation would make a genuine
`detect()` bug and a contaminated environment indistinguishable.

## 5. Notable quality observations

- Investigating design (a) properly enough to find `DIFF_COUNTER`, rather than
  taking the cheaper option and justifying it afterwards. The finding is more
  general than the handoff's framing and is worth more than the fix.
- Marking RFC-015 rule 4 "Not met" with a dated explanation instead of quietly
  leaving it asserted. That is the documentation-truth discipline this program
  keeps having to relearn.
- Catching a stale binary during F35's AT-SPI verification — noticing the tree
  still showed three bare `Changed:` labels, rebuilding, re-verifying — rather
  than reporting the first result.
- Naming the `NO_AT_BRIDGE=1` environment trap. That silently prevents
  accessibility-bus registration, and it would have read as "the app has no
  accessible tree." Worth carrying into the smoke-test docs at M4-B.
- Flagging the RFC-061/RFC-019 doubt instead of silently following or silently
  diverging.

## 6. Recommended next action

1. **C1** — guard or restructure `ancestor_has_git_is_false_…`, plus F35's move
   to RFC-019 (§4.3). One commit; re-run with `TMPDIR` inside a repo and show
   the skip rather than a pass.
2. Then **M4-B — gate integrity** (F6, F18, F24, F34, F36, F42). Handoff to
   follow once C1 is in.
3. F47 is registered for post-v1. F44 still waits on a `dioxus-desktop` release.
