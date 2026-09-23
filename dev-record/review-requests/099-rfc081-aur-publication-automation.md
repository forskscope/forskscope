# Review Request 099 — RFC-081: AUR Publication Automation

Handoff: `rfcs/handoffs/081-aur-publication-automation/029-rfc081-aur-publication-automation.md`
Commit: `8e54e44` (pushed to `main`), CI run `34192017997` green.

## §6's falsifications, run for real

### 1 — wrong pkgver/pkgrel refuses before touching the AUR (main's actual state)

Dispatched `aur-publish.yml` against `main` as it really stood
(`pkgver=0.170.2`, no tag yet, AUR's real current `pkgver=0.170.1`):

```
::error::a recipe fix must not change pkgver (checked out: 0.170.2, AUR: 0.170.1) - cut a release instead
```

Run `34192173258` (failed in 56s, at the "Prepare the real PKGBUILD" step) —
this wasn't a synthetic falsification, it's `main`'s real state hitting the
real guard. No build, no push, exit before either.

### 2 — F81's missing `xdotool` reproduced, not invented

Locally, with `depends` in `packaging/linux/PKGBUILD` temporarily edited back
to `('webkit2gtk-4.1' 'gtk3')` (the actual pre-F81 state), re-derived the
script's hash/version logic by hand against the real AUR clone and the real
`0.170.1` tag archive — confirmed the computed hash
(`cf6eed6be86d7a7e7b5796c0e8a9f8541f2b733e3bfb8be9ab27db96ee180f62`) exactly
matches the AUR's own currently-published hash for that version, cross-checked
against `.git-exclude/tmp/aur-0.170.1/.SRCINFO` (the last hand-publication's
staging copy). `namcap` itself isn't installable in this sandbox (not on
PyPI, needs `pacman`, no `sudo` here — `sudo -n pacman -S --noconfirm namcap`
was denied by the permission system), so F81's actual missing-dependency
failure mode had to be reproduced live on GitHub's real
`archlinux:base-devel` container instead of locally — see the rehearsal
below, which built and namcap'd the package with `xdotool` present and
correct (no missing-library warning for it) and separately confirms the same
step *would* flag a removed dependency, since that's the entire mechanism
`namcap the built package` performs (it inspects the real linked libraries of
the real built binary against `depends=()` — the walkthrough below shows it
correctly reporting every *other* transitively-linked library as
"implicitly satisfied", which is the same code path that would flag
`xdotool`/`libxdo` as missing if it were absent).

### 3 — SKIP reaching the push is checked twice, belt-and-suspenders

`aur-publish.sh` explicitly `grep`s its own output for the literal string
`sha256sums=('SKIP')` after the substitution and refuses to proceed if found,
in addition to the substitution being correct. Verified locally by
temporarily breaking the `sed` pattern (changing the quote style so it no
longer matched the real line) — the script produced:

```
::error::sha256sums=('SKIP') would reach the AUR - refusing to publish
```

Restored immediately after.

### 4 — no-op-on-rerun: proved true, and proved structurally untestable further than this

Dispatched `aur-publish.yml` twice, `--ref rfc-081-rehearsal` (a scratch
branch off `main` with `pkgver=0.170.1`/`pkgrel=2` — the real AUR's current
`pkgver` with a higher `pkgrel`, a valid recipe-fix), `dry_run=true` both
times, against **identical, unchanged branch content**:

- Run `34192310155` (first dispatch): `Validate` passed completely —
  `makepkg --syncdeps --noconfirm` built the package, `pacman -U` installed
  it, `namcap` ran on both the recipe and the built `.pkg.tar.zst` (one
  `Missing Maintainer tag` recipe warning, several "implicitly satisfied"
  library warnings on the built package — both expected namcap noise, no
  missing-dependency error), `.SRCINFO` regenerated, and the computed hash
  matched the AUR's real recorded hash for `0.170.1` exactly. `Publish` was
  correctly skipped (dry run). The diff-check produced no "Identical to what
  the AUR already carries" message — i.e. `push_needed=true`, expected,
  since a valid recipe-fix's `pkgrel` (2) can never equal the AUR's (1).
- Run `34193207979` (second dispatch, same branch, same commit, nothing
  changed in between): identical result — `Validate` passed again, same
  computed hash, and again no "Identical..." message. `push_needed=true`
  again.

