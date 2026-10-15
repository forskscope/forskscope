# Packaging

## Prerequisites

Build a release binary first.

- **Linux:** a host build is enough.
- **macOS and Windows:** the packaging scripts read a **target-specific** build
  directory, so build for that target.

```sh
# Linux
cargo build --release --locked
# or use the convenience script:
bash packaging/build-release.sh

# macOS (packaging/macos/build-dmg.sh reads target/aarch64-apple-darwin/release)
cargo build --release --locked --target aarch64-apple-darwin

# Windows (packaging/windows/build-zip.sh reads target/x86_64-pc-windows-msvc/release)
cargo build --release --locked --target x86_64-pc-windows-msvc
```

## Linux

**Quick install to `~/.local`:**

```sh
bash packaging/linux/install.sh
```

**Arch Linux:** install `forskscope` from the AUR.

**Do not build `packaging/linux/PKGBUILD` from this repository.**

- Its `pkgver` names the *next*, unreleased version between releases, so the
  source it downloads has no tag behind it.
- Its `sha256sums=('SKIP')` checks nothing.

The AUR copy is the verified one. `aur-publish.yml` writes the real hash and
builds, installs and `namcap`s the package before pushing it. The PKGBUILD's own
comment says the same.

**System requirements at runtime:**

| Package | Version | Notes |
|---------|---------|-------|
| `webkit2gtk-4.1` | ≥ 2.40 | WebView rendering |
| `gtk3` | ≥ 3.22 | Window and widgets |

On Debian/Ubuntu: `sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0`

## macOS

```sh
brew install create-dmg
bash packaging/macos/build-dmg.sh
```

Produces `target/forskscope-vX.Y.Z-macos-aarch64.dmg`.

**Note:** signing and notarization are not yet automated (RFC-010).
Users on macOS may need to right-click → Open on first launch.

## Windows

Build on a Windows machine or cross-compilation environment with the
`x86_64-pc-windows-msvc` target:

```sh
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
bash packaging/windows/build-zip.sh
```

For MSIX/Store packaging, `packaging/windows/` also provides an
`AppxManifest.xml` (package version mirrors the release, e.g. `0.164.0.0`)
and `Assets/` tile and logo images. Keep the manifest `Version` in sync with
the workspace version on each release (four-part `X.Y.Z.0` form).

## Source archive

The project does not build its own source archive (F43: dropped — it
duplicated GitHub's automatic one). GitHub attaches a source archive to
every tag automatically:
`https://github.com/forskscope/forskscope/archive/refs/tags/<tag>.tar.gz`
