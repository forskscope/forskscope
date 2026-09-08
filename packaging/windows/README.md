# Windows Packaging — MSIX for the Microsoft Store

**Status: manual.** Automating this is RFC-079, accepted and not implemented.
Until it is, these are the steps, and **three of them exist to avoid mistakes
this file previously invited.**

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

> **This section is deliberately thin, and that is a gap, not a style choice.**
> The Partner Center steps live only in the maintainer's head and in the browser
> UI. RFC-079 §6 requires them written down; until they are, a second person
> cannot publish a release. **If you are the maintainer reading this after
> performing a submission, write what you actually did here.**

## What automating this would take

RFC-079 (accepted, `rfcs/accepted/079-*.md`) submits the package through the
Partner Center API from CI. It is **not blocked on engineering** — one question
is open, and it needs the owner:

- **Does the existing Entra ID app registration already carry Partner Center
  permissions, or is a separate registration preferable** so publishing rights
  are isolated? (RFC-079 §9 Q4.)
- **Whichever is chosen, record the client secret's expiry.** Entra ID secrets
  last 24 months at most and often less; a lapsed one breaks releases silently,
  at whatever moment it happens.

RFC-079 §9 Q5 also requires, *before* implementation: the Store listing content
gets a tracked home in this repository as data, screenshots become committed
assets, and manifest-versus-listing precedence is written down. That is not
automation — it is making the manual thing reviewable first.
