# Review 103 — Request 100: RFC-079 Store submission automation

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `b488d24`
(and `024d4c0`, `c56ecdd`, `b47f97a`, `344343c`).
**Verdict:** **Approved.** **RFC-079 stays in `accepted/`** — the gap here is
deeper than RFC-081's, see §5.

## 1. Your deviation from §1 is better than what §1 said

RFC-079 §1 says *"stamp `AppxManifest.xml`'s `Version` from the workspace
version."* You **verify** it instead, extending RFC-081 Q3's rule that automation
never writes a version component.

**You are right and the RFC's wording was wrong.** Stamping would *overwrite*
whatever the tagged commit carried — which means a `version-sync` regression at
cut time would be silently papered over by the packaging step. Verifying surfaces
it. Your error message says so directly:

> `does not match the released tag ... - checked out the wrong commit, or a
> release gate regressed`

That second clause is the point. Confirmed the script only compares
(`store-build.ps1:5,33`) and never writes.

## 2. Three bugs that only real Windows could find

None was reachable from this sandbox — no `pwsh`, no Windows — and you found
each by dispatching, reading the runner's output, fixing, re-dispatching:

- bare relative script paths not resolving in `pwsh` as they do in `bash`;
- a leading `&` in a YAML plain scalar being an **anchor marker**, not a literal
  (caught by `actionlint` before any dispatch — the one this project's own
  tooling caught);
- **an unset GitHub secret arriving as an env var that exists and is empty**,
  not one that is absent.

The third is the one worth keeping. Your original guard checked absence, so an
empty tenant ID reached a real HTTP request and produced *"the most likely cause
is that it has expired"* for a condition with nothing to do with expiry. **A
misleading diagnostic is worse than a bare failure**, and it would have sent the
owner to Partner Center to check a secret that was never configured.

## 3. The expensive check — done, not narrated

My handoff quoted the RFC at you: *"the one most likely to be quietly dropped …
do not report it as done if the runner merely unpacked the package."*

You signed a **separate, validation-only copy** with a throwaway certificate,
trusted it and enabled sideloading on the runner only, installed it, and
confirmed the process was running — twice, with **different PIDs on different
runners** (2436, 404). That last detail is what makes it evidence rather than a
claim: a cached or reused result could not produce two PIDs. And the real
unsigned `.msix` that gets uploaded is never touched.

## 4. The bootstrapping problem, correctly diagnosed

Your first dispatch targeted the real `0.170.1` tag — the right instinct, and the
same shape that gave RFC-081 a genuine falsification. It failed with what looked
like Bug 1 again, and you established with a temporary debug step that
`store-build.ps1` **does not exist in that tag's tree**.

So no currently-tagged release can rehearse this workflow, and the first release
cut after this lands is the first that can. That is a real property of the change,
you found it rather than assuming, and you used scratch tags thereafter — never a
real release tag, never `main`.

## 5. Why RFC-079 stays in `accepted/`

Same reasoning as review 102, and **the gap is larger**.

RFC-081's rehearsal proved a real read against the real service, because the AUR
is anonymously readable. **Partner Center is not**, so authentication, the
application read, delete-then-create re-run logic, blob upload, commit, and
status poll are verified only against Microsoft's published API shape — never
against a live response.

You said it plainly: *"the first time any of `store-submit.ps1`'s
post-authentication code runs at all will be the owner's first real dispatch."*

> **Amended 2026-09-08, after review 104.** That was true when written and is
> not any more. F105's rehearsal (`34226616867`) authenticated against Entra ID
> **for real** and made one real post-authentication call to Partner Center,
> which returned a genuine `Unauthorized` — the documented symptom of an app
> registration that is not yet a member of the Partner Center account. The dev
> team reported the correction themselves rather than letting the stale claim
> stand in an approved document.
Recording this RFC as `done/` on that basis would be exactly the defect this
register keeps cataloguing. It moves when a real submission succeeds.

**And you surfaced an asymmetry I would have missed:** unlike `aur-publish.yml`,
a Store *dry run still needs the credential*, because there is no anonymous read
to prove connectivity with. That is in the workflow's own header rather than left
for a reader to discover.

## 6. Two things for the owner rather than for you

**The screenshots are Linux captures.** I looked at them: real application, real
diff, and the window chrome is generic enough to pass as Windows. Honest content,
disclosed in `listing.toml` and the README with a *Replace* instruction. **Not
blocking** — but font rendering differs between WebView2 on Windows and WebKitGTK
here, so they are worth retaking in a Windows session before they represent the
product long-term.

**Five fix-forward commits on `main`, one of them `debug:`.** Workflow files can
only be exercised where the runner reads them, so iterating on the default branch
is largely forced here — and you removed the diagnostic in the next commit. Noted
rather than criticised, and worth knowing it is visible in history.

## 7. Scope and gates

`ROADMAP.md` and `rfcs/accepted/079-*.md` left for me, as established.
`release.yml` untouched. Scratch branch and both scratch tags deleted from local
and `origin`, with `main` confirmed clean before and after each dispatch.
`actionlint` clean; all standard gates green and unchanged, since this touches no
Rust.
