#!/usr/bin/env pwsh
# RFC-079 §2, §4, §5: submit a validated MSIX to Partner Center through the
# Microsoft Store submission API. This is the only script in this
# directory that ever reads the Entra ID client secret, and the only one
# that can mutate anything outside this runner.
#
# Shares its authentication and read path with -DryRun: both authenticate
# and GET the application resource, which alone proves credentials and
# connectivity work (RFC-079's own suggested mitigation for "submission
# cannot be dry-run against production without consuming a real
# submission"). Only -DryRun stops there. Without it, the script deletes an
# existing pending submission if one exists (re-run safety - RFC-079 §5
# requires a second run to replace, not duplicate), creates a fresh one,
# replaces only its `applicationPackages` (never listings/pricing/images -
# RFC-079 §9 Q5 keeps those manual), uploads the package, commits, and
# polls status without waiting for certification to finish.
#
# Usage: store-submit.ps1 -MsixPath <path> -Tag <released-tag> [-DryRun]
#
# Required environment: STORE_TENANT_ID, STORE_CLIENT_ID,
# STORE_CLIENT_SECRET, STORE_APP_ID.

param(
    [Parameter(Mandatory = $true)][string]$MsixPath,
    [Parameter(Mandatory = $true)][string]$Tag,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

foreach ($name in @("STORE_TENANT_ID", "STORE_CLIENT_ID", "STORE_CLIENT_SECRET", "STORE_APP_ID")) {
    # An unset GitHub Actions secret reaches the runner as an env var that
    # *exists* with an empty value, not one that is absent - Get-Item alone
    # would let an empty secret through to a confusing 404 from the auth
    # endpoint instead of this clear message (found live, run 34207844616:
    # this sandbox has none of these secrets configured, and the first
    # version of this check let that reach Invoke-RestMethod anyway).
    if ([string]::IsNullOrEmpty((Get-Item "env:$name" -ErrorAction SilentlyContinue).Value)) {
        Write-Host "::error::$name is not set - see docs/src/maintainers/release.md for what the store-publish Environment must contain"
        exit 1
    }
}

$TenantId = $env:STORE_TENANT_ID
$ClientId = $env:STORE_CLIENT_ID
$ClientSecret = $env:STORE_CLIENT_SECRET
$AppId = $env:STORE_APP_ID

$ApiBase = "https://manage.devcenter.microsoft.com/v1.0/my"

# ── Authenticate. A failure here is reported with expiry named as a likely
#    cause (owner decision, handoff 030 §2) rather than a bare auth error -
#    Entra ID client secrets expire at 24 months at the most, often less,
#    and this project has no other way to notice one has lapsed. ──────────
try {
    $TokenResponse = Invoke-RestMethod -Method Post `
        -Uri "https://login.microsoftonline.com/$TenantId/oauth2/token" `
        -ContentType "application/x-www-form-urlencoded" `
        -Body @{
            grant_type    = "client_credentials"
            client_id     = $ClientId
            client_secret = $ClientSecret
            resource      = "https://manage.devcenter.microsoft.com"
        }
} catch {
    Write-Host "::error::authentication to the Microsoft Store submission API failed. If STORE_CLIENT_SECRET was not recently rotated, the most likely cause is that it has expired (Entra ID client secrets last 24 months at most, often less) - check the expiry recorded in docs/src/maintainers/threat-model.md against today's date before assuming this is a code problem. Underlying error: $($_.Exception.Message)"
    exit 1
}
$AuthHeader = @{ Authorization = "Bearer $($TokenResponse.access_token)" }
Write-Host "Authenticated to the Microsoft Store submission API."

# ── Read the application resource. This alone is the dry-run path: it
#    proves the credential and connectivity without creating or changing
#    anything (RFC-079's own suggested non-committing check). ─────────────
$App = Invoke-RestMethod -Method Get -Uri "$ApiBase/applications/$AppId" -Headers $AuthHeader
Write-Host "Application: $($App.primaryName) (id $AppId)"
if ($App.pendingApplicationSubmission) {
    Write-Host "Existing pending submission: $($App.pendingApplicationSubmission.id)"
} else {
    Write-Host "No pending submission exists."
}

if ($DryRun) {
    Write-Host "dry_run: stopping after authentication and the application read. Nothing was created, changed, or deleted."
    exit 0
}

# ── Re-run safety: replace, never duplicate. A second run against the same
#    release must not leave two submissions in flight. ────────────────────
if ($App.pendingApplicationSubmission) {
    $PendingId = $App.pendingApplicationSubmission.id
    Write-Host "Deleting existing pending submission $PendingId before creating a new one..."
    Invoke-RestMethod -Method Delete -Uri "$ApiBase/applications/$AppId/submissions/$PendingId" -Headers $AuthHeader | Out-Null
    Write-Host "Deleted."
}

# ── Create a submission. Partner Center clones the last published
#    submission's full JSON (listings, pricing, everything) - we touch only
#    applicationPackages below, never the rest, per Q5. ────────────────────
$Submission = Invoke-RestMethod -Method Post -Uri "$ApiBase/applications/$AppId/submissions" -Headers $AuthHeader
$SubmissionId = $Submission.id
$UploadUrl = $Submission.fileUploadUrl
Write-Host "Created submission $SubmissionId"

$PackageFileName = "forskscope.msix"
foreach ($pkg in @($Submission.applicationPackages)) {
    $pkg.fileStatus = "PendingDelete"
}
$NewPackage = [PSCustomObject]@{
    fileName              = $PackageFileName
    fileStatus            = "PendingUpload"
    minimumDirectXVersion = "None"
    minimumSystemRam      = "None"
}
$Submission.applicationPackages = @($Submission.applicationPackages) + $NewPackage

$SubmissionJson = $Submission | ConvertTo-Json -Depth 20
Invoke-RestMethod -Method Put -Uri "$ApiBase/applications/$AppId/submissions/$SubmissionId" `
    -Headers $AuthHeader -ContentType "application/json" -Body $SubmissionJson | Out-Null
Write-Host "Updated submission $SubmissionId to reference $PackageFileName (existing packages marked for deletion)."

# ── Package for upload: a zip containing exactly the msix, named to match
#    what the submission JSON above just declared. ─────────────────────────
$ZipDir = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-store-upload"
if (Test-Path $ZipDir) { Remove-Item -Recurse -Force $ZipDir }
New-Item -ItemType Directory -Path $ZipDir | Out-Null
Copy-Item $MsixPath (Join-Path $ZipDir $PackageFileName)
$ZipPath = Join-Path ([System.IO.Path]::GetTempPath()) "forskscope-store-upload.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path (Join-Path $ZipDir $PackageFileName) -DestinationPath $ZipPath

Write-Host "Uploading $ZipPath to the SAS URL Partner Center issued..."
Invoke-RestMethod -Method Put -Uri $UploadUrl -InFile $ZipPath `
    -Headers @{ "x-ms-blob-type" = "BlockBlob" } -ContentType "application/zip" | Out-Null
Write-Host "Uploaded."

# ── Commit. This is the point of no return for this submission - Partner
#    Center begins processing it. ──────────────────────────────────────────
Invoke-RestMethod -Method Post -Uri "$ApiBase/applications/$AppId/submissions/$SubmissionId/commit" -Headers $AuthHeader | Out-Null
Write-Host "Committed submission $SubmissionId."

# ── Poll, but do not wait for certification (RFC-079 §4: "does not wait
#    for certification to complete... the normal case"). Ten minutes at
#    30-second intervals, then report whatever status is current. ─────────
$Status = "Unknown"
$Deadline = (Get-Date).AddMinutes(10)
while ((Get-Date) -lt $Deadline) {
    Start-Sleep -Seconds 30
    $StatusResponse = Invoke-RestMethod -Method Get `
        -Uri "$ApiBase/applications/$AppId/submissions/$SubmissionId/status" -Headers $AuthHeader
    $Status = $StatusResponse.status
    Write-Host "Submission status: $Status"
    if ($Status -notin @("PendingCommit", "CommitStarted", "PreProcessing")) {
        # Reached Certification (or further) - this is the "submitted,
        # certification pending" state the workflow is allowed to report
        # and stop at. Not a claim of publication either way.
        break
    }
}

Add-Content -Path $env:GITHUB_OUTPUT -Value "submission_id=$SubmissionId"
Add-Content -Path $env:GITHUB_OUTPUT -Value "submission_status=$Status"
Write-Host ""
Write-Host "Submitted. Submission $SubmissionId is at status '$Status'."
Write-Host "This is submission, not publication - certification runs on Microsoft's side and can take hours to days."
Write-Host "Partner Center: https://partner.microsoft.com/dashboard -> App and game management -> ForskScope -> Submissions -> $SubmissionId"
