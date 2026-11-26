# Handoff 077 — 0.190.0: every tab and profile by keyboard

**From:** architect. **Date:** 2026-10-14. **Release:** `0.190.0`.
**Register:** F192. **This is the whole release.**

**This handoff implements RFC-087**
(`rfcs/accepted/087-keyboard-access-to-tabs-and-profiles.md`), accepted by the
owner on 2026-10-14. **The RFC is the authority.** Where this document and the
RFC differ, the RFC wins. If implementation shows the RFC is wrong anywhere, stop
and tell me; I take it back to the owner. Do not change the design yourselves,
and do not edit any RFC.

Your request 151 found the problem: the tab bar and the Settings profile rows cannot be reached by AT-SPI.
That is because they cannot be reached by keyboard at all.

**Today:**

- `ui/layout/tabs.rs`: every tab is a `div`, its title a `span` with `onclick`.
  Nothing has a `tabindex`, a role or a key handler. The close buttons are real
  `<button>`s, so a keyboard user can Tab to a tab's **close** button but never
  to the tab itself.
- `keyboard.rs`: no tab-switching shortcut. `global_key_action` also returns
  `Ignore` for everything while the Explorer is active (`!has_active_tab`).
- `ui/view/settings/modal.rs`: each profile row's name is a `span` with
  `onclick`. Choosing the active profile is mouse-only.

So a keyboard-only user cannot get back to the Explorer from a comparison,
cannot move between comparisons, and cannot choose a profile. The README and
the website both say *Keyboard first*.

I checked every other `div`/`span` with `onclick` in `ui/`. Each one already
has a keyboard route:

- the breadcrumb's current folder name: *Edit path* in the ⋯ menu;
- tree rows and carets: the tree's own arrow keys and Enter/Space;
- the rest: modal action rows and containers.

So the tab bar and the profile list are the scope. If you find a third
mouse-only control, **report it and do not fix it here**. It is outside the
accepted RFC.

---

## 1. The tab bar becomes a real tab list

Follow the WAI-ARIA **tabs** pattern, with automatic activation.

**Markup:**

- The bar is `role="tablist"` with an accessible name, *Open tabs* /
  *開いているタブ*.
- Each tab, the Explorer included, is `role="tab"` with `aria-selected`.
- Its accessible name is its title. A file tab with unsaved changes has
  *, unsaved* appended (Japanese to match), so the `●` dot is not the only cue.
  A tab still loading says so.
- **Roving tabindex:** only the selected tab has `tabindex="0"`; the others
  have `-1`. Tab moves into the list once and out once.
- The content area is `role="tabpanel"`, labelled by the selected tab.

**Keys on a focused tab:**

| Key | Action |
|---|---|
| ← / → | Select the previous / next tab, and focus it. Wraps. |
| Home / End | Select the first tab (the Explorer) / the last tab. |
| Delete | Close the focused tab, if it can be closed. It must go through the **same** close path as the × button and `Ctrl+W`, so unsaved changes still ask first. The Explorer tab cannot be closed; Delete does nothing on it. |

**The × buttons** get `tabindex="-1"`. They stay for the mouse. The keyboard
closes a tab with Delete on the tab, or `Ctrl+W`. That keeps one Tab stop for
the whole tab list, as the pattern intends.

**Focus after a close:** move focus to the tab that became selected, not to
nothing.

## 2. Shortcuts that work from anywhere

| Shortcut | Action |
|---|---|
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab. Wraps, Explorer included. |
| `Ctrl+PageDown` / `Ctrl+PageUp` | The same. |

- They work **while the Explorer is active**. Today the global handler ignores
  everything without an active comparison tab. Put the check where it belongs:
  tab switching needs no active tab, and saving does.
- **Keys reaching the WebView:** WebKitGTK or the window may consume
  `Ctrl+Tab` before the page sees it. Find out live which of the two pairs
  actually arrive. If one never does, ship the other, and say which in the
  response. Do not document a shortcut that does not work.
- **Text fields:** inside a text field (the path editor, the filter box, search,
  the profile name), `swallow_when_typing` already stops keys reaching the
  global handler. Leave it so: `Ctrl+Tab` in a text field does nothing, rather
  than switching tabs under the user's typing.
- **Modals:** while any modal is open, the shortcuts do nothing.
- **If neither pair arrives,** stop and tell me. The RFC's open question
  returns to the owner before anything ships.
