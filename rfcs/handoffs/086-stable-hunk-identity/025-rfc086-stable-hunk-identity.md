# Handoff 025 — RFC-086: stable hunk identity

**From:** architect. **RFC:** `rfcs/accepted/086-stable-hunk-identity.md`
(accepted 2026-09-07 — read it; §5's amendment is the reasoning this assumes).
**Release:** 0.170.0. **Register:** F47, and RFC-015 §8 rule 4's **Not met**
marker, open since F40.

## 1. The change is small; the invariant it creates is not

`hunk_id_for` (`diff/model.rs:167`) hashes five inputs. Four describe the hunk.
The fifth, `diff_id`, is a **process-global counter** bumped on every
`compute_diff` (`engine.rs:20,132`), so every recompute changes every id.

Remove it. Also remove the counter itself and `MergeSession::diff_id()`
(`session.rs:107`) — I checked, it has **zero callers** outside tests, and a
dead accessor left behind after its only input disappears is how the next reader
concludes it still means something.

## 2. What you are creating, and it must not be assumed

Without `diff_id`, **two different documents can produce equal hunk ids.**

That is safe *today* only because ids are compared within a single
`MergeSession` — `swap_in` searches `self.hunks`, and the log belongs to that
session. Right now the global counter gives that property for free. Afterwards
the code **depends** on it.

**So write it down and assert it**: a doc comment on `hunk_id_for` stating the
scope of uniqueness, and a `debug_assert` that a session's hunk ids are unique
within that session. An invariant that is only true by accident is the shape of
half this register.

## 3. `InternalInvariant` must stop being user-reachable

`swap_in` currently returns:

```rust
.ok_or(CoreError::InternalInvariant { message: "transaction references missing hunk".into() })?
```

**`InternalInvariant` should mean "this cannot happen", not "the user changed a
setting."** After §1 the identical-recompute case disappears, but the
structural-change case remains. Decide what that path returns instead and make
it a real, named condition rather than an internal-error escape hatch.

## 4. The behaviour change worth having

`change_diff_options` today prompts whenever the tab is dirty and discards the
undo stack on confirm — even when the recomputed hunks are **identical**.

After §1 it can tell the difference. **Preserve the stack when every logged hunk
id still exists; prompt and discard only when they do not.** That is the whole
user-visible win: an option toggle that does not alter hunk boundaries stops
costing you your history.

## 5. Do not touch RFC-015 — that part is mine

RFC-086's acceptance criteria include amending RFC-015 §8 rule 4 and clearing
its **Not met** marker. **That is an RFC lifecycle edit and I will do it** after
this lands, the same way you have correctly left RFC moves to me since handoff
022. Leave `rfcs/done/015-*.md` alone.

Report what you implemented; I will make rule 4 say it.

## 6. Falsification

1. **Identical recompute preserves ids** — falsify by reintroducing `diff_id`
   into the hash; the test must fail.
2. **The uniqueness invariant is enforced** — falsify by removing the
   `debug_assert` and constructing a colliding session; something must fail.
3. **A structural change still discards** — and the test must fail if history
   were silently retained and later misapplied. This is the one that matters:
   a test proving preservation is easy, a test proving we do *not* preserve
   unsafely is the one protecting against §5's rejected rebasing.
4. **`change_diff_options` no longer prompts** when hunks are unchanged, and
   still does when they are.

## 7. Scope

**In:** `diff/model.rs`, `diff/engine.rs`, `merge/session.rs`,
`ui/src/state/tab.rs`'s `change_diff_options`, tests.

**Out:** `rfcs/done/015-*.md` (§5); rebasing transactions onto changed hunks —
RFC-086 §5 rejects it explicitly as the F73/F85 defect class, and if the
implementation starts to look like it needs rebasing, **stop and tell me**
rather than building it.

**Wiring is in scope even in `forskscope-ui`** — handoff 022's scope line would
have shipped dead code and I am not repeating it.

## 8. Gates

The usual set. `ui/src/state/tab.rs:211-217` carries a doc comment stating that
rule 4 is *not* implemented and why; if §4 changes that, the comment must change
with it — a stale comment asserting a limitation the code no longer has is F92's
defect pointed the other way, and this project has produced it twice.
