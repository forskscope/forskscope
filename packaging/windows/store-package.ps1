#!/usr/bin/env pwsh
#Requires -Version 7.0
# Handoff 085 (F207): one command for the manual Store-packaging path -
# build and validate a released tag's MSIX without ever touching the
# caller's own checkout. Replaces the hand steps `packaging/windows/
# README.md` used to walk through (`git switch --detach`, the build,
# `git switch main`, `Remove-Item -Recurse -Force`), each a chance to lose
# work or delete the wrong folder.
#
# Usage: store-package.ps1 -Tag <released-tag>
#
# What this does, and does not do:
# - Builds in a temporary `git worktree`, detached at the tag - the
#   caller's own branch, uncommitted changes and current directory are
#   never touched, so there is nothing to "return to" afterwards.
# - Refuses if the tag is not already fetched locally ("Run `git fetch
#   --tags` first") - it never fetches on its own.
# - Writes only under this repo's own `target/store/<tag>/`, already
#   gitignored, clearing only that one folder first. `cargo clean` removes
#   it, along with everything else under `target/`; there is nothing else
#   to clean up here afterwards.
# - Reuses `target/store-build` as a shared Cargo build cache across runs,
#   via `$env:CARGO_TARGET_DIR` - `store-build.ps1` finds the binary
#   through it when it is set (handoff 085). This is effective starting
#   with the tag that first carries that change (0.194.0): an older tag's
#   own `store-build.ps1` still hard-codes `target/release` relative to
#   the worktree, and would fail cleanly there (a "file not found" at its
#   own `Copy-Item`, not a silently wrong package) if pointed at a shared
#   cache it does not know to look in.
# - Ends by running `store-validate.ps1 -SkipInstall` on the package it
#   just built - the checks that need no administrator rights and change
#   nothing on this machine (see that script's own header for why the
#   install+launch check is CI-only).
#
# CI keeps calling store-build.ps1 and store-validate.ps1 directly
# (`.github/workflows/store-submit.yml`) - this script is for the manual
# path only.

param(
    [Parameter(Mandatory = $true)][string]$Tag
)

$ErrorActionPreference = "Stop"

$RepoRoot = git rev-parse --show-toplevel

# ── Refuse to fetch. The tag must already be local. ────────────────────────
$Resolved = git rev-parse -q --verify "refs/tags/$Tag" 2>$null
if ($LASTEXITCODE -ne 0 -or -not $Resolved) {
    Write-Host "::error::tag $Tag was not found locally. Run 'git fetch --tags' first."
    exit 1
}
Write-Host "Found tag $Tag locally ($Resolved)."

# ── Build (and validate) in a throwaway worktree, detached at the tag - the
#    caller's own checkout (branch, uncommitted files, current directory)
#    is never touched, so there is nothing to switch back from. A prior run
#    that crashed before cleanup can leave both the directory and git's own
#    record of the worktree behind - prune the record, then remove any
#    leftover directory, before asking git to create a fresh one. ─────────
$WorktreeDir = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-store-package-$Tag"
git worktree prune 2>$null | Out-Null
if (Test-Path $WorktreeDir) { Remove-Item -Recurse -Force $WorktreeDir }

# Own the output folder - clear only this one, never anything the caller
# typed. target/ is already ignored; cargo clean removes it.
$OutDir = Join-Path $RepoRoot "target/store/$Tag"
$CacheDir = Join-Path $RepoRoot "target/store-build"
$MsixPath = Join-Path $OutDir "forskscope-$Tag.msix"

git worktree add --detach $WorktreeDir $Tag
if ($LASTEXITCODE -ne 0) { throw "git worktree add failed" }
Write-Host "Worktree: $WorktreeDir (detached at $Tag)"

$PreviousCargoTargetDir = $env:CARGO_TARGET_DIR
try {
    if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

    $env:CARGO_TARGET_DIR = $CacheDir

    Push-Location $WorktreeDir
    try {
        & pwsh -NoProfile -File (Join-Path $WorktreeDir "packaging/windows/store-build.ps1") -Tag $Tag -OutDir $OutDir
        if ($LASTEXITCODE -ne 0) { throw "store-build.ps1 failed - see above" }

        Write-Host ""
        Write-Host "Validating..."
        & pwsh -NoProfile -File (Join-Path $WorktreeDir "packaging/windows/store-validate.ps1") -MsixPath $MsixPath -Tag $Tag -SkipInstall
        if ($LASTEXITCODE -ne 0) { throw "store-validate.ps1 failed - see above" }
    } finally {
        Pop-Location
    }
} finally {
    # Restore the caller's shell exactly as found, same reasoning as
    # store-build.ps1's own FORSKSCOPE_CHANNEL restoration.
    if ($null -eq $PreviousCargoTargetDir) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $PreviousCargoTargetDir
    }
    # Never leave the worktree registered, success or failure - a failed
    # build or validation must not leave git's working-tree list dirty.
    git worktree remove --force $WorktreeDir 2>$null | Out-Null
    git worktree prune 2>$null | Out-Null
}

Write-Host ""
Write-Host $MsixPath
