# Review 091 — Request 088: documentation refresh after B5

**Reviewer:** architect
**Date:** 2026-09-01
**Reviewed:** `55789b4`, against baseline `89a8444`
**Verdict:** **Approved. B5 is closed.**

## 1. Verified

Two files, no code. The false limitation is gone from `docs/src/` entirely —
`grep` for "numeric character reference" and `&#128512` returns nothing. `mdbook
build docs` clean.

**I checked one claim that could have been imprecise.** Your §2 says saving a
decode-substituted file "is refused **entirely**". That is only true if the guard
also blocks *Save As*, and it does: `build_request` checks
`save_capability.requires_guard()` at line 14, **before** the `match target` at
line 24 that branches on an explicit destination. So both paths are refused, and
"entirely" is accurate rather than approximate.

The classification table's `Missing` row now reads `✓ (creates the file)`, and
`cli.md` says the file can be restored — both matching what `SaveCapability`
actually does.

## 2. You confirmed the claims instead of asserting them

The handoff asked how you knew each new statement was true, because "a
documentation change verified only by having been written is how the original
defect happened."

You gave a code path or a test for every clause — including the one most likely
to be assumed: that the file's tracked encoding *stays* UTF-8 after Save as
UTF-8, traced through `retry_save_as_utf8` into `handle_result`'s success arm and
pinned by a named test. That clause is the difference between "it saved once" and
"the problem is solved", and it is the one a writer would naturally take on trust.

## 3. The wording on the unrecoverable case is right

> **There is no in-app recovery for this file** — to save its original bytes, use
> a different tool.

Telling a user to use a different tool is not a comfortable sentence to write,
and it is the correct one. The bytes were lost at *decode* time, before any edit;
no save-time choice reaches them. Softening that would have left someone hunting
for a setting that cannot exist.

Equally right: you separated it explicitly from the encode case — *"a separate,
unrelated case"* — so a reader does not conclude that Save as UTF-8 might help
here too. Those two refusals look identical from the outside and have opposite
remedies.

## 4. Scope

The README's directory-CLI and directory-patch claims, `patch-export.md`'s
compatibility claim and the doc-vs-doc contradictions were left alone. They are
real, they are still open under F92, and they belong to RFC-083 and RFC-084.
Correct.

## 5. B5 is closed

All five blocking defects and all three documentation corrections are done:
F85, F86, F87, F88a, F89, and F92's control claims plus this refresh. F88b
landed alongside without blocking.

That closes audit blocker **B5**, opened 2026-09-01 from the independent audit.
**v1's No-Go now rests on B4 alone** — F44 upstream, F60 with the owner — which
is where the register said it was before the audit, and now says it honestly.
