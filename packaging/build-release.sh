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
    command -v pkg-config >/dev/null || {
        echo "Missing: pkg-config"
        exit 1
    }
    for pkg in webkit2gtk-4.1 gtk+-3.0; do
        pkg-config --exists "$pkg" || {
            echo "Missing: $pkg development files"
            echo "Debian/Ubuntu: apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev"
            echo "Arch Linux: pacman -S webkit2gtk-4.1 gtk3"
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
git ls-files -z | tar --null -czf "$ARCHIVE" --files-from -
if ! tar -tzf "$ARCHIVE" | awk '
    { path = $0; sub(/^\.\//, "", path); if (path == "Cargo.toml") found = 1 }
    END { exit found ? 0 : 1 }
'; then
    echo "ERROR: source archive does not contain Cargo.toml at archive root" >&2
    exit 1
fi
if tar -tzf "$ARCHIVE" | awk -v prefix="forskscope-v$VER" '
    { path = $0; sub(/^\.\//, "", path); if (path == prefix || index(path, prefix "/") == 1) bad = 1 }
    END { exit bad ? 0 : 1 }
'; then
    echo "ERROR: source archive contains forbidden top-level forskscope-v$VER/ directory" >&2
    exit 1
fi
if tar -tzf "$ARCHIVE" | awk -v archive_name="$(basename "$ARCHIVE")" '
    {
        path = $0
        sub(/^\.\//, "", path)
        if (path == archive_name || path == ".git-exclude" || index(path, ".git-exclude/") == 1 || path == ".git" || index(path, ".git/") == 1 || path == "target" || index(path, "target/") == 1) bad = 1
    }
    END { exit bad ? 0 : 1 }
'; then
    echo "ERROR: source archive contains generated, ignored, or local-only paths" >&2
    exit 1
fi
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
