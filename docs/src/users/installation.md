# Installation

Every release publishes four artifacts plus a source archive on the
[Releases page](https://github.com/forskscope/forskscope/releases). Pick the one
that matches your platform.

ForskScope runs entirely on your machine. It makes no network requests, has no
accounts, and collects no telemetry.

---

## Linux

### Prebuilt binary

```sh
# Resolves the newest release automatically -- no version to keep in step.
url=$(curl -s https://api.github.com/repos/forskscope/forskscope/releases/latest \
  | grep -o 'https://[^"]*-linux-x86_64\.tar\.gz')
curl -LO "$url"
tar -xzf forskscope-v*-linux-x86_64.tar.gz
./forskscope
```

You need WebKitGTK 4.1 and GTK 3 at runtime:

```sh
sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0 libxdo3   # Debian / Ubuntu
sudo dnf install webkit2gtk4.1 gtk3                            # Fedora
sudo pacman -S webkit2gtk-4.1 gtk3                             # Arch
```

`libxdo3` (F59) is required but is not pulled in by either
`libwebkit2gtk-4.1-0` or `libgtk-3-0` — a Debian/Ubuntu host without it
fails to launch at all (`error while loading shared libraries:
libxdo.so.3`), confirmed on a fresh `ubuntu-latest` host that had only
the other two packages installed.

> **Known limitation — the prebuilt binary is Debian/Ubuntu-family only.**
>
> It is built on Ubuntu and records `libxdo.so.3`, while Arch and other rolling
> distributions ship `libxdo.so.4`. The dynamic loader will not substitute one
> for the other, so on those systems it fails with:
>
> ```text
> error while loading shared libraries: libxdo.so.3: cannot open shared object file
> ```
>
> Installing `xdotool` does **not** fix this — you already have soname 4, and
> soname 3 is not available there. **Build from source instead** (below), which
> links against whatever your distribution provides.
>
> The underlying cause is fixed upstream
> ([DioxusLabs/dioxus#5749](https://github.com/DioxusLabs/dioxus/pull/5749),
> merged) and this limitation goes away once that release is picked up.

### Arch Linux

ForskScope is on the AUR: **[`forskscope`](https://aur.archlinux.org/packages/forskscope)**.

```sh
paru -S forskscope      # or: yay -S forskscope
```

It is a **source** package — it compiles on your machine and links your own
system libraries, so unlike the prebuilt tarball above it works on Arch and
other `libxdo.so.4` distributions.

> **You may be asked to choose a `cargo` provider, even with Rust already
> installed.**
>
> ```text
> :: There are 2 providers available for cargo:
> :: Repository extra:
>    1) rust  2) rustup
> ```
>
> **Nothing is wrong, and you do not need to change how you installed Rust.**
> This affects every Rust package on the AUR, not just ForskScope. `cargo` is a
> *virtual* dependency that both the `rust` and `rustup` packages provide, so
> `pacman` has to be told which package to use.
>
> It appears even when Rust is already installed **through `rustup.sh`**,
> because that installs into `~/.cargo/` and `pacman` keeps no record of it.
> `pacman` is not ignoring your toolchain — it cannot see it.
>
> **If you installed Rust with `rustup.sh` and want nothing extra installed,**
> tell `pacman` the dependency is already met:
>
> ```sh
> paru -S --assume-installed cargo forskscope
> ```
>
> **If you would rather just answer the prompt,** either option builds
> ForskScope correctly. `rustup` reads the same `~/.rustup` toolchains you
> already have; `rust` is a self-contained toolchain managed by `pacman`.
> Neither replaces or interferes with an existing `rustup.sh` installation.

The [`PKGBUILD`](https://github.com/forskscope/forskscope/blob/main/packaging/linux/PKGBUILD)
also ships in the repository if you prefer to build it by hand with
`makepkg -si`. **Note this path is not covered by the project's automated
tests** — the AUR package is what receives attention on each release.

### Build from source

Recommended on any distribution that is not Debian/Ubuntu-family, and the only
option on non-x86_64 hardware.

```sh
# Prerequisites: Rust 1.91 or newer
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev pkg-config libssl-dev

git clone https://github.com/forskscope/forskscope
cd forskscope
cargo build --release -p forskscope-ui
./target/release/forskscope
```

---

## Windows

### Microsoft Store

[**ForskScope on the Microsoft Store**](https://apps.microsoft.com/detail/9p63f7npc3mh)

The Store listing is not always current — check its version against the
[latest release](https://github.com/forskscope/forskscope/releases/latest) and
use the zip below if you need the newest build.

### Zip

Download `forskscope-vX.Y.Z-windows-x64.zip` from the
[Releases page](https://github.com/forskscope/forskscope/releases), extract, and
run `forskscope.exe`. The archive also contains the README, license, notice, and
changelog.

**Tested on Windows 11.** ForskScope will *install* on Windows 10 version
1809 or later — that is the Store manifest's declared `MinVersion`, and the
zip has no floor at all — but it is **not tested there**, and Windows 10
reached end of support on 2025-10-14. Treat it as unsupported.

ForskScope renders through the **WebView2 runtime**, which Windows 11
preinstalls. It also needs the **Visual C++ redistributable**, which a clean
Windows install does not always have. If the window opens blank, or the app
fails to start with a message about `VCRUNTIME140.dll`, install the
[WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) and
the [Visual C++ redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe).

---

## macOS

Download `forskscope-vX.Y.Z-macos-aarch64.dmg` (Apple silicon), open it, and drag
ForskScope to Applications. Requires **macOS 13.0 or later** (matches
`Info.plist`'s `LSMinimumSystemVersion`, which macOS enforces at launch).

> **The build is not signed with an Apple Developer ID and is not notarized.**
> macOS may therefore refuse to open it on first launch. Right-click the app and
> choose **Open**, or clear the quarantine attribute:
>
> ```sh
> xattr -d com.apple.quarantine /Applications/ForskScope.app
> ```

---

## Verifying a download

Release artifacts are listed with their digests in each release's notes. To
check one:

```sh
sha256sum forskscope-v*-linux-x86_64.tar.gz
```

---

## Next steps

- [Quick start](./quick-start.md) — your first comparison
- [CLI usage](../intermediate/cli.md) — arguments and exit codes
- [Git integration](../intermediate/git-integration.md) — difftool and mergetool setup
- [Troubleshooting](./troubleshooting.md) — if something does not start
