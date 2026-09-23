# Handoff 029 — RFC-081: automate AUR publication

**From:** architect. **RFC:** `rfcs/accepted/081-aur-publication-automation.md`
(accepted 2026-08-22; **re-scheduled to now on 2026-09-08**).
**Release:** 0.171.0. **Not release-blocking.**

## 1. Why this is being done now, after being deferred

RFC-081 was parked as *"nothing about it is urgent."* Five hand-publications
later — 0.167.2, 0.168.0, 0.169.0, 0.170.0, 0.170.1 — the owner reports it
costs. **The deferral was the architect's and its premise was never re-tested
against how often the manual step actually ran.** You are not inheriting a
changed design; the design is as accepted. Only the schedule moved.

## 2. Owner decisions, settled — do not re-open these

- **Credential home: a GitHub Environment with protection rules**, not a plain
  repository secret. This gates *use* of the credential rather than possession,
  and survives a workflow file being modified.
- **A dedicated AUR SSH key**, not the owner's personal one.

**State plainly in the review request that the dedicated key is not a security
control**, and do not let the implementation imply otherwise: the AUR scopes
permissions **per account, not per key**, so blast radius on compromise is
identical to the personal key. What it buys is independent revocation and
keeping the owner's key out of GitHub's secret store. Both real; neither is
containment.

## 3. The shape, from the RFC

**Trigger: `release: published`. Never on tag push.** A tag exists before the
artifacts do; publication is the point at which the release is real. This is an
acceptance criterion, not a preference.

**The hash is computed at publish time and never committed.** `pkgver` in the
in-tree `PKGBUILD` names an *unreleased* version nearly always, because
`version-sync` ties it to the workspace version and the workspace bumps
immediately after each cut. So the workflow fetches the published tag's archive,
computes the hash, and writes it into the `PKGBUILD` it pushes. **The in-tree
file stays a template.**

> **`sha256sums=('SKIP')` must never reach the AUR.** §3 checks this explicitly,
> and the RFC names it as *the check most likely to be skipped as obvious*.
> Treat that sentence as aimed at you.

**Push only `PKGBUILD` and `.SRCINFO`.** `.SRCINFO` is generated, never
committed to this repository.

## 4. §3's validation is the substance, not the plumbing

Build the package in CI, `pacman -U` it, and run `namcap` on both the recipe and
the built package.

**The RFC's argument for this is worth reading rather than skimming**, because
it is the reason the handoff exists at all: a maintainer publishing by hand
*builds the package first* — not as discipline, but because that is how they
check their own work. **Automating the push deletes that step.** Nobody would
build the Arch package at any point in the process.

That is not hypothetical: `depends` omitted `xdotool` through **three releases**,
on a path `installation.md` tells Arch users to follow, and it was found by
reading the file rather than by anything running. `namcap` reports missing
library dependencies by design and would have caught it.

So §3 does not add safety above normal practice — **it restores safety that
automation removes.**

## 5. Acceptance criteria, verbatim from the RFC

- The workflow triggers on `release: published` and never on tag push.
- **A deliberately broken `PKGBUILD` fails validation before any push.**
- `namcap` runs on both the recipe and the built package, and a deliberately
  broken one fails.
- `.SRCINFO` is generated, never committed here.
- **Re-running against the same release pushes nothing and fails nothing.**
- The AUR repository receives only `PKGBUILD` and `.SRCINFO`.
- `release.md` and `installation.md` reflect the new path.

## 6. Falsification

The RFC's criteria are already written as falsifications — build to them
directly:

1. A `PKGBUILD` with a wrong hash must fail **before** the push step.
2. A `PKGBUILD` with `depends` missing `xdotool` must fail `namcap`. This is
   F81's actual defect; reproduce it rather than inventing a different one.
3. `sha256sums=('SKIP')` reaching the push step must fail.
4. Re-running against an already-published release is a no-op — **prove it does
   not push, rather than that it does not error.**

## 7. What you cannot test, and must say so

You will not have the AUR credential, and **must not** push to the real AUR from
a test. Say in the review request exactly which steps ran and which are
structurally unverifiable without the secret — the same disclosure shape as
review 093's untestable `<select>` and review 101's compositor check.

**A dry-run mode that stops before the push is the honest way to exercise the
rest.** If you build one, say whether the real path and the dry-run path share
code or diverge — a validation chain that only runs in dry-run is worth nothing.

## 8. Scope

**In:** the new publish workflow, `packaging/linux/PKGBUILD` if the template
needs adjusting, `release.md`, `installation.md`.

**Out:** RFC-079's Store automation — different credential, different
precondition, separate handoff. Do not generalise the two into one workflow.

**The owner creates the Environment and the key.** Write the workflow to consume
a named secret and say in the review request exactly what must be created and
with what name, so the setup is a checklist rather than a puzzle.

## 9. Gates

The usual set, plus `actionlint` on the new workflow — `ci.yml` already runs it
pinned and checksummed, and F23 exists because a release workflow's syntax error
was invisible until a real cut.
