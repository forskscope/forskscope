# Review 102 — Request 099: RFC-081 AUR publication automation

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `8e54e44`.
**Verdict:** **Approved.** **RFC-081 stays in `accepted/`, deliberately** — see
§6. The implementation is complete; the RFC is not done until a push has
actually happened.

## 1. Your scope reading was right and my handoff was imprecise

You implemented Q3's **full two-trigger table** — release *and* recipe-fix —
where my handoff's prose only restated the release trigger, and you disclosed
that as a judgment call.

**You were correct.** Q3 was **CHOSEN by the owner on 2026-08-22** with an
explicit two-row table, and my handoff's own opening said *"the design is as
accepted; only the schedule moved."* You implemented the accepted design; I
narrated a subset of it while explaining the schedule change.

And Q3 contains the exact test your design has to pass:

> **if the dispatch path ever skips a check the release path runs, it has become
> the thing it was chosen instead of.**

It does not skip any. `validate` runs unconditionally with no `environment:`;
`dry_run` appears in exactly one place — the `publish` job's `if:` at line 161.
The paths share all validation and diverge only at the push.

## 2. Falsification 1 was real, not synthetic

Dispatching against `main`'s actual state produced:

```
::error::a recipe fix must not change pkgver (checked out: 0.170.2, AUR: 0.170.1)
        - cut a release instead
```

That is the guard firing on a genuine condition rather than a contrived one. I
verified the premise independently: `main`'s `PKGBUILD` really does read
`pkgver=0.170.2` while the newest tag is `0.170.1`.

## 3. Verified independently

- **The pinned AUR host key is correct.** I ran `ssh-keyscan -t ed25519
  aur.archlinux.org` and it matches the workflow's literal byte for byte.
  Pinning rather than `ssh-keyscan`-at-runtime is right, and matches this
  project's convention for the pinned-and-checksummed `actionlint` download.
- **Credential scoping is real.** `validate` (line 41) has no `environment:`;
  `publish` (158) has `environment: aur-publish` (163); `AUR_SSH_KEY` is read
  only at 173, inside `publish`. A dry run never requests the secret.
- **The two-job split has a real reason, not a stylistic one.** Job-level
  `environment:` resolves before any step's `if` narrows things, so a single-job
  version would request the credential even on a rehearsal. That is the kind of
  detail that only shows up if you actually thought about where the gate sits.
- **Checkout pins the tag** (`ref: github.event.release.tag_name`), not `main`.
  Your reasoning is exactly F103's: the post-release bump lands minutes after the
  tag and well before the draft is published, so `main` would fail the guard
  *every time*, not occasionally.
- **Five distinct refusals** plus the SKIP guard, each with its own message.

## 4. Two decisions I would not have thought to require

**Sourcing the PKGBUILD's own arrays** to pre-install dependencies, rather than
duplicating the list in YAML. The problem it solves is real — `makepkg` refuses
to run as root, and a fresh `builder` user has no sudo — and the solution means
the step *cannot drift* from the file it describes. A hand-copied list would
have gone stale the first time someone edited `depends=()`.

**`namcap` unavailable locally, so you moved the check to GitHub's real
`archlinux:base-devel` container** rather than skipping it or asserting it would
work. You then said plainly which part that does and does not prove.

## 5. The no-op branch you could not exercise, and why saying so is right

You proved determinism and idempotence up to the push — two dispatches, same
commit, same computed hash, same outcome — and then established that
`push_needed=false` is **structurally unreachable in rehearsal**: Q3 requires a
recipe fix's `pkgrel` to exceed the AUR's, which guarantees the prepared file
differs on every passing validation. Observing `false` needs content already
pushed, which needs the credential.

That is a limit, and you reached it by reasoning rather than by giving up on it.
The branch exists to prevent a spurious empty commit on a legitimate retry — a
real case, correctly guarded, honestly unverified.

## 6. Why RFC-081 stays in `accepted/`

Previous RFCs moved to `done/` when their code landed on `main`, because the
code paths had been exercised. **This one has a line that has never executed:**
`git push` to the AUR.

RFC-081's acceptance criterion *"the AUR repository receives only `PKGBUILD` and
`.SRCINFO`"* cannot be observed until a real publication happens. Moving it to
`done/` now would mean recording as shipped a mechanism whose final step is
unproven — which is this register's most-repeated defect wearing lifecycle
clothing.

**It moves to `done/` after the first successful automated publication**, and
that is the honest trigger.

## 7. What the owner must do

Your checklist is exactly what I asked for — a checklist, not a puzzle:

1. A GitHub **Environment** named `aur-publish`, with whatever protection rules
   the owner wants (a required reviewer gates every real push).
2. A secret **`AUR_SSH_KEY`** in that environment holding a dedicated private
   key, whose public half is registered on the AUR account owning `forskscope`.

And you stated the disclosure I required, in the right terms: **the dedicated
key is not a security control** — the AUR scopes per account, so blast radius is
identical to the personal key. What constrains risk is the Environment's rules.
Revocability is the real benefit, and it is worth having for that alone.

## 8. Scope

Held exactly. `release.yml` untouched — confirmed by reading it, which is the
acceptance criterion. `ROADMAP.md` and the RFC left for me. The scratch rehearsal
branch was deleted from both local and `origin`, and `main` was never touched by
either dispatch.
