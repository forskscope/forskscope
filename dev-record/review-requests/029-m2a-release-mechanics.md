# Review Request: RFC-074 M2-A — Release Mechanics

**Date:** 2026-08-02
**Reviewer stance:** release-pipeline / documentation-currency review
**Repository baseline:** `896f2c6` (`release: align trigger and gates for v1
stabilization (F19-F22, N5-N6)`)
**Repository state:** inspect the current working tree directly

## Summary

Implements RFC-074 milestone M2-A per
`rfcs/handoffs/074-v1-release-stabilization-program/m2a-release-mechanics-handoff.md`.
This is the third release-mechanics defect found in a row, all sharing one
shape — something configured plausibly, never exercised, credited as
working: R0 found the tag trigger never fired; a follow-on owner challenge
found the post-release version bump was mechanical rather than
content-driven; this slice finds `generate_release_notes: true` would have
shipped every release with empty notes, since this project commits directly
to `main` rather than merging PRs. M2-A treats the release pipeline as one
unit and fixes it, plus lands review 032's remaining documentation-currency
findings (N1–N4 were already corrected in the working tree ahead of this
slice; N5–N6 are addressed here).

This slice changes no product behaviour, cuts no release, and closes no
audit blocker. B2, B3, and B4 remain open; v1/public release stays **No-Go**.

## Addressed Items

- **F22** — release-notes composition sourced from `CHANGELOG.md` instead of
  `generate_release_notes: true`.
- **F19, F21** — `release.md`: publication/immutability policy, content-driven
  version-level rule with patch default, explicit publish-step command,
  `--cleanup-tag` caution, tagged-vs-published asymmetry stated explicitly.
- **F20** — `threat-model.md`: RFC-075 audit-history row superseding the
  `v0.148.0` claim; stale `(v0.164.0)` heading corrected.
- **N5** — `git_lines`'s doc comment corrected to describe what the code
  actually does (it doesn't trim or filter; `git tag` output never needs it).
- **N6** — the published-tag-check failure message reworded from "already
  published" to "already tagged", matching what the check actually observes
  (local tag existence, not GitHub Release publication state).

## Files Changed

- `.github/workflows/release.yml` — `Create GitHub Release` job: checkout
  added before `download-artifact`; new "Compose release notes from
  CHANGELOG" step; `body_path` replaces `generate_release_notes`
- `docs/src/maintainers/release.md` — new "Publication and immutability"
  section; post-release-bump-default paragraph under "Version scheme";
  publish step now documents the `gh release edit --draft=false` command
- `docs/src/maintainers/threat-model.md` — new `v0.165.0` audit-history row;
  `## Data flows and controls` heading corrected to `(v0.165.0)`
- `xtask/src/main.rs` — `git_lines` doc comment; fail-message wording (6
  lines changed net; ELOC unchanged at 498)
- `rfcs/handoffs/074-v1-release-stabilization-program/m2a-release-mechanics-handoff.md`
  (new) — the handoff itself, as found; not authored by this patch

Also in this commit, carried from pre-staged working-tree edits present
before this slice began (the owner/architect's M2-A planning record,
including the `0.166.0` → `0.165.1` post-release-bump correction that
motivated F19's version-level rule, and N1–N4's ROADMAP/threat-model
currency fixes not itself scoped to this handoff): `CHANGELOG.md`,
`Cargo.lock`, `Cargo.toml`, `ROADMAP.md`, `packaging/linux/PKGBUILD`,
`packaging/windows/AppxManifest.xml`, `rfcs/README.md`,
`rfcs/proposed/074-v1-release-stabilization-program.md`, `xtask/Cargo.toml`.

Not staged: `xtask/Cargo.lock` (untracked, pre-existing convention, unrelated
to this slice).

## Important Implementation Decisions

- **Checkout-before-download-artifact ordering.** `actions/checkout` cleans
  the job workspace by default. The prior job had no checkout at all (it only
  downloaded artifacts), so adding one had to go *before*
  `actions/download-artifact` or the freshly-downloaded platform artifacts
  would be wiped immediately after landing — exactly the failure mode the
  handoff flagged as "the single most likely way to break this job." Verified
  by re-reading the resulting step order, not just by construction.
- **Literal-prefix match, not regex.** `index($0, hdr) == 1` with a
  pre-built literal header string (`"## [" ver "]"`), rather than
  `$0 ~ "^## \\[" ver "\\]"`. The regex form would treat the dots in a
  version number as wildcards (`0.166.0` matching `0X166X0`); verified this
  is not just a theoretical concern by constructing that exact adversarial
  input and confirming it produces empty output (see gates below).
- **`git tag --sort=-creatordate` for the compare-link's previous tag**,
  rather than semver-aware ordering. This project's numbering is not strictly
  monotonic in a way a simple tool could parse reliably (patch levels are
  decided at release time), and creation-date order is what actually answers
  "what shipped immediately before this." Not specified by the handoff;
  chosen because `generate_release_notes` needed replacing and the compare
  link was worth preserving per §4.1's "preserve the compare link"
  requirement, which named the requirement but not the mechanism.
- **`fetch-depth: 0` on the new checkout**, so `git tag --sort=-creatordate`
  can see full tag history rather than a shallow default clone.
- Left `docs/src/maintainers/release.md`'s existing `## Version scheme`
  section's `PATCH`/`MINOR`/`MAJOR` semantic-versioning description
  unchanged; added the post-release-bump-default rule as a new paragraph
  immediately after it, since the two describe different things (what a
  version bump *means* versus what the *next* commit after a release
  defaults to before content is known).

## Differences From the Handoff

None identified. The handoff's exact `awk` script, fail-closed guard, and
policy wording (§4.1, §4.3) were used verbatim where given; the compare-link
mechanism and checkout `fetch-depth` were my own choices where the handoff
specified the requirement but not the implementation.

## Tests And Gates Run

All observed on `896f2c6`:

```text
cargo fmt --check                                            pass
cargo xtask version-sync                                     pass — v0.165.1
cargo xtask css --check                                      pass — main.css up to date
cargo xtask i18n                                              pass — 203 keys covered
cargo xtask audit-deps                                        pass
cargo audit                                                    pass — 14 allowed warnings (pre-existing, unchanged)
cargo test -p forskscope-core -p forskscope-ui-logic          pass — 943/943 (643+27+16+2+241+6+7+1)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo test --workspace                                        pass — adds forskscope-ui 14+14 + 1 doctest ignored
cargo clippy --workspace -D warnings                           pass
git diff --check                                               1 flagged line — pre-existing CRLF in
                                                                 packaging/windows/AppxManifest.xml, exactly as
                                                                 the handoff predicted; not introduced here
```

CI run for this commit: `30738340551` — Test & Lint green in 4m59s, all 17
steps pass.

**Notes-extraction — all four required cases**, run locally against the
committed `CHANGELOG.md`:

```text
ver=0.165.0 (published section)
  -> 106 lines extracted; first line matches the section's actual first
     content line; zero embedded "## [" headings (neither the 0.165.0 nor
     the 0.164.0 heading leaked into the body)

ver=0.165.1 (current in-development section, header-only so far)
  -> 1 byte (a single blank line, matching the source content exactly);
     `test -s` passes — correct, since 0.165.1 legitimately has a heading,
     just no content yet

ver=9.9.9 (no matching heading at all)
  -> 0 bytes; `test -s` fails closed as required

ver=0X165X0 (wildcard-shaped, adversarial input)
  -> 0 bytes; confirms the literal-prefix match does not treat "." as a
     regex wildcard and does not spuriously match 0.165.0
```

Also verified the full bash step body (awk extraction, `test -s` guard,
`PREV_TAG` computation, compare-link append) end-to-end locally with
`GITHUB_REF_NAME=0.165.0`: `PREV_TAG` correctly resolved to `0.164.0`, and
the composed file ended with the expected
`**Full Changelog**: https://github.com/forskscope/forskscope/compare/0.164.0...0.165.0`
line.

No YAML linter (`actionlint`, `yamllint`, `pyyaml`, `js-yaml`, `ruby`) was
available in this environment to validate `release.yml` directly; verified
by careful manual re-read of the full file for indentation/structure
consistency with the surrounding unedited jobs, and by exercising the
embedded shell logic standalone as shown above. The authoritative check is
the CI run above, which parsed and ran the file successfully — CI's YAML
parse succeeding is evidence the syntax is valid, independent of the
release-specific job's own logic (which only runs on a tag push and is not
exercised by this ordinary `main` push).

