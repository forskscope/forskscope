# RFC 079: Microsoft Store Submission Automation

**Status.** Proposed
**Tracks.** Release pipeline; Windows distribution; credential handling.
**Touches.** A new MSIX build, a new submission workflow, `AppxManifest.xml`'s
version claims, `release.md`, and the threat model.
**Depends on.** Owner decisions in §9. Sequencing relative to Gate D in §8.

## Summary

ForskScope is published on the Microsoft Store, and every submission to date has
been made by hand. There is **no MSIX build anywhere in this repository** — no
`makeappx`, no `signtool`, no packaging step in any workflow. `AppxManifest.xml`
exists as a manifest for a human to package with locally, which is why the Store
listing runs behind the GitHub releases.

This RFC automates that path: build the MSIX in CI, submit it to Partner Center
through the Store submission API, authenticated by the project's existing
Microsoft Entra ID app registration.

**The Entra ID integration is the small part.** The bulk of the work is
producing a correct, validated MSIX at all.

## Goals

- Build a Store-ready MSIX from the same commit that produced a release.
- Submit it automatically, without a long-lived publishing secret in the
  repository.
- Keep the owner's existing approval gate — a human decides what ships.
- Make a failed or rejected submission loud, and make the resulting state
  recoverable by hand.
- Keep the manifest's version claims honest as they become automated.

## Non-goals

- Automating **publication**. Store certification is asynchronous and may
  reject; this RFC automates *submission* and reports its outcome (§4).
- Replacing the GitHub Release. It remains the primary distribution channel and
  the source of the artifacts M5 tests.
- Code-signing infrastructure. Store-distributed packages are signed by
  Microsoft under the Store-assigned publisher identity (§9, Q2).
- Any other packaging change — `.deb`, `.rpm`, Flatpak, AppImage, or Homebrew
  remain out of scope.

## Why this is not simply "add a workflow step"

Three properties of the existing release process constrain the design.

### The approval gate must survive

`release.md` is explicit: CI builds artifacts and creates a **draft**;
publication is a separate, explicit owner action, and it is the point after
which a version is immutable. Submitting to the Store on **tag push** would
push a build toward users before the owner approved the GitHub release —
inverting the gate, and doing so on the channel that is *hardest* to retract.

**Therefore the submission triggers on the `release: published` event**, not on
tag push. The Store submission inherits the approval the owner already gives.

This also means a **new workflow file** and **no change to `release.yml`**,
which matters while M5 is gathering evidence about that exact workflow.

### Submission is not publication

Certification runs on Microsoft's side, takes hours to days, and can fail for
reasons no local check predicts. A workflow that reports "published" would be
claiming something it cannot observe. The workflow reports **submitted**, plus
the submission's status at the time it finished polling, and nothing stronger
(§4).

### Automation makes unsettled claims recurring

`AppxManifest.xml` currently declares `MinVersion=10.0.17763.0` (Windows 10
1809) and `MaxVersionTested=10.0.19041.0` (Windows 10 2004, predating Windows 11
entirely). Two open register items bear on those values:

- **F60** — the declared floor has no runtime evidence and none is planned; the
  oldest Windows the project claims to support has never been observed running
  the application.
- **F49b** — `MaxVersionTested` is deferred until M5's Windows evidence exists.

Today those claims reach the Store only when a human submits. Automated, they
ship on every release with nobody looking. **F60 should be settled before this
goes live** — not because automation makes the claim worse, but because it makes
it recurring and unattended.

## Design

### 1. MSIX build

A new job produces a Store-ready MSIX from the same commit and the same built
binary the Windows zip already uses. It must:

- stamp `AppxManifest.xml`'s `Version` from the workspace version, as the
  existing release gates already require of every other version carrier
  (`cargo xtask version-sync` covers this file today);
- lay out the package payload — executable, assets, manifest — under a staging
  directory;
- produce the `.msix` with `makeappx`;
- **validate before submitting** (§3).

The four-part `Version` attribute (`X.Y.Z.0`) is Store-specific and already
maintained; nothing here changes the versioning scheme.

### 2. Credentials

**Owner decision (2026-08-16): an Entra ID client secret in GitHub Actions
Secrets.** That is functionally sufficient — the Partner Center API accepts
tenant/client/secret, and it is the documented path.

Two costs follow from it, and neither blocks:

- **Entra ID client secrets expire** (24 months maximum, frequently shorter
  under tenant policy). A stored secret means releases break silently at
  expiry, at whatever moment that happens to be. **The expiry date must be
  recorded where it surfaces before it lapses** — otherwise this becomes
  another check that appears healthy until the moment it is needed, which is
  the failure shape this project has now catalogued five times.
- It is the only credential in this project whose compromise lets someone ship
  code to users under the project's identity. Scope it to the minimum Partner
  Center permissions the submission API needs, not to whatever the registration
  already has.

**Federated credentials (OIDC) remain the preferred end state** and can be
adopted later without redesign: the workflow shape is identical and only the
authentication step changes. Recorded here so the choice is a decision with
known costs rather than a default.

### 2a. Signing — resolved, nothing to handle

Authentication uses the project's existing Entra ID app registration via
**OIDC federated credentials**: GitHub's `id-token: write` permission plus a
federated credential on the app registration, scoped to this repository and to
the specific workflow.

**No client secret is stored in repository settings.** A long-lived publishing
credential would be the weakest link in a release path that otherwise enforces
dependency paths, disposes of advisories individually, and fails closed on
untrusted input. It would also be the only project secret whose compromise lets
someone ship code to users under the project's identity.

