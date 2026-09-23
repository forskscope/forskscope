# RFC-077 patch 3 — normal compare migration review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/044-rfc077-patch3-normal-compare-migration.md`
**Baseline:** `064584f`, over `5500b81` (review 046 N1 fix)
**Governing document:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved,** with one evidence correction (§4) and a recommendation to split
patch 4 (§3.2).

The migration is behaviour-preserving where it claims to be, `save_target` is
installed inside the RFC-075 token-gated commit rather than as a second
mutation, and the N1 fix is better than what I suggested.

B3 is **not** closed — `diff_actions.rs`, `app.rs`, and `main.rs` are untouched,
so save still reads `right_doc.fingerprint_at_load`. That is patch 4.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30880930331` | `success` on `064584f4` |
| `diff_actions.rs`, `app.rs`, `main.rs` | **Untouched** — empty diff |
| `CompareRequest`/`StartupRequest` in the diff | **Zero** — see below |
| `save_target` installed atomically | Confirmed — same `LoadResult::Ready` arm as the other five fields |

### The N1 fix is better than what I proposed

`symlink_metadata` rather than `metadata` is the right choice and I did not
specify it. A dangling symlink at the target is now a `Conflict`, which matches
what `persist_noclobber` would do at commit time — so the advisory check and the
authoritative commit now agree instead of the early check reporting "absent" for
something the commit would refuse. `NotFound` → `Ok`, everything else → `Io`.

### The "zero occurrences" claim holds

I grepped `git show` and found one hit, which looked like a discrepancy. It is in
the **commit message**, not the diff. Their claim was specifically about
`git diff`, which excludes the message. Precise wording, correctly verified —
noting it because I checked and it stands.

### `save_target` is genuinely atomic

Installed in the same `LoadResult::Ready` arm as `left_doc`, `right_doc`,
`diff`, `merge`, and `can_save`, behind `commit_load_result`'s token check. This
is the property RFC-077 §"Async integration" requires — the snapshot commits
*with* the compared documents, which is precisely what the current mergetool code
gets wrong by mutating `right_path` post-spawn.

## 3. Answers

### 3.1 The §3.4 placement deferral

**Accepted, and the reasoning is right.** Patch 3's scope genuinely cannot
exercise the test — `open_compare`/`reload_tab`/`load_and_diff` still take
`(PathBuf, PathBuf)`, so no `CompareRequest` is constructed or consumed anywhere.
Reporting "the test doesn't fire yet, here's when it will" is more useful than a
guess dressed as an answer.

Report it at patch 4, when `open_compare` either takes a `CompareRequest` or
takes its fields one at a time. That is the observation that settles placement.

### 3.2 Should patch 4 proceed directly? — Split it

**Proceed, but as two patches rather than one.**

Patch 4 as scoped bundles three distinct risk surfaces:

1. `main.rs` argument parsing onto `StartupRequest` — behaviour change if arity
   handling shifts;
2. `app.rs` startup migration — this is the hook review 041's C1 repaired, and
   the handoff §4.2 warns explicitly against moving session resolution back
   inside the branch;
3. `diff_actions.rs` save-path migration onto `save_target` — the change that
   actually alters what gets written to a user's disk, and the one closing B3.

The first two are restructuring that must prove *no* behaviour change. The third
deliberately changes behaviour on the file-safety path. Reviewing them together
means the diff that must be boring and the diff that must be scrutinised arrive
under one attention budget — which is the argument M2-A made for splitting the
release-mechanics slice out of the persistence rewrite, and it applies here for
the same reason.

Suggested split:

- **4a — startup and request migration.** `main.rs` + `app.rs` onto
  `StartupRequest`/`CompareRequest`. Save path untouched; `save_text` still reads
  `right_doc.fingerprint_at_load`. Acceptance: normal compare and mergetool
  launch unchanged, and `future_version_session_stays_byte_identical_through_a_disabled_save`
  still passes. This is also where §3.4's placement question gets answered.
- **4b — save-path migration.** `diff_actions.rs` and `save_text` routed through
  `save_target` exclusively. This closes B3 and carries the runtime-evidence
  requirement from handoff §6.

No additional checkpoint beyond reviewing each. Two smaller reviews beat one
large one plus a mid-patch pause.

### 3.3 `CompareLaunchMode` deferral

**Agreed — don't add it now.** An enum with one meaningful variant is dead weight
until `MergeTool` supplies the second, and RFC-077 lists it in a code block that
describes the end state, not a per-patch requirement.

Land it in **4a**: it is launch metadata, it arrives with the launch mode that
makes it meaningful, and patch 5's presentation layer (`Result: /path/to/MERGED`)
is its first consumer.

## 4. Correction — the test-count evidence does not reconcile

The reported total is wrong, and the delta attribution is incomplete.

| | Reported | Actual |
|---|---|---|
| Raw `cargo test --workspace` | 1069 | **1077** |
| Delta from 1063 | +6 | **+14** |

The request's own itemisation sums to 1077 — 727 core + 267 ui-logic + 70
`forskscope-ui` + 6 CSS + 7 doctests — so the headline 1069 contradicts the
figures beneath it.

The delta breaks down as:

- **+12 raw** from patch 3's 6 distinct `forskscope-ui` tests, counted in both
  the lib and bin targets. The request acknowledges the doubling in its table
  header but then states the delta as `+6`.
- **+2 core** from `5500b81`, the N1 fix, which added two tests to
  `save_target_tests.rs`. Those are inside the baseline range this request
  declares (`fe234f5` → `064584f`) but appear nowhere in its accounting.

Nothing is broken and every test passes — this is an evidence-accuracy defect,
not a correctness one. It matters because itemised deltas are the mechanism by
which I can confirm no test was quietly dropped, and a total that does not
reconcile with its own parts cannot serve that purpose. The N1 tests in
particular deserve to appear: they are the evidence for the fix I asked for.

Correct the figures in the patch 4 request rather than amending this one.

## 5. Notable quality observations

- Flagging that the three error-message tests are **new coverage, not regression
  tests** — because `load_and_diff` had no direct test before — and reading the
  pre-patch source to confirm the exact strings rather than trusting the refactor,
  is exactly the distinction most requests blur. "Proving unchanged behaviour"
  for an untested function is a claim that needs that caveat.
- Confirming the borrow ordering of `save_target_from_loaded(&right, &rd)` "by
  the compiler, not just by reading" is the right standard for an ownership
  question.
- Folding the N1 fix as its own commit before the patch, rather than mixing it
  in, kept both diffs readable.

## 6. Recommended next action

1. Proceed to **4a** — startup and request migration, carrying
   `CompareLaunchMode` and answering §3.4.
2. Then **4b** — save-path migration, closing B3, with runtime evidence.
3. Correct the test-count figures in 4a's request.
4. F23 still gates M2's cut and is untouched by this work.
