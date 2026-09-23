# Handoff 027 — F99: raw errors in recovery dialogs, unlabelled PathBar buttons

**From:** architect. **Release:** 0.171.0. **Register:** F99 (from your own
RFC-063 audit, review 099). **Not release-blocking.**

Two independent defects your audit found. Neither is large; both are in places
that make them worse than their size suggests.

## 1. C10 — raw OS error strings in the recovery dialogs

`overlay/modals/recovery.rs:142` and `:252`:

```rust
Err(e) => store.notify(e.to_string()),
```

**Why this location is the worst one.** These are the settings-recovery and
session-recovery failure handlers — reached when a user's configuration is
already broken and they have chosen "reset and back up the original". Handing
them `No such file or directory (os error 2)` at that moment is the exact
opposite of what a recovery dialog is for.

**The fix is smaller than it looks, and the API documents this use.**
`AppError::from_core(&e)` yields a `UserMessage`, whose `short` field carries
the doc comment:

> One-line summary **for a toast** or dialog title.

`store.notify` already takes a string and shows a toast. So route the error
through `AppError::from_core` and pass `UserMessage.short` — no modal, no
interaction with `advance_recovery_queue`, no new surface. The presenter that
exists was designed for exactly this call.

**Check whether `detail` is worth surfacing too.** `short` alone may be enough
in a toast; if the detail matters here, say what you chose and why rather than
silently dropping it.

**Also fold in the stale comments your audit found and correctly did not
touch:** `diff_actions.rs:321` and `overlay/modals/file.rs:268` both describe
the old `store.notify(e.to_string())` arm as "established" when that path was
migrated. A comment asserting a behaviour the code no longer has is F92's defect
pointed the other way, and this project has now produced it three times.

## 2. C3 — PathBar buttons announce a glyph, not an action

`dir_pane.rs:159-168`. Five buttons — Back, Forward, Up, Home, Open folder —
each with a translated `title` and a bare glyph as its content:

```rust
button { class: "path-btn", title: t(lang, "Back"), ..., "←" }
```

A screen reader announces the character. `title` is a tooltip; it is not a
reliable accessible name.

**Use `aria_label` on the button itself. Do not copy F74/F80's pattern here.**
That pattern is `role: "img"` plus `aria_label` on a **span** — correct for a
non-interactive glyph, wrong for a control: `role="img"` on a `<button>`
overrides the button's own semantics and makes it worse, not better.

**No new translations needed.** The strings already exist and are already
translated — `title: t(lang, "Back")` — so `aria_label` reuses the same call.
That makes this a five-line change.

**One thing to check rather than assume:** the help modal
(`overlay/keybindings.rs:42-43`) documents Alt+↑ and "Back/Forward buttons" but
not Home or the folder picker. If those have no keyboard route at all, say so —
that is a larger finding than this handoff and I want it reported, not fixed
here.

## 3. Falsification, and an honest limit

**C10:** reverting either handler to `e.to_string()` must fail a test asserting
the toast carries the mapped message rather than the raw one. Assert on the
*mapped* text, not merely that a toast appeared.

**C3 — state the limit rather than papering over it.** F74 recorded that whether
a label reaches the platform accessibility tree is an **AT-SPI/UIA assertion,
not a unit test**, and that limit still holds. What you *can* assert is that the
attribute is present with the expected translated string. Do that, and say
plainly in the review request that platform-level verification is not covered —
the same way review 093 disclosed the untestable `<select>`.

## 4. Scope

**In:** `overlay/modals/recovery.rs`, `ui/view/dir_pane.rs`'s PathBar, the two
stale comments, tests.

**Out:** C1's first-run persistence, C6's plain-language settings tier, C7's
narrow-layout trust marker. Your audit called all three genuinely unbuilt polish
and I agree — they are recorded in F99 and **deliberately not scheduled**. Do
not build them because you are in the neighbourhood.

**If C10's fix turns out to need a modal** rather than a toast — because the
recovery queue makes a toast unreliable at that moment — **stop and tell me**.
That would be a design change, not a substitution.

## 5. Gates

The usual set. `i18n` should stay at 244 keys if §2 reuses the existing strings;
if it does not, that is a signal the strings were not as reusable as I claimed
and I want to know.
