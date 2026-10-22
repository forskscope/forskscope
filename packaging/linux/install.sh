#!/usr/bin/env bash
# Install ForskScope on Linux after building a release binary.
# Prerequisites: cargo build --release
# Usage: bash packaging/linux/install.sh [PREFIX]
#   Default PREFIX: ~/.local

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
BINARY="$SCRIPT_DIR/target/release/forskscope"
# RFC-095 §5: the icon's SVG source, once it exists - its own file name
# matches the app ID, the Flathub/hicolor convention, so `Icon=` in the
# desktop file below needs no change once it lands.
ICON="$SCRIPT_DIR/packaging/linux/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg"
PREFIX="${1:-$HOME/.local}"

if [[ ! -f "$BINARY" ]]; then
    echo "ERROR: release binary not found at $BINARY"
    echo "Build first: cargo build --release"
    exit 1
fi

install -Dm755 "$BINARY" "$PREFIX/bin/forskscope"
install -Dm644 "$SCRIPT_DIR/packaging/linux/io.github.forskscope.forskscope.desktop" \
    "$PREFIX/share/applications/io.github.forskscope.forskscope.desktop"
echo "Installed: $PREFIX/bin/forskscope"
# RFC-096 §1: one executable - `forskscope compare`/`forskscope mcp` are
# this same binary's own subcommands, not separate programs to install.

echo "Desktop entry: $PREFIX/share/applications/io.github.forskscope.forskscope.desktop"

# RFC-095 §5: no real icon exists yet (the owner has not decided who
# draws it) - installed once it does, under the same name the desktop
# file's `Icon=` already names, so nothing else here needs to change
# when it lands.
if [[ -f "$ICON" ]]; then
    install -Dm644 "$ICON" \
        "$PREFIX/share/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg"
    echo "Icon: $PREFIX/share/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg"
else
    echo "Icon: none yet (RFC-095 §5 Open questions 1) - $ICON not found"
fi

echo
echo "To update the desktop database:"
echo "  update-desktop-database $PREFIX/share/applications"
