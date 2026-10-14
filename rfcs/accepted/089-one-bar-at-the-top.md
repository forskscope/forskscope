# RFC 089: One Bar at the Top, and a Window Title That Says What Is Open

**Status.** Accepted — review complete; implementer may start. Moves to `done/`
when the work ships (RFC-000, 5-folder variant).
**Accepted.** 2026-10-15 by the project owner.
**Scheduling.** `0.191.0`. See `ROADMAP.md` §
"Remaining proposed RFCs".
**Tracks.** The owner's question, 2026-10-14: is the app header row optimized
enough? The owner accepted the direction on 2026-10-15.
**Amends.** RFC-003 (the shell's `AppHeader` region) and RFC-057
(which placed About in the Settings dialog because the global header carried
Settings and `?`). Each carries a one-line Status note pointing here.
**Touches.**
- `crates/forskscope-ui/src/app.rs` (shell order)
- `crates/forskscope-ui/src/ui/layout/header.rs` (removed)
- `crates/forskscope-ui/src/ui/layout/tabs.rs` (the controls move here)
- `crates/forskscope-ui/assets/css/02-layout-shell.css`
- `crates/forskscope-ui/src/main.rs` (the initial title)
- `docs/src/users/*` where the header is described

## Summary

- **Remove the app's header row.** Its *Settings* and *?* buttons move to the
  right end of the tab bar.
- **The window title names what is open,** for example *app.js ↔ app.js —
  ForskScope*. The app's name stays where the system shows it and where it is
  useful.

## Motivation

The top of the window stacks three bars:

1. **The system's title bar**, which already reads *ForskScope*.
2. **The app's header** (`header.rs`): the word *ForskScope* in bold, a spacer,
   *Settings*, and *?*. It is a full row, and its only working content is two
   buttons.
3. **The tab bar.**

The header's name repeats the title bar, and the row costs content height on
every screen: a diff shows one line less, and the Explorer one row less. The
window title, meanwhile, says *ForskScope* whatever is open, so the taskbar and
the window switcher cannot tell two windows or two comparisons apart.

Browsers and editors put app-level controls at the end of the tab bar. That
convention fits here: two buttons, used occasionally.

An alternative is a single ☰ menu holding Settings, keyboard shortcuts and
About. It has fewer visible controls, but it hides help behind a menu, so it is
not adopted.

## Design

### 1. The tab bar holds the controls

- `Header` is removed from the shell. `app.rs` renders the `TabBar` first.
- Inside the tab bar, after the tab list:
  - a flexible spacer;
  - **Settings**: the same text button, same label, same action;
  - **?**: same title and action, *Keyboard shortcuts (Ctrl+/)*.
- **The two buttons are outside `role="tablist"`** (RFC-087), as siblings of the
  list inside the bar, so the tab list's arrow-key navigation never lands on
  them. They are ordinary Tab stops after the tab list.
- **They stay at the end of the bar** however many tabs are open. The tab list
  takes the remaining width and keeps its current overflow behaviour; the
  buttons never move off-screen. Under 720 px, where the tab bar already wraps
  (`90-responsive.css`), the buttons stay on the first line at its end.
- `ControlZone` (RFC-060 Part C) wraps the tab bar already, so Enter on either
  button keeps its own meaning, as it does in the header today.
- **The brand label is removed.** The name is still in:
  - the window title (§2);
  - the About dialog;
  - the Explorer's empty state.

### 2. The window title

| Selected tab | Window title |
|---|---|
| Explorer | *ForskScope* |
| A file comparison | *{tab title} — ForskScope*, e.g. *app.js ↔ app.js — ForskScope* |
| A file comparison with unsaved changes | *● {tab title} — ForskScope* |
| A folder comparison | *{tab title} — ForskScope* |

- The `{tab title}` is the same text the tab shows, so the two never disagree.
- It updates whenever the selected tab, a tab's title, or its unsaved state
  changes. Use one effect on those signals, calling the desktop window's
  `set_title`.
- **Privacy:** file names already appear in the tab bar. The window title shows
  them to the system too: the taskbar, the window switcher and screen sharing.
  That is the same convention as every editor. It sends nothing anywhere.

## Non-goals

- A menu bar or an app menu.
- Moving any other control.
- Tab overflow behaviour (scrolling or a tab menu). Today's behaviour stays.
- macOS's own app menu.

## Acceptance criteria

1. **No header row.** Settings and ? sit at the right end of the tab bar, with
   one tab and with many. Show it with screenshots in the Dark theme at 1280 px
   and at 650 px.
2. **The tab list's ← → never reaches the two buttons.** Tab reaches them after
   the tab list. Both open what they open today. Ctrl+/ still opens the
   keyboard reference.
3. **The window title follows the table in §2.** Read it from the window
   manager (`niri msg -j windows` gives each window's title) as tabs are
   selected, an unsaved change is made, and the change is saved or undone.
4. **No test or code still refers to `header.rs`'s `Header`.** The keyboard
   tests that rendered it render the tab bar instead.
5. **The user guide no longer describes a header with the app's name.**

## Open questions

None.
