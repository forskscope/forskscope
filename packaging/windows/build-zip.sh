#!/usr/bin/env bash
# Build a Windows release zip for ForskScope.
# Run in a cross-compilation environment or natively on Windows with Git Bash.
# Prerequisites: cargo build --release --target x86_64-pc-windows-msvc
# Usage: bash packaging/windows/build-zip.sh

set -euo pipefail

VER="$(awk '/^\[workspace\.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[^0-9.]/,""); print; exit}' Cargo.toml)"
BINARY="target/x86_64-pc-windows-msvc/release/forskscope.exe"
CLI_BINARY="target/x86_64-pc-windows-msvc/release/forskscope-cli.exe"
OUT="target/forskscope-v$VER-windows-x64.zip"
STAGE="target/forskscope-v$VER-windows-x64"

if [[ ! -f "$BINARY" ]] || [[ ! -f "$CLI_BINARY" ]]; then
    echo "Build first: cargo build --release --target x86_64-pc-windows-msvc"
    exit 1
fi

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BINARY" "$STAGE/forskscope.exe"
# RFC-094 §1: the Windows .zip carries forskscope-cli beside the GUI.
cp "$CLI_BINARY" "$STAGE/forskscope-cli.exe"
cp README.md LICENSE NOTICE CHANGELOG.md "$STAGE/"

if command -v zip &>/dev/null; then
    (cd target && zip -r "../$OUT" "$(basename "$STAGE")/")
elif command -v 7z &>/dev/null; then
    (cd target && 7z a "../$OUT" "$(basename "$STAGE")/")
else
    echo "ERROR: neither zip nor 7z found. Install one to produce the archive."
    exit 1
fi

echo "Created: $OUT"
