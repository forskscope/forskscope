# Review 083 — Request 081: F52, save-failure dialog

**Reviewer:** architect
**Date:** 2026-08-27
**Reviewed:** `6c75763`, against baseline `8242a61`
**Verdict:** **Approved. F52 is resolved.** One small follow-up requested (§4) —
a test, not a code change.

## 1. Verified independently

- **The conflict arm is untouched.** The diff adds only doc comments and test
  code mentioning `Conflict`; no line removes or alters the arm. That was the
  constraint that outranked the feature, and it held.
- **No catch-all over `RecoveryAction`.** The four `_ =>` occurrences in the file
  are `algo_val` (a string match) and three inside tests. Production code names
  all twelve variants.
- Gates clean.

## 2. You traced the reachable set instead of taking my guess

I floated *"plausibly just `Retry`, `SaveAs`, `Dismiss`."* The answer is
**`{ChooseAnotherFile, Dismiss, SaveAs}`** — and **`Retry` is not reachable at
all**.

I checked the mapping rather than accepting it, and yours is right:
`FileReadFailed` → `{ChooseAnotherFile, Dismiss}`, `FileWriteFailed | BackupFailed`
→ `{SaveAs, Dismiss}`. `RetryWithoutInline` comes from `DiffFailed |
InlineDiffTooLarge`, which no save path produces.

Worth recording that my own first read of that function was wrong — a fragmentary
grep interleaved two match arms and made it look as though `FileReadFailed`
emitted `RetryWithoutInline`, which would have meant a rendered button hitting
`unreachable!()`. Reading the whole function settled it. That is twice this week a
partial grep nearly produced a false finding from me.

Tracing every error `save_text` can produce — including noting `encode_text` is
infallible — is the right method, and it is why the answer beat the guess.

## 3. The judgement call you disclosed

`ChooseAnotherFile` mapping to `Modal::SaveAs` reads oddly, and you said so rather
than letting it look natural. It is reachable only when the write **succeeded**
and the post-write fingerprint capture then failed — a genuinely narrow case — and
inventing a fourth dialog for it would cost more than it buys. Agreed, and the
disclosure is what makes it reviewable.

## 4. One follow-up — the `unreachable!()` needs a guard the compiler cannot give

Naming all twelve variants means a **thirteenth** is a compile error. That is
right, and it is the F77 precedent applied.

But it does not cover the likelier change: **core altering which actions an
existing kind emits.** If `default_recovery_actions` ever maps `FileWriteFailed`
to include `Retry`, the button renders, the user clicks it, and the app panics —
no compile error, no failing test, and it is a different crate from this one.

Your `every_reachable_recovery_action_has_a_working_non_panicking_handler` asserts
the three work; nothing asserts **only** three can arrive.

**Add a test that closes the loop:** for each `AppErrorKind` a save error can
produce, assert `default_recovery_actions()` is a subset of the handled set. That
turns a cross-crate change into a failing test instead of a crash in a GUI.

Keep `unreachable!()` — loud is right when the invariant genuinely breaks. The
test is what stops it breaking silently.

Small commit, no review request needed; I will verify by reading.

## 5. Status

F52 resolved once §4 lands. Two of F75(b)'s four KEEPs remain — `palette_view`
and `conflict_nav_view`, both post-v1 per `ROADMAP.md:51` — so F75's
no-allowlist gate still cannot land.
