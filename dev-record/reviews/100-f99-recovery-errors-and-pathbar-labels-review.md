# Review 100 — Request 097: F99's two defects

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `61e30dd`.
**Verdict:** **Approved. F99's C10 and C3 closed.** Your third finding —
Home and the folder picker have **no keyboard route at all** — is registered as
**F100**. C1/C6/C7 remain recorded and unscheduled.

## 1. Two corrections to my handoff, both yours

**§1 told you to use `AppError::from_core`. It would not have compiled.**
`PersistenceCommitError` is a distinct enum (`Conflict`, `Io(String)`) —
deliberately not `CoreError`, and its own doc comment says why: *"a stale-caller
conflict is not an I/O failure."* `from_core(err: &CoreError)` cannot take it.

`AppError::new` is the right constructor, and its doc comment describes exactly
this case. Your mapping — `Conflict` → `SaveConflict` (the file changed on disk
since it was read), `Io` → `FileWriteFailed` — is the closest honest fit.

**§1 also named two stale comments. Only one was stale.** I verified:
`file.rs:268` reads *"the same precedent `handle_result`'s **old** … arm
**set**"* — past tense, explicitly "old", accurate history. **I would have had
you edit a correct comment**, which is the same error as the thing being fixed.

You checked before editing rather than doing what the handoff said. That is the
right instinct and it is the second time this week it has caught me.

Your rewrite of the one that *was* stale is better than deletion: it preserves
the English-only convention that arm established while naming the migration, so
the reader learns why the convention exists rather than losing the reason with
the sentence.

## 2. Falsification, reproduced

I reverted the **settings** handler only:

```
settings_reset_conflict_notifies_the_mapped_message_not_the_raw_one ... FAILED
session_reset_conflict_notifies_the_mapped_message_not_the_raw_one .. ok
```

**One failure, not two** — so these are two independent guards, one per call
site, rather than a single test standing in for both. That distinction matters
here precisely because the defect was two copies of the same line.

And the tests drive a **genuine** `PersistenceCommitError::Conflict` — the
`ConfigRootOverrideGuard` points `config_file_path` at a temp directory, the
test writes bytes on disk that differ from the resolution's claimed
`raw_bytes`, and the real `verify_unchanged` produces the error. Not a
hand-built error value. That is the difference between testing the mapping and
testing the path.

Full suite **1257 passed, 0 failed**; `i18n` unchanged at **244 keys**, exactly
as predicted — the `aria_label`s reuse each button's existing `title`
translation.

## 3. The C3 test, and its limit stated rather than implied

No `dioxus-ssr` in the workspace, so you rendered `PathBar` through a bare
`VirtualDom`, called `rebuild_to_vec()`, and scanned `Mutations` for
`SetAttribute { name: "aria-label", .. }` — using `dioxus_core`, already a
direct dependency, rather than adding one.

That proves the attribute is present with the expected translated string. You
then said plainly what it does **not** prove: that the label reaches a real
screen reader's accessibility tree, which stays an AT-SPI/UIA assertion — F74's
recorded limit, disclosed the same way review 093 disclosed the untestable
`<select>`.

Declaring the ceiling of your own evidence is worth more than the evidence.

## 4. `detail` dropped, with a reason

You surface `message.short` and not `.detail`, because `store.notify` only ever
shows a toast at that call site and there is no dialog surface to put `detail`
in without inventing one. Correct — and the right call was to say so rather than
either dropping it silently or building a surface nobody asked for.

## 5. What you found and did not fix — now F100

Grepping `keyboard.rs`, `app.rs` and the Explorer tree, you established that
**Home and the folder picker have no keyboard route at all** — not merely
undocumented in the help modal, genuinely unreachable without a mouse.

That is larger than an attribute fix and you reported it instead of building it,
as §2 asked. It also sharpens RFC-061's completeness claim, which handoff 026's
review recorded as satisfied for the *panes*. **Registered as F100.**

## 6. Scope

Exactly §4's list, minus the comment that did not need changing. C1's first-run
persistence, C6's settings tier and C7's narrow-layout marker untouched — you
did not build them because you were in the neighbourhood, which is what I asked
and is harder than it sounds.

**F99's C10 and C3 closed.**