Scope the federated credential as narrowly as the provider allows — repository,
workflow, and environment — so it cannot be used from an unrelated workflow or a
fork.

### 3. Validation before submission

A submission that fails certification costs days. Cheap local checks first:

- the MSIX's manifest version equals the released tag;
- `Identity`, `Publisher`, and `PublisherDisplayName` match the Store listing;
- the package contains the executable and every asset the manifest references;
- the package installs and the application launches (see §7).

A failure here fails the workflow **before** anything reaches Partner Center.

### 4. Submission and reporting

The workflow creates a submission, uploads the package, and commits it. It then
reports:

- the submission identifier;
- its status at the time polling ended;
- a link to Partner Center for the human-readable state.

It does **not** wait for certification to complete, and does **not** report
publication. If polling ends while certification is still running — the normal
case — the workflow succeeds with the status recorded.

### 5. Failure and recovery

Two failure classes, handled differently:

- **Pre-submission failure** (build, validation, authentication): the workflow
  fails, nothing reached the Store, and the GitHub release is unaffected. Retry
  after a fix.
- **Post-submission failure** (certification rejection): the workflow has
  already succeeded. The rejection arrives by mail from Partner Center, and
  recovery is a human action — correct the package and submit again, manually or
  by re-running the workflow.

**A failed Store submission must never be resolved by editing the published
GitHub release.** `0.166.0` and `0.167.0` are immutable; a Store problem is
fixed with a new version, not by mutating a shipped one.

The workflow must also be safe to re-run: a second run against the same release
either replaces the pending submission or fails clearly, and must not create
duplicate submissions silently.

### 6. What the docs must say

`release.md` gains the Store step in its actual position — after publication,
not before — and states plainly that submission is automatic while
**certification and Store publication are not**, so the owner still watches
Partner Center for the outcome.

`docs/src/users/installation.md` currently warns that the Store listing "is not
always current." Once this lands, that warning becomes false and must be
narrowed to certification lag rather than manual-submission lag.

The threat model gains the publishing-credential path: what the federated
credential can do, its scope, and what compromise would mean.

## Acceptance criteria

- An MSIX is built from the same commit as the release, with its manifest
  version matching the tag.
- Validation (§3) fails the workflow before submission when any check fails —
  demonstrated by a deliberately broken package, per this project's standing
  falsifiability requirement.
- The submission authenticates with no long-lived secret in repository settings.
- The workflow triggers on `release: published` and never on tag push.
- The workflow reports a submission identifier and status, and claims no more.
- Re-running against the same release does not create a duplicate submission.
- `release.md`, `installation.md` and the threat model reflect the new path.
- `release.yml` is unchanged.

## Testing

The falsifiability standard applies as everywhere else: each check must be
demonstrated **failing** on a deliberately broken input before it is accepted as
working. A submission workflow that cannot be shown to reject a bad package has
not been shown to check anything.

Store submission cannot be dry-run against production without consuming a real
submission. Two mitigations, and the choice belongs to implementation:

- validate the package fully (§3) so most defect classes fail locally;
- exercise the authentication and API path against a non-committing endpoint —
  reading the application's existing submission state proves credentials and
  connectivity without creating one.

## Sequencing

**Design now; implement after Gate D.**

The reasoning is timing, not risk. M5 is currently gathering platform evidence
against `0.167.0`'s exact artifacts. Adding a Windows artifact mid-matrix means
new digests and re-run rows, and RFC-078 is explicit that evidence from an older
hash cannot approve a newer artifact. The design costs nothing to settle now and
the implementation is cheap to defer.

**F60 should be decided before the first automated submission** (§"Automation
makes unsettled claims recurring").

## Open questions for the owner

1. **Timing.** Implement after Gate D as §8 proposes, or sooner? A separate
   workflow keeps the risk low either way; the cost of "sooner" is re-running
   M5's Windows rows.
2. ~~**Signing.**~~ **Closed (2026-08-16).** `Publisher="CN=C4BA37E8-8670-4C82-8365-5ECB57373921"`
   is a Store-assigned publisher identity: Microsoft signs the package on
   submission, and no code-signing certificate of the project's own is
   involved. Nothing further to handle.
3. **F60.** Still open. The owner's direction (2026-08-16) is that Windows
   release verification in CI should improve before automation — right on its
   own merits, but it **cannot close F60**: GitHub Actions offers no Windows 10
   runner, and `windows-latest` is a Server-2025-based image at kernel
   NT 10.0.26100. The gap is a machine nobody has, not a check nobody wrote, so
   no amount of CI improvement evidences Windows 10 1809. F60's three
   resolutions stand — narrow the floor to what is evidenced, obtain a Windows
   10 host (a VM is the cheap version), or state openly that 1809 is a declared
   compatibility floor carrying no runtime evidence.
4. **Entra ID app registration.** Does the existing registration already have
   the Partner Center permissions this needs, and can a federated credential be
   added to it — or is a separate registration preferable so that publishing
   rights are isolated from whatever else it does?
5. **Store listing metadata.** This RFC automates the *package*. Listing text,
   screenshots and store descriptions stay manual unless you want them in scope,
   which would be a materially larger design.

## Dependencies

- Gate D sequencing (§8).
- F60, F49b — the manifest's version claims.
- No dependency on RFC-078's outcome beyond timing; this changes no product
  behaviour and no artifact M5 tests.
