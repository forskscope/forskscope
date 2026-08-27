# RFC 079: Microsoft Store Submission Automation

**Status.** Proposed
**Scheduling.** Post-Gate-D — accepted; implementation blocked on F60. See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in this folder and nothing else (F83).
**Accepted.** 2026-08-22 by the project owner — Gate A cleared, and
**re-confirmed the same day after a self-review found two defects**, including
an acceptance criterion that contradicted the owner's own §2 credential
decision and would have failed by construction. Stays in
`proposed/` until implemented, per the 4-folder lifecycle; it moves to `done/`
when the work ships. **§9 is down to one open question — Q4, the app
registration — plus recording the client secret's expiry. Q1 and Q5 were
decided 2026-08-22; Q3 was withdrawn as never having been a question and moved
to Dependencies, where F60 blocks implementation.**
**Tracks.** Release pipeline; Windows distribution; credential handling.
**Touches.** A new MSIX build, a new submission workflow, `AppxManifest.xml`'s
version claims, `release.md`, and the threat model.
**Depends on.** **F60 — blocking** (see Dependencies). One remaining owner
decision, §9 Q4. Sequencing relative to Gate D in §8.

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
- Submit it automatically, with the narrowest credential that does the job
  (§2).
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
- **F49b** — `MaxVersionTested` claims Windows 10 2004, predating Windows 11.
  **M5's Windows evidence now exists** (M5-A on NT 10.0.26100; the owner's manual
  runs on Windows 11 build 10.0.26200, 2026-08-21), so the precondition this item
  was waiting on is met and the value can be raised on evidence. *(This read
  “deferred until M5's Windows evidence exists” until the 2026-08-22 re-review.)*
  Note the asymmetry with F60: M5 evidences the **ceiling**, and says nothing
  whatever about the **floor**.

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

`AppxManifest.xml` declares `Publisher="CN=C4BA37E8-8670-4C82-8365-5ECB57373921"`,
a **Store-assigned publisher identity**. Packages submitted under it are signed
by Microsoft during certification, so no code-signing certificate of the
project's own is involved and none needs handling in CI.

The manifest's `Identity`, `Publisher` and `PublisherDisplayName` must
nevertheless match the Store listing exactly, or the submission is rejected —
which is why §3 validates them locally before anything is uploaded.

### 3. Validation before submission

A submission that fails certification costs days. Cheap local checks first:

- the MSIX's manifest version equals the released tag;
- `Identity`, `Publisher`, and `PublisherDisplayName` match the Store listing;
- the package contains the executable and every asset the manifest references;
- the package installs and the application launches. *(This read “see §7” until
  the 2026-08-22 re-review; **there is no §7** — the Design sections run 1, 2,
  2a, 3, 4, 5, 6. The pointer was dangling.)* **This is the expensive check and
  the one most likely to be quietly dropped:** installing an MSIX on a runner
  needs the package trusted for sideloading, which a Store-signed package is not
  until Microsoft signs it — so this step must either use a temporary
  self-signed layout for validation only, or be honestly recorded as not
  performed. **Do not report it as done if the runner merely unpacked the
  package.**

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

The threat model gains the publishing-credential path: what the **stored client
secret** can do, its scope, what compromise would mean, and its expiry date —
per the owner decision in §2, which chose a stored secret over a federated
credential with the costs recorded there.

## Acceptance criteria

- An MSIX is built from the same commit as the release, with its manifest
  version matching the tag.
- Validation (§3) fails the workflow before submission when any check fails —
  demonstrated by a deliberately broken package, per this project's standing
  falsifiability requirement.
- The submission authenticates with the **stored Entra ID client secret** the
  owner chose in §2, held in GitHub Actions Secrets, scoped to the minimum
  Partner Center permissions the submission API needs — **and its expiry date is
  recorded somewhere that surfaces before it lapses.**
  *(This criterion read “authenticates with no long-lived secret in repository
  settings” until the 2026-08-22 re-review. That is the opposite of the owner's
  §2 decision — a stored client secret is precisely a long-lived secret in
  repository settings — and it survived from the original federated-credential
  draft. An implementation satisfying §2 would have failed this criterion by
  construction. Fourth and last stale federated reference; the §2 body, the
  README index row and §9 Q4 were corrected on 2026-08-21 and 2026-08-22.)*
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

The reasoning is timing, not risk — but **the timing argument as first written
has expired, corrected 2026-08-22.** It said M5 was *currently* gathering evidence
against `0.167.0`'s exact artifacts, so a new Windows artifact would mean new
digests and re-run rows. M5's CI rows are complete, the evidence is tied to
`0.167.1`, and seven code commits have landed since — so the re-run is already
mandatory and this RFC cannot avoid it by waiting.

**What actually holds this back is F60**, which blocks implementation on its own
(see Dependencies) and would otherwise be shipped unattended on every release.
The design still costs nothing to settle now and the implementation is still
cheap to defer — for that reason rather than the original one.

**F60 should be decided before the first automated submission** (§"Automation
makes unsettled claims recurring").

## Open questions for the owner

1. **Timing. CLOSED 2026-08-22 — after Gate D**, as §8 proposes. Implementing
   sooner would mean a new artifact, new digests, and a re-run of M5's Windows
   rows for a change that alters no product behaviour.
2. ~~**Signing.**~~ **Closed (2026-08-16).** `Publisher="CN=C4BA37E8-8670-4C82-8365-5ECB57373921"`
   is a Store-assigned publisher identity: Microsoft signs the package on
   submission, and no code-signing certificate of the project's own is
   involved. Nothing further to handle.
3. ~~**F60.**~~ **Withdrawn as a question 2026-08-22 — it never was one.**
   Asked what the owner had to decide here, the honest answer is *nothing*:
   this item states a dependency and then restates F60's three resolutions,
   which are F60's to make and are already recorded there. Nothing about this
   RFC changes them, and no answer given here would close anything. Listing a
   dependency among the open questions made the decision list look longer than
   it was and invited an answer that could not exist. Moved to **Dependencies**
   below, which is where it belonged. The substance is unchanged and still
   true: no amount of CI improvement can evidence Windows 10 1809, because
   GitHub offers no Windows 10 runner and `windows-latest` is a Server-2025
   image at NT 10.0.26100 — the gap is a machine nobody has, not a check nobody
   wrote.
4. **Entra ID app registration.** Does the existing registration already have
   the Partner Center permissions this needs — or is a separate registration
   preferable, so publishing rights are isolated from whatever else it does?
   *(Corrected 2026-08-22: this question previously asked whether a federated
   credential could be added, which contradicts the owner's §2 decision to use a
   stored client secret. Third and last stale federated-credential reference;
   §2's body and the README index row were corrected on 2026-08-21.)*
   **Whichever is chosen, record the secret's expiry** — 24 months maximum,
   often less, and a lapsed secret breaks releases silently.
5. **Store listing metadata. CLOSED 2026-08-22 — package only, with one
   condition added.** Automation stays out of scope. But the owner asked whether
   that recommendation is safe against future technical debt, and the honest
   answer is that **as first given it was incomplete**: "keep it manual" is not a
   null decision, and the debt it risks is not *"not automated"* — it is
   *"not owned"*.

   **What checking found.** Listing-adjacent content is **already partly
   version-controlled and nobody has said so**: `AppxManifest.xml` carries
   `DisplayName`, a `Description`, `PublisherDisplayName` and the tile/logo
   assets. What is *not* tracked is the Store listing proper — long description,
   screenshots, search terms, per-market copy. So a split already exists, and
   **nothing records which side wins when the manifest's `Description` and the
   Store listing disagree.** Automating the package while leaving that undefined
   means every automated release ships one half of a description whose other
   half nobody is tracking.

   **Also worth stating plainly, because it is easy to assume otherwise:** there
   is **no screenshot machinery today**. `packaging/render_check.py` walks the
   AT-SPI accessible tree and asserts geometry; it captures no images. Automating
   screenshots later is real work, not a small extension of something existing.

   **So the decision is: do not automate listing metadata, and do not leave it
   unowned either.** Before this RFC is implemented:

   - the Store listing content gets a tracked home in the repository, as data,
     even while publishing it stays a manual step;
   - screenshots are committed assets, not promises to regenerate;
   - the manifest-versus-listing precedence is written down.

   That converts a debt into a deferred implementation: manual publication of
   version-controlled content is a step someone can automate later from a source
   that exists. Manual publication of content living only in Partner Center is
   the debt — unreviewable, undated, and with nothing to automate *from*.

## Dependencies

- Gate D sequencing (§8).
- **F60 — blocking.** The manifest's `MinVersion` claim is a live Store
  constraint, and automation would ship it unattended on every release, so it
  must be settled before this RFC is implemented rather than alongside it.
  Moved here from §9 Q3 on 2026-08-22 — see there for why it was never a
  question. F49b — the manifest's other version claims — rides with it.
- No dependency on RFC-078's outcome beyond timing; this changes no product
  behaviour and no artifact M5 tests.
