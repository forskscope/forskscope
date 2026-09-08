# Windows Packaging — MSIX for the Microsoft Store

**Status: automated.** `.github/workflows/store-submit.yml` (RFC-079) builds,
validates, and submits the MSIX on every published release. What follows is
the manual equivalent — useful for understanding what the workflow does, for
rehearsing with `workflow_dispatch` + `dry_run: true`, or as a fallback if
the workflow is broken. **Three of the steps below exist to avoid mistakes
this file previously invited**, and the workflow avoids the same three by
construction (`packaging/windows/store-build.ps1` checks out the tag the
same way "Before you start" says to below, and stages only the four files
step 2 names — it never packs the whole `packaging/windows/` directory).

---

## Before you start: build from the tag, not from `main`

**`AppxManifest.xml` in the working tree names the *next* version, not the
released one.** The post-release bump moves it immediately after each cut, so
between releases it is always one ahead. At the time of writing, `main` declares
`0.170.1` while the newest release is `0.170.0`.

Packaging from `main` therefore submits a version to the Store that **does not
exist and was never released**. This is the same structural trap F81 records for
`PKGBUILD`'s `pkgver`, in the Windows path.

```powershell
git fetch --tags
git checkout 0.170.0          # the tag you are publishing, not main
```

**Verify before building** — this is the whole point of the step:

```powershell
Select-String -Path packaging\windows\AppxManifest.xml -Pattern 'Version="'
# must read Version="<tag>.0" — e.g. 0.170.0.0 for tag 0.170.0
```

If it does not match the tag, stop. Nothing below fixes it.

## 1. Build

```powershell
cargo build --release --locked -p forskscope-ui
```

`--locked` so the build cannot silently resolve different dependencies than the
release did.

## 2. Stage the package contents

**Do not copy the executable into `packaging\windows\`.** `makeappx pack /d`
packs *everything* in the directory it is given, so doing that ships this README
and `build-zip.sh` inside the product — and leaves an untracked binary in a
tracked directory (`packaging/windows/forskscope.exe` is not in `.gitignore`).

Stage instead:

```powershell
$stage = "$env:TEMP\forskscope-msix"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage | Out-Null

Copy-Item target\release\forskscope.exe        $stage\
Copy-Item packaging\windows\AppxManifest.xml   $stage\
Copy-Item packaging\windows\Assets             $stage\ -Recurse
```

Exactly four things belong in the package: the executable, the manifest, and the
`Assets\` tile images. Nothing else.

## 3. Pack

`makeappx.exe` lives under the Windows SDK, whose version is part of the path.
Do not hard-code it — find it, so this does not break when the SDK updates:

```powershell
$makeappx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" |
            Sort-Object FullName -Descending | Select-Object -First 1
& $makeappx.FullName pack /d $stage /p "$env:TEMP\forskscope-$(git describe --tags --abbrev=0).msix"
```

## 4. Verify the package before submitting

```powershell
& $makeappx.FullName unpack /p "$env:TEMP\forskscope-<version>.msix" /d "$env:TEMP\forskscope-msix-check"
Get-ChildItem -Recurse "$env:TEMP\forskscope-msix-check" | Select-Object FullName
```

Confirm: the executable is present, the manifest's `Version` matches the tag,
`Assets\` is present, and **`README.md` and `build-zip.sh` are not** — if they
are, step 2 was skipped and `/d` was pointed at `packaging\windows\`.

## 5. Submit to Partner Center

Manual, in the browser. Upload the `.msix` to the app's submission page and
publish.

**Signing is not part of this.** `Publisher="CN=C4BA37E8-8670-4C82-8365-5ECB57373921"`
is a Store-assigned publisher identity — Microsoft signs the package on
submission, and no code-signing certificate of this project's own is involved
(RFC-079 §2a).

## What the automated workflow does instead

`.github/workflows/store-submit.yml`, triggered by `release: published`:

1. Checks out the released tag (not `main` — see "Before you start" above)
   and runs `store-build.ps1`, which does exactly steps 1–3 above.
2. Runs `store-validate.ps1`: manifest version against the tag,
   `Identity`/`Publisher`/`PublisherDisplayName` against
   `store-listing/en-us/identity.toml`, every asset the manifest
   references is present, **and the package actually installs and
   launches** — signed with a throwaway validation-only certificate
   generated on the runner, never the real (unsigned) upload.
3. Submits through the Microsoft Store submission API: deletes any existing
   pending submission for this app first (so a re-run replaces rather than
   duplicates), creates a new submission, replaces only its
   `applicationPackages` (never listings, pricing, or images —
   `store-listing/` content is published by hand, RFC-079 §9 Q5), uploads
   the package, commits, and polls status without waiting for
   certification to finish.

**Rehearse it** with `gh workflow run store-submit.yml -f tag=<a released tag>
-f dry_run=true` (the default). This authenticates against Partner Center and
reads the application resource — proving the credential and connectivity —
and stops there. It shares every line of code with the real path up to that
point; only the create/upload/commit sequence after it is skipped.

**Re-submit an already-published release** (a packaging-only fix, or
recovering from a rejected submission) with `-f dry_run=false`. Automation
never re-triggers on its own; this is the same recovery path RFC-079 §5
documents, run by hand.

Owner setup, once: a GitHub **Environment** named `store-publish`
(Settings → Environments — add required reviewers there if you want a human
check before every real submission), holding four secrets:
`STORE_TENANT_ID`, `STORE_CLIENT_ID`, `STORE_CLIENT_SECRET` (the Entra ID app
registration's tenant, client, and client secret — RFC-079 §9 Q4: the
existing ForskScope registration, not a new one), and `STORE_APP_ID` (the
Partner Center application ID, distinct from the public Store product ID in
`installation.md`'s link). **Record `STORE_CLIENT_SECRET`'s expiry** in
`docs/src/maintainers/threat-model.md` when you create it — Entra ID secrets
last 24 months at most, often less, and a lapsed one breaks silently
otherwise.
