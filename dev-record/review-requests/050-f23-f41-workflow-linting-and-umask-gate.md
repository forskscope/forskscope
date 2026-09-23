# Review Request: F23/F41 — Workflow Linting and Umask Gate Coverage

**Date:** 2026-08-08
**Reviewer stance:** focused implementation review
**Repository baseline:** `0573de5`
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/f23-workflow-linting-handoff.md`, review 052

## 1. Implementation summary

Both register items, one file (`.github/workflows/ci.yml`), one commit, per
the handoff's explicit instruction not to split them (both add a check to
CI; splitting would mean two passes over the same file).

- **F23**: `ci.yml` installs a pinned, checksum-verified `actionlint` v1.7.12
  and runs it with no arguments, before the system-dependency install and
  Rust toolchain setup.
- **F41**: `ci.yml` adds a step that sets `umask 077` in its own shell
  process and re-runs a name-filtered subset of `forskscope-core`'s tests.

## 2. Addressed items

- **F23** — no workflow file was ever machine-parsed; `release.yml` triggers
  only on tag push, so a syntax error there was invisible until a real cut.
- **F41** — the F38 permission-regression test can only fail under a
  non-default umask; CI runs `022`.

## 3. Files changed

`.github/workflows/ci.yml` (+38 lines, two new steps), `ROADMAP.md` (F23 and
F41 marked `**Resolved.**`, F23's milestone moved to M2 per the handoff's
§7 instruction to fold F41 in and align both to M2).

## 4. Important implementation decisions

### 4.1 `actionlint`'s provenance and integrity (handoff §4.1.3)

Downloaded from `rhysd/actionlint`'s GitHub Releases (the upstream project's
own release artifacts, the standard trusted source for this tool — same
provenance class as `dtolnay/rust-toolchain` and `cargo-audit`'s crates.io
publication, both already trusted elsewhere in these workflows). Pinned to
`v1.7.12` (`actionlint_1.7.12_linux_amd64.tar.gz`), matching `cargo-audit`'s
existing `--version X --locked` discipline. Integrity: the exact sha256
recorded in the workflow (`8aca8db9...`) was read from the release's own
published `actionlint_1.7.12_checksums.txt` asset and is verified via
`sha256sum -c` before the archive is extracted — a supply-chain decision
verified independently in real CI (see §7): the checksum step reported `OK`.

### 4.2 Fail-fast placement (handoff §4.1.4)

Both new steps run immediately after checkout, before "Install system
dependencies" (apt-get) and "Install Rust stable" — a workflow syntax error
is now reported before the slowest steps in the job, not after them.

### 4.3 Shell-block checking (handoff §4.1.5)

`actionlint`'s bundled shellcheck pass surfaced **zero findings** against
either `ci.yml`'s existing eight `run:` blocks (now ten) or `release.yml`'s
eleven, confirmed both by the real CI run (§7) and independently by a manual
`shellcheck` pass I ran locally beforehand against every `run:` block
extracted verbatim from both files (see §9 for why I ran this manually).
Nothing to fix, nothing to suppress.

### 4.4 Umask test selector and its stated narrowing (handoff §4.2.4)

`cargo test -p forskscope-core permissions` — a substring filter against the
fully-qualified test name. Today this matches exactly two tests:

- `tests::save_target_tests::persist_noclobber_output_is_not_left_with_tempfiles_narrow_default_permissions`
  — the F38 regression test, mode-sensitive, the one this step exists for;
- `tests::error_tests::copy_io_hints_check_permissions` — an unrelated
  `RecoveryHint` enum-mapping assertion, caught incidentally by the same
  substring, harmless under any umask.

**The narrowing, stated explicitly**: any future test asserting on a created
file's permissions must include the substring `"permissions"` in its test
name to be picked up here automatically. This is a naming convention, not
structurally enforced — nothing fails loudly if a future permissions test is
named otherwise. Recorded in both the CI comment and `ROADMAP.md`'s F41 entry
so it's discoverable from either place.

### 4.5 Subprocess isolation (handoff §4.2.3)

`umask 077` and the filtered `cargo test` run in the same `run:` step, which
GitHub Actions always executes as a fresh shell process — no explicit
subshell parens needed; the umask does not leak into later steps. Matches
review 052 §3.1's threading argument: an in-test `libc::umask` would be a
process-global mutation racing every other test in the same `cargo test`
process (~1090 other tests), which this avoids entirely by never touching
the umask inside the test binary.

## 5. Difference from the handoff

None in scope or design. One deviation in *process*, not design — see §9.

## 6. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1094 (unchanged — no product code touched)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 223 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
cargo xtask audit-deps                                           pass
git diff --check                                                 pass
```

