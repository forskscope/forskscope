#!/usr/bin/env bash
# Build a macOS .dmg for ForskScope.
# Prerequisites: cargo build --release --target aarch64-apple-darwin
#                brew install create-dmg
# Usage: bash packaging/macos/build-dmg.sh

set -euo pipefail

VER="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
BINARY="target/aarch64-apple-darwin/release/forskscope"
DMG_DIR="target/dmg"
APP_DIR="$DMG_DIR/ForskScope.app/Contents/MacOS"
OUT="target/forskscope-$VER-macos-aarch64.dmg"

if [[ ! -f "$BINARY" ]]; then
    echo "Build first: cargo build --release --target aarch64-apple-darwin"
    exit 1
fi

rm -rf "$DMG_DIR"
mkdir -p "$APP_DIR"
cp "$BINARY" "$APP_DIR/forskscope"

# Minimal Info.plist
cat > "$DMG_DIR/ForskScope.app/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>ForskScope</string>
  <key>CFBundleIdentifier</key><string>io.github.nabbisen.forskscope</string>
  <key>CFBundleVersion</key><string>VERSION</string>
  <key>CFBundleExecutable</key><string>forskscope</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
</dict></plist>
PLIST
sed -i '' "s/VERSION/$VER/" "$DMG_DIR/ForskScope.app/Contents/Info.plist"

create-dmg \
    --volname "ForskScope $VER" \
    --window-size 500 300 \
    --icon-size 100 \
    --app-drop-link 370 150 \
    "$OUT" "$DMG_DIR/"

echo "Created: $OUT"