## Unresolved Issues And Known Limitations

- Per the handoff (§7) and prohibited-shortcuts (§10): the notes-extraction
  change is proven locally in this slice only. End-to-end evidence — a real
  tag push producing a draft release whose body is the actual composed
  CHANGELOG section — arrives at M2-B's release cut, not here. No release
  was cut or published to "prove" this change, as instructed.
  `release-notes.md` is a job-local file that never leaves the runner's
  workspace before this slice's own verification; nothing was written to the
  actual repository beyond the workflow file itself.
- Publishing remains manual by design (§4.2) — not automated, and no
  `workflow_dispatch` publish trigger was added.
- F18 (`xtask` outside `cargo fmt --check`) is untouched, as instructed;
  remains registered against M4.
- The slice closes no audit blocker. B2, B3, B4 remain open.
- `0.165.0` is published and immutable; nothing in this slice alters it.
- No dependency was added, removed, or version-changed.

## Requested Review Focus

1. Does the checkout-before-download-artifact ordering actually prevent the
   artifact-wipe failure mode, or is there a subtler interaction with
   `actions/checkout`'s default clean behaviour this review should check
   more carefully than a manual re-read caught?
2. Is the literal-prefix match (`index($0, hdr) == 1`) verified sufficiently
   by the four local cases, or does CI's actual `awk` (possibly a different
   implementation/version than this environment's) warrant closer attention
   before the next real cut exercises it?
3. Is `git tag --sort=-creatordate` an acceptable choice for the compare
   link's previous-tag lookup, given the handoff specified "preserve the
   compare link" as a requirement but not a mechanism?
4. Does `release.md`'s new content match the handoff's §4.3 wording and
   intent closely enough, particularly the tagged-vs-published asymmetry
   paragraph, which is new content beyond the handoff's verbatim blockquote?
5. Is it acceptable that YAML syntax validity for `release.yml` rests on this
   ordinary-push CI run parsing it successfully, given no YAML linter was
   available and the release-specific `Create GitHub Release` job itself is
   only exercised by a tag push (deferred to M2-B's cut, per the handoff's
   explicit "cuts no release" scope boundary)?
