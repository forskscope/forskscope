# Developer Handoff 012 — F52: give save failures a real dialog

**From:** architect
**Date:** 2026-08-27
**Register:** F52 (and F54, which named this "a real UI workstream").
Wires one of F75(b)'s four KEEP modules.
**Gate:** Not a Gate D blocker.
**Sequencing:** after handoff 011, or in parallel — they share no files.

---

## 1. Task title

Route save failures through `SaveErrorView` instead of a bare toast, without
disturbing the conflict path.

## 2. Purpose

`forskscope-ui-logic::compare::save_error` maps an `AppError` to a title, body,
path context and an **ordered list of recovery buttons**. It has 14 tests and no
consumer.

Today a failed save reaches the user as `store.notify(<string>)` — one line of
text, no path context, and **no way to act on it**. The user is told writing
failed and left to work out what to do.

The pieces to close this already exist: `handle_result` (`ui/view/diff_actions.rs`)
receives `Result<SaveOutcome, CoreError>`, and **`AppError::from_core(&CoreError)`
is already implemented** (`core/src/error/app.rs:399`). Nothing needs inventing.

## 3. The constraint that matters most

**The conflict path must not change.**

`handle_result` has an `Err(CoreError::Conflict { .. })` arm that raises
`Modal::ConfirmOverwrite(index, target)`. That is RFC-077 machinery, and review
048 C1 pinned it hard: confirming must overwrite **the exact path that save
attempted**, never whatever the tab's current target happens to be.

**Do not route `Conflict` through `SaveErrorView`.** It is not a message to
display; it is a flow with a safety property attached. Leave that arm exactly as
it is, and make everything *else* go through the new dialog.

If you find `SaveErrorView::from_error` produces something plausible for a
conflict, that is not a reason to use it here. Say so in the review request and
leave the arm alone.

## 4. Required implementation

- A new `Modal` variant carrying the `SaveErrorView` (or what the dialog needs to
  render it), alongside the existing `ConfirmOverwrite` / `SaveAs` /
  `ConfirmSaveAsOverwrite` variants.
- In `handle_result`, every error arm **except `Conflict`** builds
  `AppError::from_core(&err)`, then `SaveErrorView::from_error(&app_err, path)`,
  and raises that modal. The path argument is the target the save attempted —
  the same value the conflict arm is careful about, for the same reason.
- The dialog renders title, body, optional path, and the buttons **in the order
  the view-model gives them**, with `is_primary` marked. Ordering is a decision
  the view-model already made; do not re-sort it.
- Accessibility: follow `dir_pane.rs`'s established pattern — the modal itself
  already has precedent in `ui/overlay/modals/`, so match what is there rather
  than inventing a third convention.

### 4a. The recovery actions — enumerate before implementing

`RecoveryAction` has **twelve** variants (`Dismiss`, `ChooseAnotherFile`,
`Reload`, `SaveAs`, `OverwriteAnyway`, `OpenLimitedDiff`, `OpenAsBinary`,
`Retry`, `RetryWithoutInline`, `Cancel`, `StartFresh`, `ReportBug`). Most describe
*load* failures and cannot arise from a save.

**Work out which are actually reachable** from the `CoreError`s the save path can
produce, and implement those. For the rest, do not add a silent default — an
unreachable action reached is a bug, and it should be visible as one.

**Then pin it with a test**: every action a save-path error can produce has a
handler. That converts "I checked" into something that stays true.

Tell me the reachable set in your review request. If it is one or two — plausibly
just `Retry`, `SaveAs`, `Dismiss` — then say so, and the dialog is small.

## 5. Explicit non-change scope

- **The `Conflict` arm** (§3).
- **`precheck_save_as_target` and `SaveAsPrecheck`** — a separate, earlier gate
  (review 050 §3.2). Not this.
- **Session-save failures** (`state/session.rs:44`), which are a different path
  with a different user relationship. Out of scope; do not fold them in.
- The other KEEP modules; the no-allowlist gate; `ROADMAP.md`.

## 6. Required tests

Falsified against real behaviour, not a helper the fix introduces.

1. **A non-conflict save failure produces a `SaveErrorView`-backed modal, not a
   toast.** Drive the real `handle_result` with a real `CoreError` — the way
   review 077 fed `classify_digest_outcome` a real `Err` from
   `file_digest_equal_with_cancel` rather than a hand-built error.
   **Falsify by restoring the `notify` call.**
2. **A conflict still raises `ConfirmOverwrite`, with the attempted path.**
   **Falsify by routing `Conflict` through the new dialog** — that must fail.
   This is the regression guard for §3 and it matters more than test 1.
3. **Every reachable `RecoveryAction` has a handler** (§4a).

## 7. Acceptance criteria

- Non-conflict save failures show the dialog, with title, body, path and buttons
  in view-model order.
- The conflict flow is byte-identical in behaviour, and a test proves it.
- No `RecoveryAction` reachable from a save error is unhandled.
- Gates green, as handoff 011 §8.

## 8. Prohibited shortcuts

- **Do not route `Conflict` through `SaveErrorView`.**
- **Do not re-sort or filter the buttons.** The view-model ordered them.
- **Do not add a catch-all `_ =>` arm over `RecoveryAction`** — that is how the
  thirteenth variant gets silently swallowed, and this codebase has already
  recorded that failure shape (handoff 006 §14).
- **Do not report a falsification you did not run.**

## 9. Required review-request format

Lead with the two falsifications, especially the conflict-path one.

State plainly:
- the reachable `RecoveryAction` set, and how you determined it;
- whether any button renders without a working handler;
- that the conflict arm is untouched, and how you showed it.
