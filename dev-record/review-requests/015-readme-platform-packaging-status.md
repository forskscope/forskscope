# Review request: README platform packaging status

## Scope

This package closes the non-blocking README wording note from
`dev-record/reviews/009-docs-refresh-review.md`.

## Files to inspect

- `README.md`

## Change summary

- Changed the platform-packaging sentence from:
  - `targeting Linux, macOS, and Windows packaging`
- to:
  - `with Linux, macOS, and Windows packaging under release-readiness verification`

## Why this changed

`ROADMAP.md` correctly says runtime/platform verification remains, including
packaging verification on Linux, macOS, and Windows. The README should not imply
that all platform packages are already verified before that release-readiness
lane is closed.

## Observed verification

Passed in this thread:

- `git diff --check`
- Stale wording scan over `README.md`, `ROADMAP.md`, and `docs/src`:
  - no matches for `packaged for Linux`
  - no matches for `packaged for`
  - no matches for `targeting Linux, macOS, and Windows packaging`
  - `README.md` now contains `under release-readiness verification`

## Limitations

- This is a README wording change only.
- No runtime/platform verification was performed for this package.

## Reviewer questions

- Does the README wording now accurately match the current release-readiness
  boundary?
- Is the phrase concise enough for the README while avoiding an overclaim about
  verified platform packages?
