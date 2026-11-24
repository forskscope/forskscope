# RFC 102: Messages That Need Attention Are Heard — Errors and Warnings Take Focus

**Status.** Proposed.
**Proposed.** 2026-11-24 by the architect, for **F226**. The owner accepted
the follow-up direction the same day ("Your recommendations are
accepted"):
- state the Linux limitation plainly;
- make what matters heard through focus;
- report the gap to WebKitGTK;
- measure Windows.

This document is the design for the second and fourth.
**Scheduling.** `0.201.0`, with RFC-101 stages B–C.
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

**Every call site keeps its own severity.** Stage A lists them all, so the
owner can see which messages will take focus.

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

- **Unchanged:** a live region, never moving focus.
- **Where the platform announces them,** they're heard; on Linux today,
  they aren't, and the guide says so.

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
