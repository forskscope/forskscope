# Review Request 100 — RFC-079: Microsoft Store Submission Automation

Handoff: `rfcs/handoffs/079-microsoft-store-submission-automation/030-rfc079-store-submission-automation.md`
Commits: `024d4c0` (feat), `c56ecdd`, `91be953`, `344343c`, `b47f97a`, `b488d24`
(five fix-forward commits, all found by live rehearsal on `windows-latest` —
none of them was findable locally, since this sandbox has no Windows). CI
green on the final commit (`b488d24`, run `34208531709`).

## Falsifications, run for real

### 1 — three real bugs, found only because the rehearsal ran on real Windows CI

None of these were guessable from reading the PowerShell — this sandbox has
no `pwsh` and no Windows, so every one of them was a live dispatch failure,
diagnosed from the actual runner's output, fixed, and re-dispatched.

**Bug 1 — bare relative script paths don't resolve in `pwsh` the way they do
in `bash`.** First dispatch (run `34205834469`, tag `0.170.1`):

```
The term './packaging/windows/store-build.ps1' is not recognized as a name of a cmdlet, function, script file,
or executable program. Check the spelling of the name, or if a path was included, verify that the path is
correct and try again.
```

Fixed by adding the call operator (`& "./packaging/windows/store-build.ps1"
...`) in `c56ecdd`.

**Bug 2 — a bare `&` at the start of a YAML plain scalar is an anchor
marker, not a literal character.** `actionlint` caught this locally before
it ever reached a dispatch:

```
.github/workflows/store-submit.yml:86:13: could not parse as YAML: did not find expected alphabetic or numeric character [syntax-check]
```

Fixed by wrapping the two single-line `run:` steps in `run: |` block style
in `b47f97a` — this is the one bug this project's own static tooling did
catch, unlike the other two.

**Bug 3 — an unset GitHub Actions secret reaches the runner as an env var
that *exists* with an empty value, not one that is *absent*.** My original
guard (`Get-Item "env:$name"`) only catches the absent case. Run
`34207844616` (the first fully successful `Build and validate`, see below)
hit this in the `publish` job:

```
authentication to the Microsoft Store submission API failed. If STORE_CLIENT_SECRET was not recently rotated,
the most likely cause is that it has expired... Underlying error: Response status code does not indicate
success: 404 (Not Found).
```

This sandbox has none of `STORE_TENANT_ID`/`STORE_CLIENT_ID`/
`STORE_CLIENT_SECRET`/`STORE_APP_ID` configured — exactly the case the
guard exists to catch cleanly — but the guard let an empty tenant ID reach a
real HTTP request instead, producing a misleading "check the expiry"
message for a condition that has nothing to do with expiry. Fixed in
`b488d24` by checking `[string]::IsNullOrEmpty(...)` instead. Re-dispatched
(run `34209598183`) and confirmed the clean message:

```
STORE_TENANT_ID is not set - see docs/src/maintainers/release.md for what the store-publish Environment must contain
```

### 2 — the bootstrapping problem, and why it isn't a fourth bug

The very first dispatch targeted a real, currently-published tag
(`0.170.1`) — not a synthetic one — expecting the same shape of genuine
falsification RFC-081's review 102 got from dispatching against `main`'s
real state. It failed instead with the same "not recognized" error as Bug
1, which briefly looked like a second instance of the same bug. It was not:
`0.170.1` predates this handoff, so `store-build.ps1` simply does not exist
in that tag's tree — confirmed with a temporary debug step (`91be953`,
removed in `344343c` once its answer was in) that listed
`packaging/windows/` at that checkout and showed `Assets`,
`AppxManifest.xml`, `build-zip.sh`, `README.md` — no `store-*.ps1` files.

**This means no real, currently-tagged release can rehearse this workflow
today**, unlike RFC-081 where AUR's real state produced a genuine
falsification immediately. The first real release cut *after* this lands
will carry these scripts in its own tag, and rehearsing against it will
work the way `0.170.1` could not. Until then, rehearsal needs a scratch
tag — which is what the remaining falsifications use, disclosed as such
throughout, never against a real release tag or `main`.

### 3 — F103's version guard, falsified for real

A scratch branch (`rfc-079-rehearsal`, off `main`, `AppxManifest.xml`
`Version` set to `0.0.1.0` only there) was tagged `0.0.1`. A second tag,
`0.0.2`, was pointed at the **same commit** — so checkout succeeds, but the
tag name given to `store-build.ps1` deliberately disagrees with what the
checked-out manifest actually says. Run `34210473834`, failed in 13s
(before the expensive build):

