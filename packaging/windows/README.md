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
- Rust (MSVC toolchain), Git, and PowerShell 7;
- a clean checkout of this repository.

Run everything in **one PowerShell session**, from the repository root.

### 1. Check out the released tag, and name it once

```powershell
git fetch --tags
$tag = '0.192.0'          # the release you are submitting: the only value you type
git switch --detach $tag
$out = Join-Path $env:TEMP "forskscope-store-$tag"
```

**Why the tag and not `main`.** `AppxManifest.xml` on `main` already names the
*next* version, because the post-release bump moves it minutes after each cut.
Packaging from `main` would submit a version that was never released. The build
script refuses unless the checkout is exactly the tag.

### 2. Build the package

```powershell
./packaging/windows/store-build.ps1 -Tag $tag -OutDir $out
```

It checks that the checkout and `AppxManifest.xml` both match `$tag`, builds the
same binary the release zip uses, labelled as the Store build so *Check for
updates* sends users to the Store, and packs exactly four things:

- the program;
- the manifest;
- the `Assets\` images;
- nothing else.

It prints the package's path last: `$out\forskscope-$tag.msix`.

### 3. Check the package

```powershell
./packaging/windows/store-validate.ps1 -MsixPath "$out\forskscope-$tag.msix" -Tag $tag -SkipInstall
```

It checks that the package:

- has the right version;
- has ForskScope's registered Store identity;
- contains every file the manifest names.

It needs no administrator rights and changes nothing on your PC. It clears its
own working folder **before** it unpacks, so a file left from an earlier run
cannot hide a missing one.

**The install-and-launch check runs only in CI.** It would enable sideloading
for the whole machine, add a certificate to the machine store, and uninstall any
installed ForskScope, so the script refuses to run it on a PC unless you pass
`-AllowMachineChanges` deliberately.

### 4. Upload in Partner Center

Upload `$out\forskscope-$tag.msix` to the app's submission page, and submit.

- **Signing is not part of this.** The package's `Publisher` is a
  Store-assigned identity, and Microsoft signs the package itself (RFC-079 §2a).
- **If the workflow created a pending submission, do not edit it here.**
  Delete it first. A submission the API created can no longer be changed
  through the API once Partner Center edits it (`release.md`, *Do not open an
  automated submission in Partner Center*).

### 5. Clean up, and return to `main`

Once the upload is accepted:

```powershell
git switch main
Remove-Item -Recurse -Force $out
```

**Keep the package until the upload is accepted.** It is the file you would
upload again. Returning to `main` matters: a commit made on the detached tag
belongs to no branch and is easily lost.

---

## What the automated workflow does

`.github/workflows/store-submit.yml`, dispatched with the released tag:

1. **Checks out the tag** and runs `store-build.ps1`, as in step 2.
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