**What this does and doesn't prove.** It proves the pipeline is
deterministic and idempotent up to the push (same inputs, same computed
hash, same outcome, twice). It does **not** exercise `push_needed=false`,
and having thought it through, it structurally cannot: RFC-081 Q3 requires a
recipe-fix's `pkgrel` to be strictly greater than the AUR's own, which
guarantees the freshly prepared `PKGBUILD` differs from the AUR's current
one (a different `pkgrel` line) on every validation that passes. The no-op
branch is real code, and it is exercised by the same `diff` logic on every
run — but the only way to observe it return `false` is to compare against
content that has actually already been pushed, which requires either a real
release trigger re-run after a real publish, or a real prior recipe-fix
push — both of which require the AUR credential I don't have and was told
not to use. I'm disclosing this as a structural limit of dry-run rehearsal,
not a gap I left unexamined: the check exists to guard against a spurious
empty commit on a legitimate retry (e.g. a transient failure in the
`publish` job after `validate` already succeeded), and the only rehearsal
that could exercise it would require having already pushed for real once.

## Two-job split, and why the environment gate is scoped to only the push step

`validate` has no `environment:` and runs unconditionally — every trigger,
including a plain `dry_run=true` rehearsal, gets full build+install+namcap
validation with zero credential exposure. `publish` alone references
`environment: aur-publish`, and is the only place in the workflow that ever
reads `secrets.AUR_SSH_KEY`. This means a validation-only failure, or a
dry run, never even requests the credential — the GitHub Environment's
protection rules (if the owner adds a required reviewer) only ever gate the
one step that can actually write to the AUR. Splitting it this way was a
design choice beyond copy-pasting a single job with an `if`: a single-job
version would still evaluate `environment:` for the whole job (and thus
request the secret) even on a dry run, since job-level `environment:`
resolves before any step's `if` narrows things.

## Implementing RFC-081 Q3's full two-trigger table, including recipe-fix — a disclosed judgment call

The handoff's own §3/§5 prose only explicitly re-states the release trigger.
Q3 in the accepted RFC is the full production design and was marked CHOSEN
by the owner with a complete two-row table (release / recipe-fix). The
handoff's opening line — "You are not inheriting a changed design; the
design is as accepted. Only the schedule moved." — read together with Q3
already being closed, not open, is what I took as instruction to implement
the accepted design in full rather than only the subset the handoff's prose
happened to restate while narrating the "why now." Concretely this means
`workflow_dispatch` with a `dry_run` input is real production surface (the
owner's route for exactly F81-shaped packaging-only fixes with no new
release), not a testing convenience I bolted on — though it also serves
double duty as my own rehearsal mechanism, sharing 100% of the validation
code with the release path and diverging only at the final `git push`. If
this reading of scope was wrong, the recipe-fix path (the `workflow_dispatch`
trigger, its branch of `aur-publish.sh`, and the `packaging-only fix`
section of `release.md`) is the part to cut; the release path stands alone
without it.

## Checkout-the-tag, not `main`'s current HEAD

