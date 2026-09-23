# Review Request: Release Archive Contract

**Date:** 2026-07-09
**Reviewer stance:** release/process/packaging review
**Repository state:** inspect the current working tree directly

The reviewer should inspect the project directly. This request intentionally
does not include the whole codebase or full diff.

## Summary

Review the release archive contract implementation.

The previous repository state was contradictory:

- project rules required source archives to place files directly at archive root;
- release scripts, release workflow, docs, and Arch PKGBUILD expected a
  `forskscope-vX.Y.Z/` top-level directory.

This change resolves the contract in favor of the project rule: the source
archive has no top-level parent directory. Files unpack directly into the
extraction destination.

## Scope Followed

- Updated local release source archive creation to stop adding
  `forskscope-vX.Y.Z/`.
- Updated local release source archive creation to archive only tracked files.
- Added source archive layout verification to the local release script.
- Updated GitHub release workflow source archive creation to stop adding
  `forskscope-vX.Y.Z/`.
- Updated GitHub release workflow source archive creation to write into
  `$RUNNER_TEMP` with `git archive`, outside the archived repository root.
- Added source archive layout verification to the GitHub release workflow.
- Updated Arch PKGBUILD to build/package from `$srcdir` instead of
  `$pkgname-$pkgver`.
- Updated Arch PKGBUILD source filename to `forskscope-v$pkgver.tar.gz`.
- Updated packaging docs and maintainer release docs.
- Added archive-layout verification to the release checklist.

## Files Changed

Primary files to inspect:

- `.github/workflows/release.yml`
- `packaging/build-release.sh`
- `packaging/linux/PKGBUILD`
- `packaging/README.md`
- `docs/src/maintainers/release.md`

## Design Decisions And Assumptions

- The canonical contract is the existing project rule: source archive entries
  live directly at archive root.
- Archive filename remains versioned as `forskscope-vX.Y.Z.tar.gz`.
- The no-parent contract applies to the source archive. Platform binary
  archives are outside this decision.
- The local script and release workflow both verify:
  - `Cargo.toml` exists at archive root;
  - no `forskscope-vX.Y.Z/` top-level directory appears.
- The local script and release workflow reject archive self-inclusion and
  local-only/generated paths such as `.git-exclude/`, `.git/`, and `target/`.
- Arch PKGBUILD assumes makepkg extracts the source archive into `$srcdir`; it
  no longer changes into a project-version subdirectory.

## Tests And Gates Run

Observed passing after the implementation:

```text
bash -n packaging/build-release.sh
bash -n packaging/linux/PKGBUILD
mdbook build docs
git diff --check
```

Observed in-memory archive-layout verification:

```text
VER=$(awk '/^\[workspace\.package\]/{f=1} f&&/^version[[:space:]]*=/{gsub(/[^0-9.]/,"",$0); print; exit}' Cargo.toml)
git ls-files -z \
  | tar --null -czf - --files-from - \
  | tar -tzf - \
  | awk -v prefix="forskscope-v$VER" -v archive="forskscope-v$VER.tar.gz" '{ path=$0; sub(/^\.\//,"",path); if (path=="Cargo.toml") found=1; if (path==prefix || index(path,prefix"/")==1) bad=1; if (path==archive || path==".git-exclude" || index(path,".git-exclude/")==1 || path==".git" || index(path,".git/")==1 || path=="target" || index(path,"target/")==1) hygiene=1 } END { if (!found) exit 1; if (bad) exit 2; if (hygiene) exit 3 }'
```

This confirmed that `Cargo.toml` is present at archive root and that the
forbidden `forskscope-vX.Y.Z/` parent directory, archive self-entry, and local
ignored/generated paths are absent.

## Generated Artifacts

- `mdbook build docs` generated `docs/book/` and `rfcs/index.html`.
- Those generated files were removed from the final working tree.
- No release archive file was written to disk for this verification; the layout
  and content-hygiene check used a tracked-file tar pipeline.

## Known Limitations

- `packaging/build-release.sh` was syntax-checked and its archive-layout/content
  checks were verified in-memory, but the full script was not run end to end
  because it performs a release build.
- The GitHub workflow YAML was edited and inspected, but not executed.
- Arch `makepkg` was not run in this environment.
- This does not complete the broader CI/release gate alignment task; it only
  resolves and verifies the source archive layout contract.

## Recommended Next Step

Reviewer should verify:

1. The no-parent source archive contract is acceptable as the canonical release
   rule.
2. Local script and GitHub workflow checks correctly fail if a
   `forskscope-vX.Y.Z/` parent directory is reintroduced.
3. PKGBUILD behavior is correct for an archive that extracts files directly
   into `$srcdir`.
4. Docs and packaging instructions no longer imply source archives unpack to a
   versioned top-level directory.
