# RFC 081: AUR Publication Automation

**Status.** Proposed
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

### 1. F81 first — this RFC cannot ship before it

Two defects, both in the file this RFC would publish:

- `sha256sums=('SKIP')` — never refreshed, so `makepkg` verifies nothing on a
  network fetch. **Automating publication of an unverified fetch would make a
  supply-chain gap recurring and unattended**, which is exactly the reasoning
  RFC-079 §"Automation makes unsettled claims recurring" applies to the Store
  manifest.
- `depends` omits `xdotool` — see above.

Neither is this RFC's work; both are its precondition.

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

- **Q1 — maintainer rights.** Does the account that published `0.22.13` still
  hold them? An AUR package unmaintained for that long may have been **orphaned**
  or adopted. **Nothing in this RFC is actionable until that is known**, and it
  is the one question no design decision can route around.
- **Q2 — the `0.22.13 → 0.167.2` jump.** Publishing current is assumed. Say so
  if you want an intermediate step; I see no reason for one.
- **Q3 — `pkgrel` policy.** A new upstream version resets `pkgrel=1`. Packaging-only
  fixes bump it. Who decides, and does automation ever bump it unattended?
  Recommendation: never — a `pkgrel` bump means the recipe changed, which is a
  human change.
- **Q4 — a dedicated SSH key** (recommended) or an existing one with AUR access?
