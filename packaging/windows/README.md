# Windows Packaging — MSIX for the Microsoft Store

**Status: automated, by explicit dispatch, and not currently submitting.**
`.github/workflows/store-submit.yml` (RFC-079) builds, validates and submits the
MSIX when dispatched (`gh workflow run store-submit.yml -f tag=X.Y.Z -f
dry_run=false`).

- It has no `release:` trigger (F147/F158).
- **No submission has succeeded since 0.174.0:** it fails at the Partner Center
  step, which waits on the owner's account work (F106 (b)).
- So the Store listing is not updated with each release (F189).

Until then, a Store release is built here and uploaded by hand. The steps below
use the same scripts the workflow uses.

---

## Building and submitting by hand

**You need:**

- Windows 10 or 11, with the Windows SDK (for `makeappx.exe`);
- **PowerShell 7** (`pwsh`) — not *Windows PowerShell*, the older `powershell.exe`
  that ships with Windows itself. Every script here starts with `#Requires
  -Version 7.0` and refuses to run under it, with that message, before doing
  anything;
- Rust (MSVC toolchain) and Git;
- **a Git clone of this repository** — not a ZIP download. A ZIP marks the
  scripts it contains as downloaded from the internet, and PowerShell 7's
  default execution policy (`RemoteSigned`) then blocks them from running at
  all. A `git clone` carries no such mark. Do not change your execution policy
  to work around this — clone instead.

### 1. Fetch the released tag

```powershell
git fetch --tags
```

### 2. Build and check the package, in one command

```powershell
pwsh -File ./packaging/windows/store-package.ps1 -Tag 0.193.0
```

This never touches your checkout — it builds in a temporary `git worktree`,
detached at the tag, so your own branch, any uncommitted changes, and your
current folder are all untouched throughout. There is nothing to switch back to
afterwards.

- **If the tag is not fetched yet,** it stops and tells you to run `git fetch
  --tags` first — it never fetches on its own.
- It builds the same binary the release zip uses, labelled as the Store build
  so *Check for updates* sends users to the Store, and packs exactly four
  things: the program, the manifest, the `Assets\` images, and nothing else.
- It then validates the package — the right version, ForskScope's registered
  Store identity, every file the manifest names — with no administrator rights
  and no change to your PC (`-SkipInstall`; the install-and-launch check that
  needs those runs only in CI).
- It writes only to this repository's own `target/store/0.193.0/`, already
  ignored by Git, clearing that one folder first so a file left from an earlier
  run cannot hide a missing one. `cargo clean` removes it along with everything
  else under `target/`; there is nothing else to clean up by hand.
- It prints the package's path last:
  `<repo>\target\store\0.193.0\forskscope-0.193.0.msix`.

### 3. Upload in Partner Center

Upload the printed `.msix` to the app's submission page, and submit.

- **Signing is not part of this.** The package's `Publisher` is a
  Store-assigned identity, and Microsoft signs the package itself (RFC-079 §2a).
- **If the workflow created a pending submission, do not edit it here.**
  Delete it first. A submission the API created can no longer be changed
  through the API once Partner Center edits it (`release.md`, *Do not open an
  automated submission in Partner Center*).
- **Keep the package until the upload is accepted** — it is the file you would
  upload again.

---

## What the automated workflow does

`.github/workflows/store-submit.yml`, dispatched with the released tag:

1. **Checks out the tag** and runs `store-build.ps1` directly — the same
   script `store-package.ps1` runs inside its own temporary worktree above.
2. **Runs `store-validate.ps1`** with the full install-and-launch check. It signs
   a throwaway, validation-only copy, installs it, launches it, and tears it all
   down. The uploaded package stays unsigned.
3. **Submits through the Microsoft Store submission API:**
   - deletes any pending submission first, so a re-run replaces it rather than
     duplicating it;
   - replaces only the package, never the listing, pricing or images (those are
     published by hand, RFC-079 §9 Q5);
   - uploads, commits, and reports the status without waiting for
     certification.

**Rehearse** with `gh workflow run store-submit.yml -f tag=<released tag> -f
dry_run=true`, the default. It authenticates and reads the app's state, then
stops before creating anything.

**Re-submit a published release** (a packaging-only fix, or after a rejection)
with `-f dry_run=false`.

### Owner setup, once

- **A GitHub Environment named `store-publish`** (Settings → Environments). Add
  required reviewers there for a human check before every real submission.
- **Three secrets** in it: `STORE_TENANT_ID`, `STORE_CLIENT_ID` and
  `STORE_CLIENT_SECRET`, from the existing ForskScope Entra ID app registration
  (RFC-079 §9 Q4).
- **Record `STORE_CLIENT_SECRET`'s expiry** in
  `docs/src/maintainers/threat-model.md` when you create it. These secrets last
  24 months at most, and a lapsed one fails silently.

**The application ID is not a secret.** The Store ID `9P63F7NPC3MH` is public, so
it is tracked in `store-listing/en-us/identity.toml` (`store_id`), not set up.
*(A `STORE_APP_ID` secret, if one exists from before F105, is inert and can be
deleted.)*