```
AppxManifest.xml Version (0.0.1.0) does not match the released tag (0.0.2.0) - checked out the wrong commit, or a release gate regressed
```

### 4 — the identity check, falsified for real

`store-listing/en-us/identity.toml`'s `publisher_display_name` was
temporarily changed to `wrong-owner` on the rehearsal branch (tag `0.0.1`
moved to the new commit). Run `34210601221`: the build succeeded, and
validation failed exactly where expected — after the version check, before
the expensive install+launch check:

```
OK: manifest Version matches the released tag (0.0.1.0)
OK: Identity Name matches the tracked Store identity
OK: Identity Publisher matches the tracked Store identity
##[error]PublisherDisplayName (nabbisen) does not match the tracked Store identity (wrong-owner)
OK: package contains forskscope.exe
...
```

(The four checks after the failure still ran and reported `OK` — the
script collects every check before exiting, matching `store-validate.ps1`'s
own design of reporting all failures at once rather than stopping at the
first. Only the fourth, expensive check is skipped once any of the first
three fail — confirmed by its absence from this run's log.)

### 5 — the expensive check, done for real, twice

Two full, unmodified rehearsal runs (`34207844616` and `34209598183`, the
second after Bug 3's fix) both built the real MSIX, unpacked and validated
it, signed a **separate, validation-only copy** with a throwaway
self-signed certificate, trusted that certificate and enabled sideloading
on the runner only, installed the signed copy, and confirmed the process
was actually running:

```
OK: package installed
OK: forskscope.exe is running (PID 2436) after installing and launching the signed validation copy
```

and, on the second run:

```
OK: forskscope.exe is running (PID 404) after installing and launching the signed validation copy
```

Two different PIDs on two different runners — not a cached or reused
result. The real (unsigned) `.msix` that gets uploaded is never touched by
the signing step; only a copy is.

## Cleanup

The scratch branch (`rfc-079-rehearsal`) and both scratch tags (`0.0.1`,
`0.0.2`) were deleted, locally and on `origin`, after the falsifications
above. `main`'s `AppxManifest.xml` and `store-listing/` were confirmed
unchanged before and after every dispatch (`git status`/`git diff`,
checked repeatedly through the rehearsal cycle, not only at the end).

## Design decisions

**The version check verifies, never writes** — extending RFC-081 Q3's rule
("automation never writes a version component") to this RFC, even though
§1's design text here says "stamp." By the time a tagged commit is checked
out, `cargo xtask version-sync` has already required its manifest `Version`
to be correct at commit time; `store-build.ps1` only confirms that held
(catching F103's actual failure mode — packaging from the wrong commit —
not a value it would otherwise need to write). This is a judgment call
extending an established project precedent to a new RFC whose own text
used different wording; if that reading is wrong, the fix is one `sed`
replacing a comparison with an assignment.

**Two-job split, `store-publish` environment.** Same shape as RFC-081's
`aur-publish`: `build_and_validate` has no `environment:` and runs
unconditionally; `publish` alone references `environment: store-publish`
and is the only place `secrets.STORE_*` is ever read. **Unlike AUR, this
does not make a dry run credential-free.** Partner Center has no anonymous
read the way the AUR's git remote does, so even proving connectivity
(RFC-079's own suggested dry-run mitigation) requires authenticating. The
`store-publish` environment therefore gates the dry-run and real paths
equally — disclosed explicitly in the workflow's own header comment, not
left for a reader to notice.

**Re-run safety is delete-then-create, not check-then-fail.** RFC-079 §5
allows either "replaces the pending submission" or "fails clearly."
`store-submit.ps1` deletes any existing `pendingApplicationSubmission`
before creating a new one, so a second run always leaves exactly one
submission in flight, never two, never zero. This could not be exercised
against the real API (no credential), so it is verified against Microsoft's
own documented submission-object shape (`pendingApplicationSubmission.id`,
`DELETE .../submissions/{id}`) rather than live behavior — see "what could
not be tested" below.

**Automation never touches listings, pricing, or images.** `POST
.../submissions` clones the last published submission's full JSON;
`store-submit.ps1` modifies only `applicationPackages` before `PUT`-ing it
back. `store-listing/` exists so a human has a reviewable source to copy
from when updating the Store listing by hand (RFC-079 §9 Q5) — it is never
read by any script.

**Manifest-versus-listing precedence** (Part A's third requirement):
`store-listing/en-us/listing.toml` is the source; `AppxManifest.xml`'s
`Description` is a derivative of `listing.toml`'s `short_description`, kept
in sync by review discipline (nothing enforces this mechanically — the
manifest schema takes literal text, not a pointer). Written down in
`store-listing/README.md`.

## Part A: screenshots

Real screenshots of the actual running application (F102/F81-adjacent
precedent: this project does not fabricate evidence), captured on this
sandbox's Linux/Wayland desktop via `niri msg action screenshot-window`
against a freshly built release binary — **not native Windows captures**,
disclosed in both `listing.toml` and `store-listing/README.md`. The
rendered UI is the same Dioxus/WebView surface the Windows build ships (same
RSX, same CSS), but this environment has no Windows GUI to capture from
directly. Both screenshots (2569×1426) clear the Store's documented minimum
(1366×768).

## What could not be tested

**Any real contact with Partner Center at all** — not authentication, not
the application read, not a submission. This sandbox has none of
`STORE_TENANT_ID`/`STORE_CLIENT_ID`/`STORE_CLIENT_SECRET`/`STORE_APP_ID`
configured and was told not to acquire them. This is a deeper gap than
RFC-081 left: the AUR is anonymously readable over HTTPS, so that
rehearsal proved a real read against the real service; Partner Center has
no equivalent, so `store-submit.ps1`'s authentication call, its
`pendingApplicationSubmission` read, the delete-then-create re-run logic,
the blob upload, the commit, and the status poll are all verified only
against Microsoft's published API documentation (fetched and cross-checked
during this handoff, not recalled from training data) and the shape of the
one real failure this sandbox *could* produce (an auth 404 against an empty
tenant ID) — never against a live response. **The first time any of
`store-submit.ps1`'s post-authentication code runs at all will be the
owner's first real dispatch after the Environment and secrets exist.**

Same disclosure shape as reviews 093, 101, and 102's AUR-push gap — stated
plainly, not quietly narrowed to "submission is tested."

## What the owner must create

1. A GitHub **Environment** named `store-publish` (Settings → Environments
   — add required reviewers there for a human check before every real
   submission; the workflow only references the environment by name and
   defers entirely to whatever rules are configured on it).
2. Four secrets inside that environment:
   - `STORE_TENANT_ID`, `STORE_CLIENT_ID`, `STORE_CLIENT_SECRET` — the
     existing ForskScope Entra ID app registration's tenant, client, and
     client secret (RFC-079 §9 Q4, closed 2026-09-08: the existing
     registration, not a new one).
   - `STORE_APP_ID` — the Partner Center **application ID**, which is a
     different value from the public Store product ID already linked in
     `installation.md` (`9p63f7npc3mh`).
3. **Record `STORE_CLIENT_SECRET`'s expiry** in
   `docs/src/maintainers/threat-model.md` §6, where a placeholder line
   (`<owner fills in when the secret is created>`) is waiting for it —
   nothing in this repository can supply that date; it is known only inside
   Partner Center at creation time. Entra ID client secrets expire at 24
   months at the most, often less, and a lapsed one breaks submission
   silently (`store-submit.ps1` reports the auth failure with expiry named
   as a likely cause, but that is a loud failure *at* expiry, not a warning
   *before* it — recording the real date is the only way to get the latter).

## Scope

In: `.github/workflows/store-submit.yml`, `packaging/windows/store-build.ps1`,
`store-validate.ps1`, `store-submit.ps1`, `packaging/windows/store-listing/`
(listing content, identity record, two screenshots), `release.md`,
`installation.md`, `threat-model.md` §6, `packaging/windows/README.md`.
Out, untouched: RFC-081's AUR automation (separate credential, separate
workflow, not generalized together); listing-metadata *submission*, which
Q5 keeps manual by decision; `ROADMAP.md`; `rfcs/accepted/079-*.md` — both
left for the architect.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core + 29 diff-corpus + 16
merge-corpus + 5 patch-apply + 133 ui-lib + 133 ui-bin + 204 ui-logic + 5
css-coverage + 6 core doctests — unchanged by this handoff, it touches no
Rust code), `cargo xtask css --check`, `version-sync` (v0.170.2), `i18n`
(246 keys), `rfc-sync` (15 RFCs), `cargo xtask audit-deps`, `git diff
--check`, `mdbook build docs` — all clean. `actionlint` (v1.7.12, matching
`ci.yml`'s pin, `shellcheck` 0.11.0 confirmed present) — clean over the new
workflow, no-args over the whole `.github/workflows/` tree (it also caught
Bug 2 above, before any dispatch). CI run `34208531709` for the final
commit (`b488d24`) confirmed green. Six live dispatches against
`windows-latest` (`34205834469` through `34210601221`) exercised the real
build/validate pipeline, found and confirmed three real fixes, and
falsified two of validate's four checks for real; the scratch branch and
both scratch tags used for them are deleted.
