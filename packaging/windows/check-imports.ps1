#!/usr/bin/env pwsh
# RFC-091 §4 (F45, F206): lists an .exe's imported DLLs and fails if any is
# VCRUNTIME*.dll, MSVCP*.dll, or not on an explicit allow-list of Windows
# system DLLs. One script, three callers - release.yml's Windows job (the
# release itself), windows-check.yml's crt-import-check (the on-demand,
# dispatchable proof), and store-build.ps1 (the Store package, which must
# keep the same promise) - so the gate the release actually runs cannot
# silently diverge from the one a dispatch proves. F206: it already had,
# for 0.193.0 - the check existed only in windows-check.yml, dispatched by
# hand, and the release's own Windows job never called anything like it.
#
# Takes the path of an .exe directly, rather than a cargo invocation
# (`cargo objdump --bin ... --release`) tied to one specific build just
# having happened in the current directory - all three callers can point
# this at whatever binary they just produced or packed, including
# store-build.ps1's staged copy, without this script needing to know
# anything about how it got there.
#
# Uses `llvm-objdump` directly, under the toolchain's own sysroot (the
# `llvm-tools` rustup component - `dtolnay/rust-toolchain@stable` with
# `components: llvm-tools`, or `rustup component add llvm-tools`), not
# `cargo-binutils`: cargo-binutils' subcommands resolve their target from
# cargo build flags (`--bin`/`--release`), not an arbitrary path, which is
# exactly the limitation this script exists to not have. One fewer
# installed tool too - `cargo install cargo-binutils` is no longer needed
# by any of the three callers.
#
# Usage: check-imports.ps1 -ExePath <path-to-exe>

param(
    [Parameter(Mandatory = $true)][string]$ExePath
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ExePath)) {
    Write-Host "::error::$ExePath does not exist"
    exit 1
}

$bytes = (Get-Item $ExePath).Length
Write-Host "$(Split-Path $ExePath -Leaf): $bytes bytes ($([math]::Round($bytes / 1MB, 2)) MiB)"

$sysroot = (rustc --print sysroot | Out-String).Trim()
$llvmObjdump = Get-ChildItem (Join-Path $sysroot "lib\rustlib\*\bin\llvm-objdump.exe") -ErrorAction Stop |
    Select-Object -First 1
if (-not $llvmObjdump) {
    Write-Host "::error::llvm-objdump.exe not found under the toolchain sysroot ($sysroot) - is the llvm-tools component installed?"
    exit 1
}

$dump = & $llvmObjdump.FullName -p $ExePath 2>&1 | Out-String
Write-Host "--- llvm-objdump -p (private headers) ---"
Write-Host $dump

$imports = [regex]::Matches($dump, '(?m)^\s*DLL Name:\s*(\S+)\s*$') |
    ForEach-Object { $_.Groups[1].Value } |
    Sort-Object -Unique

Write-Host "--- Imported DLLs (unique) ---"
$imports | ForEach-Object { Write-Host $_ }

if ($imports.Count -eq 0) {
    Write-Error "no 'DLL Name:' lines were found in llvm-objdump's output — the parse itself may be wrong, not the binary. Full output is above."
    exit 1
}

# Explicit allow-list (RFC-091 §4: "a new import must be
# justified, as on Linux"), built from what forskscope.exe
# actually imports after the hybrid-CRT change — not a
# speculative superset. Each entry is a Windows system DLL
# present on every supported Windows 10+ install, never the
# Visual C++ Redistributable this change exists to stop
# depending on.
$allowExact = @(
    "kernel32.dll",            # core Win32 API
    "user32.dll",              # windowing, messages
    "gdi32.dll",               # graphics primitives
    "ntdll.dll",               # always present, loaded by the OS loader itself
    "advapi32.dll",            # registry, security, service APIs
    "shell32.dll",             # shell integration (rfd's native dialogs)
    "shlwapi.dll",             # shell path/string helpers
    "comctl32.dll",            # common controls (native dialogs/menus)
    "ole32.dll",               # COM — how WebView2 is reached
    "oleaut32.dll",            # COM automation (BSTR etc.)
    "ws2_32.dll",              # networking (update check, HTTPS)
    "crypt32.dll",             # TLS certificate handling
    "secur32.dll",             # SSPI/Schannel — TLS handshake (update check, HTTPS)
    "bcryptprimitives.dll",    # Windows CNG crypto primitives (reached via rustls/ring)
    "dwmapi.dll"               # desktop window manager (drop shadows etc.)
)
# The Universal CRT's own API-set forwarders (api-ms-win-crt-*,
# six of them: convert/heap/locale/math/runtime/stdio/string) and
# core OS API-set forwarders (api-ms-win-core-*, synch): Windows
# 10+ system components that resolve to kernel32.dll/ucrtbase.dll
# via OS-level API Set redirection, not anything this project
# links against directly or redistributes — matched by prefix
# rather than naming every one.
$allowPrefixes = @("api-ms-win-crt-", "api-ms-win-core-")

$bad = $imports | Where-Object {
    $name = $_
    $isVcrt = $name -match '^(?i)VCRUNTIME'
    $isMsvcp = $name -match '^(?i)MSVCP'
    $isAllowedExact = $allowExact -contains $name
    $isAllowedPrefix = $false
    foreach ($p in $allowPrefixes) {
        if ($name.ToLower().StartsWith($p)) { $isAllowedPrefix = $true }
    }
    $isVcrt -or $isMsvcp -or -not ($isAllowedExact -or $isAllowedPrefix)
}

if ($bad) {
    Write-Error "$(Split-Path $ExePath -Leaf) imports a DLL not on the allow-list (or a VCRUNTIME*/MSVCP* dependency the hybrid CRT exists to remove): $($bad -join ', ')"
    exit 1
}
Write-Host "All imports are on the allow-list; no VCRUNTIME*/MSVCP* present."
