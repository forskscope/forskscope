# Release archive contract review

Reviewed:

- `dev-record/review-requests/004-release-archive-contract.md`
- `.github/workflows/release.yml`
- `packaging/build-release.sh`
- `packaging/linux/PKGBUILD`
- `packaging/README.md`
- `docs/src/maintainers/release.md`

## Verdict

Needs changes.

The no-parent source archive decision is coherent, and the PKGBUILD/docs mostly
follow that contract. However, the archive creation commands still have release
artifact integrity problems that the new layout checks do not catch.

## Blocking findings

1. The GitHub release workflow can include the source archive inside itself.

   `.github/workflows/release.yml:30` creates `forskscope-v${VER}.tar.gz` in the
   repository root while archiving `.`. A scratch reproduction of this exact tar
   shape exited successfully and listed the output archive as an entry:

   ```text
   ./
   ./Cargo.toml
   ./forskscope-v0.0.0.tar.gz
   ```

   The current verification at `.github/workflows/release.yml:36` only checks
   that `Cargo.toml` is at archive root and that `forskscope-v${VER}/` is absent,
   so it would not reject this self-embedded archive. Write the archive outside
   the directory being archived, or explicitly exclude the output filename and
   add a verification check that the archive does not contain itself.

2. The local release script archives ignored local review/temp files.

   `packaging/build-release.sh:35` archives `.` and excludes only `./target` and
   `./.git`. In this checkout, `.git-exclude/` is ignored by `.gitignore:17` and
   not tracked by Git, but the in-memory archive-layout check still included:

   ```text
   .git-exclude/
   .git-exclude/tasks/
   dev-record/reviews/
   dev-record/review-requests/
   .git-exclude/tmp/
   ```

   This violates release artifact hygiene for the local source archive path and
   means the documented local script can ship private review/process artifacts or
   temp files while still passing the new root-layout check. Build source
   archives from tracked files, for example with `git archive`, or add explicit
   exclusions for ignored/local-only paths and verify no ignored paths are
   present.

## Non-blocking findings

1. The negative parent-directory check fails, but not always for the diagnostic
   reason shown in the implementation.

   When the old `--transform "s|^\./|forskscope-v$VER/|"` behavior is
   simulated, the combined in-memory check exits because `Cargo.toml` is no
   longer at archive root before reporting the forbidden parent directory. That
   is acceptable for enforcement, but if precise diagnostics matter, combine the
   two checks into one pass that reports both conditions.

2. The no-parent PKGBUILD shape is acceptable as long as the source array stays
   single-source.

   `packaging/linux/PKGBUILD:18` and `packaging/linux/PKGBUILD:23` now build and
   package from `$srcdir`, which matches a no-parent source tarball. If future
   Arch packaging adds extra source files that also extract or copy into
   `$srcdir`, revisit this because root-level source extraction has higher
   collision risk than a versioned source directory.

## Requirement checks

- Canonical no-parent source archive contract: pass as a decision. The docs and
  scripts now consistently describe files at archive root.
- Local script no longer adds `forskscope-vX.Y.Z/`: pass, but blocked by ignored
  local-file inclusion.
- Local script layout verification: partial. It checks root `Cargo.toml` and
  absence of the versioned parent, but does not verify tracked-source hygiene.
- GitHub workflow no longer adds `forskscope-vX.Y.Z/`: pass, but blocked by
  self-inclusion risk.
- GitHub workflow layout verification: partial. It checks root `Cargo.toml` and
  absence of the versioned parent, but does not reject self-embedded archives.
- Arch PKGBUILD builds from `$srcdir`: pass for the current single-source
  package.
- Docs no longer imply a versioned top-level source directory: pass for the
  touched public maintainer/packaging docs.

## Observed evidence

Commands observed during this review pass:

- `bash -n packaging/build-release.sh` - passed.
- `bash -n packaging/linux/PKGBUILD` - passed.
- `git diff --check` - passed.
- In-memory current-layout check - passed for root `Cargo.toml` and no
  `forskscope-vX.Y.Z/` parent, but showed `.git-exclude/` was included.
- Negative parent simulation - failed the layout check when the old versioned
  parent transform was reintroduced.
- Scratch tar self-output reproduction - tar exited successfully and included
  the output archive in the archive listing when the output file was created
  inside the archived root.

## Missing evidence

- `packaging/build-release.sh` was not run end-to-end because it performs a
  release build.
- The GitHub release workflow was not executed.
- `makepkg` was not run.
- `mdbook build docs` was not rerun in this review pass to avoid generating
  docs output while the archive blockers are still open.

## Recommended next action

Keep the no-parent archive contract, but fix archive construction before
accepting this change:

- In the GitHub workflow, create the source tarball outside the archived root or
  exclude the archive output filename and verify it is absent from the archive.
- In the local script and manual docs, build the source archive from tracked
  files or explicitly exclude ignored/local-only paths such as `.git-exclude/`.
- Extend the layout check to enforce those content-hygiene rules, not just root
  `Cargo.toml` and absence of `forskscope-vX.Y.Z/`.
