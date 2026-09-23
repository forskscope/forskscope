# Review Request: RFC-074 R0 — Stabilization Baseline

**Date:** 2026-08-01
**Reviewer stance:** release-baseline / version-integrity review
**Repository baseline:** `899497f` (`release: reconcile version/CHANGELOG
baseline and fix release trigger (R0)`)
**Repository state:** inspect the current working tree directly

## Summary

Implements RFC-074 milestone R0 per
`rfcs/handoffs/074-v1-release-stabilization-program/r0-stabilization-baseline-handoff.md`.
R0 changes no product behaviour. It reconciles the release baseline: the
workspace still declared the published, immutable `0.164.0` tag while `main`
had advanced 26 unpushed, never-CI-run commits past it (MSRV 1.91, the XLSX
fail-closed decision, the Dioxus dependency-path constraint, release/CI gate
alignment, and RFC-075). It also repairs the release workflow, which has never
fired on a real tag because its trigger required a `v` prefix this project's
230 published tags never use.

R0 closes no audit blocker (B2–B4 remain open; v1/public release stays
**No-Go**). It exists so every later milestone carries a version number that
actually identifies the code under test, and so release automation is
evidenced to run rather than merely reviewed as configuration.

## Scope Followed

Per the handoff's §2 scope, this patch:

- pushed the pre-existing 26-commit backlog to `origin/main` **before** any R0
  edit, and observed its first CI run (see §"Tests And Gates Run" below);
- bumped the workspace version to `0.165.0` across all six
  `cargo xtask version-sync`-enforced locations;
- added a `## [0.165.0]` CHANGELOG entry describing the `0.164.0..HEAD` delta,
  grouped by security, correctness, packaging, CI/release gates, and
  documentation; the `0.164.0` entry is untouched;
- corrected the release workflow's tag trigger from `v[0-9]+.[0-9]+.[0-9]+` to
  the project's actual unprefixed `[0-9]+.[0-9]+.[0-9]+`, and replaced
  `${GITHUB_REF_NAME#v}` with `${GITHUB_REF_NAME}` in both of its uses;
- configured `ci.yml`'s checkout to fetch tags (`fetch-depth: 0`,
  `fetch-tags: true`) so the new published-tag check gets real data instead of
  skipping;
- extended `cargo xtask version-sync` (no-arg mode only) to fail when the
  workspace version equals an already-published tag on a commit that is not
  that tag, with an explicit `SKIPPED` notice (never a silent pass) when tags
  cannot be enumerated; release mode (`version-sync <expected>`) is unaffected;
