# Developer Handoff 007 — F75(a) + F76: wire `RowStatusKind`, delete `DigestState`

**From:** architect
**Date:** 2026-08-22
**Register:** F75 (its wiring half), F76 (both instances, folded in). F80 — not in scope.
**Gate:** Neither is a Gate D blocker. This work is authorised *during* the F44/F60
wait, not deferred behind it — see §3.

---

## 1. Task title

Make the Explorer emit `EqualityEvidence` and render it through
`RowStatusKind`, delete `DigestState`, and let F76's two defects fall out.

## 2. Purpose

`forskscope-ui-logic/src/explore/status.rs` says in its own module doc that
`RowStatusKind` *"replaces the ad-hoc `DigestState` enum in `ui/dir_pane.rs`"*.
It does not — nothing in `forskscope-ui` references it. Its 16 tests are green
about code no user reaches.

Wiring it fixes **F76** by construction rather than by separate effort:

- **Type mismatch** — a directory on one side and a same-named file on the other
  is currently `DigestState::Unique`, labelled *"Only on this side"*, which is
  **false**: it exists on both sides. `EqualityEvidence::TypeMismatch` maps to
  `Different`, which is true.
- **Read failure** — currently `Err(_) => DigestState::Different`, asserting a
  verdict nothing established. `EqualityEvidence::Error` maps to `Error`.

## 3. Background — ordering, and a correction you should know about

RFC-080 Q4 originally said **F76, then F75**. That was my recommendation and it
was incoherent: F76's fix would have added states to `DigestState`, the enum F75
deletes. Handoff 006 §6 told you the opposite — *"F76 stays scheduled with F75…
adding states to a condemned enum would be waste"* — and that version is right.
Corrected 2026-08-22; **F76 is a consequence of this wiring, not a predecessor.**

This work is also being done **now rather than after Gate D**, by the owner's
decision. The reason it was deferred — avoiding a matrix re-run — expired: M5's
evidence is tied to `0.167.1` and seven code commits have landed since, so a
re-cut and full re-run are already mandatory. RFC-080 itself still waits, because
it is a feature and this is not.

## 4. Two defects in `RowStatusKind` you must fix before wiring it

Found while checking whether wiring was mechanical. **It is not.** Both are in
the unwired module, and its own tests pass over both.

- **`EqualityEvidence::Unknown` maps to `RowStatusKind::Computing`**
  (`status.rs:95`). *Not attempted* and *in progress* are different claims, and
  rendering the second for the first gives a row a **spinner that never
  resolves**.
- **`RowStatusKind` has no `NotCompared`.** That is the state a directory row has
  shown since `16c35f1` (F74). Wiring the enum as it stands would **silently undo
  F74's fix** — a Gate D blocker's repair, reversed by a refactor.

These are not incidental. They are why this handoff exists rather than a
find-and-replace.

## 5. Change scope

- `crates/forskscope-ui-logic/src/explore/status.rs` — the two fixes above
- `crates/forskscope-ui/src/ui/view/explorer.rs` — emit evidence
- `crates/forskscope-ui/src/ui/view/dir_pane.rs` — render through `RowStatusKind`;
  `DigestState` deleted
- `crates/forskscope-ui/src/ui/view/explorer/filter.rs` — the `hide_eq` predicate
- `crates/forskscope-ui/src/i18n.rs`, CSS — as the states require

## 6. Explicit non-change scope

- **The other eight unwired `ui-logic` modules, and the no-allowlist gate.** That
  is F75's other half and a separate handoff. Do not touch `command_bar`,
  `conflict_nav_view`, `load_guard`, `palette_view`, `save_error`, `scroll_sync`,
  `summary`, `tab_state`.
- **F80** — Deep Compare's five unlabelled status glyphs. Different view,
  different enum (`RecStatus`), separate finding.
- **No RFC-080 work** — no tiers, no size cap, no *tier-1 match* state.
- **Deep Compare is untouched.** It renders `RecStatus`, not `DigestState`.

## 7. Required implementation

### 7a. Fix `status.rs` first, on its own

Add a `NotCompared` kind with glyph, CSS class and screen-reader label, and map
`EqualityEvidence::Unknown` to it. **Do this before wiring anything**, so the
module's tests fail meaningfully rather than being retrofitted to match whatever
the wiring produced.

### 7b. The Explorer emits evidence, not display states

