# Release Process

## Planning a cut

Before choosing a version number or a theme, run:

```
git log <last tag>..HEAD
```

**This is the first step of planning, not a retrospective check.** `0.172.1`
was planned as a security patch with scope frozen to three findings, while
the branch already carried two releases' worth of finished work by the time
it was cut (F146, from review 131) — a tag on `main` ships the tree, not the
plan, so that patch was never what the cut would actually have produced.
Name the release's theme from what this command shows is actually there,
rather than from whatever was most recently worked on.

---

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
14. F109: dispatch `windows-check.yml` (`gh workflow run windows-check.yml`)
    for the release commit and confirm the run is green — proves the
    WebView2 detection check on a real `windows-latest` runner before the
    release build ships it.

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

> A version is **published** once its GitHub Release exists and is not a
> draft. `release.yml` creates it already published (F156), so that is the
> moment the release workflow finishes.
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

   **If the tag is pushed and no draft appears, do not re-cut the tag.** A
   pushed tag with no draft means one of `release.yml`'s jobs failed —
   `release.md`'s other recovery advice assumes a draft already exists,
   which is not this state. Check which job failed:
   ```sh
   gh run rerun <run-id> --failed
   ```
   re-runs only the failed jobs, keeping the platform builds that already
   succeeded — a full re-cut would rebuild everything and burn a second
   tag for a problem that was never in the code. **Re-cut the tag only if
   the fix requires a code change** — `F102` (0.170.0's cut: `render.yml`'s
   F34 step launched the app while `xvfb`'s X server was still starting,
   and the check reported "never registered" for a process that had
   already exited) established this exact shape as environmental rather
   than a regression, and `render_check.py` now retries a start-up crash
   itself (a small, bounded number of attempts, logging each one) — so a
   plain re-run is the first thing to try, not a last resort.
4. **The release publishes itself — there is no approval gate on *visibility*
   (F156).** `release.yml` creates the release already published, so the
   version becomes immutable the moment the workflow's last job succeeds.
   Inspect the artifacts and composed notes **after** the fact; if something is
   wrong, supersede it with a new patch version, because the published release
   cannot be re-cut.

   > **The owner authorised this on 2026-10-01.** The draft previously held
   > each release until the owner published it, which is the approval gate
   > RFC-079 and RFC-081 both rest on. It was removed because a draft is
   > invisible on the repository front page and `releases/latest` does not
   > resolve to one, so releases were being missed.
   >
   > **The gate moved rather than disappeared (F147/F158, handoff 061).**
   > `aur-publish.yml` and `store-submit.yml` no longer trigger on this release
   > existing — neither has a `release:` trigger at all, and F158 found that a
   > `GITHUB_TOKEN`-authored publish event never started them in the first
   > place, so there was nothing actually guarding this. Each now runs only on
   > an explicit `workflow_dispatch`: see steps 5 and 6 below. The gate is on
   > distribution, not visibility — exactly the trade this paragraph used to
   > say still needed making.