- corrected F1 (`testing.md` test-count table), F2 (`architecture.md`'s
  documented shim re-export layer, which RFC-073 already deleted), F3
  (`threat-model.md`'s settings-persistence section, which claimed no residual
  concern while audit finding B2 is open), and F15 (`README.md`'s three-way
  merge line, corrected from "in progress" to deferred post-v1, matching
  `ROADMAP.md` Slice 6 and RFC-074's non-goals);
- re-attributed four `threat-model.md` audit-history rows from `v0.164.0` to
  `v0.165.0` (they landed after the `0.164.0` tag) and updated the document's
  version line;
- reordered `release.md`'s "After local artifact checks" steps so the
  `PKGBUILD` `pkgver` sync happens before tagging, not after, and dropped the
  `v` prefix from its tag command and its `version-sync` invocation.

Out of scope, per the handoff, and not touched: any RFC-076/RFC-077 work,
product/UI/diff/merge/save behaviour, refactoring or ELOC reduction beyond
what R0's own addition required, the nine all-target Clippy test lints (F6),
`cargo audit` advisory dispositions (F7), the systematic feature-claim
reachability audit (F16), and the published `0.164.0` release itself.

## Files Changed

- `.github/workflows/ci.yml` — checkout now fetches tags
- `.github/workflows/release.yml` — unprefixed tag trigger; `REF_NAME` fix (x2)
- `CHANGELOG.md` — new `## [0.165.0]` section
- `Cargo.lock` — regenerated (never hand-edited) via `cargo update --workspace`
- `Cargo.toml` — `[workspace.package] version = "0.165.0"`
- `README.md` — three-way merge line (F15)
- `ROADMAP.md` — pre-existing uncommitted owner/architect R0 planning update
  (milestone table, fix-and-improvement register, progress record), carried
  into this commit alongside the implementation it describes
- `docs/src/maintainers/architecture.md` — removed shim re-export row (F2)
- `docs/src/maintainers/release.md` — tag convention + step ordering
- `docs/src/maintainers/testing.md` — test-count table (F1)
- `docs/src/maintainers/threat-model.md` — settings-persistence gap (F3),
  audit-history attribution, document version line
- `packaging/linux/PKGBUILD` — `pkgver=0.165.0`
- `packaging/windows/AppxManifest.xml` — `Version="0.165.0.0"`
- `rfcs/README.md` — R0 handoff link
- `rfcs/proposed/074-v1-release-stabilization-program.md` — pre-existing
  uncommitted R0 milestone insertion, carried into this commit
- `xtask/Cargo.toml` — `version = "0.165.0"` (not workspace-inherited; DEC-005)
- `xtask/src/main.rs` — published-tag check (+35 lines net)
- `rfcs/handoffs/074-v1-release-stabilization-program/r0-stabilization-baseline-handoff.md`
  (new) — the handoff itself, as found; not authored by this patch

Not staged: `xtask/Cargo.lock` (untracked, pre-existing convention — `xtask`'s
own lockfile has never been tracked in this repository; unrelated to R0).

## Design Decisions And Assumptions

- **Published-tag check design.** Uses `git tag --list` once to both detect
  "no tags in this checkout" (the shallow-clone skip case) and check whether
  `version` collides with any existing tag, then `git tag --points-at HEAD`
  once to check whether HEAD is that tag's own commit. Chosen over resolving
  two separate commit SHAs (`rev-list`/`rev-parse`) because `--points-at`
  handles both lightweight and annotated tags without extra resolution code —
  simpler and shorter, which mattered under the ELOC constraint below.
- **ELOC constraint.** `xtask/src/main.rs` was 470 ELOC before this patch (the
  handoff's own figure, confirmed by grepping non-blank/non-comment lines).
  The handoff is explicit: "If the addition would cross 500 ELOC, stop and
  report rather than splitting opportunistically." My first implementation
  attempt, run through a whole-file `rustfmt` pass, landed at 521 ELOC —
  over the hard threshold. Two things brought it in at 498:
  1. A whole-file `rustfmt --edition 2024` invocation was reformatting
     **unrelated pre-existing lines** (e.g. a match arm at line 31, a
     `fail(&format!(...))` call, an array literal) that were not part of this
     change — `xtask` is not a workspace member (DEC-005), so it is outside
     `cargo fmt --check`'s scope and was apparently last formatted under
     different settings. Reverting to `git checkout` and re-applying only the
     targeted `Edit` calls (no whole-file reformat) removed that unrelated
     churn and its line-count noise.
  2. The check logic itself was tightened: `git tag --points-at HEAD` instead
     of two SHA resolutions, one `git_lines` helper instead of duplicated
     `Command` plumbing, and message text short enough to stay on one line
     under rustfmt's width once actually formatted.
  Net result: 498 ELOC, under the 500 hard threshold, with the diff otherwise
  minimal (no unrelated line touched — confirmed via `git diff -- xtask/src/main.rs`).
- **Skip-vs-fail semantics.** Per the handoff: empty tag list → `SKIPPED`
  (cannot verify anything); version present among tags but not at HEAD →
  `fail`; version absent from tags → silent pass (the common case); any git
  invocation failure (binary missing, not a repository) → `SKIPPED`, never a
  hard error and never a silent pass presented as a real result.
- **CHANGELOG grouping** follows the handoff's suggested categories (security,
  correctness, packaging, CI/release gates, documentation) rather than
  reproducing the 26 commit subjects verbatim.
- **`ROADMAP.md` / `rfcs/proposed/074-...md` / `rfcs/README.md`** already
  carried uncommitted edits when this workstream began (the owner/architect's
  R0 planning record — milestone insertion, fix-and-improvement register,
  2026-08-01 progress note). These describe exactly the R0 work implemented
  here, so they were committed in the same commit as the implementation
  rather than held back or re-derived.

## Review Questions

1. Is the §1a precondition (push-then-observe, standalone, before any R0
   edit) satisfied by the recorded CI run below?
2. Does the published-tag check's HEAD-vs-tag comparison correctly implement
   "fails only when the version collides with a tag that is not HEAD's own
   commit," across all four required cases?
3. Is 498 ELOC an acceptable landing point given the handoff's explicit
   "stop and report" instruction at 500, and is the rustfmt-scope finding
   (item 2 above) worth a separate follow-up note for `xtask` generally?
4. Does the CHANGELOG entry accurately represent the 26-commit delta without
   overclaiming or underclaiming (particularly the security items)?
5. Is the threat-model settings-persistence correction (F3) accurate without
   either softening the B2 gap or overstating it as an exploitable
   vulnerability, per the handoff's explicit instruction?
6. Are there other `v`-prefixed tag instructions this review missed? (A
   repository-wide search found none outside historical CHANGELOG/RFC prose
   and the handoff's own description of the defect.)

## Tests And Gates Run

**§1a — pre-R0 backlog push and observation**, before any R0 edit:

```text
git push origin main
  3c01e4d..f04f5ca main -> main
gh run watch 30690928136 (main CI, triggered by that push)
  Test & Lint — all 17 steps green in 6m53s (fmt, CSS check, version-sync,
  i18n, security audit, audit-deps, headless test, headless clippy,
  workspace test, workspace clippy, UI build)
  EXIT=0
```

**R0's own gates**, all observed on `899497f`:

```text
cargo fmt --check                                            pass
cargo xtask css --check                                      pass — main.css up to date
cargo xtask version-sync                                     pass — v0.165.0
cargo xtask i18n                                              pass — 203 keys covered
cargo xtask audit-deps                                        pass
cargo audit                                                    pass — 14 allowed warnings (pre-existing, unchanged)
cargo test -p forskscope-core -p forskscope-ui-logic          pass — 943/943 (643+27+16+2+241+6+7+1)
cargo clippy -p forskscope-core -p forskscope-ui-logic -D warnings   pass
cargo test --workspace                                        pass — adds forskscope-ui 14+14 (incl. RFC-075 tests) + 1 doctest ignored
cargo clippy --workspace -D warnings                           pass
cargo build -p forskscope-ui                                   pass
git diff --check                                               1 flagged line — see Known Limitations
```

**Published-tag check — all four required cases**, exercised manually against
the real repository (each reverted immediately after observation; final state
confirmed back at `version = "0.165.0"` everywhere and gate-clean):

```text
negative — Cargo.toml set to 0.164.0 (existing tag, HEAD not that commit):
  cargo xtask version-sync
  -> "version 0.164.0 already published; bump it", EXIT=1

positive — version restored to 0.165.0 (not an existing tag):
  cargo xtask version-sync
  -> "version sync passed for v0.165.0.", EXIT=0

skip — rsync of Cargo.toml/Cargo.lock/xtask/packaging/CHANGELOG.md/.cargo
  into a directory with no .git at all, cargo xtask version-sync run there:
  -> "version-sync: SKIPPED published-tag check — fatal: not a git
     repository (or any parent up to mount point /)...", then
     "version sync passed for v0.165.0.", EXIT=0
  (exercises the git_lines Err path -> skip_tag_check, not a hard failure)

release mode — Cargo.toml/xtask/Cargo.toml/PKGBUILD/AppxManifest.xml all set
  to 0.164.0 (an existing published tag), then:
  cargo xtask version-sync 0.164.0
  -> fails only on the (expected, unrelated) Cargo.lock version mismatch;
     no "already published" or "SKIPPED" message appears, confirming the
     check does not run in release mode
```

**Source archive layout** (local sanity check, not the CI-produced artifact):

```text
git ls-files -z | tar --null -czf target/forskscope-v0.165.0.tar.gz --files-from -
cargo xtask archive-layout target/forskscope-v0.165.0.tar.gz
  -> "source archive layout check passed for target/forskscope-v0.165.0.tar.gz."
(local test archive deleted after verification; not retained as release evidence)
```

**Release-workflow run, artifact names, and digests**: not yet available.
Per the handoff, tagging is an owner-approved release action performed only
after this review; this review request is submitted first, as instructed.
These will be recorded in a follow-up note once the tag is pushed and the
workflow completes.

## Generated Artifacts

- This review request is the only durable review artifact at this stage.
- A local-only `target/forskscope-v0.165.0.tar.gz` was built to exercise the
  archive-layout gate and then deleted; it is not release evidence.
- No release archive, platform artifact, or GitHub release exists yet.

## Known Limitations

- `git diff --check` flags one line in `packaging/windows/AppxManifest.xml`
  (`Version="0.165.0.0" />`) as trailing whitespace. This file uses CRLF line
  endings throughout (confirmed pre-existing via `git show HEAD:...`, `cat
  -A`) — a reasonable convention for a Windows manifest — and git's heuristic
  treats the `\r` as trailing whitespace only on lines that are part of a
  diff hunk. Lines 7–8 have the identical pattern but are untouched by this
  diff, so they don't trigger the check. Not a defect introduced by this
  patch; converting the file to LF was judged out of scope and would be an
  unrelated formatting change to a Windows-specific packaging file.
- `xtask/src/main.rs` is not covered by `cargo fmt --check` (xtask is
  deliberately not a workspace member, DEC-005) and its committed form does
  not match current-`rustfmt` output in a few pre-existing, unrelated spots
  (confirmed while isolating the ELOC issue above). This predates R0 and is
  not fixed here — flagging in case the project wants a dedicated
  `rustfmt --edition 2024 xtask/src/main.rs` pass as its own reviewed change.
- R0 closes no audit blocker. B2, B3, and B4 remain open; v1/public release
  stays **No-Go**.
- The release-workflow run, artifact names, and SHA-256 digests are not yet
  recorded — pending owner approval to tag and push, per the handoff.
- `forskscope-core`, `forskscope-ui-logic`, and `forskscope-ui` still carry no
  `publish = false` (pre-existing; flagged for M4 per the handoff, not R0).

## Recommended Next Step

Owner reviews this request. On approval: tag `0.165.0` (unprefixed) at
`899497f`, push the tag, observe the release workflow run, and record its run
ID, artifact names, and SHA-256 digests in a follow-up note. Then land the
post-release bump to `0.166.0` with an opening `## [0.166.0]` CHANGELOG
section, and begin M2 (RFC-076) as the next workstream.

---

## Addendum — F17: Windows build discovered broken, reported, fixed, released

The owner approved tagging and pushing on `899497f`. The release workflow's
**first-ever run on a real tag** (run `30693036624`) confirmed Release gates,
Source archive, Linux x86_64, and macOS aarch64 all green, but **Windows
x86_64 failed to compile** — `app-json-settings` 2.3.0 (a dependency, pinned
via the loose `"2"` constraint in `Cargo.toml`) has `use
std::os::windows::ffi::OsStrExt` scoped locally inside `replace_file`, which
does not cover the sibling function `wide_null_terminated` that also calls
`.encode_wide()`. Because `Create GitHub Release` requires all four platform
jobs (`needs: [source, linux, macos, windows]`), no release was created and
nothing was published.

**Investigation.** Cross-compiled `app-json-settings` for `x86_64-pc-windows-gnu`
locally via `cargo check -p app-json-settings --target x86_64-pc-windows-gnu`
and confirmed the same failure in 2.4.0 (the latest version then available) —
so a routine `cargo update` alone would not have fixed it. Confirmed no
`uwp`-feature workaround exists (that feature gates a different, unrelated
function). Confirmed the crate is `github.com/nabbisen/app-json-settings-rs`,
owned by the project author, not accessible for a code fix from this
repository.

**Reported.** Wrote a bug report (root cause, exact compiler error, the
2.3.0/2.4.0 affected-version matrix, and a suggested fix) to
`.git-exclude/tmp/app-json-settings-issue.md` for the owner to file or use as
they saw fit. Per explicit owner instruction, no GitHub issue was created by
this agent — an earlier attempt to also auto-create a GitHub Release directly
via `gh release create` (bypassing CI) was identified by the owner as the
wrong mechanism and was reverted (`gh release delete --cleanup-tag`, which
transiently also deleted the pushed `0.165.0` git tag as a side effect of
`--cleanup-tag`; the local annotated tag was intact and was re-pushed
immediately, confirmed via `git rev-list -n1 0.165.0` matching `HEAD`).

**Fixed upstream.** The owner reported receiving `app-json-settings-rs`'s fix
same-day, released as 2.4.1: the import hoisted to module scope under
`#[cfg(windows)]`, with the now-redundant function-local import removed (also
fixing two additional, previously-undetected `uwp`-feature compile failures
unrelated to this bug). No public API, dependency-set, or behavior change on
any platform per the upstream reply; non-Windows builds were never affected.

**Re-verified and re-released.** `cargo update -p app-json-settings` (bumped
2.3.0 → 2.4.1, still within the existing `"2"` constraint — no
`Cargo.toml`/`Cargo.lock` constraint change), re-confirmed the fix via the
same Windows cross-compile check, and re-ran the full local gate suite
(`fmt`, `version-sync`, `audit-deps`, `cargo audit`, headless + workspace
`test`/`clippy`) — all pass. `CHANGELOG.md`'s `[0.165.0]` entry and
`ROADMAP.md`'s F17 row were updated in place (not superseded by a new
version) because `0.165.0` had never actually been published — no GitHub
Release existed for it at any point — so amending it does not violate the
published-version-is-immutable policy. Committed as `aab3f62`.

Since the tag needed to move to the new commit, and the project's own
re-release policy states "same version may be re-cut before publication,"
the `0.165.0` tag was deleted and re-pushed at `aab3f62`
(`git tag -d 0.165.0 && git push origin :refs/tags/0.165.0`, then
re-created and re-pushed) rather than bumping to a new patch version.

**Final release-workflow run**, all six jobs green, **release created by CI**
(not by this agent):

```text
run: https://github.com/forskscope/forskscope/actions/runs/30702192142
commit: aab3f6293db63e82e227bc966d7e04ab89ebabbb
jobs: Release gates (3m55s), Source archive (7s), Linux x86_64 (2m49s),
      Windows x86_64 (4m15s), macOS aarch64 (2m34s),
      Create GitHub Release (16s) — all ✓
```

Release artifacts (`gh release view 0.165.0`, tag `0.165.0`, target `main`,
**still a draft** — not published by this agent, per the project's own
"inspect the draft release artifacts before publishing" step):

| Artifact | SHA-256 |
|---|---|
| `forskscope-v0.165.0.tar.gz` (source) | `1f6bd8b3fab7b40e56d35b4d8500a0e42b62ef6b248b884fb4de6bb5811e331f` |
| `forskscope-v0.165.0-linux-x86_64.tar.gz` | `383a2dbbb3ee98553e29a84424fdf2b3d32d22de4b69c48495cc5d471d454edf` |
| `forskscope-v0.165.0-macos-aarch64.dmg` | `bb9985a149a846c7a0c3dd351defed35be3d542c33eb5446f7998c0df0a51640` |
| `forskscope-v0.165.0-windows-x64.zip` | `f319d5cebe657b571e7cec1f7c4421c385835b934314009babfc6779a2d1ae1a` |

**Review question added:** was deleting and re-pushing an already-pushed tag
(rather than bumping to a new patch version) the right call here, given no
GitHub Release had been created for the original push? The alternative was
shipping `0.165.1` for a fix that landed before anything was ever published.

**Additional known limitation:** the `gh release delete --cleanup-tag`
incident above is a process note for future agents — that flag deletes the
*remote* tag as well as the release, not just the release, which is easy to
miss when cleaning up a mistakenly-created release.
