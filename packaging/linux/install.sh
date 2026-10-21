#!/usr/bin/env bash
# Install ForskScope on Linux after building a release binary.
# Prerequisites: cargo build --release
# Usage: bash packaging/linux/install.sh [PREFIX]
#   Default PREFIX: ~/.local

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
BINARY="$SCRIPT_DIR/target/release/forskscope"
CLI_BINARY="$SCRIPT_DIR/target/release/forskscope-cli"
PREFIX="${1:-$HOME/.local}"

if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: release binary not found at $BINARY"
    echo "Build first: cargo build --release"
    exit 1
fi

install -Dm755 "$BINARY" "$PREFIX/bin/forskscope"
install -Dm644 "$SCRIPT_DIR/packaging/linux/forskscope.desktop" \
    "$PREFIX/share/applications/forskscope.desktop"
echo "Installed: $PREFIX/bin/forskscope"

# RFC-094: the headless program, built alongside forskscope by the same
# `cargo build --release` - optional here only in the sense that an older
# checkout's build directory might not have it yet.
if [[ -f "$CLI_BINARY" ]]; then
    install -Dm755 "$CLI_BINARY" "$PREFIX/bin/forskscope-cli"
    echo "Installed: $PREFIX/bin/forskscope-cli"
fi

echo "Desktop entry: $PREFIX/share/applications/forskscope.desktop"
echo
echo "To update the desktop database:"
echo "  update-desktop-database $PREFIX/share/applications"