5. **Publish to the AUR by dispatching `.github/workflows/aur-publish.yml`
   yourself** (RFC-081) — publishing the release above does not start it
   (F147/F158, handoff 061):

   ```sh
   gh workflow run aur-publish.yml -f mode=release -f tag=${VER} -f dry_run=false
   ```

   **Rehearse first if in doubt**: the same command with `dry_run=true` (the
   default) runs every check below and stops before the push. The `validate`
   job checks out `packaging/linux/PKGBUILD` as it existed at the tag (not
   whatever `main` has moved to since — the post-release bump usually lands
   within minutes of the tag being pushed), computes the real source hash from
   the tag's own GitHub archive, and refuses to proceed if `pkgver` does not
   match the release or `pkgrel` is not `1`. Only then does it build the
   package (`makepkg --syncdeps`), install it (`pacman -U`), and run `namcap`
   on both the recipe and the built package — the same check that would have
   caught F81's missing `xdotool` `depends` entry, which three hand-published
   releases did not. A failure at any of these steps leaves the AUR untouched.
   Watch it run under the "AUR Publish" workflow in the Actions tab, or check
   the [`forskscope` AUR page](https://aur.archlinux.org/packages/forskscope)
   directly. A scheduled check (`audit.yml`) reports if the AUR ever falls
   behind the latest release — it would have caught the three-release drift
   between `0.170.1` and `0.173.0` on its first run.

   ### Pushing to the AUR by hand

   **Why this is here.** This is the fallback for when the workflow above
   fails for some other reason — a GitHub outage, a changed AUR host key, a
   new validation failure worth looking at by hand before retrying — not the
   expected path. (It was the *only* path through F147: `publish` read
   `secrets.AUR_SSH_KEY`, a name defined in no scope, and the artifact it
   downloaded never contained `.SRCINFO`, because `actions/upload-artifact`
   defaults to `include-hidden-files: false` and the file begins with a dot.
   Both are fixed.) **The validation is not wasted** — `validate` still
   builds, installs and `namcap`s the package in an Arch container, which is
   the part that protects the AUR, and the recipe pushed below is the one it
   validated.

   > **These blocks are POSIX shell.** If your login shell is `fish`, as this
   > project's maintainer's is, start one first — `bash` — and run them there.
   > `VAR=value`, `${VAR}`, `if … then … fi` and `unset` all differ in fish, and
   > the variables set in step 1 are used by every later step, so they have to
   > share one shell session.

   Everything below happens inside one throwaway directory, so the last step
   removes every trace in one command. Run the stages in order and look at the
   output between them — the push reaches a public registry and is not worth
   pasting blind.

   **1 — collect the validated recipe.**

   ```sh
   VER=0.174.0                                   # the version you just published
   WORK=$(mktemp -d)                             # never empty, so step 6 is safe
   RUN=$(gh run list --workflow=aur-publish.yml --limit 1 --json databaseId \
         --jq '.[0].databaseId')

   gh run download "$RUN" -n aur-recipe -D "$WORK"
   ```

   `validate` will be green on that run and `publish` red: that is the expected
   shape today. **`aur-recipe` is a workflow artifact, not a release asset** — it
   is not attached to the GitHub Release and will not appear there. In a browser
   it is at the bottom of the run's summary page, under *Artifacts*.

   **2 — regenerate the file the artifact cannot carry, and check it.**

   ```sh
   cd "$WORK"
   makepkg --printsrcinfo > .SRCINFO

   curl -sL -o "$WORK/tag.tar.gz" \
     "https://github.com/forskscope/forskscope/archive/refs/tags/${VER}.tar.gz"

   grep -E '^(pkgver|pkgrel)=' PKGBUILD
   if [ "$(sha256sum "$WORK/tag.tar.gz" | cut -d' ' -f1)" \
        = "$(grep -oP "(?<=sha256sums=\(')[0-9a-f]{64}" PKGBUILD)" ]; then
       echo "source hash matches the published tag"
   else
       echo "HASH MISMATCH - do not push"
   fi
   ```

   `pkgver` must equal the release and `pkgrel` must be `1`. The hash is compared
   rather than printed for you to match by eye, because two 64-character strings
   are exactly what an eye skips.

   **3 — look at what you are about to publish.**

   ```sh
   cat "$WORK/PKGBUILD" "$WORK/.SRCINFO"
   ```

   **4 — push, and only these two files.**

   ```sh
   git clone ssh://aur@aur.archlinux.org/forskscope.git "$WORK/aur"
   cp "$WORK/PKGBUILD" "$WORK/.SRCINFO" "$WORK/aur/"
   git -C "$WORK/aur" add PKGBUILD .SRCINFO
   git -C "$WORK/aur" commit -m "${VER}-1"
   git -C "$WORK/aur" push
   ```

   **5 — confirm it landed.** Two caches sit between a successful push and what
   you see, and **both will show you the previous version**:

   - the **RPC listing** (`rpc/v5/info`) rebuilds on a delay — do not use it here;
   - and **cgit itself serves a cached page** for a short while after a push.

   So read the package's git state *with a cache-busting parameter*. Without the
   `&_=1`, a confirmation run within a minute of a successful push reports the old
   version and reads exactly like a failed push:

   ```sh
   curl -s 'https://aur.archlinux.org/cgit/aur.git/plain/.SRCINFO?h=forskscope&_=1' \
     | head -4
   ```

   Observed on the `0.177.0` and `0.178.0` publishes: the plain URL returned the
   previous `pkgver` immediately after a push whose workflow had reported success
   on both jobs; the same URL with a parameter returned the new one first try.

   **6 — clean up.** The clone, the downloaded archive and the recipe all live
   under `$WORK`, so one command removes them:

   ```sh
   rm -rf "$WORK" && unset WORK RUN VER
   ```

   Run it from outside that directory — `cd ~` first if step 2's `cd` left you
   inside it. If the shell has been closed since, `mktemp -d` paths live under
   `/tmp` and go on the next reboot regardless.

   **If every dispatch above is skipped, nothing fails loudly until the next
   day.** The AUR simply stays on the previous version — it sat three
   releases behind between `0.170.1` and `0.173.0` for exactly that reason,
   with nothing reporting it at the time. `audit.yml`'s scheduled check now
   catches this within a day either way (F147).
6. **Submit to the Microsoft Store by dispatching
   `.github/workflows/store-submit.yml` yourself** (RFC-079) — publishing the
   release above does not start it either (F147/F158, handoff 061):

   ```sh
   gh workflow run store-submit.yml -f tag=${VER} -f dry_run=false
   ```

   **This still returns `Unauthorized`** until the Partner Center tenant
   association exists (F106, open and unrelated to this dispatch mechanism),
   so Store submissions are made by hand in Partner Center today (F108), using
   the listing copy kept with the owner's working files. What follows
   describes the dispatched path as designed, and applies once that account
   work is done. It checks out
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
version, exactly F81's `xdotool` fix — uses the same `aur-publish.yml`
dispatch as step 5 above, with a different `mode`: `recipe-fix`, which is also
the input's default, so it needs no flag of its own. Run it once the
`pkgrel` bump is committed to `main`:

```sh
gh workflow run aur-publish.yml -f dry_run=false
```

It runs every check `mode=release` does, against `main`'s current
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

Recovering from a rejected Store submission, or a packaging-only fix, uses the
same `store-submit.yml` dispatch as step 6 above, against the already-published
tag that needs resubmitting:

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
