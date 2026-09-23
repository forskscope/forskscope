# Developer Handoff 006 — F79: an unreadable entry must never vanish

**From:** architect
**Date:** 2026-08-22
**Register:** F79. **F76 is *not* in scope** — see §6.
**Gate:** F79 is a **Gate D blocker** (architect assessment, recorded in its entry).

---

## 1. Task title

Give the recursive walk a way to say "I could not read this", and make every
consumer show it instead of dropping it.

## 2. Purpose

`forskscope-core::dir::recursive` discards read failures at every level:

- a per-entry `metadata()` failure is `continue`d past (`recursive.rs:151`,
  `:195`, `:262`) — the entry is **absent from the result**;
- every recursive descent is `let _ = walk_and_merge_fast(..)` / `walk_and_merge(..)`
  (`:207`, `:274`), so a directory that cannot be opened takes its **entire
  subtree** out of the result;
- the **top-level** calls are the same (`:99`, `:121`), so if the right root
  cannot be opened the walk returns only left-side entries — which reads as
  *every file is `LeftOnly`*, a confident verdict produced by a failed read.

`RecStatus` has no error variant to report any of it with.

**This is shipped behaviour with three consumers, and it is worse than a display
bug in all three:**

- **Deep Compare** — the entry has no row, so it is invisible; and
  `BatchCopyButtons` builds its manifest from the entries the walk returned, so
  it is silently excluded from *copy all changed*. The user runs a merge, is
  shown success, and content they never knew existed was never considered.
- **`patch/directory.rs`** — generates directory patches from `recursive_diff`,
  so an unreadable file is silently absent from a patch that claims to represent
  the difference between two trees.
- **`merge_plan.rs`** — documented as building on `recursive_diff` output.

## 3. Background

Found by re-reviewing RFC-080 at the owner's request. The RFC asserted "no new
engine work"; checking that claim against `recursive.rs` surfaced this. The RFC
was wrong, and the reason it was wrong is a defect in shipped code.

Same family and consequence as F78 — a merge tool silently not merging — but
**strictly easier to reach**: F78 needed a race, this needs one file or folder
the process cannot read, and no timing at all.

## 4. Applicable RFC and requirements

- **RFC-080 §1** now records this as a precondition; that RFC's error handling is
  not expressible until this lands. Do **not** implement RFC-080 here.
- Core already has the vocabulary in a different type:
  `EqualityEvidence::Error { message }` (`dir/index.rs:199`). The recursive walk
  simply does not use it.

## 5. Change scope

- `crates/forskscope-core/src/dir/recursive.rs` — the variant and the walk
- `crates/forskscope-core/src/dir.rs` — exports
- `crates/forskscope-core/src/patch/directory.rs` — handle the new variant
- `crates/forskscope-ui-logic/src/explore/deep_filter.rs` — filtering and counts
- `crates/forskscope-ui/src/ui/view/deep_compare.rs` — display, gating, summary

## 6. Explicit non-change scope

- **F76 is not in scope, and I said otherwise to the owner before checking.**
  F76's two instances live in `explorer.rs`'s `DigestState`, a *different* enum
  in a different crate; F79 is `RecStatus`. They are the same family, not the
  same fix. F76 stays scheduled with F75, which deletes `DigestState` outright —
  adding states to an enum already condemned would be waste.
- **No RFC-080 work.** No tiers, no size cap, no new status vocabulary beyond the
  one variant below.
- **Do not change what a *successful* comparison reports.** Every currently
  correct verdict must be byte-identical afterwards.
- F74, F75, F77, F78 — untouched.

## 7. Required implementation

### 7a. The variant — payload-free, and this is a decision not a shortcut

Add **`RecStatus::Unreadable`** with **no payload**.

`RecStatus` derives `Copy` and is passed by value at 42 call sites, including
component props (`deep_compare.rs:346`) and `can_copy_left_to_right(status:
RecStatus)`. **A `String` payload would break `Copy`** and turn a contained fix
into a 42-site refactor inside a Gate D blocker. The diagnostic detail — which
side failed, and why — is worth having and is **deliberately deferred**; the
user-facing requirement is *"this entry could not be read, do not treat it as a
verdict"*, and that needs no message.

If you conclude the message is indispensable, say so in the review request
rather than breaking `Copy` unannounced.

### 7b. The walk stops discarding

- A per-entry `metadata()` failure produces an `Unreadable` entry **at that
  path**, instead of `continue`.
- A directory that cannot be opened produces **one `Unreadable` entry for the
  directory itself**. Its subtree is still absent — that is unavoidable, nothing
  read it — but it is now *visibly* absent rather than silently so.
- **The root case needs a caller-visible signal.** If a root cannot be opened,
  the caller must be able to distinguish that from *"the tree is empty"* and
  from *"everything is on the other side"*. A synthetic entry at the empty
  relative path is **not** acceptable — `explorer.rs` filters empty rel-paths and
  Deep Compare would render it as a nameless row. **The shape is yours to
  choose** (a richer return type is the obvious candidate); the constraint is
  that an unopenable root must never read as `LeftOnly` for every entry. State
  what you chose and why.

