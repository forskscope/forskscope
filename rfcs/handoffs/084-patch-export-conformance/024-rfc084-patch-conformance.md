# Handoff 024 — RFC-084: patch export conformance, and F92's remaining claims

**From:** architect. **RFC:** `rfcs/accepted/084-patch-export-conformance.md`
(accepted 2026-09-05). **Release:** 0.170.0. **Register:** F91, F92.

## 1. The organizing fact

**The documentation corrections and the code fixes are the same work.**
`patch-export.md:4` says *"The patch is compatible with `patch -p1` and `git
apply`"* — that is false **because of** the defects below. Fix the code and the
sentence becomes true. Correct the prose alone and you have documented a defect
instead of fixing it.

So do not treat §5 as a tidy-up pass at the end. Each claim resolves *as* its
defect resolves, and which way it resolves is the outcome of the work.

## 2. §1 — CRLF patches do not apply, and the test that should have caught it

`write_lines` (`patch/unified.rs`) strips the real terminator into
`NewlineMarker` and then appends a **hardcoded `'\n'`**. So every CRLF file —
meaning the Windows platform this project ships for — produces a patch that
**both** tools the documentation names will reject.

**The interesting part is why it survived.** `crates/forskscope-core/tests/patch_apply.rs`
is a differential test against real `git apply` and `patch -p1`. It is well
designed. It has **zero CRLF cases**.

> Coverage of a format is coverage of its *variants*. A differential test that
> only ever feeds LF input proves the tool agrees with itself about LF.

Fix the terminator handling, and add CRLF and mixed-newline cases to that
differential test. Mixed-newline files must round-trip.

## 3. §2 — two path defects

- **A path containing a space defeats `patch -p1`.** Standard unified-diff
  quoting rules apply; follow what `git diff` itself emits rather than inventing
  a scheme.
- **`display_path`'s `filter_map` silently drops a non-UTF-8 path component**,
  emitting a path that is **not the one compared**. Silent is the problem: a
  patch naming a different file than it diffed is worse than a refusal. Decide
  whether to lossily represent it or refuse, and say which — but it must not be
  silent.

## 4. §3 and §5 — two smaller ones

- **Context lines ignore the user's setting.** `export_patch` passes
  `PatchOptions::default()`, while `patch-export.md` says exported context
  follows the setting. Thread the real value.
- **Exporting with no changes is a silent no-op.** The user gets no file and no
  message. Tell them.

## 5. Decided, not open — do not wire directory patch export

RFC-084 Q1 is closed: **correct `README.md`, do not wire it.**

`patch_from_directories` exists and is tested. Wiring it means a new export
surface, a destination picker and a progress story for a large tree — a feature,
not a conformance fix.

**Note this deliberately**, because it cuts against a reflex this project has
built: `patch_from_directories` is another built-tested-unwired layer (F75's
count). **The fix for an unwired layer is a decision about whether it should
exist — not reflexive wiring.** Here the answer is no, and `README.md:101` must
stop claiming it.

## 6. F92's remaining claims

Two of the worst are **already gone** — `file-types.md`'s save-guard claim
(corrected by handoff 019) and `threat-model.md`'s *"fuzzing confirmed in test
suite"*. I verified both, and confirmed no fuzz or property testing exists
anywhere, so the removal was the right resolution rather than a cover-up.

Concrete instances remaining that this handoff owns:

- **`README.md:101`** — *"export a unified-diff `.patch` file from any file or
  directory comparison; compatible with `patch -p1` and `git apply`"*. **Two
  false claims in one line.** The directory half becomes true only if §5 is
  reversed, which it is not; the compatibility half becomes true when §2 and §3
  land.
- **`patch-export.md:4-5` and `:22`** — the same compatibility claim, plus
  *"standard POSIX unified diff"*.

Sweep the rest of F92's list while you are in these files. **If a claim's defect
is not in this handoff's scope, correct the claim to what is true rather than
leaving it or fixing scope you were not given** — that is what B5's handoff 019
did, and it is the right shape.

## 7. Falsification

For each, demonstrate the test failing against the shipped defect:

1. A patch exported from a **CRLF** file applies with **both** `git apply` and
   `patch -p1` — falsify by restoring the hardcoded `'\n'`.
2. Mixed-newline files round-trip.
3. A path containing a **space** applies with both tools.
4. A **non-UTF-8** path component is never silently dropped.
5. Exported context matches the user's setting.
6. Exporting with no changes tells the user.

## 8. Scope

**In:** `core/src/patch/unified.rs`, `core/src/patch/build.rs`,
`ui/src/ui/view/diff_actions.rs`, `tests/patch_apply.rs`, `README.md`,
`intermediate/patch-export.md`.

**Out:** wiring `patch_from_directories`; encoding work (0.169.0, shipped);
`xlsx.rs`.

**Wiring is in scope even when it lands in `forskscope-ui`** — handoff 022's
scope line would have shipped dead code, and I am not repeating it. If a fix
needs a UI surface that does not exist yet, use your judgement as you did with
`ConfirmEncodingChange` and disclose it.

## 9. Gates

The usual set. `patch_apply.rs` shells out to real `git` and `patch`; if either
is unavailable in your environment, say so rather than skipping the differential
cases silently.