- List **only the pair or pairs that arrive** in the keyboard reference overlay
  (`keybindings.rs`) and in `docs/src/intermediate/keyboard.md`.

## 3. The active profile becomes a radio group

The profile list chooses exactly one, which is a radio group. Use native
`<input type="radio">` with a `<label>`, so the browser gives the keyboard
behaviour (arrows move and select) and the right semantics for free.

- Group label: *Active compare profile* / *使用する比較プロファイル*.
- Selecting one does what clicking the name does today: it sets
  `active_profile` and persists.
- The `×` delete button for user profiles stays a real button. Keep it out of
  the radio group's arrow-key path, but reachable by Tab.
- The visual stays as it is: no new visible control, and the row looks the
  same. Hide the native radio circle only if you keep a visible focus ring and
  a visible selected state. This is still inside *Advanced* (the owner's
  settings principle).

## 3a. Rider: F194, save the active tab when it changes

Found by the documentation audit (review 160). 0.189.0 saves the session only
when a tab opens, closes or swaps sides. Clicking a tab does not save, and
nothing saves on quit. So the restored tab is the one active at the last
open or close, not the one the user was on.

**Save the session whenever the active tab changes**, by any route:

- a click;
- this release's keys and shortcuts;
- a close that selects a neighbour.

Put the save in the one place the active tab is set, not at each caller.
This is a small fix by the owner's threshold: no RFC, and the CHANGELOG
records it.

**Test:**

- Open two tabs.
- Select the first.
- Show that `session.json`'s `active_tab` is 0 without any open or close.
- Falsify by removing the save.

**Live, the way request 151 could not:**

1. Open two comparisons.
2. Switch to the first by keyboard.
3. Quit.
4. Relaunch: the first is selected.

## 4. Verification

Use F171's procedure: `GTK_A11Y=atspi`, AT-SPI focus, `wtype` keys, and
`settings.json` / `session.json` or the AT-SPI tree as observables.
Screenshots work (`niri msg action screenshot-window --id <id> --path
<absolute>`); run the app sandboxed, with invented files only.

1. **Unit tests through the functions the app calls:**
   - tab-index arithmetic: wrap, Home/End, close-then-select;
   - `global_key_action` returning tab switching with no active tab, and
     nothing while a modal is open or a text field has focus;
   - the radio selection persisting.
   Falsify each.
2. **Live:**
   - **Tab list:**
     - Open two file comparisons (CLI args or a session file).
     - From the Explorer, `Ctrl+Tab` twice reaches the second comparison.
     - In the tab list, `←` and Home return to the Explorer.
     - Delete on a comparison tab closes it, and a dirty one asks first.
   - **Observables:** read the AT-SPI tree for `role tab` and `selected`
     states. A dirty tab's name says *unsaved*. Show that AT-SPI can now
     `do_action` on a tab, which request 151 could not.
   - **Profiles:** open Settings → Advanced, Tab into the profile group, press
     `↓`, and `settings.json`'s `active_profile` changes.
   - **Screenshots** of the tab bar with focus on a tab, and of the profile
     group, both in dark. Report anything that looks different from before
     except the focus ring.
3. **Each of RFC-087's acceptance criteria 1 to 6,** named one by one in the
   response, with the evidence for each.
4. **What request 151 could not do:** select the second of two restored tabs
   live, quit, relaunch, and it is selected. F67's restore, driven end to end at
   last.

## Scope

- **In:** §1 to §3, the keyboard reference, `keyboard.md`, i18n (English and
  Japanese). **No RFC edits:** RFCs are the architect's.
- **Out:**
  - Reordering tabs.
  - New visible controls.
  - Any other keyboard shortcut.
  - Context menus (#111).
  - `CHANGELOG.md`, `ROADMAP.md`, the version.

## Gates

The standard set, including the Windows-target clippy with `--all-targets` and
`cargo fmt --manifest-path xtask/Cargo.toml --check`, `cargo xtask` with
`css --check`, `i18n`, `rfc-sync`, `audit-deps`, `version-sync`,
`ui-logic-connectivity`, `ui-logic-docs`, `xlsx-fixtures --check`,
`pages-origin-check` and `pages-link-check` on an assembled site,
`mdbook build docs`, `actionlint`, and a green CI run with its ID.

Reply in `.git-exclude/review-request/152-every-tab-and-profile-by-keyboard.md`.