### 7c. Every consumer handles it

- **Deep Compare** — its own glyph and CSS class, plus a localised label via
  `t(lang, …)` and the `role="img"` + `aria_label` pattern F74 established.
  **Never copyable in either direction** (`can_copy_left_to_right` /
  `can_copy_right_to_left` must both be false): a file that could not be read is
  not a file that should be written. `can_cmp` false. It must be **visible in the
  default view and under the "different" filter** — the whole point is that it
  stops being invisible — and it must be **counted in the summary** beside
  equal/changed/left/right, or a user reading the counts will still not know.
- **`deep_filter.rs`** — `is_different` must **not** claim `Unreadable` is
  different. It is not a verdict.
- **`patch/directory.rs`** — decide, and state your reasoning: does an unreadable
  entry fail patch generation, or is it recorded in the patch? **Silently
  omitting it is not an option**, because that is the defect. This is the one
  place in this handoff where I do not have a strong prior; argue it.

## 8. Required tests

Demonstrated failing against the shipped defect — not against a helper the fix
introduces. Reviews 072 and 074 both turned on that distinction.

1. **An unreadable file appears as `Unreadable`, not absent.**
2. **An unreadable directory appears as `Unreadable`, and its parent still
   lists.**
3. **An unopenable root is distinguishable from an empty tree**, and does not
   render as every-entry-`LeftOnly`.
4. **`Unreadable` is not copyable and not in the batch manifest.**

For 1–3, **falsify by restoring the discard** (`continue`, or `let _ =`), confirm
the entry vanishes, restore.

### The falsifiability hazard in these tests — read before writing them

Making something unreadable is **platform-specific**, and this is where these
tests can quietly become worthless:

- **Use `#[cfg(unix)]`** with `PermissionsExt` (`chmod 000`). There is no
  equivalent one-liner on Windows, and a test that silently does nothing there is
  worse than one that is honestly absent.
- **`chmod 000` does not stop root.** If the suite runs as root — containers
  often do — the read succeeds, the entry appears normally, and the test
  **passes for the wrong reason**. Detect it: if the path is still readable after
  the permission change, **skip with an explicit message**, never assert.
  A vacuous green here is exactly the failure this program keeps cataloguing.
- **Restore permissions in the test**, or the temp directory cannot be cleaned up.
- Say plainly in the review request which platforms actually execute these
  checks. "Tests pass" must not stand in for "tests ran".

## 9. Required documentation updates

None. **Do not edit `ROADMAP.md`.**

## 10. Acceptance criteria

- Restoring any discarded error makes a test fail.
- `RecStatus` still derives `Copy`, or the review request explains why not.
- An unreadable entry is visible, uncopyable, uncounted as a verdict, and
  excluded from the copy manifest.
- An unopenable root is distinguishable from an empty tree.
- `patch/directory.rs` does not silently omit it.
- Every previously correct verdict is unchanged.
- Gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo xtask css --check`,
  `cargo xtask i18n`, `git diff --check`.

## 11. Prohibited shortcuts

- **Do not map `Unreadable` to `Changed`** to avoid touching match arms. That is
  F76's second instance — a status asserting more than was measured — reproduced
  deliberately.
- **Do not let a permission test pass when the permission change had no effect.**
- **Do not break `Copy` silently.**
- **Do not report a falsification you did not run.**

## 12. Relevant code or module boundaries

Core stays GTK-free and Dioxus-free. `deep_filter.rs` is in `ui-logic`, which
depends only on `forskscope-core` — keep it that way.

## 13. Compatibility and security constraints

`RecStatus` is `pub` and `#[non_exhaustive]` is not currently on it; adding a
variant is a breaking change for any external matcher. There are none outside
this workspace. No persistence format carries `RecStatus`.

## 14. Known risks

- **42 match sites.** `clippy -D warnings` will find the non-exhaustive ones;
  that is the tool doing its job, not a reason to add a catch-all arm. **A `_ =>`
  arm anywhere in this change is a defect** — it is how the next variant gets
  silently mishandled.
- **The subtree of an unreadable directory stays absent.** That is correct and
  unavoidable. Do not attempt partial recovery.

## 15. Required evidence

- Observed failure output for each falsification in §8, quoted.
- Which platforms executed the permission tests, and what happens on the others.
- The root-signal shape you chose, and why.
- Your `patch/directory.rs` decision, and the argument for it.

## 16. Required review-request format

As in requests 071–073. Lead with the falsifications and their output.

State plainly:
- whether any `_ =>` arm was added anywhere;
- whether `RecStatus` still derives `Copy`;
- whether the permission tests actually ran, on which platforms, and whether any
  of them skipped.
