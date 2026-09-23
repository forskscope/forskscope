# Review Request: F38 — `persist_noclobber` Permissions Are Umask-Derived

**Date:** 2026-08-08
**Reviewer stance:** focused implementation review
**Repository baseline:** `c54f2f4`
**Governing documents:** review 051 §3.3 (direction), `ROADMAP.md` F38

## Summary

Review 051 §3.3 assigned F38 and settled its direction: `persist_noclobber`
runs only for `TargetPrecondition::MustBeAbsent`, so "preserve the existing
target's mode" is inapplicable by construction — there is no existing file.
The only coherent property is "the permissions the normal save path
(`atomic_replace`) would have produced," which is umask-derived. Implemented
exactly that, per §3.3's stated direction, with no design decisions of my own
beyond the mechanism.

## Files Changed

`crates/forskscope-core/src/save.rs`, `crates/forskscope-core/src/tests/save_target_tests.rs`,
`ROADMAP.md` (F38 marked `**Resolved.**`, matching the F26/F17 convention).

## The fix

`persist_noclobber_with_hook` previously created the temp file via
`tempfile::NamedTempFile::new_in(dir)` (defaults to `0600`) and then called
`set_permissions(fs::Permissions::from_mode(0o644))` unconditionally — correct
only under `umask 022`, and the exact defect review 048 flagged: under
`umask 077`, a normal save (`atomic_replace`, plain `fs::write`) produces
`0600`, but the old no-clobber path still produced `0644` — more permissive
than the environment asked for.

Replaced with `tempfile::Builder::new().permissions(fs::Permissions::from_mode(0o666)).tempfile_in(dir)`.
`Builder::permissions` requests a mode from the OS at file-creation time; the
kernel applies the process umask to it the same way it does for `fs::write`'s
default `0o666` create mode (confirmed against `tempfile` 3.27.0's own doc
comment on `Builder::permissions`, which states exactly this: the file "likely
won't actually be created with 0o666 permissions because it's restricted by
the user's umask"). This needed no unsafe umask query/reset (`libc::umask`
read-then-restore) and touches no process-global state.

## The test

The old test asserted the literal `0o644` — which was pinned to the same
umask assumption as the bug, so it couldn't have caught this if run under a
different umask (the CI runner's default umask happens to be `022`). Replaced
with: create a plain `fs::write` reference file in the same directory in the
same test, read its mode, and assert `persist_noclobber`'s output matches
that — the property review 051 stated ("a file created by the no-clobber path
has the permissions it would have had via the normal save path"), not one
environment's instance of it.

## Verification

- `cargo test -p forskscope-core persist_noclobber` — 5/5 pass.
- Ran the permissions test explicitly under `umask 077` (`(umask 077 &&
  cargo test ...)`) — passes. Under the *old* implementation this would have
  failed against my new reference-based assertion (`atomic_replace`/`fs::write`
  would produce `0600`, the old hardcoded path still `0644`), which is what
  makes the test meaningful rather than vacuous.
- Full workspace suite: 1094 passed, unchanged (one existing test rewritten
  in place, no tests added).

## Not addressed here

- **F9** (audit N2 — `atomic_replace` loses an overwritten file's original
  mode, since a temp-write-then-rename replaces mode along with content).
  Review 051 §3.3 was explicit this is a separate finding, not F38's — a
  user's `0600` file saved normally comes back `0644` under a default umask.
  Untouched by this change.
- F23 — still gates M2's cut, unchanged.
- Per review 051 §3, F38 was "the only item left" before M3 closes; I have
  not independently verified there's nothing else outstanding against M3 — a
  decision on M3's closure is the architect's/owner's, not mine to declare
  from this patch alone.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1094 (unchanged)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 223 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
cargo xtask audit-deps                                           pass
git diff --check                                                 pass
```

CI run `31229639704`: success, on `c54f2f4`.

## Note: a concurrent `ROADMAP.md` commit

While implementing this, `f48bc00` ("docs: settle F38 design direction -
umask-derived, not preserve-mode") appeared in local history ahead of my own
commits — not something I authored in this session. Its content matches
review 051 §3.3 exactly and required no reconciliation; I built on top of it
and additionally marked F38 `**Resolved.**` once the fix landed (`c54f2f4`),
consistent with the F26/F17 convention already in `ROADMAP.md`. Flagging in
case it's useful to know the roadmap note and the implementation were done
in two separate passes rather than one.

## Requested Review Focus

1. Is `tempfile::Builder::permissions(0o666)` the right mechanism, or is
   there a reason the codebase would prefer an explicit umask query instead
   (e.g. if `Builder`'s behavior isn't guaranteed portable the same way)?
2. Does the `ROADMAP.md` F38 entry read correctly as `**Resolved.**`, or
   should it wait for your own confirmation before being marked done?
3. Whether M3 is now ready to close, or whether something else remains that
   isn't visible from this patch alone.
