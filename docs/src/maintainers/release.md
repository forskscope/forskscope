# Release Process

## Pre-release checklist

1. All tests pass: `cargo test -p forskscope-core -p forskscope-ui-logic`
2. Clippy clean: `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
3. `CHANGELOG.md` updated with the new version and date.
4. `version` bumped in the workspace `Cargo.toml` (`[workspace.package]`).
5. Completed RFCs moved from `rfcs/proposed/` to `rfcs/done/`; `rfcs/README.md` updated.
6. `ROADMAP.md` current state paragraph updated if the milestone is significant.

---

## Building the release archive

The release is a `.tar.gz` of the Cargo workspace, excluding the `target/`
directory. The archive unpacks to `forskscope-vX.Y.Z/` at the extraction root
(no nested intermediate directory).

Use the release script (handles version extraction and archive naming automatically):

```sh
bash packaging/build-release.sh
```

Or manually:

```sh
# Extract version from [workspace.package] — never grep '^' version directly
# as that may match dependency entries
VER=$(awk '/^\[workspace\.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[^0-9.]/,""); print; exit}' Cargo.toml)

# 1. Copy the working tree (avoids polluting the source directory)
cp -r . /tmp/forskscope-${VER}

# 2. Remove the build artefacts immediately
rm -rf /tmp/forskscope-${VER}/target

# 3. Create the archive
tar \
  --exclude="/tmp/forskscope-${VER}/target" \
  -czf "forskscope-v${VER}.tar.gz" \
  -C /tmp \
  "forskscope-${VER}" \
  --transform "s|^forskscope-${VER}|forskscope-v${VER}|"
```

Verify the archive unpacks correctly:

```sh
tar -tzf "forskscope-v${VER}.tar.gz" | head -5
# Expected: forskscope-vX.Y.Z/CHANGELOG.md, forskscope-vX.Y.Z/Cargo.toml, …
# No intermediate directory between forskscope-vX.Y.Z/ and the files.
```

---

## Archive naming

| File | Contents |
|------|----------|
| `forskscope-vX.Y.Z.tar.gz` | Source archive for the release |

---

## Version scheme

ForskScope uses semantic versioning (`MAJOR.MINOR.PATCH`). During the v0.x
pre-release phase:

- `PATCH` bumps for bug fixes and documentation updates within a stable feature set.
- `MINOR` bumps for new user-visible features or significant internal changes.
- `MAJOR` will be 1 when the first stable public release ships (RFC-041).

---

## After the archive

1. Upload the archive to the project release page.
2. Tag the commit: `git tag -a v${VER} -m "Release v${VER}"`.
3. Update `pkgver` in `packaging/linux/PKGBUILD` to match the workspace version.
   A comment in the file notes this requirement; failing to do so causes stale Arch packages.

---

## Checking the Rust edition and MSRV

The workspace `Cargo.toml` declares `rust-version = "1.85"` (the minimum
supported Rust version). Verify the build succeeds on the declared MSRV before
releasing.

```sh
rustup install 1.85
cargo +1.85 test -p forskscope-core -p forskscope-ui-logic
```
