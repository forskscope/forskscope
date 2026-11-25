# RFC 102: Messages That Need Attention Are Heard — Errors and Warnings Take Focus

**Status.** Implemented in `0.201.0` (handoffs 115, 116; reviews 238–243). Passive notices on Windows are still being confirmed (F241).
**Proposed.** 2026-11-24 by the architect, for **F226**. The owner accepted
the follow-up direction the same day ("Your recommendations are
accepted"):
- state the Linux limitation plainly;
- make what matters heard through focus;
- report the gap to WebKitGTK;
- measure Windows.

This document is the design for the second and fourth.
**Accepted.** 2026-11-24 by the project owner ("Yes."), with `0.201.0`'s
scope, warnings included.
**Scheduling.** `0.201.0`:
- stage A (the list, and the Windows measurement) runs alongside RFC-101
  stage B (handoff 115);
- stage B follows the owner's review (handoff 116).
**Amended.** 2026-11-25, by the owner's answers to stage A's list: the
large-file notice is reclassified as info (§1).
**Amended.** 2026-11-25, after stage A's Windows measurement: passive
notices announce through one permanent, in-place live region (§3).
**Touches.**
- `crates/forskscope-ui`: the notice (toast) component, `state.rs`'s
  `Notice`, focus handling, and the CSS;
- the i18n table;
- the guide's accessibility and known-limitations pages.

## Summary

- **An error or warning notice takes keyboard focus when it appears,** so
  a screen reader speaks it on every platform, including Linux, where
  live regions are silent (F226).
- **Success and information notices stay as they are:** passive, and
  never moving focus.
- **Focus returns** to where it was when the notice is dismissed.
- **Windows is measured first:** whether WebView2's live regions reach UI
  Automation decides only what the guide says, not this design.

## Motivation

F226's measurement (reply 195, review 235):
- **on WebKitGTK 2.54.3, no form of live region produces the event Orca
  announces;**
- **focus changes are spoken.**

So, on Linux, every notice the app shows without moving focus is silent
for a screen-reader user. That includes **"Could not save settings"**
(F62) and every other error. **A user who can't see the screen doesn't
learn that something failed.**

**WCAG 4.1.3** asks that *status* messages be announced without taking
focus. **An error that needs the user's attention isn't only a status
message:** taking focus for it is the established pattern (an alert
dialog). Doing it for errors and warnings only keeps the interruption for
what deserves it.

## Design

### 1. Which notices

| Severity | Today | After |
|---|---|---|
| **Error** | a toast; stays until clicked | **takes focus** (§2) |
| **Warning** | a toast | **takes focus** (§2) |
| **Info** | a toast; dismisses itself | unchanged: passive |
| **Success** | a toast; dismisses itself (some offer *Open*) | unchanged: passive |

**Every call site keeps its own severity,** with one exception the owner
decided after stage A's list (2026-11-25): **"Large file — inline diff
disabled." becomes an info notice.** It describes the view, needs no
action, and appears on every open of a medium-sized file. The other
warning, an unrecognised encoding replaced by UTF-8, still takes focus,
because it concerns the user's data.

### 2. How a notice takes focus

- **The toast keeps its place and look,** bottom centre, so sighted users
  see what they see today.
- **When an error or warning appears,** focus moves to the toast's own
  focusable element:
  - **its *Dismiss* button**, named "Dismiss" and described by the
    message (`aria-describedby`), so a screen reader speaks the message
    as focus arrives;
  - or **its action button,** where it has one, described the same way.
- **`role="alertdialog"`** on the toast while it holds focus, labelled by
  its message.
- **Escape or *Dismiss* closes it,** and **focus returns** to the element
  that had it. If that element is gone, focus goes to the main view's own
  starting point.
- **While a dialog is open,** the toast sits above it (z-index 110, F225)
  and takes focus the same way. **Focus returns into the dialog,**
  inside its focus trap.
- **Typing isn't swallowed.** A notice that arrives while a text field
  has focus still takes focus, because errors are rare and need
  attention. **The text typed so far is kept,** and Escape returns to the
  field.
- **Two errors in quick succession:** the newer replaces the older, as
  today, and focus stays on the toast.

### 3. Passive notices

**Amended 2026-11-25, after stage A's Windows measurement** (reply 203,
review 239):
- **The finding:** on WebView2, a region inserted with its text (the
  toast's own shape) raised `LiveRegionChanged` 0–1 times in 3;
- **while a permanent region updated in place** (cleared, then set),
  and `role="alert"`, raised it every time.

**So:**
- **every passive notice is announced through one permanent, visually
  hidden live region** (`role="status"`), mounted once with the app;
  - **cleared, then set** for each notice;
  - never moving focus.
- **The visible toast stays as it is,** but no longer carries the live
  region itself, **so nothing is announced twice.**
- **On Windows, passive notices are then heard.** On Linux they aren't
  (no shape is announced there, F226), and the guide says so for each
  platform.
- **Their text and placeholders** come from the catalog, isolated as
  RFC-101 decides (review 239 §2.1).

### 4. Windows: measure first

**Stage A:**
- **a scratch CI run on `windows-latest`** (with the architect's go-ahead
  on its job file) hosts F226's five shapes in WebView2;
- it records, through UI Automation, whether each raises
  `LiveRegionChanged`.

**The result decides only what the guide says about Windows.** The focus
design (§2) applies on every platform regardless, so behaviour is the
same everywhere.

## Non-goals

- **Making passive notices heard on Linux.** That needs WebKitGTK, and the
  report is the owner's.
- **Search counts and the Explorer's status.** They stay passive. A user
  can reach them with the keyboard. Making them speak on Linux would mean
  stealing focus for information that doesn't need it.
- **macOS measurement.** VoiceOver can't be automated in CI in a way that
  captures speech (reply 195).

## Security and privacy

- **No network, and no new data flow.**
- **A scratch CI run** leaves a public trace. It needs the architect's
  go-ahead on its job file, and is deleted afterwards.

## Verification

1. **Stage A:**
   - the list of every error and warning call site, with its message, for
     the owner;
   - **the Windows measurement table.**
2. **Markup tests:**
   - an error notice renders `role="alertdialog"` with its *Dismiss* named
     and described by the message;
   - a success notice renders neither.
3. **Focus,** through the real event path:
   - an error moves focus to *Dismiss*;
   - Escape returns it to the field that had it, with its text intact;
   - inside an open dialog, focus returns into the dialog.
4. **Orca, Level 2** (F226's harness: the private accessibility bus,
   audio blocked):
   - **an error notice is spoken,** with its message;
   - three runs.
5. **RFC-098's gate stays green.** The toast's occlusion allowance is
   unchanged.

## Open questions

None. One choice can be revisited after the owner sees stage A's list:
**warnings take focus too.**
