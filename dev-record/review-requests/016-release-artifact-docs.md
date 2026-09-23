# Review request: release artifact docs

## Scope

This package aligns the maintainer release documentation with the platform
artifacts now produced by the release workflow.

## Files to inspect

- `docs/src/maintainers/release.md`

## Change summary

- Renamed `Archive naming` to `Release artifacts`.
- Kept the source archive entry.
- Added entries for:
  - `forskscope-vX.Y.Z-linux-x86_64.tar.gz`
  - `forskscope-vX.Y.Z-macos-aarch64.dmg`
  - `forskscope-vX.Y.Z-windows-x64.zip`
- Changed `After the archive` to `After local artifact checks`.
- Updated the post-build order to match the tag-triggered release workflow:
  - tag the commit
  - push the tag
  - inspect the draft GitHub release artifacts created by the workflow
  - update the Arch `pkgver` follow-up

## Why this changed

The release workflow now publishes more than the source archive:

- Linux binary tarball
- macOS DMG
- Windows zip with executable plus README/license/notice/changelog

The maintainer release doc still described only the source archive. This update
keeps the release process documentation aligned with the current workflow and
packaging helpers.

## Observed verification

Passed in this thread:

- `git diff --check`
- `mdbook build docs`
- Release-artifact reference scan across:
  - `docs/src/maintainers/release.md`
  - `packaging/README.md`
  - `.github/workflows/release.yml`
- Process-order scan confirmed:
  - `docs/src/maintainers/release.md` now has `After local artifact checks`
  - tag creation appears before draft release inspection
  - the old "upload before tag" wording is gone

## Limitations

- This is documentation alignment only.
- No live GitHub Actions release workflow was observed.
- No platform runtime/package verification was performed for this package.

## Reviewer questions

- Does the release artifact table now accurately reflect the workflow outputs?
- Is the Windows artifact contents description specific enough without making
  the table too verbose?
