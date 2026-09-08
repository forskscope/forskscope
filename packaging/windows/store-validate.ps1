#!/usr/bin/env pwsh
# RFC-079 §3/§4: validate a built MSIX before it ever reaches Partner
# Center. A failure here fails the workflow before anything is uploaded.
#
# Four checks, in the order §4 lists them:
#   1. the MSIX's manifest version equals the released tag;
#   2. Identity/Publisher/PublisherDisplayName match this project's own
#      tracked record of what Partner Center has registered (self-
#      consistency, not a live Partner Center check - see
#      store-listing/en-us/identity.toml's own comment for why);
#   3. the package contains the executable and every asset the manifest
#      references;
#   4. the package installs and the application launches, for real -
#      not unpacked-and-inspected. This is the check RFC-079 names as
#      "the one most likely to be quietly dropped": it signs a *separate,
#      validation-only* copy of the MSIX with a throwaway self-signed
#      certificate (the real, uploaded MSIX is never touched by this - it
#      stays unsigned, exactly as Partner Center expects to sign it
#      itself), trusts that certificate and enables sideloading on this
#      runner only, installs the signed copy, launches it, and confirms
#      the process is actually running before tearing everything down.
#
# Usage: store-validate.ps1 -MsixPath <path> -Tag <released-tag>

param(
    [Parameter(Mandatory = $true)][string]$MsixPath,
    [Parameter(Mandatory = $true)][string]$Tag
)

$ErrorActionPreference = "Stop"
$RepoRoot = git rev-parse --show-toplevel
$Fail = $false

function Fail-Check($msg) {
    Write-Host "::error::$msg"
    $script:Fail = $true
}

# ── Unpack for inspection - this does not stand in for the install+launch
#    check below; it only lets checks 1-3 read the package contents. ──────
$UnpackDir = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-store-validate-unpack"
if (Test-Path $UnpackDir) { Remove-Item -Recurse -Force $UnpackDir }

$MakeAppx = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction Stop |
    Sort-Object FullName -Descending | Select-Object -First 1
& $MakeAppx.FullName unpack /p $MsixPath /d $UnpackDir /overwrite
if ($LASTEXITCODE -ne 0) { throw "makeappx unpack failed" }

[xml]$Manifest = Get-Content -Raw (Join-Path $UnpackDir "AppxManifest.xml")

# ── 1. Manifest version equals the released tag. ───────────────────────────
$ExpectedVersion = "$Tag.0"
$ActualVersion = $Manifest.Package.Identity.Version
if ($ActualVersion -ne $ExpectedVersion) {
    Fail-Check "manifest Version ($ActualVersion) does not match the released tag ($ExpectedVersion)"
} else {
    Write-Host "OK: manifest Version matches the released tag ($ActualVersion)"
}

# ── 2. Identity/Publisher/PublisherDisplayName match the tracked record. ───
$IdentityTomlPath = Join-Path $RepoRoot "packaging/windows/store-listing/en-us/identity.toml"
$Identity = @{}
foreach ($line in Get-Content $IdentityTomlPath) {
    if ($line -match '^\s*#') { continue }
    if ($line -match '^\s*(\w+)\s*=\s*"([^"]*)"\s*$') {
        $Identity[$Matches[1]] = $Matches[2]
    }
}

$checks = @(
    @{ Name = "Identity Name"; Actual = $Manifest.Package.Identity.Name; Expected = $Identity["identity_name"] },
    @{ Name = "Identity Publisher"; Actual = $Manifest.Package.Identity.Publisher; Expected = $Identity["publisher"] },
    @{ Name = "PublisherDisplayName"; Actual = $Manifest.Package.Properties.PublisherDisplayName; Expected = $Identity["publisher_display_name"] }
)
foreach ($c in $checks) {
    if ($c.Actual -ne $c.Expected) {
        Fail-Check "$($c.Name) ($($c.Actual)) does not match the tracked Store identity ($($c.Expected))"
    } else {
        Write-Host "OK: $($c.Name) matches the tracked Store identity"
    }
}

# ── 3. The package contains the executable and every asset the manifest
#    references. ────────────────────────────────────────────────────────
$RequiredFiles = @("forskscope.exe")
$RequiredFiles += $Manifest.Package.Properties.Logo
$visualElements = $Manifest.Package.Applications.Application.VisualElements
$RequiredFiles += $visualElements.Square150x150Logo
$RequiredFiles += $visualElements.Square44x44Logo
$RequiredFiles += $visualElements.DefaultTile.Wide310x150Logo

