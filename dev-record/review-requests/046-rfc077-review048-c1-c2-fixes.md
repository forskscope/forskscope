# Review Request: RFC-077 — Review 048 C1/C2 Fixes

**Date:** 2026-08-04
**Reviewer stance:** focused correction review, file-safety-critical path
**Repository baseline:** `b95ed39` (`ui: fix review 048 C1/C2 - overwrite
confirmation carries its real target`)
**Governing documents:** `dev-record/reviews/048-rfc077-patch4-startup-and-save-path-review.md`;
RFC-077

## Summary

Both corrections from review 048 applied.

**C1** — `Modal::ConfirmOverwrite` carried only a tab index; confirming
always force-saved to `tab.save_target`, which could differ from the Save As
destination that actually produced the conflict. `ConfirmOverwrite` now
carries the exact attempted target (`request.target`, threaded straight
through from `build_request`'s result), and a new `confirm_overwrite`
function forces only that path. `save_tab` lost its `force` parameter — it
was always `false` at every remaining call site once the confirmed-overwrite
flow moved to its own function. `OverwriteModal` now displays the target
path so the user can see what they're confirming.

**C2** — a blocked Save destination (directory, binary, unreadable) made
`build_request` return `None`, indistinguishable from the ordinary "nothing
to save" case. `build_request` now returns a `RequestOutcome` enum
(`Ready`/`NotSaveable`/`Blocked`); a blocked destination reports
`describe_block(reason)` via `store.notify` instead of silently doing
nothing. The same fix applies to a plain Save whose tab `save_target` is
itself `Blocked` — not just the Save As path review 048 named.

## Files Changed

`state.rs` (`Modal::ConfirmOverwrite(usize, PathBuf)`), `ui/view/diff_actions.rs`
(`RequestOutcome`, `dispatch`, `confirm_overwrite`, `describe_block`,
`build_request`'s force-for-explicit-target branch), `ui/overlay/modals.rs`
(removed `save_tab_force` — superseded), `ui/overlay/modals/file.rs`
(`OverwriteModal` takes `target: PathBuf`, shows it, calls
`confirm_overwrite`), `ui/view/diff.rs` (+`confirm_overwrite` re-export),
`ui/view/diff/toolbar.rs` + `app.rs` (`save_tab`'s dropped `force` arg),
`ui/view/settings.rs` (`ModalLayer`'s new match arm shape).

## Design Notes

- **`save_tab`/`save_as`/`confirm_overwrite` now share one `dispatch` tail**
  rather than each duplicating the `save_text` + `handle_result` call —
  `confirm_overwrite` is the only one that passes `force: true`, and it's the
  only place `TargetPrecondition::Force` for an explicit path can originate,
  matching RFC-077 ("`Force` is constructed only after an explicit overwrite
  confirmation").
- **`describe_block` is not run through `t()`.** `handle_result`'s existing
  `Err(e) => store.notify(e.to_string())` arm was already the precedent for
  this function's error messages staying English-only; matching it rather
  than introducing a new i18n boundary for just this one function felt like
  the smaller, more consistent change. Flagging in case this reads as an
  oversight rather than a deliberate match.
- **Force still fails closed on a blocked target.** `build_request`'s
  explicit-target branch checks `Blocked` before applying `force` — RFC-077:
  "Force... still rejects unsupported target kinds unless the user chose a
  different valid path." A confirmed overwrite of a destination that became
  a directory (or otherwise unsupported) between confirmation and retry
  still reports the block rather than forcing through.

## What C1's fix does *not* claim

The exact TOCTOU race review 048 described (destination changes between
`inspect_save_target` and `save_text`'s `check_precondition`, both inside one
synchronous `build_request`/`save_text` call with no externally-observable
gap) isn't reproduced live in this round — I don't have a code-level test
seam for it in the UI layer (unlike `persist_noclobber_with_hook`'s
before-commit hook in core), and adding one felt like scope creep for a
review-fix cycle rather than the fix itself. What *is* verified, live: the
type-level fix makes the wrong-target case unrepresentable
(`Modal::ConfirmOverwrite` has nowhere to lose the target between conflict
and confirmation), and the same code path is exercised end-to-end with a
real (non-racing) external modification — see runtime evidence below. If you
want the exact race reproduced, I'd need to add a UI-layer equivalent of the
core hook; say so and I will.

## Runtime Evidence

Rebuilt the debug binary, ran both scenarios under `.git-exclude/tmp/rfc077-review048/`
(repo-relative scratch), driven via AT-SPI.

**C2 — blocked target reports an error, doesn't silently no-op.** Launched
`forskscope local.txt remote.txt <a-directory>` (mergetool mode, merged path
is an existing directory — `SaveTargetState::Blocked` at preparation time via
`inspect_save_target`). Applied a hunk, clicked "Save merge result": toast
reads exactly `"Cannot save here: not a regular file."` — `describe_block`'s
output. The directory was left untouched (`ls -la`/`file` after: still a
directory, unmodified).

**C1 — confirmed overwrite writes to the displayed target, not a stale one.**
Normal two-file compare, applied a hunk, then externally modified the right
input (`echo "externally modified content" >> right2.txt`) before clicking
Save — real conflict, not synthetic. Screenshot confirms `OverwriteModal`
displays the exact conflicting path
(`.../rfc077-review048/right2.txt`). Clicked "Overwrite":
```
right2.txt:     "line one\nline two\nline three\n"                    (the merge result)
right2.txt.bak: "line one\nline TWO changed\nline three\nexternally modified content\n"  (exact pre-overwrite bytes)
left2.txt:      unchanged, byte-identical
```
The write landed at the displayed path, backed up the externally-modified
bytes exactly, and touched nothing else.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1094 (685+27+16+2 core, 267 ui-logic, 84 forskscope-ui, 6 css, 7 doctests), exactly 1090 + 4
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 220 keys (unchanged; describe_block deliberately not t()-wrapped, see above)
cargo xtask css --check                                          pass
cargo xtask audit-deps                                           pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
```

CI run `30919210679`: success, on `b95ed39`.

**Test-count delta:** +4 (2 distinct `describe_block` tests × 2 targets —
non-empty-for-every-reason, and detail-inclusion for the two variants that
carry a message).

## Not Addressed Here

- F38 (permission/umask) — registered by review 048 for "before M3 closes,"
  not this cycle.
- Patch 5 scope (target-transition integration tests, presentation, Save-As
  confirmation *dialog* UX) — unchanged from review 048's disposition.

## Requested Review Focus

1. Is the `dispatch`/`RequestOutcome` restructuring the right shape, or would
   you rather `confirm_overwrite` stayed a thinner wrapper closer to the
   pre-fix `save_tab_force`?
2. `describe_block` staying outside `t()` — confirm that's the right call
   given the existing precedent, or should this be the point where
   save-path error messages start getting translated?
3. Acceptable to leave C1 verified structurally + via a real (non-racing)
   external-modification conflict, or is a UI-layer test seam (mirroring
   core's `persist_noclobber_with_hook`) warranted before M3 closes?
