# Handoff 002 — F74: the Explorer claims directories are equal without comparing them

**From:** architect
**Date:** 2026-08-18
**Register:** F74 (defect), F75 (related, **not** in scope — read §5)
**Gate:** Gate D input. Assume this blocks the v1 candidate.

## 1. The defect

`crates/forskscope-ui/src/ui/view/explorer.rs:251-258`:

```rust
if is_dir {
    let state = if cp.is_dir() {
        DigestState::Equal
    } else {
        DigestState::Unique
    };
    digest_map.write().insert(DigestKey::Common(rel), state);
}
```

A directory row is marked **`Equal`** because a directory of the same name exists
on the other side. Its contents are never examined — no digest, no recursion, no
metadata, no deferral to the deep scan. Two directories whose contents differ
both render `✓`.

Reported by the project owner on 2026-08-18 running the published Windows build:
a `✓` in the Explorer tab for a pair whose Deep Compare tab then listed real
differences. This is a false *equal* in a diff tool — the direction where the
user stops looking.

**Two more defects travel with it. All three are in scope.**

- **`explorer/filter.rs:138-148`** — the *hide identical* filter matches
  `Some(DigestState::Equal)` on `DigestKey::Common` with no directory exemption.
  Switch it on and directories containing differences **disappear from the
  pane**. A misleading mark becomes invisible content.
- **`dir_pane.rs:286`** — the status glyph renders as
  `span { class: "tree-status {st_cls}", "{st_icon}" }`: glyph and colour, **no
  accessible label**, while the `bin` badge three lines above does carry a
  `title`. RFC-009 §7 forbids status by styling alone; a screen reader gets a
  bare `✓`. Your fix touches this same `match`, so repair it here.

## 2. The design decision — taken, not open

Do not re-derive this and do not substitute a different answer without coming
back to me first.

1. **A directory row must show a status meaning *not compared*** — a state
   distinct from both `Equal` and `Different`, with its own glyph and CSS class.
   Add a variant to `DigestState`. `Unique` is not it; `Unique` means *present on
   one side only*, which remains correct and unchanged for the `!cp.is_dir()`
   branch.
2. **`hide_eq` must never hide a directory row**, whatever its state. Hiding is
   for rows proven identical, and a directory is never proven identical here.
3. **Every status gets an accessible label**, the new one included, in the same
   `match`. Route it through `t(lang, …)` like the `bin` badge — new strings mean
   `cargo xtask i18n` has something to say, so run it.

**Explicitly out of scope — do not do these:**

- **Do not make the Explorer recurse or digest subtrees.** Directory verdicts are
  a *feature*, already provided by Deep Compare. Eagerly digesting every subtree
  on every navigation is a performance decision nobody has taken, and it is not
  yours or mine to take inside a bug fix.
- **Do not wire `RowStatusKind`** (see §5).

## 3. Falsifiability — the usual standard, no exceptions

Every check you add must be **demonstrated failing** on a deliberately broken
input, and the failing run recorded. A test that passes against both the fixed
and the broken code proves nothing, and this program has rejected that twice.

Concretely, at minimum:

- Two directories, same name both sides, **differing contents** → the row's state
  is *not compared*, never `Equal`. Break the fix, watch it fail, record it.
- The same pair with *hide identical* **on** → the row is still visible. Break the
  exemption, watch it fail, record it.
- Every `DigestState` variant yields a non-empty accessible label.

`filter.rs` and the digest block are plain functions over a `HashMap`; these are
unit tests in `forskscope-ui-logic`-style, not runtime evidence. **No RFC-078
harness work is needed for this handoff.**

## 4. What this does *not* need to fix

F74's register entry carries the full reasoning; the short version is that the
Explorer showing an honest *not compared* for directories is the correct end
state, not a stopgap. You are not building toward a recursive verdict later.

## 5. F75 — related, and deliberately not yours right now

`forskscope-ui-logic/src/explore/status.rs` contains `RowStatusKind`, whose own
module doc says it *"replaces the ad-hoc `DigestState` enum"*. It does not —
nothing in `forskscope-ui` references it. It is one of **nine** `ui-logic`
modules with no consumer anywhere in the workspace (F75, extending F54).

Wiring it would be the tidier fix and it is **not** what I want now: a UI-wide
type swap during v1 stabilization is the larger risk, and F75 is scheduled
post-Gate-D as one deliberate change that deletes `DigestState` and carries the
RFC-009 label path across properly. Fix F74 **inside `DigestState`**. Your work
will be superseded by that change rather than duplicated by it, which is
intended.

If you think that sequencing is wrong, say so in your review request rather than
acting on it.

## 6. Deliverable

- The fix, with the demonstrated-failing evidence for each check.
- Gates green: `cargo fmt --check`, `clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace`, `cargo xtask css --check`, `cargo xtask
  i18n`.
- A review request in `dev-record/review-requests/`, numbered next in sequence.

State plainly in the request whether the *hide identical* exemption and the
accessible labels are both done — those are the two most likely to be dropped,
because neither is what the bug report was about.