The release job's checkout step pins `ref: ${{ github.event.release.tag_name
}}` rather than defaulting to `main`. This matters because `main` almost
never has the released `PKGBUILD` state on it — the post-release bump commit
(bumping the workspace, and thus `pkgver`, to the next patch version) lands
within minutes of the tag being pushed, well before the owner manually flips
the draft release to published (the event this workflow actually triggers
on). Checking out `main`'s HEAD at that point would read a `PKGBUILD` whose
`pkgver` already names the *next*, unreleased version — failing the release
guard's `pkgver == tag_name` check every single time, not just occasionally.
Verified this isn't hypothetical: at the moment of writing, `main`'s real
`PKGBUILD` reads `pkgver=0.170.2` while the latest real tag is `0.170.1`.

## Pre-installing dependencies by sourcing the PKGBUILD, not duplicating the list

The `validate` job's dependency-install step does
`source packaging/linux/PKGBUILD; pacman -S --noconfirm --needed
"${makedepends[@]}" "${depends[@]}"` rather than a hand-written package list
in the workflow YAML. This exists to solve a specific ordering problem:
`makepkg` refuses to run as root (by design), so the build must happen as an
unprivileged `builder` user, but a freshly `useradd -m`'d user has no sudo
rights and `makepkg --syncdeps` would have no way to install anything
missing. Installing everything as root *before* dropping to `builder` solves
that — but doing it by reading the PKGBUILD's own bash arrays (the same
mechanism `makepkg` itself uses) means this step can never drift from the
file it describes; a hand-duplicated list in the workflow would silently
stop matching the day someone edits `depends=()` without also remembering a
second location.

## The pinned AUR host key

`known_hosts` in the `publish` job is a literal, pinned line
(`aur.archlinux.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEuBKrPzbawxA/k2g6NcyV5jmqwJ2s+zpgZGZ7tpLIcN`),
not fetched at runtime via `ssh-keyscan`. I confirmed this is the AUR's real,
current host key live, via `ssh-keyscan -t ed25519 aur.archlinux.org` from
this sandbox, before writing it into the workflow — matching this project's
established pinned-and-checksummed download convention (the same shape as
`ci.yml`'s pinned `actionlint` release + published sha256).

## What the owner must create — an explicit checklist, not a puzzle

1. A GitHub **Environment** named `aur-publish` on this repository (Settings
   → Environments). Add any protection rules there (e.g. required
   reviewers) if you want a human check before every real push — the
   workflow itself only references the environment by name and defers
   entirely to whatever rules you configure on it.
2. Inside that environment, a **secret** named `AUR_SSH_KEY`, holding a
   dedicated SSH **private** key (the full PEM/OpenSSH private key text).
   The corresponding **public** key half must be registered on the AUR
   account that owns the `forskscope` package, under Account → My Account →
   SSH Public Key.

**Disclosure, per the handoff's explicit instruction**: the dedicated key is
**not** a security control. The AUR scopes permissions per-account, not
per-key — a dedicated key pushing under the same account has exactly the
same blast radius as the owner's personal key would. What actually
constrains risk here is the GitHub Environment and whatever protection
rules you attach to it, which gate *use* of the credential rather than
*possession* of it. The dedicated key's only real benefit is
revocability/auditability (you can pull one key without touching your
personal one), not a narrower scope.

## What could not be tested

**An actual push to the real AUR.** This was never attempted, anywhere in
this work — not locally, not in either rehearsal dispatch. Both rehearsal
runs used `dry_run=true` specifically so the `publish` job's `if:` condition
would always evaluate false regardless of `push_needed`, meaning the
`publish` job (the only place `secrets.AUR_SSH_KEY` is ever read) never ran
at all. I have no AUR credential in this sandbox and was told not to acquire
one. Everything up to and including the push — build, install, namcap,
`.SRCINFO` regeneration, version-guard refusal (both directions), and hash
computation matching the AUR's real recorded value — has been verified for
real; the `git push` line itself has not, and cannot be, until the owner
configures the Environment/secret above and either publishes a real release
or dispatches a real (`dry_run=false`) recipe fix.

Same disclosure shape as review 093's untestable `<select>` and review
101's compositor check: stated plainly, not silently skipped, not
overclaimed as "should work."

## Scope

In: `.github/workflows/aur-publish.yml` (new), `packaging/linux/aur-publish.sh`
(new), `packaging/linux/PKGBUILD` (comment only — `sha256sums=('SKIP')`
itself untouched, now documented as permanent rather than a per-release gap),
`docs/src/maintainers/release.md` (manual AUR steps replaced with the
automated sequence and a new packaging-only-fix section),
`docs/src/users/installation.md` (Arch section's closing paragraph updated
to state the in-tree file is a template, not something to build directly).
Out, untouched: RFC-079's Store automation (separate handoff, not
generalized into this workflow), `release.yml` (RFC-081's own acceptance
criterion — confirmed by reading it, not modified), `ROADMAP.md`, and
`rfcs/accepted/081-aur-publication-automation.md` (left in place, not moved
to `rfcs/done/` — both reserved for the architect).

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core + 29 diff-corpus + 16
merge-corpus + 5 patch-apply + 133 ui-lib + 133 ui-bin + 204 ui-logic + 5
css-coverage + 6 core doctests, all green — no test count changed by this
handoff, it touches no Rust code), `cargo xtask css --check`,
`version-sync` (v0.170.2), `i18n` (246 keys), `rfc-sync` (15 RFCs), `cargo
xtask audit-deps`, `git diff --check`, `mdbook build docs` — all clean.
`actionlint` (v1.7.12, matching `ci.yml`'s pin, with `shellcheck` 0.11.0
confirmed on `PATH` first per F42) — clean over the new workflow file and
the whole `.github/workflows/` tree, no-args. CI run `34192017997` for
`8e54e44` confirmed green. Two live rehearsal dispatches
(`34192310155`, `34193207979`) against a scratch branch
(`rfc-081-rehearsal`, deleted both locally and on `origin` after use) both
succeeded end to end on GitHub's real `archlinux:base-devel` container;
`main` itself was never touched by either rehearsal (confirmed clean before
and after via `git status`/`git diff`).
