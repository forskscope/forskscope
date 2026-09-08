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
   before the release gates pass. Leave `pkgrel=1` and `sha256sums=('SKIP')`
   exactly as they are — both are filled in automatically at publish time (see
   step 5 below) and neither should ever be edited by hand in this file.
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
5. **Publishing the release triggers `.github/workflows/aur-publish.yml`
   automatically** (RFC-081) — nothing further to do by hand. It checks out
   `packaging/linux/PKGBUILD` as it existed at the tag (not whatever `main` has
   moved to since — the post-release bump usually lands within minutes of the
   tag being pushed, long before the owner publishes the draft), computes the
   real source hash from the tag's own GitHub archive, and refuses to proceed
   if `pkgver` does not match the release or `pkgrel` is not `1`. Only then does
   it build the package (`makepkg --syncdeps`), install it (`pacman -U`), and
   run `namcap` on both the recipe and the built package — the same check that
   would have caught F81's missing `xdotool` `depends` entry, which three
   hand-published releases did not. A failure at any of these steps leaves the
   AUR untouched. Only `PKGBUILD` and a freshly generated `.SRCINFO` are ever
   pushed; watch it run under the "AUR Publish" workflow in the Actions tab, or
   check the [`forskscope` AUR page](https://aur.archlinux.org/packages/forskscope)
   directly once it finishes.
6. **Publishing the release also triggers `.github/workflows/store-submit.yml`
   automatically** (RFC-079) — nothing further to do by hand. It checks out
   the released tag, builds the MSIX, validates it (manifest version against
   the tag, `Identity`/`Publisher`/`PublisherDisplayName` against the tracked
   Store identity, every manifest-referenced asset present, and — the
   expensive check — **installs and launches the package for real**, signed
   with a throwaway validation-only certificate the workflow generates and
   discards; the real, unsigned MSIX that gets uploaded is never touched by
   that signing step), then submits through the Microsoft Store submission
   API. **This is submission, not publication** — Microsoft's certification
   runs asynchronously and can take hours to days, so the workflow reports a
   submission ID and its status at the time polling ended and stops; watch
   [Partner Center](https://partner.microsoft.com/dashboard) for the
   certification outcome. A failure before submission leaves Partner Center
   untouched; a rejection after submission arrives by mail and is a human
   recovery (see below) — never fixed by editing the published GitHub
   release, which is immutable.

   > ### Do not open an automated submission in Partner Center
   >
   > **Once the API creates a submission, change it only through the API.**
   > This is Microsoft's own constraint, not a project convention:
   >
   > > If you use Partner Center to change a submission that you originally
   > > created by using the API, **you will no longer be able to change or
   > > commit that submission by using the API**. In some cases, the submission
   > > could be left in an error state where it cannot proceed … you must
   > > delete the submission and create a new submission.
   >
   > **This is easy to walk into precisely because the manual habit is the
   > right one everywhere else.** Every release before RFC-079 was published by
   > opening Partner Center and working there, so opening an automated
   > submission to adjust one field is the natural reflex — and it strands the
   > submission, requiring deletion and a fresh one.
   >
   > **Reading Partner Center is fine**, and is what step 6 above tells you to
   > do for the certification outcome. **Editing a submission the API created
   > is not.** If something needs changing, either re-run the workflow (see
   > *Resubmitting to the Store*) or delete the submission in Partner Center
   > first and then create a new one — never edit in place.
   >
   > Recorded as **F106(a)**, from Microsoft's submission-API prerequisites.

## A packaging-only fix, with no new release

Bumping `pkgrel` — for a `PKGBUILD` change that does not need a new upstream
version, exactly F81's `xdotool` fix — has no route through the steps above,
because nothing in them fires without a release. Run the same workflow by hand
instead, once the `pkgrel` bump is committed to `main`:

```sh
gh workflow run aur-publish.yml -f dry_run=false
```

It runs every check the release path does, against `main`'s current
`PKGBUILD` — except `pkgver` must equal what the AUR **already** carries (a
recipe fix never changes the upstream version; cut a release instead if it
does), and `pkgrel` must be strictly greater than the AUR's. Automation never
writes either value — only a human commit does, and the workflow's only job is
to refuse a wrong one (RFC-081 Q3).

**Rehearse first if in doubt**: `gh workflow run aur-publish.yml -f
dry_run=true` (the default — omitting `-f dry_run` does this) runs the
identical build, install, and `namcap` checks and stops before the push. It is
the same code path with one fewer step, not a separate one that could pass
while the real path fails.

## Resubmitting to the Store, with no new release

Recovering from a rejected Store submission, or a packaging-only fix, has no
route through the automatic trigger either — nothing in
`store-submit.yml` fires without a **new** release being published. Run it by
hand against the already-published tag instead:

```sh
gh workflow run store-submit.yml -f tag=0.170.1 -f dry_run=false
```

It builds and validates exactly as the release trigger does, then deletes
any existing pending submission for the app before creating a new one — a
second run never leaves two submissions in flight, it replaces the first.

**Rehearse first if in doubt**: `gh workflow run store-submit.yml -f
tag=0.170.1 -f dry_run=true` (the default) authenticates against Partner
Center and reads the application's current state — proving the credential
and connectivity — then stops before creating, uploading, or committing
anything. Unlike the AUR path, this dry run still needs the real credential:
Partner Center has no anonymous read the way the AUR's git remote does, so
there is no zero-credential way to prove the connection works.

---

## Checking the Rust edition and MSRV

The workspace `Cargo.toml` declares `rust-version = "1.91"` (the minimum
supported Rust version). Verify the build succeeds on the declared MSRV before
releasing.

```sh
rustup install 1.91
cargo +1.91 test -p forskscope-core -p forskscope-ui-logic
```
