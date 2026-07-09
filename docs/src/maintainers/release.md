# Release Process

## Pre-release checklist

1. All tests pass: `cargo test -p forskscope-core -p forskscope-ui-logic`
2. Clippy clean: `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
3. Format clean: `cargo fmt --check`
4. Security audit passes under the checked-in policy: `cargo audit`
5. Reviewed security dependency paths are enforced: `cargo xtask audit-deps`
6. CSS generated artifact is current: `cargo xtask css --check`
7. Source archive layout is verified by `packaging/build-release.sh` or the release workflow.
8. `CHANGELOG.md` updated with the new version and date.
9. `version` bumped in the workspace `Cargo.toml` (`[workspace.package]`).
10. Completed RFCs moved from `rfcs/proposed/` to `rfcs/done/`; `rfcs/README.md` updated.
11. `ROADMAP.md` current state paragraph updated if the milestone is significant.

---

## Building the release archive

The release is a `.tar.gz` of tracked Cargo workspace files. The source archive
has no top-level parent directory; files unpack directly into the extraction
destination.

Use the release script (handles version extraction and archive naming automatically):

```sh
bash packaging/build-release.sh
```

Or manually:

```sh
# Extract version from [workspace.package] — never grep '^' version directly
# as that may match dependency entries
VER=$(awk '/^\[workspace\.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[^0-9.]/,""); print; exit}' Cargo.toml)

git ls-files -z | tar --null -czf "target/forskscope-v${VER}.tar.gz" --files-from -
```

Verify the archive unpacks correctly:

```sh
tar -tzf "target/forskscope-v${VER}.tar.gz" | awk '{p=$0; sub(/^\.\//,"",p); print p}' | head -5
# Expected includes Cargo.toml at archive root, not forskscope-vX.Y.Z/Cargo.toml.
tar -tzf "target/forskscope-v${VER}.tar.gz" | awk '{p=$0; sub(/^\.\//,"",p); if (p=="Cargo.toml") found=1} END{exit found ? 0 : 1}'
tar -tzf "target/forskscope-v${VER}.tar.gz" | awk -v prefix="forskscope-v${VER}" '{p=$0; sub(/^\.\//,"",p); if (p==prefix || index(p,prefix"/")==1) bad=1} END{exit bad ? 1 : 0}'
tar -tzf "target/forskscope-v${VER}.tar.gz" | awk -v archive="forskscope-v${VER}.tar.gz" '{p=$0; sub(/^\.\//,"",p); if (p==archive || p==".git-exclude" || index(p,".git-exclude/")==1 || p==".git" || index(p,".git/")==1 || p=="target" || index(p,"target/")==1) bad=1} END{exit bad ? 1 : 0}'
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

The workspace `Cargo.toml` declares `rust-version = "1.91"` (the minimum
supported Rust version). Verify the build succeeds on the declared MSRV before
releasing.

```sh
rustup install 1.91
cargo +1.91 test -p forskscope-core -p forskscope-ui-logic
```
