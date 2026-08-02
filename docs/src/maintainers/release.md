# Release Process

## Pre-release checklist

1. All tests pass: `cargo test -p forskscope-core -p forskscope-ui-logic`
2. Clippy clean: `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings`
3. Format clean: `cargo fmt --check`
4. Security audit passes under the checked-in policy: `cargo audit`
5. Reviewed security dependency paths are enforced: `cargo xtask audit-deps`
6. CSS generated artifact is current: `cargo xtask css --check`
7. Version metadata is synchronized: `cargo xtask version-sync`
8. Release tag matches the workspace version: `cargo xtask version-sync "${GITHUB_REF_NAME}"`
9. Japanese localization covers `t(...)` UI keys: `cargo xtask i18n`
10. Source archive layout is verified: `cargo xtask archive-layout target/forskscope-vX.Y.Z.tar.gz`
11. `CHANGELOG.md` updated with the new version and date.
12. `version` bumped in the workspace `Cargo.toml` (`[workspace.package]`).
13. Completed RFCs moved from `rfcs/proposed/` to `rfcs/done/`; `rfcs/README.md` updated.
14. `ROADMAP.md` current state paragraph updated if the milestone is significant.

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
cargo xtask archive-layout "target/forskscope-v${VER}.tar.gz"
```

---

## Release artifacts

| File | Contents |
|------|----------|
| `forskscope-vX.Y.Z.tar.gz` | Source archive for the release |
| `forskscope-vX.Y.Z-linux-x86_64.tar.gz` | Linux x86_64 release binary |
| `forskscope-vX.Y.Z-macos-aarch64.dmg` | macOS aarch64 DMG |
| `forskscope-vX.Y.Z-windows-x64.zip` | Windows x64 release zip with README, license, notice, changelog, and executable |

---

## Version scheme

ForskScope uses semantic versioning (`MAJOR.MINOR.PATCH`). During the v0.x
pre-release phase:

- `PATCH` bumps for bug fixes and documentation updates within a stable feature set.
- `MINOR` bumps for new user-visible features or significant internal changes.
- `MAJOR` will be 1 when the first stable public release ships (RFC-041).

**Post-release bump default.** This document is the authoritative,
content-driven source for version level — not a mechanical roadmap rule. The
commit immediately after a release bumps the workspace version to the next
**patch** level by default. That satisfies the version invariant (the
workspace version must never equal an already-published tag) while claiming
nothing about the next release's content. At release time, once the
accumulated changes are visible, the owner confirms whether the level should
be promoted from patch to minor (or major). The level cannot be known before
the content is: an earlier roadmap rule that bumped to the next minor
mechanically, on every post-release commit, pre-committed a release to a
scope before that scope existed.

---

## Publication and immutability

> A version is **published** once its GitHub Release leaves draft state.
> Before that point the tag may be re-cut: delete the remote tag, re-tag the
> corrected commit, and record the re-cut in that version's CHANGELOG entry.
> After that point the version is immutable — supersede it with a new patch
> version. Never re-cut a tag whose release has left draft, even to fix a
> broken build.

**Tagged versus published is not the same line, on purpose.**
`cargo xtask version-sync`'s no-arg check keys on **tag existence** — the
workspace version must never equal an already-pushed tag. This policy defines
**published** as **out of draft**, a later and stricter line. That asymmetry
is intentional: a pushed tag is the practical, earliest point where a version
number can collide, so the automated check catches it there. Publication is
the point of no return for the release itself. Do not read the two as
contradictory or try to make `version-sync` key on draft state instead — the
tag check needs to run before any release exists to check.

**Caution:** `gh release delete --cleanup-tag` deletes the **remote tag** as
well as the release. The flag name reads as scoped to the release; it is not.
If cleaning up a mistakenly created release, delete the release only
(`gh release delete <tag>`) unless you specifically intend to also remove the
tag, and if the tag disappears unexpectedly, recover it from the local
annotated tag object (`git tag -l <tag>`) and re-push.

---

## After local artifact checks

1. Update `pkgver` in `packaging/linux/PKGBUILD` to match the workspace version.
   A comment in the file notes this requirement; failing to do so causes stale
   Arch packages, and `cargo xtask version-sync` requires this to already match
   before the release gates pass.
2. Tag the commit: `git tag -a ${VER} -m "Release ${VER}"`. Tags are unprefixed
   (`X.Y.Z`, no `v`) — the release workflow trigger only matches that form.
3. Push the tag. The release workflow builds the source and platform artifacts,
   composes release notes from the tag's `CHANGELOG.md` section, and creates a
   **draft** GitHub release. It does not publish anything by itself.
4. **Publish is a separate, explicit owner action — this is the approval gate,
   not a formality.** Inspect the draft release artifacts and composed notes,
   then publish:
   ```sh
   gh release edit "${VER}" --draft=false
   ```
   Before that command runs, the version is only tagged. After it runs, the
   version is published and immutable per the policy above.

---

## Checking the Rust edition and MSRV

The workspace `Cargo.toml` declares `rust-version = "1.91"` (the minimum
supported Rust version). Verify the build succeeds on the declared MSRV before
releasing.

```sh
rustup install 1.91
cargo +1.91 test -p forskscope-core -p forskscope-ui-logic
```
