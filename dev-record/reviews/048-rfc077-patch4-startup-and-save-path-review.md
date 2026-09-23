# RFC-077 patch 4a+4b — startup and save-path migration review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/045-rfc077-patch4-startup-and-save-path-migration.md`
**Baseline:** `5be4086`, over `b387f59`
**Governing documents:** RFC-077; `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`; review 047
**Review mode:** Independent verification of a file-safety-critical path. No implementation changes made.

## 1. Verdict

**Corrections Required.** Two findings, both in the Save As path, both against
RFC-077's own acceptance criteria.

**B3 itself is closed** and the runtime evidence proves it end to end. The
mergetool path — the defect this workstream exists to fix — is correct:
compared inputs untouched, merged target written, pre-existing content backed
up. That is real and I am not asking for it to be redone.

The corrections are in Save As, which patch 4b also migrated, and which the
request's own "Not Addressed Here" section partly anticipates. B4 remains open;
v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1090**, exactly 1077 + 13 |
| `cargo clippy --workspace -- -D warnings` | Pass |
| CI run `30883973553` | `success` on `5be40860` |
| `build_request` reads `right_doc`/`right_path` | **Neither** — `save_target` only |
| `handle_result` updates `save_target`, not `right_doc.fingerprint_at_load` | Confirmed |
| Test-count arithmetic | Reconciles against the corrected 1077 baseline |

The test-count correction from review 047 was carried and the new figures check
out. Naming the previously misreported number rather than quietly restating it
is the right handling.

### B3 is genuinely closed

The runtime evidence is the strongest this project has produced. Existing merged
target: `merged.txt` receives the result, `merged.txt.bak` holds the exact
pre-save bytes, `local.txt` and `remote.txt` byte-identical. Missing target:
created, no spurious `.bak`. Both headers show compared inputs, never the output.

That is the defect RFC-077 was written for, demonstrated rather than argued.

### The permission finding is exactly why runtime evidence is required

`NamedTempFile` defaults to `0600`, so no-clobber-created files were private
where `atomic_replace`-created files were not. Nothing in the type model could
have surfaced that — it took running the binary and looking at `ls -la`. Finding
it, fixing it, and adding a mode-asserting test is precisely the behaviour the
handoff's runtime-evidence requirement exists to produce.

## 3. Corrections required

### C1 — A confirmed overwrite after a Save As conflict writes to the wrong target

`save_as` now passes `force: false`, so a conflict is reachable. On conflict,
`handle_result` sets `Modal::ConfirmOverwrite(index)`. That modal calls
`save_tab_force` → `save_tab(store, index, true)` → `build_request(store, index,
true, **None**)`.

`target: None` means the tab's own `save_target` — **not** the Save As
destination the user chose.

```text
user: Save As → /path/X
  inspect(X) → MustMatch(fingerprint_now)
  something changes X, or X is created after a MustBeAbsent inspection
  save_text → Conflict
  → ConfirmOverwrite modal
user: "Overwrite"
  → save_tab(force) → build_request(..., None) → tab.save_target
  → writes to the tab's original target, not X
```

The user asked to write to X, confirmed an overwrite, and the write lands
elsewhere. A `.bak` is created, so nothing is destroyed unrecoverably — but this
is a wrong-target write, which is the exact defect class B3 is about, in the
patch that closes B3.

Reachability is narrow: it needs the destination to change between inspection and
write. That is why this is a correction rather than a blocker on the milestone.
But `ConfirmOverwrite(index)` carries only the tab index; it cannot express
*which* target was attempted. The confirmation has to carry the attempted target,
or Save As needs its own confirmation path.

### C2 — A blocked Save As destination fails silently

When `inspect_save_target` returns `SaveTargetState::Blocked`, `build_request`
returns `None` and `save_as` returns with no message. Choose a directory as the
Save As destination and nothing happens at all.

