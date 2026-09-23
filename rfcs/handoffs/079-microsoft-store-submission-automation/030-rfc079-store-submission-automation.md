# Handoff 030 — RFC-079: automate Microsoft Store submission

**From:** architect. **RFC:** `rfcs/accepted/079-microsoft-store-submission-automation.md`
(accepted 2026-08-22; **re-scheduled to now on 2026-09-08**, Q4 closed same day).
**Release:** 0.171.0 or later. **Not release-blocking.**
**Queued behind handoff 029** — that is queue order, not a dependency.

## 1. Why now

RFC-079 was parked *"after Gate D"* on reasoning that assumed a matrix run was
imminent. Six releases later it has not happened, and the owner has performed
**six manual MSIX submissions**. The deferral was the architect's error; the
design is unchanged.

**This also closes F103.** On 2026-09-08 the owner submitted `0.170.1` to the
Store while `0.170.0` was the newest release, because
`packaging/windows/README.md` said to build from the working tree — where
`AppxManifest.xml` names the *next* version. Every other artifact is gated by
`version-sync` against the tag; the MSIX is not. **§2's first bullet is that
gate.**

## 2. Owner decisions, settled

- **Existing Entra ID app registration** — already scoped to ForskScope, so a
  second would isolate it from nothing (Q4, closed 2026-09-08).
- **Credential in a GitHub Environment with protection rules**, not a plain
  repository secret — gating *use* rather than possession, and surviving a
  workflow file being modified.
- **The client secret expires** (24 months max, often less) and **a lapsed one
  breaks releases silently**. The workflow must fail with a message that names
  expiry as a likely cause, rather than surfacing a bare auth error.

---

# Part A — the precondition, which is not automation

RFC-079 §9 Q5 requires this **before** the automation ships, and it is real
work, not paperwork:

- **Store listing content gets a tracked home in this repository, as data** —
  long description, search terms, per-market copy.
- **Screenshots become committed assets**, not promises to regenerate. Note
  there is **no screenshot machinery today**: `render_check.py` walks the AT-SPI
  tree and asserts geometry, capturing no images. Generating them is real work,
  not a small extension.
- **Manifest-versus-listing precedence is written down.** `AppxManifest.xml`
  already carries `DisplayName`, `Description` and `PublisherDisplayName`; the
  Store listing carries its own. **Nothing records which wins when they
  disagree**, and automating the package while that is undefined ships one half
  of a description whose other half nobody tracks.

**The point is not to automate the listing** — Q5 explicitly keeps that manual.
It is that manual publication of *version-controlled* content is a step someone
can automate later; manual publication of content living only in Partner Center
is the debt, with nothing to automate *from*.

If Part A turns out larger than it reads, **say so and stop** — it can become its
own handoff.

---

# Part B — the automation

## 3. Build

A job producing a Store-ready MSIX from the **same commit and same built binary
the Windows zip already uses**. It must:

- **stamp `AppxManifest.xml`'s `Version` from the workspace version** — this is
  F103's fix and the reason the manual path went wrong;
- stage the payload — executable, assets, manifest — under a staging directory;
- produce the `.msix` with `makeappx`;
- validate before submitting.

The four-part `Version` (`X.Y.Z.0`) is Store-specific and already maintained.
Nothing here changes the versioning scheme.

**`packaging/windows/README.md` documents the manual equivalent** and was
rewritten on 2026-09-08 to build from the tag and stage into a temp directory
rather than packing `packaging/windows/` wholesale. Read it — the traps it
names are the ones the workflow must not reproduce.

## 4. Validate — and one check you must not fake

- the MSIX's manifest version equals the released tag;
- `Identity`, `Publisher`, `PublisherDisplayName` match the Store listing;
- the package contains the executable and every asset the manifest references;
- **the package installs and the application launches.**

That last one, in the RFC's own words:

> **This is the expensive check and the one most likely to be quietly dropped.**
> Installing an MSIX on a runner needs the package trusted for sideloading,
> which a Store-signed package is not until Microsoft signs it — so this step
> must either use a temporary self-signed layout for validation only, or be
> **honestly recorded as not performed. Do not report it as done if the runner
> merely unpacked the package.**

Take that as addressed to you. Either implement it properly or say it is not
done — unpacking is not installing, and a report claiming otherwise is worse
than the gap.

A failure here fails the workflow **before** anything reaches Partner Center.

## 5. Submit, and stop

Create a submission, upload, commit it. Report the submission identifier, its
status when polling ended, and a Partner Center link.

**Do not wait for certification and do not report publication.** Polling ending
while certification runs is the *normal* case, and the workflow succeeds with
the status recorded.

## 6. Recovery

- **Pre-submission failure** — nothing reached the Store, the GitHub release is
  unaffected, retry after a fix.
- **Post-submission failure** — certification rejection arrives by mail; recovery
  is human.

> **A failed Store submission must never be resolved by editing the published
> GitHub release.** Published releases are immutable; a Store problem is fixed
> with a new version, not by mutating a shipped one.

**Re-run safety:** a second run against the same release either replaces the
pending submission or fails clearly. **It must not create duplicate submissions
silently** — prove that by running it twice, not by reading the API docs.

## 7. What you cannot test

You will not have the credential and **must not** submit to the real Store.
State exactly which steps ran and which are structurally unverifiable without
it — the disclosure shape of reviews 093, 101 and (expected) 029.

If you build a dry-run path, say whether it shares code with the real one. A
validation chain that only runs in dry-run is worth nothing.

## 8. Scope

**In:** the MSIX build job, validation, the submission workflow, `release.md`,
Part A's tracked listing content.

**Out:** RFC-081's AUR automation (handoff 029 — different credential, different
shape; do not generalise them into one workflow); listing-metadata *submission*,
which Q5 keeps manual by decision.

**The owner creates the Environment and the secret.** Say exactly what must
exist and under what name, so setup is a checklist rather than a puzzle.

## 9. Gates

The usual set, plus `actionlint` on the new workflow. F23 exists because a
release workflow's syntax error was invisible until a real cut — and F102 exists
because a release workflow step with no retry failed one.
