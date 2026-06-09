# Packaging

## Prerequisites

Build a release binary first:

```sh
cargo build --release --locked
# or use the convenience script:
bash packaging/build-release.sh
```

## Linux

**Quick install to `~/.local`:**

```sh
bash packaging/linux/install.sh
```

**Arch Linux (AUR-style):**

Copy `packaging/linux/PKGBUILD` and the release source archive into a
directory, then run `makepkg -si`.

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

## Source archive

The source archive is the primary release artifact:

```sh
# Produces target/forskscope-vX.Y.Z.tar.gz
# Unpacks to: forskscope-vX.Y.Z/(files)
bash packaging/build-release.sh
```
