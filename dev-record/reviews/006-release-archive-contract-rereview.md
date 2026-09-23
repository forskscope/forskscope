# Release archive contract re-review

Reviewed:

- `dev-record/review-requests/004-release-archive-contract.md`
- `.github/workflows/release.yml`
- `packaging/build-release.sh`
- `packaging/linux/PKGBUILD`
- `packaging/README.md`
- `docs/src/maintainers/release.md`
- Prior review: `dev-record/reviews/005-release-archive-contract-review.md`

## Verdict

Accept with notes.

The two blocking findings from the previous review have been addressed. The
source archive contract is now consistently no-parent-directory, the local
release script archives tracked files only, and the GitHub workflow writes the
source archive outside the repository root via `$RUNNER_TEMP`.

## Blocking findings

None.

## Non-blocking findings

1. The maintainer manual command is Bash-specific while the fenced block is
   marked `sh`.

   `docs/src/maintainers/release.md:38` uses process substitution:

   ```sh
   --files-from <(git ls-files -z)
   ```

   That is fine for Bash, but not POSIX `sh`. Either mark the block as `bash` or
   use the pipeline form from `packaging/build-release.sh:35`:
   `git ls-files -z | tar --null -czf "$ARCHIVE" --files-from -`.

2. The release docs still describe the source archive primarily as the Cargo
   workspace excluding `target`.

   `docs/src/maintainers/release.md:21` is directionally correct for users, but
   the implementation now more precisely creates a tracked-file source archive.
   Consider saying "tracked workspace files" so the docs match the `git
   ls-files` and `git archive` behavior.

## Requirement checks

- Canonical no-parent source archive contract: pass. Public maintainer and
  packaging docs now say files unpack directly into the extraction destination.
- Local script no longer adds `forskscope-vX.Y.Z/`: pass.
  `packaging/build-release.sh:35` archives `git ls-files` directly with no
  transform.
- Local script no longer includes ignored local-only paths: pass.
  The tracked-file tar pipeline excludes `.git-exclude/`, `.git/`, and `target/`
  in the observed in-memory check.
- Local script layout/content verification: pass. `packaging/build-release.sh:36`
  checks root `Cargo.toml`, `packaging/build-release.sh:43` rejects a
  `forskscope-v$VER/` parent, and `packaging/build-release.sh:50` rejects
  archive self-entry plus generated/local-only paths.
- GitHub workflow avoids archive self-inclusion: pass. `.github/workflows/release.yml:30`
  uses `git archive` and writes the output to `$RUNNER_TEMP`, outside the
  repository root being archived.
- GitHub workflow layout/content verification: pass. `.github/workflows/release.yml:38`
  checks root `Cargo.toml`, `.github/workflows/release.yml:45` rejects the
  versioned parent, and `.github/workflows/release.yml:52` rejects self-entry
  and local/generated paths.
- Arch PKGBUILD no-parent behavior: pass. `packaging/linux/PKGBUILD:18` and
  `packaging/linux/PKGBUILD:23` build and package from `$srcdir`, matching a
  source archive that extracts files directly into `$srcdir`.

## Observed evidence

Commands observed during this re-review pass:

- `bash -n packaging/build-release.sh` - passed.
- `bash -n packaging/linux/PKGBUILD` - passed.
- `git diff --check` - passed.
- In-memory local tracked-file archive check - passed:
  - `Cargo.toml` present at archive root.
  - No `forskscope-vX.Y.Z/` parent.
  - No archive self-entry.
  - No `.git-exclude/`, `.git/`, or `target/` entries.
- `git archive --format=tar.gz HEAD` inspection - passed for root
  `Cargo.toml` and no `.git-exclude/`, `.git/`, or `target/` entries.
- `mdbook build docs` - passed.

Generated docs output from `mdbook build docs` (`docs/book/` and
`rfcs/index.html`) was removed after verification.

## Missing evidence

- `packaging/build-release.sh` was not run end to end because it performs a
  release build.
- The GitHub release workflow was not executed.
- Arch `makepkg` was not run.

## Recommended next action

Accept this release archive contract change. Before final docs freeze, adjust
the manual archive command fence or syntax so it is clearly Bash-compatible, and
consider describing the source archive as tracked workspace files rather than
only "excluding target".
