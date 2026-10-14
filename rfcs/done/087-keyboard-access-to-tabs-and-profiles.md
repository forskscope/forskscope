# RFC 087: Keyboard Access to Tabs and Compare Profiles

**Status.** Implemented (`0.190.0`, `81d4601`, review 161). Both shortcut pairs reach the page in WebKitGTK; Windows (WebView2) is unverified.
**Accepted.** 2026-10-14 by the project owner.
**Scheduling.** Planned for `0.190.0`. See `ROADMAP.md` § "Remaining proposed RFCs".
**Tracks.** F192: keyboard users cannot switch tabs or choose a compare profile.
**Amends.** RFC-019 (the shortcut map, §7 and §10) and RFC-060 (the global handler's scope). Each carries a one-line Status note pointing here. The decisions live here.
**Touches.**
- `crates/forskscope-ui/src/ui/layout/tabs.rs` (tab bar)
- `crates/forskscope-ui/src/keyboard.rs` (`global_key_action`)
- `crates/forskscope-ui/src/ui/view/settings/modal.rs` (profile list)
- `crates/forskscope-ui/src/ui/overlay/keybindings.rs` (keyboard reference)
- `docs/src/intermediate/keyboard.md`
- i18n

## Summary

Make every tab and the active compare profile reachable and operable from the
keyboard:

- the tab bar becomes a tab list with arrow keys;
- `Ctrl+Tab` and `Ctrl+PageDown` switch tabs from anywhere, the Explorer
  included;
- the active profile becomes a radio group.

Nothing visible changes except a focus ring.

## Motivation

RFC-019 §10 requires every command-backed control to have a *keyboard path*. Two
have none:

- **The tab bar** (`tabs.rs`). Each tab is a `div` whose title is a `span` with
  `onclick`, with no `tabindex`, role or key handler. The close buttons are real
  buttons, so the keyboard can reach a tab's **close** button but never the tab.
- **The profile rows** in Settings → *Advanced* (`settings/modal.rs`). Each name
  is a `span` with `onclick`.

Nothing else covers them:

- `keyboard.rs` has no tab-switching shortcut.
- `global_key_action` ignores every key while no comparison tab is active, which
  is always the case in the Explorer (RFC-060's handler scope).

So a keyboard-only user cannot return to the Explorer from a comparison, move
between comparisons, or make a profile active. The README and the website both
say *Keyboard first*.

Found by the dev team on 2026-10-14, when AT-SPI could not reach these elements
while verifying 0.189.0. An audit of every other `div`/`span` with `onclick` in
`ui/` found a keyboard route for each:

- the breadcrumb's current folder name: *Edit path* in the ⋯ menu;
- tree rows and carets: the tree's arrow keys and Enter/Space;
- modal containers: no route needed.

## Design

### 1. The tab bar is a tab list

The WAI-ARIA tabs pattern, with automatic activation.

- `role="tablist"`, labelled *Open tabs* / *開いているタブ*. Each tab, the
  Explorer included, has `role="tab"` and `aria-selected`. The content area is
  `role="tabpanel"`, labelled by the selected tab.
- **Roving tabindex:** only the selected tab has `tabindex="0"`, so the list is
  one Tab stop.
- **Keys on a focused tab:**

  | Key | Action |
  |---|---|
  | ← / → | Select and focus the previous / next tab. Wraps. |
  | Home / End | The first tab (the Explorer) / the last tab. |
  | Delete | Close the focused tab, through **the same path** as its × button and `Ctrl+W`, so unsaved changes still ask first. Does nothing on the Explorer tab, which cannot be closed. |

- **The × buttons** leave the Tab order (`tabindex="-1"`). They stay for the
  mouse, and the keyboard closes a tab with Delete or `Ctrl+W`.
- **After a close,** focus moves to the tab that became selected.
- **Not colour alone** (RFC-019 §10): a tab with unsaved changes has *, unsaved*
  in its accessible name, and a tab still loading says so. The dot and the
  spinner stay as they are.

### 2. Tab switching from anywhere

**Additions to RFC-019's shortcut map** (§7, level 3, *global app command
shortcuts*):

| Shortcut | Action |
|---|---|
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab, wrapping, the Explorer included |
| `Ctrl+PageDown` / `Ctrl+PageUp` | The same |

- **Change to RFC-060's handler scope.** The *no active tab* check moves from the
  top of `global_key_action` to the commands that need a comparison: save, undo,
  apply, search and close. Tab switching is the one command that runs without
  one.
- **RFC-060's ownership rule is unchanged.** A modal, or a focused text field,
  takes every key first. `Ctrl+Tab` typed in the path editor, the filter, search
  or the profile name switches nothing.
- **Delivery:** the WebView or the window may consume one pair before the page
  sees it. Ship only the pair that arrives, and list only that one in the keyboard
  reference and `keyboard.md`.

### 3. The active compare profile is a radio group

Native `<input type="radio">` elements with labels, in a group labelled *Active
compare profile* / *使用する比較プロファイル*. The arrow keys move the selection,
and selecting persists exactly as clicking the name does today.

- The `×` delete button for user profiles stays a real button. It is reachable
  by Tab, outside the radio group's arrow-key path.
- The appearance does not change. If the native circle is hidden, the selected
  state and the focus ring stay visible.
- It stays inside Settings → *Advanced*, under the owner's rule that detailed
  settings stay hidden (2026-10-14).

## Non-goals

- Reordering tabs.
- Any new visible control.
- Any other new shortcut.
- Context menus (#111).
- The command palette (RFC-019 §8).

## Acceptance criteria

1. From the Explorer, `Ctrl+Tab` (or `Ctrl+PageDown`, whichever arrives) reaches
   each comparison in turn and wraps back to the Explorer.
2. In the tab list:
   - ← / → / Home / End behave as §1 says.
   - Delete closes a clean comparison tab at once, and asks first on a dirty one.
3. While a modal is open or a text field has focus, the tab shortcuts do nothing.
4. With *Advanced* open, Tab reaches the profile group, ↓ changes the active
   profile, and `settings.json`'s `active_profile` changes.
5. AT-SPI can reach and activate a tab. Its tree shows `tab` roles and `selected`
   states. A dirty tab's name says *unsaved*.
6. Unit tests through the functions the app calls cover:
   - tab-index arithmetic (wrap, Home/End, close-then-select);
   - `global_key_action` returning tab switching with no active tab, and nothing
     under a modal or a text field;
   - the radio selection persisting.
   Each test fails when its behaviour is removed.

## Open questions

1. **Which shortcut pair reaches the page in WebKitGTK and WebView2?** Answered
   by measurement during implementation, not by assumption. If neither arrives,
   this RFC returns to the owner before it ships.