RFC-077 §"Mergetool target preparation" requires the opposite for unsupported
targets: "Produce a blocking, user-visible target error with Save As and Cancel
where safe." A silent no-op is not a user-visible error, and unlike C1 this needs
no race — it is reachable on the first try.

The `Blocked` reason is already computed and carries a message; it just is not
shown.

## 4. Answers to the requested review focus

### 4.1 Landing 4a and 4b together

**Your call was right and mine was wrong.** I specified 4a as "save path
untouched; `save_text` still reads `right_doc.fingerprint_at_load`" — which is
internally inconsistent, because 4a's entire purpose is making `right_path` mean
the compared input. Save reading it would then write to `<remote>`. I did not
trace the intermediate state before recommending the split.

You traced it, saw that the intermediate is strictly worse than the bug being
fixed, and kept the review granularity — two commits — while refusing to
publish a broken stopping point. That is the correct resolution, and pushing back
with the reason rather than complying was the right instinct.

### 4.2 The `0o644` permission fix

**Correct to fix, but the constant is wrong — it ignores umask.**

`atomic_replace` uses `fs::write`, which creates `0666 & ~umask`. A user with
`umask 077` gets `0600` from a normal save and `0644` from the no-clobber path.
The mergetool output is then more permissive than that user's environment asked
for — for a tool whose users diff credentials and production logs, that is the
wrong direction to err.

The defect is that one product now has two save paths with different permission
behaviour. Whether the fix derives the mode from umask (a probe file in the same
directory is the portable way to learn it) or preserves an existing target's
mode is an implementation choice.

This is adjacent to F9 (audit N2, "does not preserve the original file's
permissions/extended metadata") but distinct — F9 is about *preserved* files,
this is about *newly created* ones. **Registered as F38.**

### 4.3 Save As confirmation — patch 5, but with a caveat worth stating

**Patch 5 is the right place for the confirmation UI**, and flagging it rather
than letting it look closed was correct.

The caveat: for Save As, the derived precondition is doing almost no work. It is
computed from a contemporaneous inspection, so `MustMatch` is satisfied
microseconds later and the file is overwritten without asking. RFC-077's
acceptance criterion "Save As never bypasses conflict checks merely because a
path was selected" is met in letter — checks run — while its test-design
requirement "select an existing Save As destination: overwrite confirmation is
required" is not met at all.

Track it as an open RFC-077 acceptance item, not a review-request note. C2 above
is the reachable part of the same gap and should not wait.

### 4.4 Runtime-evidence breadth

**Sufficient for B3; not yet sufficient for RFC-077.**

Those are different claims and the distinction matters. B3 is "mergetool save
target and fingerprint are not updated atomically with load completion" — closed,
and the two runtime cases demonstrate it. The deleted/replaced/externally-modified
transitions are RFC-077 acceptance criteria, and the handoff assigns them to
patch 5.

So: claim B3 closed, do not claim RFC-077 complete, and carry the transitions
into patch 5 with runtime coverage for at least the externally-modified case —
it is the one a real user hits.

## 5. Notable quality observations

- Disagreeing with review 047's split, with a traced reason, is worth more than
  compliance would have been. The reasoning generalises: an intermediate state
  that regresses a safety property should not exist as a reviewable commit even
  briefly.
- Carrying review 047's test-count correction explicitly, including naming the
  previously misreported figure, keeps the evidence chain honest.
- Distinguishing what was proven by runtime observation from what was asserted
  from the diff — the `(merge)` title suffix confirmed through AT-SPI because the
  tab strip truncates — is the right standard.

## 6. Recommended next action

1. Apply **C2** now; it is reachable without a race and is a small change.
2. Apply **C1** before M3 closes — `ConfirmOverwrite` must carry the attempted
   target.
3. Patch 5: target transitions, presentation, Save As confirmation, docs.
4. **F38** (permission/umask) before M3 closes.
5. F23 still gates M2's cut.