No Rust dependency added, removed, or version-changed (handoff §8) —
`actionlint` is installed as a CI binary, not a crate dependency.

## 7. Falsifiability demonstrations (handoff §5)

### 7.1 F41 — performed locally, evidence below

Reverted `crates/forskscope-core/src/save.rs`'s `persist_noclobber_with_hook`
to the pre-F38 behavior (`NamedTempFile::new_in` + hardcoded
`set_permissions(0o644)`), in the working tree only — never committed or
pushed. Ran the exact filtered command from both umasks:

```text
$ (umask 022 && cargo test -p forskscope-core permissions)
test result: ok. 2 passed; 0 failed; ...

$ (umask 077 && cargo test -p forskscope-core permissions)
test tests::error_tests::copy_io_hints_check_permissions ... ok
test tests::save_target_tests::persist_noclobber_output_is_not_left_with_tempfiles_narrow_default_permissions ... FAILED

thread '...' panicked at crates/forskscope-core/src/tests/save_target_tests.rs:280:5:
assertion `left == right` failed: expected 600 (a plain fs::write's mode in the same directory), got 644 — a no-clobber-created file must not be more restrictive than an ordinary save
  left: 420
 right: 384

test result: FAILED. 1 passed; 1 failed; ...
```

Exactly the contrast the handoff asked for: the mutant is green under CI's
own mask and fails only under `077`, with the exact message review 052
predicted. Restored immediately after (`git checkout -- save.rs`), confirmed
clean (`git status --short`) and re-passing before continuing.

### 7.2 F23 — not performed locally; performed implicitly by real CI instead

**I could not produce this evidence.** Downloading and executing the
`actionlint` binary in this session's sandbox was denied by the permission
system on the first attempt; I asked explicitly whether to allow it, was
told yes, and the identical class of action (download+verify+extract+run,
and separately `sudo pacman -S actionlint` as an alternative source) was
denied again. Per this session's own operating instructions, I did not
attempt a third variant of the same action.

What I have instead: `actionlint`'s **first real execution anywhwere** was
this patch's own CI run (`31247867953`, §9), which is not the same evidence
the handoff asked for (a demonstrated failure on a deliberate syntax error,
then reverted) — it only shows the tool ran and found nothing wrong with
the two files as they stand today. I have not demonstrated that a syntax
error *would* be caught, only that valid YAML passes. This is a real gap
against §5's requirement, not a substitute for it, and I'm flagging it as
such rather than presenting the clean run as if it were the falsifiability
demo.

Mitigations applied given I couldn't run the tool itself: a manual
`shellcheck` pass (§4.3) over every `run:` block in both workflow files, and
close manual review of indentation/structure for the two new steps (both
inserted via exact string-anchored edits preserving the file's existing
6/8/10-space step indentation, verified with `cat -A` for stray
whitespace/tabs).

## 8. Anything `actionlint` surfaced that I reported rather than fixed

Nothing — zero findings against either workflow file, confirmed in real CI
(§9).

## 9. Unresolved issues and known limitations

- **§7.2 above** — F23's required falsifiability demonstration was not
  performed; only implicitly and partially covered by the real CI run
  passing cleanly. If this needs closing before F23 can stand as
  `**Resolved.**`, I don't have a way to produce it in this session without
  a different sandbox permission outcome.
- Per handoff §6: `ci.yml` does not run on tag pushes (`release.yml` does,
  triggered by `tags`, not `branches`), so a release cut does not re-lint.
  Accepted per the handoff: content is linted when it lands on `main`, and a
  tag points at an already-linted commit. Recording this here per the
  handoff's own instruction to "note it... so the limit is recorded rather
  than assumed away," and per §6's explicit direction, the lint was **not**
  added to `release.yml`'s preflight to work around this.
- `ROADMAP.md`'s F23 milestone was moved from "before M2's release cut" to
  `M2`, matching F41's column, per handoff §7's instruction to align both
  now that F41 folds in. If the more specific "before M2's release cut"
  phrasing was load-bearing for some downstream reference, flag it and I'll
  restore it.

## 10. Requested review focus

1. Whether §7.2's gap (no local F23 falsifiability demo, only a clean real
   CI run) is acceptable given the sandbox constraint, or whether F23 needs
   to be held short of `**Resolved.**` until that evidence exists some other
   way (e.g., you running the mutation yourself, or a follow-up in a
   different environment).
2. Whether the `"permissions"` substring-name convention (§4.4) is
   sufficient documentation of the narrowing, or whether it should be
   enforced more strongly (e.g., a `#[test]` attribute macro, a naming lint)
   given F41 exists precisely because an existing check silently stopped
   covering what it was credited for.
3. Whether folding F23 and F41 into one commit, per the handoff's explicit
   instruction, reads correctly here or should have been split regardless.