foreach ($rel in ($RequiredFiles | Select-Object -Unique)) {
    $full = Join-Path $UnpackDir ($rel -replace '\\', [IO.Path]::DirectorySeparatorChar)
    if (-not (Test-Path $full)) {
        Fail-Check "package is missing $rel, referenced by the manifest"
    } else {
        Write-Host "OK: package contains $rel"
    }
}

if ($Fail) {
    Write-Host "::error::validation failed before the expensive install+launch check - stopping here"
    exit 1
}

# ── 4. Install and launch, for real. Self-signs a SEPARATE copy for this
#    check only; $MsixPath itself (the one that gets uploaded) is never
#    modified. ───────────────────────────────────────────────────────────
$Publisher = $Manifest.Package.Identity.Publisher
$SignedCopy = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-validation-signed.msix"
Copy-Item $MsixPath $SignedCopy -Force

$Cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher `
    -KeyUsage DigitalSignature -FriendlyName "ForskScope validation-only cert (not for submission)" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")

$PfxPath = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-validation.pfx"
$PfxPassword = ConvertTo-SecureString -String ([guid]::NewGuid().ToString()) -Force -AsPlainText
Export-PfxCertificate -Cert $Cert -FilePath $PfxPath -Password $PfxPassword | Out-Null

$SignTool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction Stop |
    Sort-Object FullName -Descending | Select-Object -First 1
& $SignTool.FullName sign /fd SHA256 /f $PfxPath /p ($PfxPassword | ConvertFrom-SecureString -AsPlainText) $SignedCopy
if ($LASTEXITCODE -ne 0) { throw "signtool sign failed" }

# Trust the validation-only cert and allow sideloading, on this runner only.
$CertBytes = $Cert.Export("Cert")
$DerPath = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-validation.cer"
[System.IO.File]::WriteAllBytes($DerPath, $CertBytes)
Import-Certificate -FilePath $DerPath -CertStoreLocation "Cert:\LocalMachine\TrustedPeople" | Out-Null

$UnlockKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock"
if (-not (Test-Path $UnlockKey)) { New-Item -Path $UnlockKey -Force | Out-Null }
Set-ItemProperty -Path $UnlockKey -Name "AllowAllTrustedApps" -Value 1 -Type DWord

try {
    Add-AppxPackage -Path $SignedCopy -ErrorAction Stop
    Write-Host "OK: package installed"

    $Pkg = Get-AppxPackage -Name $Manifest.Package.Identity.Name
    if (-not $Pkg) { throw "Add-AppxPackage succeeded but Get-AppxPackage found nothing" }

    $Aumid = "$($Pkg.PackageFamilyName)!App"
    Start-Process "explorer.exe" -ArgumentList "shell:appsFolder\$Aumid"

    $Proc = $null
    for ($i = 0; $i -lt 20; $i++) {
        Start-Sleep -Seconds 1
        $Proc = Get-Process -Name "forskscope" -ErrorAction SilentlyContinue
        if ($Proc) { break }
    }

    if (-not $Proc) {
        Fail-Check "the package installed but forskscope.exe never appeared in the process list after launch - installing is not the same as launching, and this did not launch"
    } else {
        Write-Host "OK: forskscope.exe is running (PID $($Proc.Id)) after installing and launching the signed validation copy"
        Stop-Process -Id $Proc.Id -Force -ErrorAction SilentlyContinue
    }
} finally {
    # Best-effort teardown - never let cleanup failure mask (or be masked
    # by) the check's own result above.
    Get-AppxPackage -Name $Manifest.Package.Identity.Name -ErrorAction SilentlyContinue |
        Remove-AppxPackage -ErrorAction SilentlyContinue
    Remove-Item "Cert:\LocalMachine\TrustedPeople\$($Cert.Thumbprint)" -ErrorAction SilentlyContinue
    Remove-Item "Cert:\CurrentUser\My\$($Cert.Thumbprint)" -ErrorAction SilentlyContinue
    Remove-Item $PfxPath, $DerPath, $SignedCopy -ErrorAction SilentlyContinue
}

if ($Fail) {
    exit 1
}
Write-Host "All validation checks passed for $MsixPath"
