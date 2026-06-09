#!/usr/bin/env bash
# Build release binaries for the current platform and produce a source archive.
# Usage: bash packaging/build-release.sh

set -euo pipefail
cd "$(dirname "$0")/.."

VER="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
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
