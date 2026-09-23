# RFC-077 target-model checkpoint review (patches 1–2)

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/043-rfc077-target-model-checkpoint.md`
**Baseline:** `fe234f5` (`core+ui-logic: RFC-077 patches 1-2 — target model, precondition, no-clobber`)
**Governing document:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md` §4.5 checkpoint
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Accept with notes. Proceed to patches 3 and 4.**

The type and precondition boundary is sound, the no-clobber commit is genuinely
atomic rather than check-then-write, and the checkpoint stopped exactly where the
handoff asked — before anything in `forskscope-ui` moved.

One doc/code mismatch (§4), and one placement answer that is conditional rather
than settled (§3.4).

B3 is **not** closed — the production race in `app.rs` is untouched by design.
B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo test --workspace` | Pass — **1063** (680+27+16+2+29+29+267+6+6+1), exactly 1031 + 32 |
| `crates/forskscope-ui/` | **Untouched** — empty diff across the checkpoint |
| `tempfile` promotion | Threat-model row present, naming role, locality, and re-audit |
| Core references to `CompareRequest`/`StartupRequest` | **None** — the ui-logic placement creates no circularity |
| `save.rs` / `compare_prep.rs` / `startup.rs` ELOC | 148 / 121 / 177 — all well under threshold |

### `persist_noclobber` is correct

This is the part that had to be right, and it is:

- `NamedTempFile::new_in(dir)` puts the temp beside the target, so the commit is
  a same-filesystem operation rather than a cross-device copy;
- bytes are written in full before the commit is attempted;
- `persist_noclobber` is the atomic primitive — not `exists()` followed by a
  rename, which is the race RFC-077 explicitly rejects;
- `AlreadyExists` maps to `CoreError::Conflict`; every other error stays `Io`;
- on failure the `PersistError` carrying the `NamedTempFile` is dropped inside
  the `map_err`, so the temp is removed without touching the competing target —
  RFC-077's stated requirement, satisfied by construction rather than by an
  explicit cleanup path that could be skipped.

The `_with_hook` seam is the right shape: a closure between full write and
commit, `pub(crate)` so the race seam is not public API. That is the
deterministic alternative to sleeps the handoff asked for.

### `check_precondition` reuses the right thing

Building `MustMatch` on `check_external_state` rather than a second fingerprint
comparison is the correct call, and the doc comment citing review 038's C1 —
conflict and I/O must stay distinguishable — shows the precedent was read rather
than rediscovered.

## 3. Answers to the review questions

### 3.1 `compare_prep.rs` as its own module

**Keep it separate.** `save.rs` is about committing bytes; `compare_prep.rs` is
about deriving *what to commit where*. Merging them would conflate preparation
with commit and push `save.rs` toward the threshold for no structural gain.

The mirroring you noticed — `TargetExpectation` beside `TargetPrecondition` — is
worth a cross-reference in both doc comments rather than co-location. One is the
snapshot taken at load time, the other is the assertion made at write time; they
should be readable together without living together.

### 3.2 Reusing `check_external_state`

**Right call, and stronger than the request claims.** You describe the missing-
target-now-conflicts change as a "behaviour refinement". It is actually
conformance: RFC-077 §"Core commit semantics" says `MustMatch` "conflicts when
the path is missing, is no longer a regular text target, or its current
fingerprint differs." `save_text`'s inline `&& target.exists()` skip does not
satisfy that; `check_external_state` does.

Worth naming what you fixed in passing: `check_external_state` was tested code
with zero production callers — the same shape as audit finding B2, where core's
tested persistence models were not the ones running. Wiring it is the right
direction of travel.

### 3.3 `NotAPlainFile` versus a dedicated `Directory` variant

**Wait.** `classify()` does not distinguish directories from other non-regular
entries, and inventing a distinction the layer below does not make would put the
type ahead of the evidence. RFC-077's test requirement is behavioural — a
directory at the target blocks the save and is not removed — and
`NotAPlainFile` satisfies it.

Add the variant when the presentation layer shows it needs to say "this is a
folder" specifically. Deciding it then costs one enum arm.

### 3.4 `StartupRequest`/`CompareRequest` placement — accept, conditionally

**Accept for now**, with a falsifiable test for whether it holds.

Your reasoning is sound as far as it goes: parsing `argv` is not core's concern,
core has no CLI concept, and RFC-075's precedent put orchestration types in
`ui-logic`. I verified there is no circularity — core references neither type,
and `ui-logic` depends on core, not the reverse.

But the request/result pair now straddles the crate boundary: `CompareRequest`
in `ui-logic`, `PreparedCompare` in core, with the UI destructuring one to feed
the other. That is workable, not obviously right.

**The test that settles it:** if patch 3 finds itself passing `CompareRequest`'s
fields into a core function one at a time — `left_input`, `right_input`,
`save_destination` — that is the signal the type belongs in core, and moving it
then is nearly free because nothing outside has depended on it yet. If the UI
keeps genuinely orchestrating between the two, the split is correct and should
be written into both module docs so the next reader does not re-litigate it.

Report which way it went at patch 3. Do not treat this answer as settled.

### 3.5 Proceed?

**Yes.** Patches 3 and 4 as sequenced.

## 4. Finding

### N1 — `check_precondition`'s doc describes an error path the code cannot take

The doc states that "only a metadata read failure inside `MustBeAbsent`'s
existence check propagates as `Io`." The implementation uses
`target.exists()` (`save.rs:155`), which returns `bool` and swallows metadata
errors — a path that exists but cannot be stat'd reports **absent**. No `Io` can
propagate from that branch.

**The safety property still holds**, and that is worth being explicit about:
`persist_noclobber` is the authoritative check, and it refuses on `AlreadyExists`
regardless of what `check_precondition` concluded — including for a dangling
symlink, which `exists()` also reports as absent. The early check is advisory;
the commit is the guarantee. That layering is correct.

So this is a documentation defect in a file-safety module, not a safety defect.
Either use `fs::symlink_metadata` and propagate genuine errors, or correct the
sentence to describe what the code does. I would slightly prefer the former —
"permission denied" and "absent" are different things a user may need told before
a write is attempted — but either is acceptable, and the doc must not claim
behaviour that is not there.

Fold into patch 3.

## 5. Notable quality observations

- Stopping precisely at the checkpoint boundary, with `forskscope-ui` verifiably
  untouched, is what makes a checkpoint worth having. The C1 property is
  preserved by construction, and you said so in those terms rather than claiming
  a check you had not run.
- Reporting the `inspect_save_target` classification-order problem as something
  found "the hard way" through a failing directory test is more useful than a
  clean narrative would have been — it tells me the test caught it, not review.
- Declining to make the race seam public API is the right instinct about test
  affordances leaking into contracts.
- The `--all-targets` clippy result is reported with its nine pre-existing
  locations named and confirmed outside this checkpoint's files, rather than
  waved at.

## 6. Recommended next action

1. Proceed to patch 3 — normal-compare migration proving unchanged behaviour —
   carrying N1.
2. Report the §3.4 outcome at patch 3.
3. Patch 4 closes B3 in production; that is the one that needs runtime evidence
   for the three-argument mergetool path, per the handoff §6.
4. F23 remains the gate on M2's cut and is unaffected by this work.
