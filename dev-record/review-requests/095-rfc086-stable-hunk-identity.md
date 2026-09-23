# Review Request 095 — RFC-086: stable hunk identity (F47)

Handoff: `rfcs/handoffs/086-stable-hunk-identity/025-rfc086-stable-hunk-identity.md`
Commit: `77666ec` (pushed to `main`). CI run `34083837787`: green.

## §6's four falsifications, run for real

### 1 — identical recompute preserves hunk ids

Falsified by reintroducing a process-global counter (`AtomicU64`,
`fetch_add`) into `engine.rs` and XORing it into every `hunk_id_for`
result, matching the original defect's shape:

```
thread 'tests::diff_tests::identical_recompute_produces_identical_hunk_ids'
panicked:
assertion `left == right` failed: recomputing the same document with
unchanged options must yield identical hunk ids, or MergeSession's
undo/redo log can never survive a recompute (F47, RFC-086)
  left: [18134537940161729630, 12887648063739527716]
 right: [18134537940161729629, 12887648063739527719]
```

The UI-level preserve test failed the same way with the counter present
(confirmed, not just inferred from the core test).

### 2 — the uniqueness invariant is enforced

Falsified by deleting the `debug_assert!` in `MergeSession::from_diff`:

```
thread 'merge::session::tests::from_diff_panics_on_duplicate_hunk_ids_in_debug_builds'
panicked at .../session.rs:328:8:
note: test did not panic as expected
```

The colliding session (two hand-built hunks sharing `hunk_id: 1`) is
constructed directly — `MergeSession`'s own public API can never produce
two same-document hunks with equal ids on its own, so this white-box test
lives in a `#[cfg(test)] mod tests` inside `session.rs` itself (matching
the project's existing pattern in `encoding.rs`/`xlsx.rs`) rather than the
external `src/tests/` black-box suite.

### 3 — a structural change still discards

Falsified by making `MergeSession::is_compatible_with` unconditionally
return `true`:

```
thread 'state::tab::tests::change_diff_options_defers_to_confirmation_when_the_tab_is_dirty'
panicked at .../tab/tests.rs:250:9:
a dirty tab must show the confirm dialog, not apply immediately
```

This is the one the RFC calls "the one that matters" — it's the test that
fails if history were silently retained and later misapplied, not merely
one that proves preservation works.

### 4 — `change_diff_options` no longer prompts when hunks are unchanged, still does when they are

Two tests, two falsifications. Reverting the new compatibility branch to
dead code (`if false && compatible`) fails the "no prompt when unchanged"
test:

```
thread 'state::tab::tests::change_diff_options_applies_without_prompting_when_dirty_but_hunks_are_unchanged'
panicked at .../tab/tests.rs:293:9:
hunks are unchanged, so no confirmation should be needed
```

— and falsification 3 above is the "still prompts when changed" half.
All four restored and green afterward; ran the full workspace suite after
each restoration, not just the one test that caught it.

## Design decisions disclosed

**`is_compatible_with` checks the whole session's hunks, not just the
ones referenced by the undo/redo log.** The RFC's wording ("every logged
hunk still exists") is ambiguous between "every hunk in an undo/redo
transaction" and "every hunk this session tracks." I chose the stronger,
whole-session reading: `tab.merge.hunks()` — not `tab.diff.hunks` — is
what actually renders (`ui/view/diff.rs:346`), so if even one *untouched*
hunk moved, adopting the new diff while keeping the old session would
leave the rendered hunks and the diff document describing different
structures, independent of whether anything was ever applied. The
narrower log-only check would have let that case through. Both checks
happen to coincide for every scenario `change_diff_options` actually
produces (options never change document content, only how it's
segmented), so this is a safety margin against a subtly wrong future
reader of `is_compatible_with`, not a behavior difference today.

**`swap_in`'s missing-hunk case: `InternalInvariant` → `Conflict`.**
Checked whether this is actually reachable via `change_diff_options`'s
new code and concluded no: `MergeSession.hunks` never shrinks after
construction, and every undo/redo transaction is only ever pushed after a
successful lookup by id, so a session's own log can never point at a
hunk missing from its own `hunks`. Both branches of the new logic are
safe by construction — the compatible path leaves the session untouched,
the incompatible path fully rebuilds via `from_diff` with empty stacks.
I changed the error type anyway because the *type signature* is public
API independent of today's call graph, and `apply_left_to_right` already
treats the identical failure shape ("hunk id no longer matches") as
`Conflict` a few lines above — `swap_in` returning something with
`RecoveryHint::ReportBug` for the same conceptual failure its sibling
calls recoverable was the actual inconsistency. Tested by constructing
the invalid state directly (`MergeSession::empty()` plus a hand-built
stale `MergeTransaction`), since the public API can't produce it.

**Test fixtures for the compatible/incompatible split**, both against
`dirty_tab()`'s existing "one/two" vs "one/TWO" content: `inline_mode`
(Lazy → EagerForSmallHunks) for "compatible," since inline-span
computation never touches `hunk_id_for`'s inputs (ordinal/kind/ranges) —
it only decorates rows *within* an already-classified hunk. `ignore_case`
for "incompatible," since it makes "two"/"TWO" compare equal, collapsing
the fixture's one Replace hunk to Equal — a genuine boundary change, not
a relabeling. The original test used `ignore_whitespace`, which doesn't
touch this fixture's content at all; that's exactly the pre-RFC-086 test
gap this handoff exists to close, so I replaced it rather than leaving it
green by coincidence.

## What I did not touch, per §5

`rfcs/done/015-undo-redo-transaction-log.md` — left alone, as instructed;
that's the architect's edit once the RFC-015 rule 4 amendment is decided
to have landed. Also untouched: `ROADMAP.md`'s F47 register entry, moving
RFC-086 to `done/`, and any RFC lifecycle bookkeeping — following the
convention established since handoff 022.

## Scope

Touched exactly what §7 named: `core/src/diff/model.rs`,
`core/src/diff/engine.rs`, `core/src/merge/session.rs`,
`ui/src/state/tab.rs`'s `change_diff_options`, and tests — plus
`core/src/diff.rs` (dropping `DiffId` from the module's re-export list,
required by removing the type) and `ui/src/state.rs` (the
`Modal::ConfirmDiffOptionChange` doc comment, which named the exact
behavior this handoff changed and would otherwise have gone stale —
`change_diff_options`'s own comment at `tab.rs:211-217` the handoff
flagged directly is rewritten in full). `DiffId` is removed entirely
(zero remaining callers, confirmed by grep before removing); so is
`MergeSession::diff_id()`.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (736 core / 121 ui-lib / 121 ui-bin /
200 ui-logic, all green — +3 core tests, +1 ui-lib test over the
pre-handoff baseline; one pre-existing core test,
`diff_hunk_ids_survive_a_second_diff_call`, was inverted rather than
added alongside, since it directly encoded the now-removed
counter-based assumption), `cargo xtask css --check`, `version-sync`,
`i18n` (244 keys, unchanged — no new UI strings), `rfc-sync`,
`audit-deps`, `git diff --check`, `mdbook build docs` — all green. `cargo
audit`: exit 0, the same 14 pre-existing, unrelated warnings as the last
two reviews (no dependency added).
