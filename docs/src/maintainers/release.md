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
10. `CHANGELOG.md` updated with the new version and date.
11. `version` bumped in the workspace `Cargo.toml` (`[workspace.package]`),
    and **`xtask/Cargo.lock` staged with it** (F64). `xtask` has no
    dependencies, so that lock pins nothing and its only content is the
    version — but it is tracked, and any `cargo xtask` invocation after a
    bump rewrites it. Leave it out of the bump commit and the working tree
    goes dirty immediately, which is how a generated file gets swept into an
    unrelated commit. **This cannot be gated:** a `version-sync` check was
    written and removed, because running `cargo xtask` rebuilds xtask and
    repairs the lock *before* the check reads it — the gate could never fail.
12. Completed RFCs moved from `rfcs/proposed/` to `rfcs/done/`; `rfcs/README.md` updated.
13. `ROADMAP.md` current state paragraph updated if the milestone is significant.

---

## Source archive (F43: dropped)

ForskScope no longer builds its own source archive. It used to produce one
with the top-level parent directory stripped, so `PKGBUILD` could `cd
"$srcdir"` directly — but that stripped-prefix archive was otherwise
byte-for-byte identical to GitHub's own automatic per-tag source archive
(`https://github.com/forskscope/forskscope/archive/refs/tags/<tag>.tar.gz`),
and the custom build step's only stated justification (checksum stability)
never applied: `PKGBUILD`'s `sha256sums=('SKIP')` never checked one, and
`source=` was a bare local filename makepkg never fetched. `PKGBUILD` now
points at GitHub's tarball directly and `cd`s into its actual top-level
directory (`$pkgname-$pkgver`, GitHub's own naming), Arch's conventional
form. `packaging/build-release.sh` and `cargo xtask` no longer build or
verify a source archive at all.

---

## Release artifacts

| File | Contents |
|------|----------|
| `forskscope-vX.Y.Z-linux-x86_64.tar.gz` | Linux x86_64 release binary |
| `forskscope-vX.Y.Z-macos-aarch64.dmg` | macOS aarch64 DMG |
| `forskscope-vX.Y.Z-windows-x64.zip` | Windows x64 release zip with README, license, notice, changelog, and executable |

A source archive is still attached to every GitHub Release automatically —
GitHub generates one for every tag regardless of what this project does.

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
4. Refresh `PKGBUILD`'s `sha256sums` against the real, now-tagged source:
   `updpkgsums` (or `sha256sum` the tag tarball directly) once the tag from
   step 2 is pushed — GitHub's per-tag archive URL is fetchable immediately,
   before the release workflow finishes. `sha256sums=('SKIP')` is committed
   in the tree between releases because no real tag exists to hash yet; it
   must not stay `SKIP` once one does.
5a. **Publish to the AUR, by hand, on an Arch-family machine.** The AUR package
   is `forskscope`; publishing is a `git push` to
   `ssh://aur@aur.archlinux.org/forskscope.git` of a repository containing only
   `PKGBUILD` and a generated `.SRCINFO`.

   ```sh
   git clone ssh://aur@aur.archlinux.org/forskscope.git aur-forskscope
   cp packaging/linux/PKGBUILD aur-forskscope/
   cd aur-forskscope
   updpkgsums                                   # replaces SKIP with the real tag hash
   makepkg --printsrcinfo > .SRCINFO
   makepkg -si                                  # BUILD AND INSTALL IT — see below
   git commit -am "${VER}" && git push
   ```

   **`makepkg -si` is not optional.** A hand-published recipe has no CI behind
   it, and building it locally is the only thing that catches a packaging defect
   before every Arch user compiles it. `depends` omitted `xdotool` for three
   releases — the package installed cleanly and then failed to start — and no
   check would have found it, because nothing builds this package but you.

   **`sha256sums` must not be `SKIP` in what you push.** The committed template
   carries `SKIP` because `pkgver` names an untagged version between releases;
   `updpkgsums` fills it in against the tag you just pushed. RFC-081 §"What
   `sha256sums` is for" explains why it matters — briefly, it is the only
   integrity mechanism the AUR has.

   **`pkgrel` is `1` for a new version.** Bump it only when the recipe changes
   without a new release, and publish that the same way.

   Automating this is RFC-081, deferred.

5. **Publish is a separate, explicit owner action — this is the approval gate,
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
