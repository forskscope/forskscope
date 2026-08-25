# RFC 081: AUR Publication Automation

**Status.** Proposed
**Accepted.** 2026-08-22 by the project owner — Gate A cleared. Q1 closed: the
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

**Option A — publish straight to current. (Recommended.)**
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

**Option A — automation never bumps `pkgrel`; publish only on release.**
*Pros:* simplest; exactly one path to the AUR; the release approval gate is the
only gate there is.
*Cons:* **recipe-only fixes are stranded** until an unrelated release. A
packaging defect that breaks startup would sit unpublished while the code it
packages is fine. That is the case in front of us right now.

**Option B — automation detects a recipe change and bumps `pkgrel` itself.**
*Pros:* fixes reach users promptly with no human step.
*Cons:* **a second trigger path that bypasses the release approval gate**, which
is the property §4 was written to protect. It also means automation deciding
*what counts as a recipe change* — a comment edit, a whitespace change — and
inventing a version number nobody approved. **Not recommended.**

**Option C — two paths: automatic on release, manual dispatch for recipe fixes.
(Recommended.)**
On `release: published`, publish with `pkgrel=1`. For a recipe-only fix, a
`workflow_dispatch` the owner triggers, publishing the in-repo `PKGBUILD` with
whatever `pkgrel` the owner committed.
*Pros:* fixes reach users without waiting for a release; **the gate survives,
because a human still acts in both paths**; automation never invents a version
number, in either path.
*Cons:* one more documented path in `release.md`.

**One refinement that applies whichever is chosen, and closes fact (1):**
**automation never *bumps* `pkgrel`, but it does *verify* it.** If `pkgver`
differs from what the AUR currently carries, `pkgrel` must be `1`; if `pkgver` is
unchanged, `pkgrel` must be **greater** than what is published. Both are cheap
checks against the AUR's own `.SRCINFO`, and they catch the drift nothing catches
today. This is a check, not a decision — it never writes a value, it refuses a
wrong one.

- **Q4 — a dedicated SSH key, or the one already registered?** The owner's
  existing configuration works, which makes reuse tempting. **Recommendation:
  add a dedicated key**, and for a narrower reason than “least privilege” —
  **the AUR scopes permissions per account, not per key**, so a second key grants
  exactly the same rights and does *not* limit what CI could publish. What it
  does buy is real but specific: it is **independently revocable** without
  locking the owner out of their own account, and it keeps a personal key — one
  that may reach other packages the owner maintains — off a CI system. Stating
  that precisely matters, because “use a dedicated key” usually implies a scope
  reduction that is not available here.
