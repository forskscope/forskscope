# Review Request 101 — F105: the Store ID is public identity, not a secret

Handoff: `dev-record/handoffs/031-f105-store-id-is-not-a-secret.md`
Commit: `cd61049` (pushed to `main`). CI run `34226016124` confirmed green.

## Falsifications, run for real

### 1 — `store_id` missing, caught cleanly and named

Handoff §4 requires this to fail "with a message naming the file — not a
generic parse error, and not the 'secret not set' message." A scratch
commit removed `store_id` from `identity.toml` (real credentials
untouched). Run `34227362802`: build succeeded, submit failed in 13s —
before ever touching a credential, since the identity read now runs first:

```
store_id is missing from D:\a\forskscope\forskscope\packaging\windows\store-listing\en-us\identity.toml - this is the Store submission API's applicationId, not a secret, and must be tracked there (F105)
```

### 2 — Bug 3's guard, unchanged but no longer independently re-provable here

Handoff §4 also asks to prove the credential guard "still refuses cleanly
when a real credential is unset... rather than assuming it does." I could
not re-run that exact scenario: **this repository's `store-publish`
environment now holds real values for `STORE_TENANT_ID`, `STORE_CLIENT_ID`,
and `STORE_CLIENT_SECRET`** (see finding below) — a change made outside
this handoff, and one I have no way to temporarily undo for a test without
touching the owner's real credential setup, which is out of scope (§5:
"Out: the other three secrets"). What I can state instead: the guard's code
is byte-for-byte unchanged from review 103's fix
(`store-submit.ps1`'s `[string]::IsNullOrEmpty((Get-Item "env:$name"
...).Value)` check) — only its position moved, to run *after* the new
`store_id` block instead of before it. Confirmed by diff, not by inspection
alone: run `34226616867` (§3 below) exercised this exact code path and it
executed correctly — the real secrets it found were non-empty, so it passed
through without firing, which is the guard behaving correctly on the
opposite input from Bug 3's case, not a rerun of Bug 3's case itself.

## An unplanned, out-of-scope finding: real credentials exist and authentication works

Run `34226616867` — dispatched to prove the normal (`store_id` present)
path — did not stop at the credential guard the way I expected. The log
showed real, non-empty (masked) values for all three secrets, and then:

```
Authenticated to the Microsoft Store submission API.
```

**The Entra ID OAuth flow succeeded for real** — the owner has configured
the `store-publish` environment's three credential secrets since review
103, ahead of this handoff. The very next call, reading the application
resource with `store_id` ("9P63F7NPC3MH") as the `applicationId` path
parameter, got a real, structured response from Partner Center itself:

```
{ "code": "Unauthorized", "message": "A valid account could not be found with given authorization token.
CorrelationId=e0181563-e860-4275-8721-0f486298aecd" }
```

This is a real Partner Center rejection, not a malformed-request error or a
generic 404 — the request reached the service and was well-formed enough to
get a specific, coded answer. The likely cause is that the Entra ID app
registration has not yet been added as a user with API access inside the
Partner Center account itself (a separate, one-time Partner Center-side
step, distinct from the Entra ID app registration existing at all). **This
is not an F105 defect and I have not attempted to fix it** — it is squarely
"anything about the submission flow itself," which §5 puts out of scope,
and it requires Partner Center account access I do not have. Flagging it
because it is real, live evidence and the owner should know before assuming
the next real dispatch will get further than this.

## Design decision: `store_id` is checked before the credential guard

`identity.toml` needs no credential to read, so the read now runs first,
ahead of `STORE_TENANT_ID`/`STORE_CLIENT_ID`/`STORE_CLIENT_SECRET`. Not
required by the handoff, but it is what makes falsification 1 possible to
exercise without real credentials, and there is no ordering dependency
between the two checks that this reorder violates. Disclosed as a judgment
call rather than assumed obvious.

## Where it went: exactly where the handoff said

`store_id = "9P63F7NPC3MH"` in `store-listing/en-us/identity.toml`,
alongside `identity_name`/`publisher`/`publisher_display_name`.
`store-submit.ps1` reads it with the same line-based parse
`store-validate.ps1` already used for this file — no new parsing approach
introduced. `STORE_APP_ID` is gone from `store-submit.yml`'s env block and
from the credential-presence guard's list.

## For the owner: `STORE_APP_ID` is now inert

Per handoff §5: if `STORE_APP_ID` was set as an environment secret while
this was still required, **it can be deleted** — nothing reads it anymore.
Leaving it in place is harmless (it was always public) but implies it is
still consumed, which it is not.

## Scope

In: `identity.toml`, `store-submit.ps1`, `store-submit.yml`, and the three
documentation locations handoff §3 named — checked with `grep`, not
memory, per its explicit instruction: `packaging/windows/README.md` (the
secrets list, now three), `docs/src/maintainers/threat-model.md` §6 (the
credential-list sentence), and confirmed no other tracked file mentioned
`STORE_APP_ID` outside `ROADMAP.md` (F104/F105's own register entries,
untouched, the architect's). Out, untouched: the three real secrets
themselves; the submission flow's own logic beyond where `$AppId` comes
from.

## Gates

`cargo fmt --check`, `cargo xtask version-sync/i18n/rfc-sync/css --check`,
`cargo xtask audit-deps`, `git diff --check`, `mdbook build docs` — all
clean (no Rust touched, so no test/clippy delta). `actionlint` clean over
`store-submit.yml`. CI run `34226016124` for `cd61049` confirmed green.
Two live dispatches against `windows-latest` (`34226616867`, `34227362802`)
exercised the real build/validate/submit pipeline and produced both
falsifications above; the scratch branch (`f105-rehearsal`) and its tag
(`0.0.3`, moved once between the two scenarios rather than duplicated) are
deleted, locally and on `origin`. `main` confirmed clean before and after
both dispatches.

## A correction to review 100 while writing this one

Review 100 said "the first time any of `store-submit.ps1`'s
post-authentication code runs at all will be the owner's first real
dispatch." That was accurate when written; it no longer is. This handoff's
own rehearsal (`34226616867`) authenticated for real and made one real
post-authentication call. Noting the correction here rather than letting a
stale claim stand uncorrected in an approved document.
