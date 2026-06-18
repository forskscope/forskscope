#!/usr/bin/env bash
# Build release binaries for the current platform and produce a source archive.
# Usage: bash packaging/build-release.sh

set -euo pipefail
cd "$(dirname "$0")/.."

# Extract the version from the [workspace.package] section. Read only the first
# `version = "..."` line that appears after the [workspace.package] header so the
# value cannot be confused with a dependency version elsewhere in the file.
VER="$(awk '/^\[workspace\.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[^0-9.]/,"",$0); print; exit}' Cargo.toml)"
if [[ -z "$VER" ]]; then
    echo "ERROR: could not determine version from [workspace.package] in Cargo.toml" >&2
    exit 1
fi
echo "=== ForskScope v$VER release build ==="

# ── System dependency check (Linux) ─────────────────────────────────────────
if [[ "$(uname)" == "Linux" ]]; then
    for pkg in libwebkit2gtk-4.1-dev libgtk-3-dev; do
        dpkg -l "$pkg" &>/dev/null || {
            echo "Missing: $pkg  (run: apt-get install $pkg)"
            exit 1
        }
    done
fi

# ── Rust build ───────────────────────────────────────────────────────────────
echo "Building release binary…"
cargo build --release --locked
echo "Binary: target/release/forskscope"

# ── Source archive ───────────────────────────────────────────────────────────
ARCHIVE="target/forskscope-v$VER.tar.gz"
tar \
    --exclude='./target' \
    --exclude='./.git' \
    -czf "$ARCHIVE" \
    --transform "s|^\./|forskscope-v$VER/|" \
    .
echo "Source archive: $ARCHIVE"

# ── Platform-specific binary archive ────────────────────────────────────────
if [[ "$(uname)" == "Linux" ]]; then
    OUT="target/forskscope-v$VER-linux-x86_64.tar.gz"
    tar -czf "$OUT" -C target/release forskscope
    echo "Linux binary archive: $OUT"
elif [[ "$(uname)" == "Darwin" ]]; then
    echo "For macOS DMG: bash packaging/macos/build-dmg.sh"
fi

echo
echo "Done. Release artifacts:"
ls -lh target/*.tar.gz target/*.zip 2>/dev/null || true
