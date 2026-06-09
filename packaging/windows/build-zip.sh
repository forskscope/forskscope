#!/usr/bin/env bash
# Build a Windows release zip for ForskScope.
# Run in a cross-compilation environment or natively on Windows with Git Bash.
# Prerequisites: cargo build --release --target x86_64-pc-windows-msvc
# Usage: bash packaging/windows/build-zip.sh

set -euo pipefail

VER="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
BINARY="target/x86_64-pc-windows-msvc/release/forskscope.exe"
OUT="target/forskscope-$VER-windows-x64.zip"
STAGE="target/forskscope-$VER-windows-x64"

if [[ ! -f "$BINARY" ]]; then
    echo "Build first: cargo build --release --target x86_64-pc-windows-msvc"
    exit 1
fi

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$BINARY" "$STAGE/forskscope.exe"
cp README.md LICENSE NOTICE CHANGELOG.md "$STAGE/"

if command -v zip &>/dev/null; then
    (cd target && zip -r "../$OUT" "$(basename "$STAGE")/")
elif command -v 7z &>/dev/null; then
    7z a "$OUT" "$STAGE/"
else
    echo "ERROR: neither zip nor 7z found. Install one to produce the archive."
    exit 1
fi

echo "Created: $OUT"
