# RFC 081: AUR Publication Automation

**Status.** Proposed
**Scheduling.** Deferred — accepted; the automation waits, the package recovery rides each release by hand. See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in this folder and nothing else (F83).
**Accepted.** 2026-08-22 by the project owner — Gate A cleared.
**Disposition, 2026-08-22:** the owner accepted separating the two halves this
RFC ran together. **Recovering the package and automating the publish are not
the same task.** The automation is **deferred** — nothing about it is urgent, and
the two remaining questions (the source hash, and the credential's home) can be
answered whenever. **The recovery is done (2026-08-27):** published by hand at
`0.167.2`, AUR commit `d6ca5f6` — verified against the AUR repository itself:
real source hash, `.SRCINFO` regenerated, `xdotool` present. The package is
current for the first time since `0.22.13`, 145 versions back. **What remains
for this RFC is only the automation.** The owner
works on an Arch-family system, so building the package locally before pushing
*is* §3's validation — the ordinary practice this RFC exists to preserve once
automation removes it. Q1 closed: the
owner confirms maintainer rights are intact and the AUR account configuration is
unchanged since `0.22.13`. Stays in `proposed/` until implemented.
**Tracks.** Release pipeline; Linux distribution; credential handling.
**Touches.** `packaging/linux/PKGBUILD`, a new publish workflow, `release.md`,
`installation.md`, the threat model.
**Depends on.** **F81 — blocking** (the shipped `PKGBUILD` is not currently
publishable). Owner confirmation of AUR maintainer rights (§8 Q1).
**Origin.** Owner request, 2026-08-22: the AUR package
[`forskscope`](https://aur.archlinux.org/packages/forskscope) stopped at
**0.22.13** and should be recovered and kept current automatically.

## Summary

The repository has carried a maintained `PKGBUILD` all along —
`packaging/linux/PKGBUILD`, at `pkgver=0.167.2`, kept in step with the workspace
version by `cargo xtask version-sync`. What has never existed is the step that
takes it to the AUR: publishing is a `git push` to `aur.archlinux.org`, and
nobody has done it since **0.22.13**.

So this is not a packaging project. The package is written. **This RFC automates
one push and, far more importantly, establishes that what gets pushed is
correct** — because of an asymmetry with every other channel this project ships.

## The asymmetry that shapes the whole design

**Every other artifact is built here and tested here. An AUR package is built on
the user's machine, by the user, from a recipe we publish.**

A broken GitHub release artifact is a bad download. A broken `PKGBUILD` is a
build failure, or worse a successful build that will not run, reproduced
identically on every machine that installs it — and the first the project hears
of it is an out-of-date flag or a comment on a web page nobody is watching.

Two consequences run through this document:

- **Validation must build the package, not inspect it.** Anything less is
  checking that a recipe looks plausible.
- **The failure surface is the user's terminal**, so the bar is higher than for
  an artifact we can re-cut.

## Goals

- Publish the current `PKGBUILD` to the AUR on release, automatically.
- Prove, before publishing, that it builds and installs on a clean Arch system.
- Keep the owner's approval gate: a human decides what ships.
- Close F81 first, so the thing being automated is worth automating.

## Non-goals

- **A `forskscope-bin` package.** A second package is a second maintenance
  surface, and §"Why this matters more than it looks" argues the source package
  is the one that works today. Revisit separately.
- **Other distributions.** No `.deb`, `.rpm`, Flatpak, AppImage, Homebrew.
- **Replaying the 145 missed versions.** The AUR carries one version, the
  current one. History is not owed.
- **Responding to AUR comments or out-of-date flags.** Human work, and it should
  stay that way.

## Why this matters more than it looks

**The AUR path is the only Linux channel that works on Arch-family today.**

F44 records that the published `linux-x86_64` tarball does not start on any
distribution shipping `libxdo.so.4` — Arch and CachyOS confirmed — because it was
built on `ubuntu-latest` against soname 3. That blocks Gate D and waits on an
upstream `dioxus-desktop` release nobody here controls.

**A source build is immune to it.** `makepkg` compiles on the user's machine and
links whatever `libxdo` that machine provides. The soname mismatch cannot occur.

So while F44 holds the binary channel closed for Arch users, this channel could
be open — **once F81's second defect is fixed**, because `depends` currently omits
`xdotool` and the package therefore installs cleanly and then fails to start with
the very error F44 describes.

That inverts the usual sequencing argument. This is not a nice-to-have deferred
behind Gate D; for one platform family it is the *working* distribution path, and
it is three lines from being trustworthy.

## Design

### 1. The source hash — this RFC's work, not a precondition

**Corrected 2026-08-22.** An earlier draft listed `sha256sums=('SKIP')` as an F81
defect this RFC waits on. Checking *why* it had never been refreshed showed the
instruction to refresh it is unperformable for almost all of the repository's
life, and the fix belongs here.

`cargo xtask version-sync` requires `pkgver` to equal the workspace version, and
the workspace bumps immediately **after** each release. So `pkgver` names an
**unreleased** version nearly always — today it is `0.167.2`, and the newest tag
is `0.167.1`. There is no tarball to hash. The hash is computable only between
tagging and the post-release bump, and committing it in that window puts a value
in the tree that is right for about one commit and wrong afterwards.

**So the hash is computed at publish time and never committed.** The workflow
runs after `release: published`, when the tag certainly exists, fetches that
tag's archive, computes the hash, and writes it into the `PKGBUILD` it pushes.
The in-tree file stays a template.

Two consequences worth stating:

- **`sha256sums=('SKIP')` must never reach the AUR.** §3 checks that explicitly,
  and it is the check most likely to be skipped as obvious.
- **The in-tree file is no longer something a user should copy.** §6's
  `installation.md` change is what makes that true rather than merely intended.

**What F81 does still contribute:** its second defect, `depends` omitting
`xdotool`, was fixed on 2026-08-22 — the binary links `libxdo` (F44) and the
package installed cleanly and then failed to start without it. That one was a
real precondition and it is closed.

### 2. What publishing actually is

The AUR is a git remote. Publishing means:

1. Regenerate `.SRCINFO` — `makepkg --printsrcinfo > .SRCINFO`. **The AUR rejects
   a push whose `.SRCINFO` disagrees with its `PKGBUILD`**, and it is generated,
   never hand-edited.
2. Commit both to a repository whose *only* contents are `PKGBUILD` and
   `.SRCINFO` — the AUR repo is not this repository, and pushing more is a
   common way to get it wrong.
3. `git push` over SSH to `ssh://aur@aur.archlinux.org/forskscope.git`.

Note `.SRCINFO` does not exist in this repository today and should not be
committed here: it is a build product of the file beside it, and two copies drift.

### 3. Validation — build it, do not inspect it

In an Arch container, before any push:

- `makepkg --syncdeps --noconfirm` — it **builds**;
- the built package **installs** (`pacman -U`);
- `namcap` on both `PKGBUILD` and the built package. **Namcap reports missing
  library dependencies by design and would have caught F81's `xdotool` omission
  without anyone thinking of it** — which is the argument for having it, not a
  footnote;
- `.SRCINFO` regenerates identically to what is about to be pushed;
- the source hash is **not** `SKIP`.

A failure here fails the workflow, and nothing reaches the AUR.

**Why this is not gold-plating, measured against how AUR packages are normally
maintained.** Building the package in CI and running `namcap` are both uncommon;
most AUR packages are published by hand with no CI at all. That comparison looks
unfavourable until you notice what the manual process contains: **a maintainer
publishing by hand builds the package locally first** — not as a discipline, but
because that is how they check their own work. That build *is* the validation. It
is implicit, unwritten, and it catches exactly this defect class: a missing
`depends` surfaces the moment you install what you built on a machine that lacks
it.

**Automating the push deletes that step.** Nobody would build the Arch package at
any point in our process — not a maintainer, not CI, nobody. This is not a
hypothesis: `depends` omitted `xdotool` through **three releases**, on a path
`installation.md` tells Arch users to follow, and it was found by reading the file
rather than by anything running. The ordinary manual process would have caught it
on the first build.

So §3 does not add safety above normal practice. It **restores** safety that
automation removes. Automating the push without it would leave this project worse
than the hand-maintained baseline — faster at shipping something nobody
verified.

**What this still cannot prove:** that the built binary *runs*. A container has
no display server. Launch remains RFC-078's business, and it has no row for this
path — see §6.

### 4. Trigger and credentials

**Trigger on `release: published`**, never on tag push — the same reasoning
RFC-079 §"The approval gate must survive" sets out. The owner's publish decision
is the gate; the AUR inherits it rather than pre-empting it.

**Credential: an SSH private key** in GitHub Actions Secrets, whose public half is
registered on the AUR account. It is the only mechanism the AUR offers.

Its properties, stated rather than assumed:

- **It does not expire**, unlike RFC-079's Entra ID secret. That removes a
  failure mode and adds a different one: nothing will ever force a rotation, so
  rotation must be a decision rather than an event.
- **Compromise means someone can publish a package to Arch users under the
  project's identity.** That is the same class as RFC-079's, and the threat model
  should carry both together rather than treating them as unrelated.
- It should be a **dedicated key for this purpose**, not a personal key that
  happens to have AUR access.

### 5. Failure and recovery

- **Pre-push failure** (build, namcap, `.SRCINFO`): workflow fails, the AUR is
  untouched, the GitHub release is unaffected.
- **Post-push failure** (it built here and not on a user's machine): recovery is
  a new `pkgrel` or a new version. **The AUR has no "unpublish"**, so the
  validation in §3 is the only real protection.
- **A failed AUR publish must never be resolved by editing the published GitHub
  release.** Same rule as RFC-079 §5.

The workflow must be safe to re-run: pushing the same content twice is a no-op,
and it must not create an empty commit or a spurious `pkgrel` bump.

### 6. What the docs must say

`release.md` gains the AUR step in its real position — after publication — and
`installation.md`'s Arch section changes from *"copy this PKGBUILD and run
makepkg"* to installing from the AUR, which is what an Arch user expects.

**`installation.md` should also stop implying the PKGBUILD path is tested.** It
is not, and F81 exists because nobody noticed for three releases.

## Acceptance criteria

- F81 is closed before this ships.
- The workflow triggers on `release: published` and never on tag push.
- A deliberately broken `PKGBUILD` fails validation **before** any push —
  demonstrated, per this project's falsifiability standard.
- `namcap` runs on both the recipe and the built package, and a deliberately
  removed dependency is caught by it.
- `.SRCINFO` is generated, never committed to this repository.
- Re-running against the same release pushes nothing and fails nothing.
- The AUR repository receives only `PKGBUILD` and `.SRCINFO`.
- `release.md` and `installation.md` reflect the new path.
- `release.yml` is unchanged.

## Testing

The falsifiability standard applies: each check demonstrated **failing** on a
deliberately broken input. A publish workflow that cannot be shown to reject a
broken package has not been shown to check anything.

Unlike RFC-079's Store submission, **this one is fully rehearsable**: the AUR is
a git remote, so pushing to a scratch remote exercises the entire path — key,
`.SRCINFO`, commit shape, push — without touching the real package. There is no
excuse for an unrehearsed first run, and the rehearsal should be part of the
work rather than a promise.

## Sequencing

**F81 first, then this, and neither waits for Gate D.**

RFC-079 defers because F60 blocks it. This has no equivalent blocker, and the
reasoning that deferred other work — avoiding a matrix re-run — expired when
`main` moved seven code commits past the evidenced candidate.

More to the point: **while F44 holds, this is the only working Linux channel for
Arch-family users**, and deferring it behind a gate that F44 itself blocks would
be waiting on the very thing that makes this path valuable.

## Open questions for the owner

- **Q1 — maintainer rights. CLOSED 2026-08-22** — the owner confirms the account
  that published `0.22.13` still holds them, with its configuration unchanged.
  The package was not orphaned or adopted.
### Q2 — the `0.22.13 → 0.167.2` jump

**The only real risk was version ordering, and it is checked and absent.** Arch
compares version segments numerically, not lexically, so `167` sorts above `22`
despite being shorter as a string. Verified with the real tool:

```
vercmp 0.167.2 0.22.13  ->  1     (first is newer)
vercmp 0.22.13 0.167.2  -> -1     (first is older)
```

So existing installs see a normal upgrade and **no `epoch=` is needed**. An
`epoch` is the escape hatch for versions that sort wrongly, it is permanent once
added, and adding one unnecessarily would be a scar on the package forever.

**Option A — publish straight to current. CHOSEN 2026-08-22 by the owner.**
*Pros:* one push; the AUR only ever carries one version, so intermediate ones
would be visible to nobody; nothing to maintain afterwards.
*Cons:* the AUR git history shows a 145-version gap. That is an accurate record
of what happened.

**Option B — publish one or more intermediate versions first.**
*Pros:* a tidier-looking history.
*Cons:* every intermediate would have to build, which means resurrecting old
`PKGBUILD`s against today's toolchain — real work with a real failure mode, in
service of a history no user reads. **Nobody can install an intermediate**: the
AUR serves HEAD only.

**Option C — delete and re-request the package.**
*Pros:* none that apply. This exists for renames and for packages that should not
exist.
*Cons:* discards the package's history and its existing users' upgrade path, and
re-requesting invites a review that Option A does not need.

### Q3 — `pkgrel` policy, and the gap it exposes

**Why there are two version fields at all**, since the question came up and will
come up again.

`pkgver-pkgrel` separates **two different authorities**, not two halves of one
number:

- **`pkgver` belongs to upstream.** It is not the packager's to invent. Other
  packages express constraints against it (`forskscope>=0.167.2`), users match it
  against release notes, and upstream will eventually publish the next value
  itself.
- **`pkgrel` belongs to the packager.** It is a namespace the packager owns,
  which cannot collide with anything upstream will later choose.

Aggregating them — publishing a recipe fix as `0.167.2.1` or `0.167.3` — means
**writing into upstream's namespace**: claiming a release that does not exist,
and colliding with the real one when it arrives. Every major distribution makes
the same split for the same reason (Debian's `1.2.3-4`, RPM's `Version` /
`Release`).

**What makes it feel redundant here is that this project is both parties.** The
usual packager is a third party who *cannot* change `pkgver`, so the split is
self-evident. We can change it, which makes one number look sufficient — and it
is not, because the two still change at different times for different reasons
even when one person holds both roles.

**The aggregating option, named so it is rejected on cost rather than
overlooked:** we *could* cut a real upstream release for every packaging fix, and
never use `pkgrel` at all. In most projects that is merely wasteful. Here it is
expensive: under RFC-078 a version is an evidence obligation — new artifacts, new
digests, matrix rows re-run — so a one-word `depends` change would drag a full
platform-acceptance cycle behind it. The cheap field exists precisely to avoid
that, and this project has more reason to use it than most.


**`pkgrel` counts packaging revisions within one upstream version.** It resets to
`1` when `pkgver` changes and increments when the *recipe* changes but upstream
does not.

**Two facts make this less academic than it sounds.**

**(1) Nothing checks `pkgrel` today.** `cargo xtask version-sync` covers `pkgver`
(`xtask/src/main.rs:290`) and nothing covers `pkgrel`; it is a bare literal in the
file. So it can drift: leave it at `2` after a recipe fix, bump `pkgver`, and the
AUR gets `0.168.0-2` when no `0.168.0-1` ever existed. Harmless to users,
incorrect as a record, and invisible.

**(2) A recipe-only fix has no route to users under §4's trigger.** Today's
`xdotool` fix (`b8163c0`) is exactly that case: it fixes an install that
completes and then fails to start, it changes no source code, and under
"publish on `release: published`" it cannot reach an Arch user **until the next
release happens for unrelated reasons.** That is the gap this question is really
about.

**The governing rule, corrected 2026-08-22 after the owner rejected the first
version of Option C:**

> **Automation never writes a version component. `pkgver` and `pkgrel` both come
> from a human commit; automation verifies them and refuses to publish when they
> are wrong.**

The first draft of Option C said *"on `release: published`, publish with
`pkgrel=1`"*, and put "verify, never write" below as a refinement. The owner's
objection is exact: **that is Option B's defect in a different place.**
Automation asserting `1` is automation inventing a value, even when `1` is the
value a human would have chosen. Verification is not a refinement of this
design; it **is** the design.

**Option A — publish only on release; no other path.**
*Pros:* one trigger, one gate, nothing to misuse.
*Cons:* **recipe-only fixes are stranded** until an unrelated release. Today's
`xdotool` fix is that case — it repairs an install that completes and then fails
to start, and would wait on a code release it has nothing to do with.

**Option B — automation detects a recipe change and bumps `pkgrel` itself.**
*Pros:* fixes ship with no human step.
*Cons:* automation decides what counts as a recipe change and **writes a version
nobody approved**, on a trigger outside the release gate. **Rejected.**

**Option C — two triggers, and automation writes no version in either.
CHOSEN 2026-08-22 by the owner.**

**The second path exists to give recipe fixes the same validation the release
path gets — not to add convenience.** That distinction is the whole reason C
beats D, and it should survive into the implementation: if the dispatch path
ever skips a check the release path runs, it has become the thing it was chosen
instead of.

| | Trigger | What is published | What automation verifies |
|---|---|---|---|
| Release | `release: published` | the committed `PKGBUILD`, unmodified | `pkgver` equals the released tag; `pkgrel` is `1` |
| Recipe fix | `workflow_dispatch`, owner-triggered | the committed `PKGBUILD`, unmodified | `pkgver` equals what the AUR carries; `pkgrel` is **greater** than it |

Every value in the published file came from a commit. Automation's job is to
**refuse**, not to fill in: a release whose `PKGBUILD` says `pkgrel=2` fails, and
so does a recipe fix that forgot to bump it. Both checks read the AUR's own
`.SRCINFO`, so they compare against what is actually published rather than what
anyone believes is published.

*Pros:* fixes ship without waiting for a release; a human acts in both paths;
automation invents nothing.
*Cons:* two documented paths instead of one.

**Option D — automate the release path only; the owner pushes recipe fixes by
hand.**
*Pros:* the strongest reading of the rule — no dispatch surface at all, and a
recipe fix is human end to end.
*Cons:* **it skips the validation, which is the entire value of the workflow.** A
hand-push publishes an unbuilt, un-`namcap`-ed recipe straight to users — which
is exactly how F81's `xdotool` omission would have shipped again. It also
requires the AUR repository and the signing key on the owner's machine.

### What `sha256sums` is for, since the question came up

`source=()` names what `makepkg` downloads — for us, a GitHub per-tag archive,
fetched **on the user's machine, at build time**, possibly long after we
published the recipe. `sha256sums=()` is the expected hash of that download.
`makepkg` verifies it **before** building; `SKIP` means it does not verify at
all.

**The AUR has no other integrity mechanism.** Packages are not signed, recipes
are not reviewed, and there is no server-side check of anything. The
`sha256sums` line is the only thing tying our recipe to specific bytes. Without
it, whatever arrives is compiled and then installed by `pacman` **as root**.

The bytes can differ without anyone attacking anything:

- **GitHub's auto-generated archives are not guaranteed byte-stable.** GitHub
  changed archive generation in 2023 and checksums shifted for tags that already
  existed, breaking packages across many distributions. A recorded hash turns
  that into a loud failure; `SKIP` turns it into a silent difference.
- **A moved or re-pushed tag** changes the archive. We control our tags, and the
  hash is what makes an accident visible.

So the hash provides two things: **tamper-evidence**, and **proof the user is
compiling the same bytes the project tested**. RFC-078 ties every other artifact
to an exact digest; this path currently has no such tie at all.

**The cost of having one, stated honestly:** if GitHub regenerates an archive,
installs break until the package is republished. That is the correct behaviour —
failing beats silently building different bytes — but it is a maintenance
obligation `SKIP` does not carry, and it is the real reason `SKIP` is tempting.

### Who computes it — automation, or the owner by hand

**Recommendation: automation, and the distinction is whether a human can
meaningfully approve the value.**

`pkgver` and `pkgrel` are **decisions**. A person can read `pkgrel=2` and judge
it right or wrong. That is why automation must not write them.

A hash is **not a decision**. Nobody inspects 64 hex characters and forms a
view; the only way to check one is to recompute it, which is what automation
would be doing. Requiring the owner to transcribe it adds ceremony without
adding judgement — and a typo fails closed at the user's machine, wasting a
publish cycle to catch a mistake that only manual entry could introduce.

It is also **derived from something the owner already approved**: the tag they
published. There is exactly one correct value and it is a measurement of an
approved artifact, not a claim about one.



**The source hash.** §1 establishes it cannot be committed: `pkgver` names an
untagged version for almost all of the repository's life, so there is no tarball
to hash. The workflow computes it from the tag the owner published.

Stated plainly so it is a decision rather than an oversight — this **is**
automation writing into the published file. It differs from a version number in
two ways. It is **derived, not decided**: a measurement of an artifact the owner
already approved, with exactly one correct value. And it **fails closed on the
user's machine** — a wrong hash makes `makepkg` refuse, loudly, before building,
whereas a wrong version number misrepresents silently and permanently.

If the owner disagrees, the alternative is supplying the hash by hand at dispatch
time, trading a derived value for a transcription step.

- **Q4 — a dedicated SSH key, or the one already registered?**
  **Recommendation: a dedicated key — but do not accept it as a security
  control, because it is not one.**

  **What it does not do.** The AUR scopes permissions **per account, not per
  key**. A dedicated key has exactly the same rights as the owner's personal
  one: it can push to every package that account maintains. Blast radius on
  compromise is **identical**. Anyone who can run the workflow, read the secret,
  or modify the workflow file on the default branch can publish to any of those
  packages under the project's identity.

  **What it does do**, and it is worth having: **independent revocation** — if CI
  is compromised, that key is revoked without the owner losing access to their
  own account — and the owner's personal key never sits in a third party's
  secret store.

  **What would actually constrain the risk**, and is the thing to decide rather
  than the key: **hold the secret in a GitHub Environment with protection
  rules.** Environment secrets are available only to jobs that reference that
  environment, and the environment can require a reviewer and restrict which
  branches may reach it. That gates *use of the credential* rather than
  possession of it, and it survives a workflow file being changed. **The same
  applies to RFC-079's Store credential**, and the two should be decided
  together rather than one at a time.