`classify_entry` currently returns `DigestState`. It should produce
`EqualityEvidence` — core's vocabulary, which already distinguishes the cases the
Explorer has been flattening. Then:

- a directory whose counterpart is a directory → the evidence meaning *not
  compared* (see 7a);
- a directory whose counterpart is a **file**, or the mirror → `TypeMismatch`,
  **not** `LeftOnly`/`RightOnly`. This is F76's first instance;
- a failed digest → `Error`, **not** `DigestDifferent`. This is the second.

**One shape decision is yours, and it has a real trade-off.** `DigestState` is
`Copy`; `EqualityEvidence::Error { message: String }` is not. So either the map
stores evidence (richer, loses `Copy`, and RFC-080 will later want the evidence
for its labels) or it stores `RowStatusKind` and discards the detail (`Copy`
preserved, message lost). **Pick one and argue it in the review request.** I lean
toward storing the evidence, because a message nobody kept cannot be shown later
— but I have not weighed the `Copy` ripple and you will have.

### 7c. Rendering and filtering

`dir_pane.rs` renders from `RowStatusKind`, using its `glyph()`, CSS class and
label, with the `role="img"` + `aria_label` pattern F74 established. **`DigestState`
must not exist when you are done.**

`filter.rs`'s `hide_eq` must keep F74's guarantee: a **directory row is never
hidden**, whatever its state, because the Explorer never proves a directory
identical. That guard is currently written against `is_dir` directly, which is
correct — keep it that way, and keep its test.

## 8. Required tests

Falsified against the shipped defect, not against a helper the change introduces.

1. **A type mismatch is not reported as present-on-one-side.** Real temp dirs: a
   directory named `X` on the left, a file named `X` on the right. Falsify by
   restoring the `Unique` classification.
2. **A failed comparison is not reported as `Different`.** Falsify by restoring
   `Err(_) => Different`.
3. **`Unknown` does not render as `Computing`.** Falsify by restoring the old
   mapping in `status.rs`.
4. **A directory pair still shows *not compared*** — F74's behaviour survives the
   refactor. Falsify by removing `NotCompared` from the mapping.
5. **F74's existing checks still pass and still bite.** Re-run the two
   falsifications from review 073 (the real-path classification, and the
   `hide_eq` directory exemption) against the converted code and confirm each
   still fails. If either has gone quiet, the refactor weakened it.

## 9. Required documentation updates

None. **Do not edit `ROADMAP.md`.**

## 10. Acceptance criteria

- `DigestState` does not exist.
- `grep RowStatusKind crates/forskscope-ui/src` returns real call sites.
- Every check in §8 fails when its defect is restored.
- No `_ =>` arm is added on any status enum.
- Gates green: `cargo fmt --check`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo test --workspace`, `cargo xtask css --check`,
  `cargo xtask i18n`, `git diff --check`.

## 11. Prohibited shortcuts

- **Do not keep `DigestState` alongside `RowStatusKind`.** Two vocabularies is
  the defect, not the fix.
- **Do not map anything to `Computing` to avoid adding `NotCompared`.** That is
  §4's first defect, chosen deliberately.
- **Do not retrofit `status.rs`'s tests to whatever the wiring happens to
  produce.** Fix the mappings first (§7a), then wire.
- **Do not report a falsification you did not run.**

## 12. Relevant code or module boundaries

`ui-logic` depends only on `forskscope-core` — keep it that way.
`EqualityEvidence` is already a core type, so the Explorer emitting it introduces
no new dependency.

## 13. Compatibility and security constraints

No persistence format carries either enum. No public API outside this workspace.

## 14. Known risks

- **This refactor can silently undo F74**, which is why §8.4 and §8.5 exist. F74
  was a Gate D blocker; a refactor that quietly reverses it would be the worst
  outcome available here.
- **`Copy` loss** if you store evidence — see §7b. Expect it to ripple into props
  and helper signatures. That is a real cost and may be the deciding factor;
  measure it rather than assuming either way.

## 15. Required evidence

- Observed failure output for each of §8's five falsifications, quoted.
- The storage-shape decision (§7b) and the argument for it.
- Confirmation that `DigestState` is gone.

## 16. Required review-request format

As in requests 071–074. Lead with the falsifications.

State plainly:
- whether F74's two checks still bite after the refactor;
- what you stored in the map, and what it cost;
- whether `RowStatusKind` gained anything beyond `NotCompared`.
