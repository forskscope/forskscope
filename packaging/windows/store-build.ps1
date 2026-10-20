#!/usr/bin/env pwsh
#Requires -Version 7.0
# RFC-079: build a Store-ready MSIX from the same commit and the same built
# binary the Windows zip already uses.
#
# This never writes AppxManifest.xml's Version - it only verifies it, the
# same "automation never writes a version component" rule RFC-081 §Q3
# established for the AUR path (see packaging/linux/aur-publish.sh). Every
# other release gate already keeps the manifest's Version in sync with the
# workspace version at commit time (cargo xtask version-sync), so by the
# time a tagged commit is checked out, its manifest is already correct or
# the release gates would have failed before the tag was ever pushed. This
# script's job is to catch the one case that check cannot: packaging from
# the wrong commit (F103 - main's post-release bump names the *next*
# version within minutes of the tag, well before anyone dispatches
# store-submit.yml for it).
#
# Usage: store-build.ps1 -Tag <released-tag> -OutDir <output-directory>
# Runs outside CI too: $env:GITHUB_OUTPUT is only written when it is set,
# and FORSKSCOPE_CHANNEL is restored to whatever it was before this script
# ran (or removed, if it was unset) once the build finishes.
#
# Handoff 085 (F207): finds the built binary through $env:CARGO_TARGET_DIR
# when it is set (store-package.ps1 points it at a shared build cache for
# its worktree build), falling back to this repo's own "target" the way
# this script and CI always have when it is not.

param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = "Stop"

$RepoRoot = git rev-parse --show-toplevel

# ── Refuse unless HEAD is exactly the tag. Stronger than the manifest
#    check below alone, which a commit between the tag and the post-release
#    bump would also pass (its manifest still reads the old version). ────
$CheckedOutTag = git describe --tags --exact-match 2>$null
if ($LASTEXITCODE -ne 0 -or $CheckedOutTag -ne $Tag) {
    Write-Host "::error::HEAD is not exactly tag $Tag (git describe --tags --exact-match returned '$CheckedOutTag') - checked out the wrong commit"
    exit 1
}
Write-Host "HEAD is exactly tag $Tag"

$ManifestPath = Join-Path $RepoRoot "packaging/windows/AppxManifest.xml"
$ExpectedVersion = "$Tag.0"

# XmlDocument.Load, not Get-Content -Raw cast to [xml]: it reads the file's
# own bytes and honours the XML declaration's encoding itself, rather than
# PowerShell's own text-encoding guess for Get-Content - Windows PowerShell
# 5.1's guess is the host's ANSI code page (CP932 on Japanese Windows) for a
# UTF-8 file with no BOM, which this manifest is (its Description has
# emoji). That guess turned one emoji's bytes into a stray quote and broke
# the parse (F207) - this reads correctly regardless of the host's locale.
$Manifest = New-Object System.Xml.XmlDocument
$Manifest.Load($ManifestPath)
$ActualVersion = $Manifest.Package.Identity.Version

if ($ActualVersion -ne $ExpectedVersion) {
    Write-Host "::error::AppxManifest.xml Version ($ActualVersion) does not match the released tag ($ExpectedVersion) - checked out the wrong commit, or a release gate regressed"
    exit 1
}
Write-Host "AppxManifest.xml Version matches the released tag: $ActualVersion"

# ── Build the same binary the Windows zip release artifact uses. ──────────
# F180: this build is updated by the Store, not by the app's own Check for
# updates button - FORSKSCOPE_CHANNEL tells the About dialog to say so
# ("Update through the Microsoft Store") instead of linking a GitHub
# download.
$PreviousChannel = $env:FORSKSCOPE_CHANNEL
Push-Location $RepoRoot
try {
    $env:FORSKSCOPE_CHANNEL = "store"
    cargo build --release --locked -p forskscope-ui
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    # Restore the caller's shell exactly as found - without this, a later
    # `cargo build` in the same session (CI or a PC) would keep picking up
    # FORSKSCOPE_CHANNEL=store.
    if ($null -eq $PreviousChannel) {
        Remove-Item Env:FORSKSCOPE_CHANNEL -ErrorAction SilentlyContinue
    } else {
        $env:FORSKSCOPE_CHANNEL = $PreviousChannel
    }
    Pop-Location
}

# ── RFC-091 §4 / F206: the Store package must keep the same hybrid-CRT
#    promise the zip does - checked here, on this exact binary, before it
#    is staged and packed, not left to a separate, easily-forgotten step.
#    Run as a real child process (`pwsh -File`, not `&` in this process):
#    check-imports.ps1's own `exit` on failure would otherwise end this
#    whole non-interactive pwsh process immediately, skipping the `throw`
#    below (and any future cleanup added after it) rather than letting
#    this script observe and report the failure on its own terms. ────────
# Handoff 085: `cargo build` above already honours $env:CARGO_TARGET_DIR
# when it is set (store-package.ps1's worktree build points it at a shared
# cache) - find the binary the same way, rather than assuming $RepoRoot's
# own "target", which would be empty for that build.
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $RepoRoot "target" }
$BuiltExe = Join-Path $TargetDir "release/forskscope.exe"
& pwsh -NoProfile -File (Join-Path $RepoRoot "packaging/windows/check-imports.ps1") -ExePath $BuiltExe
if ($LASTEXITCODE -ne 0) { throw "import gate failed - see above" }

# ── Stage exactly four things: executable, manifest, tile assets. Nothing
#    else - packaging/windows/README.md and store-*.ps1 must never end up
#    inside the package (the same trap this script's own manual-path
#    ancestor, packaging/windows/README.md, documents and fixed). ─────────
$Stage = Join-Path $OutDir "stage"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Path $Stage | Out-Null

Copy-Item $BuiltExe (Join-Path $Stage "forskscope.exe")
Copy-Item $ManifestPath (Join-Path $Stage "AppxManifest.xml")
Copy-Item (Join-Path $RepoRoot "packaging/windows/Assets") (Join-Path $Stage "Assets") -Recurse

# ── Pack. makeappx.exe's path is versioned by the installed Windows SDK -
#    discover it rather than hard-coding a version that will break when the
#    runner image updates its SDK. ─────────────────────────────────────────
$MakeAppx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction Stop |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $MakeAppx) {
    Write-Host "::error::makeappx.exe not found under the Windows Kits SDK path"
    exit 1
}

$MsixPath = Join-Path $OutDir "forskscope-$Tag.msix"
& $MakeAppx.FullName pack /d $Stage /p $MsixPath /overwrite
if ($LASTEXITCODE -ne 0) { throw "makeappx pack failed" }

# GITHUB_OUTPUT is unset outside CI - writing to it there would error after
# the MSIX is already built, so only write it when it exists. Either way,
# the path is always the last line of output, for a human or a workflow
# step to read.
if ($env:GITHUB_OUTPUT) {
    Add-Content -Path $env:GITHUB_OUTPUT -Value "msix_path=$MsixPath"
}
Write-Host $MsixPath
