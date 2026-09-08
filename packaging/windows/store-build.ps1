#!/usr/bin/env pwsh
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
# version within minutes of the tag, well before the owner publishes the
# draft release this workflow triggers on).
#
# Usage: store-build.ps1 -Tag <released-tag> -OutDir <output-directory>

param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = "Stop"

$RepoRoot = git rev-parse --show-toplevel
$ManifestPath = Join-Path $RepoRoot "packaging/windows/AppxManifest.xml"
$ExpectedVersion = "$Tag.0"

[xml]$Manifest = Get-Content -Raw $ManifestPath
$ActualVersion = $Manifest.Package.Identity.Version

if ($ActualVersion -ne $ExpectedVersion) {
    Write-Host "::error::AppxManifest.xml Version ($ActualVersion) does not match the released tag ($ExpectedVersion) - checked out the wrong commit, or a release gate regressed"
    exit 1
}
Write-Host "AppxManifest.xml Version matches the released tag: $ActualVersion"

# ── Build the same binary the Windows zip release artifact uses. ──────────
Push-Location $RepoRoot
try {
    cargo build --release --locked -p forskscope-ui
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

# ── Stage exactly four things: executable, manifest, tile assets. Nothing
#    else - packaging/windows/README.md and store-*.ps1 must never end up
#    inside the package (the same trap this script's own manual-path
#    ancestor, packaging/windows/README.md, documents and fixed). ─────────
$Stage = Join-Path $OutDir "stage"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Path $Stage | Out-Null

Copy-Item (Join-Path $RepoRoot "target/release/forskscope.exe") (Join-Path $Stage "forskscope.exe")
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

Write-Host "Built $MsixPath"
Add-Content -Path $env:GITHUB_OUTPUT -Value "msix_path=$MsixPath"
