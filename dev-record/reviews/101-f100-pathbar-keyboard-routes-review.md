# Review 101 — Request 098: F100 PathBar keyboard routes

**Reviewer:** architect. **Date:** 2026-09-08. **Reviewed:** `88ec5f1`.
**Verdict:** **Approved. F100 closed.** The verification gap you flagged is
real, is the third of its kind, and is registered as **F101** — not against this
change.

## 1. The falsification predicted in the handoff, reproduced exactly

I hardcoded `is_left = true` in the Alt+Home branch:

```
alt_home_navigates_the_focused_pane_to_the_home_directory ......... ok
alt_home_targets_the_right_pane_when_the_right_pane_is_focused .... FAILED
```

**One failure, and the right one.** The left-focused test passes by coincidence
— the hardcoded value happens to match — and the right-focused test catches it.
That is the asymmetry the handoff named as "the one that matters", and it played
out precisely.

Full suite **1267 passed, 0 failed**. `i18n` **246, +2** — as predicted, and the
inverse signal from handoff 027 where an unchanged count was the confirmation.

## 2. No shadowing, verified by ordering

`dispatch_pathbar_shortcut` is called at `tree.rs:70`, **before** the `TreeKey`
match at `:75`, and its branch is gated on `Modifiers::ALT`. So bare `Home`
still falls through to `TreeKey::Home`. Checked the call order rather than
taking it from the report.

## 3. The control experiment is the best thing in this review

The handoff asked you to verify `Alt+Home` is not intercepted by the compositor,
and said to propose an alternative if it did not reach the app.

You could not drive DOM focus to `#aligned-tree` — no mouse automation in this
environment. **Instead of reporting "couldn't test" or quietly claiming a pass,
you sent the already-shipped `Alt+↑` under identical conditions and observed the
same null result.**

That converts an inconclusive test into a conclusive one about the *instrument*:
the absence of an effect is a property of the harness, not of the new binding. A
negative control is exactly the right instrument check, and it is the difference
between "my change might be broken" and "I cannot observe either binding here".

You also did the part that *was* decidable: confirmed `~/.config/niri/cfg/keybinds.kdl`
binds only `Mod+Home` and `Mod+Ctrl+Home`, so nothing intercepts `Alt+Home` in
this environment. I verified that independently.

**Conclusion: no evidence against the binding, and no positive runtime
confirmation.** Recorded as such rather than rounded to a pass.

## 4. The help-modal test had no prior shape

`overlay/keybindings.rs` had zero tests. You said so, then adapted F99's
technique rather than inventing one silently — rendering through a bare
`VirtualDom` and scanning for `Mutation::CreateTextNode` instead of
`SetAttribute`, because `KbRow`'s content is dynamic text children rather than
attributes. Correct adaptation, and the reason for the difference is stated.

## 5. One thing you did that nobody asked for

The help entries read *"Go to home directory **(focused pane)**"* and *"Open a
folder **(focused pane)**"*.

The handoff flagged the focus-scope limitation as something for *me* to hear an
argument about. You instead **told the user**, in the one place they will look.
That is the better answer: a shortcut whose scope is documented is not a
limitation, it is a specification.

## 6. F101

Three reviews now carry the same structural gap — RFC-083's encoding `<select>`
(review 093), F99's `aria-label` reaching the accessibility tree (review 100),
and this binding's live delivery. Plus F74's original AT-SPI note.

That is a class, not three incidents, and it has a home: **RFC-078's P-cases are
manual and owner-executed for exactly this reason.** F101 records the class and
routes it there rather than leaving each review to rediscover it.

**F100 closed.** Scope held: global `keyboard.rs` untouched, no wider RFC-061
re-audit, and you reported finding no other unbound controls rather than
implying you had not looked.
