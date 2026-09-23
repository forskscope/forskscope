# Review Request: RFC-074 M2-A — C1 Follow-up

**Date:** 2026-08-02
**Reviewer stance:** targeted correction review
**Repository baseline:** `fe9940e` (`release: fail-closed on empty CHANGELOG
content, not byte length (C1)`)
**Repository state:** inspect the current working tree directly
**Responds to:** `dev-record/reviews/033-m2a-release-mechanics-review.md`,
mandatory correction C1

## Summary

Applies review 033's mandatory correction. `test -s release-notes.md` tested
byte length, not content; a heading-only CHANGELOG section — exactly what the
post-release bump opens by design, and all `version-sync` asserts is that the
heading exists — extracts to a single newline, which is one byte and passes
`test -s`. Because the compare link is appended after the guard, that
composes to a blank line plus the compare link: the bare-compare-link defect
F22 exists to eliminate, reproduced inside its own fix. This was reachable at
the very next post-release tag, not a contrived case — the review's own
example was the `0.165.1` section already sitting in this tree.

Replaced the guard with `grep -q '[^[:space:]]' release-notes.md`, per the
handoff amendment the architect already landed (§4.1, §5). This is a short,
targeted change; no other part of the M2-A slice is reopened.

## Addressed Item

- **C1** — fail-closed guard now tests for actual content, not byte presence.

## Files Changed

- `.github/workflows/release.yml` — one line: the guard condition.

Also committed (already amended by the architect ahead of this change, per
review 033's own recommended next action, not authored by this patch):
`ROADMAP.md` (F23 registered), and
`rfcs/handoffs/074-v1-release-stabilization-program/m2a-release-mechanics-handoff.md`
(§4.1/§5 amended to specify the content-based guard and the fifth test case).

## Tests And Gates Run

Observed on `fe9940e`:

```text
cargo fmt --check                                            pass
cargo xtask version-sync                                     pass — v0.165.1
cargo xtask css --check                                      pass
cargo xtask i18n                                              pass — 203 keys
cargo xtask audit-deps                                        pass
cargo test -p forskscope-core -p forskscope-ui-logic          pass — 943/943
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
```

CI run `30741678224`: Test & Lint green in 4m22s, all 17 steps pass.

**All five required extraction cases**, re-run against the committed
`CHANGELOG.md` with the new `grep -q` guard:

```text
ver=0.165.0 (published section, has content)
  -> 5850 bytes; grep -q: PASS

ver=0.165.1 (heading-only section — this is C1's exact reproduction case)
  -> 1 byte; grep -q: FAIL -> guard exits non-zero (was the bug: previously
     passed test -s)

ver=9.9.9 (no matching heading)
  -> 0 bytes; grep -q: FAIL -> guard exits non-zero

ver=0X165X0 (wildcard-shaped, adversarial)
  -> 0 bytes; grep -q: FAIL -> guard exits non-zero
```

Also re-ran the full step body end-to-end with `GITHUB_REF_NAME=0.165.1`:
confirmed the guard now emits
`::error::CHANGELOG section for 0.165.1 is missing or empty` and exits 1,
where it previously would have silently proceeded to compose a
blank-line-plus-compare-link body.

## Known Limitations

Unchanged from the parent request (`029-m2a-release-mechanics.md`): the
notes-extraction change is still proven locally only; end-to-end evidence
(a real tag push producing a draft release body) arrives at M2-B's cut. N1
(`release.yml` YAML validity unproven — no linter available in this
environment) is unchanged; F23 is registered against M4 to close it
structurally via `actionlint` in CI.

## Requested Review Focus

Whether the single-line fix and the five-case re-verification are sufficient
to close C1, or whether anything else in the notes-composition step should be
re-examined now that the guard's failure mode is understood.
