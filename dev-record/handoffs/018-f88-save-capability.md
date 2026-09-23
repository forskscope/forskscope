# Developer Handoff 018 — F88: one source of truth for whether a save is possible

**From:** architect
**Date:** 2026-09-01
**Register:** F88a (High, **blocking**) and F88b (not blocking).
**Governing: RFC-082 §D3** (accepted). **This closes audit blocker B5.**

---

## 1. Task title

Replace the pair-wide `can_save` expression with a capability derived from
`EditabilityClass` and `SaveTargetState`.

## 2. Purpose

```rust
let can_save = ld.kind.is_mergeable_text() && rd.kind.is_mergeable_text();
```

That consults neither `had_decode_errors` nor the encoding label, and
`EditabilityClass` — including `requires_save_guard()` — has **zero production
call sites**. Two defects follow.

### 2a. F88a — a file that decoded with replacement characters saves silently

**And F87 does not cover this.** I verified the gap rather than assuming it:

```
input: UTF-8 BOM + an invalid byte
  decode → label "UTF-8", had_decode_errors = TRUE
  re-encode as UTF-8 → bytes DIFFER from the original
  F87's lossy flag → FALSE
```

The replacement characters are valid UTF-8, so encoding them back is not lossy
and F87's runtime guard stays silent — while the bytes on disk change. **That is
the case `requires_save_guard()` exists for, and it is reachable.**

Use that exact input as your fixture: a BOM forces the UTF-8 interpretation, so
detection cannot fall back to a lossless single-byte encoding the way it does for
a bare `0xFF`.

### 2b. F88b — you cannot restore a deleted file

`can_save` requires **both** inputs to be `Text`. A missing right side is
`FileKind::Missing`, so the entire merge and save toolbar disappears — while
`inspect_save_target` already reports that path as `Writable { MustBeAbsent }`
and `cli.md` advertises exactly this workflow.

**Not release-blocking** (owner decision): a feature gap, not a data-loss path.
Fixed here because it is the same expression.

## 3. The design decision — compose, do not change `from_kind`

`EditabilityClass::from_kind` maps `FileKind::Missing → ReadOnly`, so a naive
"use the right side's `EditabilityClass`" would leave F88b exactly as broken.

**Do not change that mapping.** `ReadOnly` is correct for *a document*: you
cannot edit a file that is not there. The question this handoff asks is
different — *can the merge result be written to the target?* — and it composes
three facts:

| Input | Question it answers |
|---|---|
| Both sides' `FileKind` | is a text merge possible at all? |
| Both sides' `EditabilityClass` | does saving need a guard? |
| The target's `SaveTargetState` | can this path be written? |

**`Missing` is empty text, not an unsupported kind.** Treat it as a side that
contributes no content and needs no guard. That is the whole of F88b.

A capability that answers *saveable / saveable-with-guard / blocked(reason)* is
the shape; name it and place it as you see fit, but it must be **one** function
with those three inputs, not a boolean assembled at the call site.

## 4. What the guard does when required

`ReadWriteWithGuard` must not silently permit the save. Route it through the
dialog F87 built — `SaveErrorView` and `Modal::SaveError` — with a distinct
`AppErrorKind`; **do not reuse `EncodeLossy`**, which means something else
(*these characters cannot be written*) and would produce a message naming
characters that are not the problem.

The honest message here is *this file was read with substitutions, so saving it
will not reproduce the original bytes* — and the escape is different too: there
is no "save as UTF-8" that helps, because UTF-8 is already the problem. Offer
`Dismiss`, and say plainly what saving would do.

**Review 083's subset test must be extended again**, deliberately, for whatever
kind you add.

## 5. Explicit non-change scope

- **`EditabilityClass::from_kind`** — §3.
- **F87's runtime guard** — it stays; the two are complementary, and §2a shows
  neither covers the other.
- **The conflict arm, `persist_noclobber`, the `.bak` clobbering** (a separate
  audit finding).
- `ROADMAP.md`, the RFCs.

## 6. Required tests

1. **A file that decoded with replacement characters cannot be saved without the
   guard.** Use §2a's fixture. **Falsify by restoring the pair-wide `can_save`**
   — the save must go through and the bytes must change.
2. **The bytes really do change**, asserted directly, so the test documents *why*
   the guard exists rather than only that it fires.
3. **A missing right side can be created by saving** (F88b), and the toolbar is
   available for it.
4. **A binary or spreadsheet side is still not saveable** — the capability must
   not have widened into permitting what `is_mergeable_text` correctly refused.
5. **Review 083's subset test passes** with the new kind in its set.

## 7. Acceptance criteria

- `EditabilityClass` and `requires_save_guard()` have real call sites.
- The pair-wide `can_save` expression is gone.
- §2a's file cannot be saved silently.
- A deleted file can be restored.
- Binary and spreadsheet sides remain unsaveable.
- No catch-all over `RecoveryAction` or `AppErrorKind`.
- Gates green, as handoff 016 §6.

## 8. Prohibited shortcuts

- **Do not change `Missing`'s `EditabilityClass`.** §3.
- **Do not reuse `EncodeLossy`.** §4.
- **Do not assemble the capability at the call site** from two booleans — that
  is the shape being removed.
- **Do not loosen review 083's subset assertion.**
- **Do not report a falsification you did not run.**

## 9. Required review-request format

Lead with test 1's falsification, and quote the byte difference from test 2.

State plainly:
- where the capability lives and what its three inputs are;
- what the guard's message says, and why it is not `EncodeLossy`'s;
- that binary and spreadsheet sides are still refused, and how you showed it.

**This is B5's last item.** When it closes, say so — the register's
release-blocking outcomes need updating and that is mine to do.
