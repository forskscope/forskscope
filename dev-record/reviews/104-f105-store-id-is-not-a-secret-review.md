# Review 104 — Request 101: F105, the Store ID out of secrets

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `cd61049`.
**Verdict:** **Approved. F105 closed.** Your out-of-scope finding is confirmed
against Microsoft's own prerequisites, and reading that page surfaced **two
further constraints nobody has recorded** — registered as **F106**.

## 1. The implementation

`store_id = "9P63F7NPC3MH"` at `identity.toml:34`, read at
`store-submit.ps1:47` **before** the credential guard at `:53`, guard list down
to three. `STORE_APP_ID` survives only in `README.md`'s explanation of its own
removal, which is correct.

Reading the identity **before** the credentials was not required and is the
right call: it is what makes falsification 1 exercisable without credentials at
all. Disclosed rather than slipped in.

## 2. Falsification 1 is exact

```
store_id is missing from ...\identity.toml - this is the Store submission API's
applicationId, not a secret, and must be tracked there (F105)
```

Names the file, explains what the value is, and cites the finding. Not a parse
error, and — the thing §4 actually guarded against — **not the "secret not set"
message**, which would now be actively misleading.

## 3. Falsification 2: the right call, honestly reported

You could not re-run Bug 3's scenario because the owner has since configured
real credentials, and undoing that would have meant touching their setup —
out of scope by §5.

**Reporting that is better than manufacturing a test for it.** You showed the
guard's code is byte-for-byte unchanged and that run `34226616867` exercised
that exact path on the opposite input. Stating "this is the same code, exercised
on the other branch, not a rerun of the original case" is precise about what was
and was not proven.

## 4. Your correction to review 103 is accepted

Review 103 said *"the first time any of `store-submit.ps1`'s post-authentication
code runs at all will be the owner's first real dispatch."* Your rehearsal
authenticated for real and made one post-authentication call, so that sentence
is no longer true. **Correcting a claim in an already-approved document rather
than letting it stand is exactly right**, and I have amended review 103.

## 5. Your Partner Center diagnosis is confirmed — verbatim

You inferred the app registration had not been added inside Partner Center.
Microsoft's prerequisites say precisely that:

> 2. Next, from the **Users** page in the **Account settings** section of
> Partner Center, **add the Azure AD application** … **Make sure you assign this
> application the Manager role.**

So `Unauthorized — A valid account could not be found with given authorization
token` is the documented symptom of a registration that authenticates against
Entra ID but is not a member of the Partner Center account. Your reading of a
live error you had never seen before was correct.

## 6. Two constraints that page contains and nobody has recorded — F106

Reading it to check your diagnosis turned up two things that change how this
automation must be operated:

**(a) The API and the browser become mutually exclusive per submission.**

> If you use this API to create a submission … be sure to make further changes
> to the submission **only by using the API, rather than in Partner Center**. If
> you use Partner Center to change a submission that you originally created by
> using the API, **you will no longer be able to change or commit that
> submission by using the API** … the submission could be left in an error state
> … you must delete the submission and create a new submission.

**The owner has published by hand for months.** Once automation creates a
submission, opening it in Partner Center to adjust something — the natural
habit — can strand it. That is an operational trap, not a code defect, and it
belongs in `release.md` before the first real submission, not after someone
learns it.

**(b) Pricing Version 2 makes the API unusable for pricing.**

> You can't use this API with apps or add-ons that are on Pricing Version 2. A
> product is on Pricing Version 2 if there's a **Review price per market**
> button in the **Pricing** section.

Worth one look. RFC-079 keeps pricing manual anyway, so this likely does not
block us — but "likely" is not "checked", and it is a ten-second check the owner
can do while in Partner Center adding the application.

## 7. Scope

Exactly §5's list, verified by `grep` rather than memory as instructed —
including confirming no tracked file outside `ROADMAP.md` still mentions
`STORE_APP_ID`. Scratch branch and tag deleted; `main` clean before and after
both dispatches.

**F105 closed.**
