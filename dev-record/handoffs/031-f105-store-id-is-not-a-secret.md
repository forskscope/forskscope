# Handoff 031 — F105: the Store ID is public, and should not be a secret

**From:** architect. **Register:** F105. **Small.** **Owner's finding.**

## 1. What is wrong

`store-submit.yml` requires a fourth secret, `STORE_APP_ID`, holding
`9P63F7NPC3MH`.

**That value is public.** `README.md:39` already links to
`https://apps.microsoft.com/detail/9p63f7npc3mh`, it is in the Store URL any
user sees, and it grants nothing on its own. The owner asked why it needs
defining manually as a secret, and the answer is that it does not.

Three costs, none large, all avoidable:

- **A setup step that buys nothing.** Three secrets carry real credentials; the
  fourth carries a value already in the repository.
- **A confusing failure mode.** An unset `STORE_APP_ID` produces the same
  "not set" refusal as a missing credential — you already had to fix that guard
  once (Bug 3, review 103), and this makes it fire for a non-secret.
- **It implies the value is sensitive**, which invites someone to treat a
  public identifier as one.

## 2. Where it belongs

**`packaging/windows/store-listing/en-us/identity.toml`.**

That file's own doc comment already describes exactly this kind of value:

> These … values were registered with Partner Center when the app was first
> reserved there … and Partner Center ties the app's identity to them
> permanently.

The Store ID is assigned at reservation, never changes, and is Store identity.
It belongs beside `identity_name`, `publisher` and `publisher_display_name`, not
in a credential store. `store-validate.ps1` already reads this file.

Add `store_id = "9P63F7NPC3MH"`, have `store-submit.ps1` read it from there, and
drop `STORE_APP_ID` from the workflow's env block and from the guard's
required-secret list.

**If you think a GitHub Actions *variable* is the better home than a tracked
file, say so rather than assuming my answer** — that is a real alternative. My
reasoning for the file: it is reviewable in a diff, it sits with the three values
it is conceptually part of, and it needs no per-environment setup at all. But
you have been closer to this workflow than I have.

## 3. Documentation to follow it

- `packaging/windows/README.md` — the secrets list becomes **three**.
- `docs/src/maintainers/threat-model.md` §6 — it currently names
  `STORE_APP_ID` among what lives in the `store-publish` environment. It will
  not.
- Anywhere else the four-secret count appears.

**Check rather than trust that list** — I have now been wrong twice in two days
about where `STORE_APP_ID` is described, and `grep` is cheaper than my memory.

## 4. Falsification

1. Removing `store_id` from `identity.toml` must fail with a message naming the
   file — not a generic parse error, and not the "secret not set" message, which
   would be actively misleading now.
2. The guard must still refuse cleanly when a **real** credential is unset —
   prove Bug 3's fix survives this change rather than assuming it does.

## 5. Scope

**In:** `identity.toml`, `store-submit.ps1`, `store-submit.yml`, the three
documentation locations.

**Out:** the other three secrets; anything about the submission flow itself.

**Note the owner may already have set `STORE_APP_ID`** while unblocked. Once
this lands it becomes inert rather than harmful — say in the review request that
it can be deleted, so it does not sit there implying it is still read.

## 6. Gates

The usual set plus `actionlint`. No Rust is touched.
