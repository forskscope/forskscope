# Installation

Every release publishes four artifacts plus a source archive on the
[Releases page](https://github.com/forskscope/forskscope/releases). Pick the one
that matches your platform.

ForskScope runs entirely on your machine, has no accounts, and collects no
telemetry. It makes no network requests unless you click **Check for
updates** in the About dialog (ℹ, in Settings) — that sends one request to
GitHub to read the latest version number. Like any web request it carries
your IP address, and it identifies itself as `ForskScope/<version>`.
Nothing else is sent, and nothing runs in the background.

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
sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0    # Debian / Ubuntu
sudo dnf install webkit2gtk4.1 gtk3                     # Fedora
sudo pacman -S webkit2gtk-4.1 gtk3                      # Arch
```

> **Built on Ubuntu.** The prebuilt tarball is built on Ubuntu, so it needs a
> glibc at least as new as that build runner's. Measured on an Arch-based
> machine: it starts there too. It has not been run on other distributions.

### Arch Linux

ForskScope is on the AUR: **[`forskscope`](https://aur.archlinux.org/packages/forskscope)**.

```sh
paru -S forskscope      # or: yay -S forskscope
```

It is a **source** package — it compiles on your machine and links your own
system libraries, rather than Ubuntu's.

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
also ships in the repository, but it is a **template**, not something to copy
and build directly: `sha256sums=('SKIP')` is permanent there, filled in with a
real, verified hash only by the automation that publishes to the AUR on each
release, which also builds the package and runs `namcap` on it before
publishing — the AUR copy is what is actually checked, not this file. Building
this exact copy by hand with `makepkg -si` skips that verification entirely.

### Build from source

The only option on non-x86_64 hardware, or on a distribution whose glibc is
older than the prebuilt binary's Ubuntu build runner.

```sh
# Prerequisites: Rust 1.91 or newer
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev pkg-config libssl-dev

git clone https://github.com/forskscope/forskscope
cd forskscope
cargo build --release -p forskscope-ui
./target/release/forskscope
```

---

## Windows

### Microsoft Store

[**ForskScope on the Microsoft Store**](https://apps.microsoft.com/detail/9p63f7npc3mh)

Every published release is submitted to the Store automatically. The listing
can still lag briefly — Microsoft's certification runs asynchronously after
submission and can take hours to days — so if the Store's version looks
behind, that is certification catching up, not a missed submission. Check
its version against the
[latest release](https://github.com/forskscope/forskscope/releases/latest)
and use the zip below if you need the build immediately.

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
preinstalls. If it is missing, ForskScope detects this at startup and shows a
message box offering to open the download page; choosing not to still closes
the app (exit code 3), since there is nothing to render into otherwise. It
also needs the **Visual C++ redistributable**, which a clean Windows install
does not always have — that failure looks different: the app fails to start
with a message about `VCRUNTIME140.dll`, before ForskScope's own code can run
at all. Install the
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

## Verifying your download

Every release publishes a fourth file, **`SHA256SUMS`**, listing a SHA-256
digest for each of the three platform assets, in the standard `sha256sum`
format:

```sh
# Linux — download forskscope-v*-linux-x86_64.tar.gz and SHA256SUMS first
sha256sum -c SHA256SUMS --ignore-missing

# macOS — download the .dmg and SHA256SUMS first
grep macos-aarch64 SHA256SUMS | shasum -a 256 -c
```

```powershell
# Windows
Get-FileHash forskscope-vX.Y.Z-windows-x64.zip -Algorithm SHA256
# then compare against the matching line in SHA256SUMS by hand
```

Both commands check only the file you downloaded. Without `--ignore-missing`
(Linux) or the `grep` (macOS), they would also try the two platform assets you
did not download and report them as `FAILED`, even though your file is fine.

**This protects against a corrupted or incomplete download, not against
the build host being compromised** — `SHA256SUMS` is computed on the same CI
runner that builds the assets, by the same workflow. It confirms the file
you have matches what that build produced, not who produced it. Nothing
in this release signs a build independently of GitHub's own hosting.
Signing is a separate, open question (F46) and is not part of this
release.

Each file on a release's page also shows its own SHA-256 digest, which
GitHub computes independently when the file is uploaded — a second,
independent source for the same number, if you want to cross-check
`SHA256SUMS` itself rather than trust the file that names it.

---

## Next steps

- [Quick start](./quick-start.md) — your first comparison
- [CLI usage](../intermediate/cli.md) — arguments and exit codes
- [Git integration](../intermediate/git-integration.md) — difftool and mergetool setup
- [Troubleshooting](./troubleshooting.md) — if something does not start
