# RFC-074 F23 Developer Handoff: Workflow Linting and Umask Gate Coverage

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M2 — the last item before M2's release cut
**Register items.** F23 (primary), F41 (folded in)

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend RFC-074 first, then
update this handoff to match.

## 1. Summary

Two register items, one file, one pass. Both are the same defect in different
clothes: **a gate that cannot observe the thing it is credited with checking.**

- **F23** — no workflow file is ever parsed. `release.yml` triggers only on tag
  push, so an edit to it is unvalidated until a real cut, where a syntax error
  means no release at all. This was found at review 033 because neither the
  implementer nor the reviewer had a YAML parser available to check the file
  they were both editing.
- **F41** — the F38 permission test asserts against a `fs::write` reference
  file, which is correct, but it can only *fail* under a non-default umask. CI
  runs `umask 022`. Confirmed by mutation at review 052: reverting to the
  hardcoded `0o644` passes on CI's mask and fails only under `077`. The fix is
  currently unprotected against regression.

They are folded together because both add a check to CI, and splitting them
would mean two passes over the same file and two review rounds for no benefit.

This slice changes no product behaviour and closes no audit blocker. B4 remains
open; the v1/public release decision remains **No-Go**.

## 2. Why this precedes the cut

M2's exit gate requires the release mechanics to be verified at a real release
cut. That cut runs `release.yml` — the one file in this repository that has
never been machine-validated. Linting it *after* the cut it was supposed to
protect would repeat the R0 lesson exactly: a release path credited as working
because it was configured plausibly, not because anything exercised it.

## 3. Scope

In scope:

- `actionlint` over every workflow file, running on every push and pull request;
- a CI step that exercises the permission assertions under a mask where a
  hardcoded-mode regression is observable;
- whatever fixes the new checks surface in the existing two workflows;
- register updates for F23 and F41.

Not in scope:

- F9, F40 — separate findings, both M4;
- any change to what the workflows *do*. This slice makes them checkable; it
  does not redesign them. If `actionlint` surfaces something that is a genuine
  behavioural defect rather than a lint, **report it, do not fix it here** —
  it becomes its own register entry.
- cutting the release itself. That is the next step, and the owner's.

## 4. Required properties

Stated as properties rather than mechanisms, deliberately. Choose the
implementation; justify it in the review request.

### 4.1 Workflow linting (F23)

1. **Every file in `.github/workflows/` is checked** — discovered from the
   directory, not enumerated in a list. A hardcoded file list would reproduce
   the exact defect F23 exists to close: a gate blind to what nobody remembered
   to name. Adding a third workflow later must be covered with no CI edit.
2. **The linter version is pinned**, not floating. `cargo audit` is already
   pinned to `--version 0.22.2 --locked` in `ci.yml`; match that discipline.
3. **The artifact's integrity is established** — a recorded digest for a
   downloaded binary, or a source already trusted by these workflows. Say which
   you chose and why in the review request; this is a supply-chain decision,
   not a convenience one.
4. **A workflow syntax error is reported quickly** — before or in parallel with
   the Rust toolchain install and the cached build, not after them. A ten-minute
   wait to learn that a YAML file has a bad indent defeats the purpose.
5. **Shell-block checking is not silently disabled.** `actionlint` runs
   shellcheck over `run:` blocks, and `release.yml` has seven of them. If any
   finding is a genuine false positive, suppress it *narrowly and in the file*,
   with a comment saying why. A blanket disable to reach green would leave the
   check present and hollow — F23's own shape.

### 4.2 Umask gate coverage (F41)

1. **Every test that asserts on file permissions runs at least once under a
   non-default umask.** Today that is the `persist_noclobber` permissions test;
   F9 will add more to the same area.
2. **The mask must be one where the old defect is observable.** `022` is not.
   `077` is what review 052's mutation used and is the natural choice.
3. **A subprocess is the only safe mechanism.** umask is process-global and
   Rust runs tests as threads in one process, so an in-test `libc::umask` would
   race the other ~1090 tests. A `run:` step is its own shell, so setting the
   mask there is naturally contained and cannot leak into other steps.
4. **State the narrowing.** If you select tests by filter rather than running a
   whole suite twice — which is the right call for CI time — the review request
   must say what the selector covers and how a *future* permission test would
   come to be included. An unstated cap reads as "covered everything."

## 5. Falsifiability — required evidence

Both checks exist because something passed that should have failed. Neither is
accepted on "the step ran and was green." Required in the review request:

1. **F23** — introduce a deliberate syntax error into a workflow file, show the
   new check failing on it, and revert. A malformed key or bad indentation is
   enough. Include the observed error output.
2. **F41** — revert `save.rs` to the pre-F38 behaviour (`NamedTempFile::new_in`
   plus `set_permissions(0o644)`), show the new step failing, and restore.
   Review 052 §2 records the expected message:
   `expected 600 (a plain fs::write's mode in the same directory), got 644`.
   Show also that the *existing* default-umask step still passes on the mutant —
   that contrast is the whole finding.

Do this on a scratch commit or in the working tree; do not push the mutations.

## 6. Anticipated friction

- **`actionlint` will probably not come up clean on the first run.** Expect
  shellcheck findings in `release.yml`'s `run:` blocks — unquoted expansions
  are the common one. Fix them; they are real. If a fix would change what a
  step does rather than how it is written, stop and report instead (§3).
- **`ci.yml` does not run on tag pushes** (it triggers on `branches`, and
  `release.yml` on `tags`), so a release cut will not re-lint. That is
  acceptable: the content was linted when it landed on `main`, and a tag points
  at a linted commit. Note it in the review request so the limit is recorded
  rather than assumed away.
- **Do not add the lint to `release.yml`'s preflight** to work around the
  above. The preflight's job is to gate a cut on the tree's state; re-linting
  there would be a second copy of the same check, drifting independently.

## 7. Register updates

- **F23** — mark `**Resolved.**` when the fix lands and its falsifiability
  evidence is in the review request, per the F26/F17/F38 convention. Record
  which linter version was pinned and how its integrity is established.
- **F41** — same, and move its milestone from M4 to M2, since it lands here.

## 8. Constraints

- `0.165.0` is published and immutable. Nothing in this slice may alter it.
- No Rust dependency is added, removed, or version-changed. `actionlint` is a
  CI tool, not a crate dependency; `cargo xtask audit-deps` and `cargo audit`
  must still pass with the reviewed `.cargo/audit.toml` exceptions intact.
- No product behaviour changes. The workspace test count should be unchanged
  unless you add a test, and if you do, say why.
- No real user paths, host names, or secrets in workflow files or evidence.

## 9. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F23, F41);
3. changed files;
4. important implementation decisions — especially the linter's provenance and
   integrity mechanism (§4.1.3), the umask test selector and its stated
   narrowing (§4.2.4), and any shellcheck suppression with its justification;
5. any difference from this handoff or from RFC-074;
6. executed gates with observed output;
7. **the two falsifiability demonstrations from §5, with their observed
   failure output** — not a claim that they were performed;
8. anything `actionlint` surfaced that you reported rather than fixed (§3);
9. unresolved issues and known limitations;
10. requested review focus.
