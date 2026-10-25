#!/usr/bin/env bash
# Install ForskScope on Linux after building a release binary.
# Prerequisites: cargo build --release
# Usage: bash packaging/linux/install.sh [PREFIX]
#   Default PREFIX: ~/.local

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
BINARY="$SCRIPT_DIR/target/release/forskscope"
# RFC-095 §5: the icon's SVG source - its own file name matches the app ID,
# the Flathub/hicolor convention, so `Icon=` in the desktop file below needs
# no change.
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

# Review 189 §3.1: the desktop file was renamed from forskscope.desktop to
# io.github.forskscope.forskscope.desktop (RFC-095 §5) - an install over an
# older copy under this same prefix would otherwise leave both, showing two
# ForskScope entries in the menu.
OLD_DESKTOP_FILE="$PREFIX/share/applications/forskscope.desktop"
if [[ -f "$OLD_DESKTOP_FILE" ]]; then
    rm -f "$OLD_DESKTOP_FILE"
    echo "Removed old desktop entry: $OLD_DESKTOP_FILE"
fi
# RFC-096 §1: one executable - `forskscope compare`/`forskscope mcp` are
# this same binary's own subcommands, not separate programs to install.

echo "Desktop entry: $PREFIX/share/applications/io.github.forskscope.forskscope.desktop"

# RFC-095 §5: the icon, under the same name the desktop file's `Icon=`
# already names. The guard stays even though the file now exists in a
# normal checkout - a missing icon (a sparse or partial checkout) still
# reports itself here, not a hard failure.
if [[ -f "$ICON" ]]; then
    install -Dm644 "$ICON" \
        "$PREFIX/share/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg"
    echo "Icon: $PREFIX/share/icons/hicolor/scalable/apps/io.github.forskscope.forskscope.svg"
else
    echo "Icon: not found at $ICON (expected in a normal checkout)"
fi

echo
echo "To update the desktop database:"
echo "  update-desktop-database $PREFIX/share/applications"
