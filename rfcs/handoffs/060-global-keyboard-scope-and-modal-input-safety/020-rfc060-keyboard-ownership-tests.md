# Handoff 020 — RFC-060: make the keyboard-ownership rule enforceable

**From:** architect. **Priority:** not release-blocking. Gate D has no
in-project blocker; take this when you have capacity.
**RFC:** `rfcs/done/060-global-keyboard-scope-and-modal-input-safety.md`
**Register:** F54's *"what stops the sixth?"*, in the keyboard layer.

## 1. Why this exists

RFC-060 was moved to `done/` on 2026-09-02 because its **rule** ships: the
global `onkeydown` yields to modals and to text inputs. Its **stated purpose**
did not:

> so the class of bug cannot reappear as new shortcuts or new input surfaces are
> added

Nothing enforces that today. A new text input that forgets `stop_propagation`
silently re-creates the original defect — Ctrl+S writing the file behind an
overwrite dialog, Enter applying a hunk while you are typing. The guard is
correct and entirely remembered.

**This is the fourth+ instance of the project's own catalogued pattern**: the
right thing is built, then nothing holds it in place.

## 2. Correcting my own count before you rely on it

The register and my report to the owner said **six surfaces** carry the guard.
That is wrong, and I found it while scoping this. Several of those are
`onclick` guards, which have nothing to do with keyboard ownership. The
**keyboard**-relevant guards are:

| File | Line | Shape |
|---|---|---|
| `app.rs` | 118 | `modal_open` → swallow all global shortcuts |
| `ui/view/dir_pane.rs` | 186, 192 | `stop_propagation` (comment cites RFC-060 W1) |
| `ui/view/search.rs` | 90, 99 | `stop_propagation` |
| `ui/view/explorer/filter.rs` | 41 | blanket `onkeydown: stop_propagation` |
| `ui/view/settings/modal.rs` | 27 | `stop_propagation` (cites RFC-060 W1) |

`ui/overlay/keybindings.rs:21` is `onclick` only — **not** a keyboard guard.
Verify this table yourself before building on it; I have been wrong about this
file set once already today.

## 3. The trap, stated first

`app.rs`'s guard lives **inside a Dioxus event closure**. It cannot be called
from a test as written. The obvious workarounds are both worse than no test:

- **Re-implementing the decision in the test** produces a green check that
  measures a copy, not the shipped path. This project already has a name for
  that outcome: *a green gate credited with more than it measures.*
- **A structural grep** for `stop_propagation` across `onkeydown` handlers is a
  decoration — it passes for a handler that calls it in the wrong branch.

**So the work is to make the rule testable, not to test around it.**

## 4. Part A — extract the global-key decision

Lift the decision out of the closure into a pure function, and leave the closure
as a thin dispatcher that calls it.

Shape (yours to refine — the signature matters more than the name):

```rust
fn global_key_action(key: &Key, mods: Modifiers, modal: &Modal, has_active_tab: bool)
    -> GlobalKeyAction   // Ignore | CloseModal | MoveFocus(i32) | Save | ...
```

Requirements:

- **Behaviour must not change.** This path guards Ctrl+S against writing behind
  an overwrite dialog. Escape's semantics are subtle and deliberate: it closes
  an ordinary modal, and **must not** close `SettingsRecovery` / `SessionRecovery`
  (RFC-076 patch 6 — a recovery choice is never made by an accidental keypress).
  Preserve that exactly.
- **Wire it in the same change.** `ui-logic` has nine modules that exist without
  consumers (F75). If this lands unwired it becomes the tenth and this handoff
  will have made things worse. Put it wherever it is genuinely used from; a
  private module inside `forskscope-ui` is fine.

## 5. Part B — make the input obligation structural

Part A does not stop a *new text input* from forgetting `stop_propagation` —
that is a per-surface obligation, and per-surface obligations are exactly what
gets forgotten.

Give it **one shared helper** (a small wrapper component, or a
`swallow_when_typing(&Event<KeyboardData>)` helper) and convert the four
keyboard surfaces in §2 to it. The obligation is then discharged by
construction: a new input uses the helper, or it is visibly not using it in
review.

If you conclude a shared helper is the wrong shape here — for instance because
the four call sites differ more than they look — **say so and propose the
alternative rather than converting them anyway.** You were right to push back on
handoff 018 and I would rather have that than compliance.

## 6. Falsification — the part that decides whether this was worth doing

For **each** test, demonstrate it **failing** against the shipped defect:

1. Remove `app.rs`'s `modal_open` early return → a test asserting Ctrl+S does not
   save while a modal is open must **fail**.
2. Remove the `SettingsRecovery`/`SessionRecovery` exclusion → a test asserting
   Escape does not dismiss a recovery dialog must **fail**.
3. Remove `stop_propagation` from **one** converted input surface → a test
   asserting typing there does not fire the global action must **fail**.

Paste the actual failure output, as you did for F88. **Falsify against the
shipped path, not against the helper the fix introduces** — that distinction is
the whole point of §3.

## 7. Scope

**In:** `app.rs`'s keydown closure, the four surfaces in §2, the new pure
function and helper, their tests.

**Out:** every `onclick` guard; new shortcuts; changing any key binding;
`keybindings.rs`; RFC-063/072 work. If Part A tempts you into a wider `app.rs`
refactor, stop — `app.rs` is not in scope beyond this closure.

## 8. Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo xtask css --check`, `version-sync`, `i18n`,
`rfc-sync`, `git diff --check`.

RFC-060 is in `done/` and **stays there** — this handoff discharges a deferred
note, it does not reopen the RFC. Do not move the file.

## 9. Reporting

Usual review request under `dev-record/review-requests/`. Tell me if §5's
shared helper turns out to be the wrong shape; that is the judgement call in
this handoff, and it is yours to make.
