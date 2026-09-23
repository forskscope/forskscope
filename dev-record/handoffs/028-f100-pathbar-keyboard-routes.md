# Handoff 028 — F100: Home and the folder picker have no keyboard route

**From:** architect. **Release:** 0.171.0. **Register:** F100 — **your finding**,
reported during handoff 027 rather than fixed there, which was correct.
**Not release-blocking.**

## 1. What this closes

Two of `PathBar`'s five controls cannot be reached without a mouse. Back,
Forward and Up are fine — Alt+↑ is bound at `explorer/tree.rs:59` and documented
at `overlay/keybindings.rs:42-43`. **Home and Open-folder are neither bound nor
documented.**

This also narrows a claim we closed too broadly. **RFC-061 was approved as
*fully implemented*** (review 096) on the strength of the focused-pane model —
F6, `handle_key` dispatching by focus, a visible indicator. That was true **of
the panes** and not of the path bar above them. Your finding is what made the
difference visible, and it is worth knowing that "keyboard-completable" survived
a review it should not have.

## 2. The collision map — checked, not assumed

I read both handlers before proposing keys.

**Taken in the tree handler** (`explorer/tree.rs:52-79`): `F6`,
`Alt+ArrowUp`, and bare `ArrowUp/Down/Left/Right`, `Enter`, **`Home`**, `End`,
`Escape`, `Space`.

> **Bare `Home` is already `TreeKey::Home`** — jump to the first row. Do not
> take it.

**Taken globally** (`keyboard.rs:97-117`): `Escape`, `F7`, `F8`, `F3`, `Enter`,
and `Ctrl` + `s` `z` `y` `w` `/` `f`.

**So `Ctrl+O` is free, and `Alt+Home` is free.**

## 3. Proposed bindings — argue with these if you disagree

- **Open folder → `Ctrl+O`.** Near-universal convention, and unclaimed.
- **Home directory → `Alt+Home`.** Bare `Home` is taken; `Alt+Home` is the
  browser convention for "home", and **`Alt` is already this handler's modifier
  idiom** — `Alt+↑` for parent. Two `Alt`+navigation-key bindings read as a set.

**Where they belong: the tree's `onkeydown`, beside `Alt+↑` — not the global
handler.** Three reasons: `PathBar` exists only in the Explorer; both actions
must act on the **focused pane**, which is state the tree handler already has
and the global handler does not; and `Alt+↑` set that precedent, so splitting
the family across two handlers would be worse than the limitation it removes.

**The consequence, stated so you can object:** like `Alt+↑`, these fire only
when the tree has focus. If you think that is too narrow — that `Ctrl+O`
especially should work anywhere in the Explorer — **say so before building it**.
That is a real design question and I would rather have the argument than a
silent widening.

**One runtime check:** `Alt+Home` may be intercepted by the compositor on some
Linux desktops. Verify it actually reaches the app before committing to it; if
it does not, propose an alternative rather than shipping a binding that silently
does nothing.

## 4. The help modal is not optional here

F99's C3 audit found `overlay/keybindings.rs` documents Alt+↑ and the
Back/Forward buttons but **not** Home or the folder picker — for either input
method. F100's disposition is a binding **and** a help entry for each.

A shortcut nobody can discover is barely better than no shortcut. Both go in the
same change.

**`i18n` will grow — expect that.** New help-modal strings need Japanese
translations. This is the opposite of handoff 027, where an unchanged count was
the signal; here a static 244 would mean the help entries were not added.

## 5. Falsification

1. Removing either binding fails a test asserting the action fires for that key
   — assert the **action**, not that the key was seen.
2. Each binding acts on the **focused pane**: with the right pane focused,
   `Alt+Home` must navigate the *right* path bar. Falsify by hardcoding `left`
   and watching it fail. This is the one that matters — `Alt+↑` already had this
   property and a new binding that ignores focus would be a regression the tests
   should catch.
3. Removing a help-modal entry fails a test. `overlay/keybindings.rs` should
   already have a shape for this; if it does not, say so rather than inventing
   one silently.

## 6. Scope

**In:** `explorer/tree.rs`'s keydown handler, `overlay/keybindings.rs`, i18n
strings, tests.

**Out:** any other unbound action; RFC-061's wider re-audit — if you notice
*other* controls with no keyboard route while you are in here, **report them,
do not bind them.** That list becomes a scheduling decision, and F100 exists
because you drew that line correctly once already.

## 7. Gates

The usual set, plus `i18n` green with the new keys translated.
