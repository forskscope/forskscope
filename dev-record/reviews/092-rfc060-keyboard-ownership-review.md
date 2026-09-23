# Review 092 — Request 089: RFC-060 keyboard-ownership tests

**Reviewer:** architect. **Date:** 2026-09-02.
**Reviewed:** `59cd070`, against `59cd070^`.
**Verdict:** **Approved, with one required follow-up.** The follow-up is not a
nitpick — it leaves the only *real* defect this change fixed unprotected.

## 1. You found a shipped defect the handoff did not name

The handoff asked you to make an existing rule enforceable. It framed the work
as refactoring. **It was not.** You disclosed this as *"a real gap, not just a
refactor"* and you are right — I verified it against the parent commit:

```rust
// 59cd070^:ui/view/search.rs
onkeydown: move |e| {
    match e.key() {
        Key::Escape => { e.stop_propagation(); … }
        Key::Enter  => { e.stop_propagation(); … }
```

`stop_propagation` was called **only inside the `Escape` and `Enter` arms**.
Every other key bubbled. So **Ctrl+S typed into the search box saved the active
tab behind it**, and the same held for `dir_pane.rs`'s path input while renaming
a directory. That is the exact class of defect RFC-060's purpose statement
names, shipped, in the file the RFC pointed at.

Neither I nor RFC-060 caught it. You did, while doing something else, and you
flagged it as a behaviour change instead of letting it pass as a refactor.

## 2. Required follow-up — the fix to §1 is untested

**This is the finding.** I removed `swallow_when_typing` from `search.rs:90` and
ran the full workspace suite:

```
test result: ok. 109 passed; 0 failed
test result: ok. 200 passed; 0 failed
… all green
```

**Nothing failed.** The same is true for `dir_pane.rs:188`.

So the distribution of test coverage is exactly inverted:

| Surface | Was it actually broken? | Extracted + tested? |
|---|---|---|
| `explorer/filter.rs` | **No** — already blanket; the change is a no-op | **Yes** — `filter_input_keydown`, falsified |
| `search.rs` | **Yes** — real defect | **No** — inline in a closure |
| `dir_pane.rs` | **Yes** — real defect | **No** — inline in a closure |

The one surface that needed no fix is the one with the test. The two that
carried a genuine bug are protected by nothing but the line being present.

And this is precisely the failure the handoff existed to end. §1 of that
document: *"nothing stops a seventh input surface from omitting the guard."*
After this change, nothing stops the **second and third** from omitting it
either — a future edit to either closure deletes the guard and every gate stays
green.

**Required:** give `search.rs` and `dir_pane.rs` the same treatment you gave
`filter.rs` — lift each `onkeydown` body into a named function the component
calls, test it, and falsify by deleting the call from the shipped function.
`filter_input_keydown` is the pattern; it works, and you already proved it works.

## 3. Falsifications 1–3: sound, and test 3 for the right reason

All three re-run correctly. Test 3 matters most and you built it right:
`FilterBar` at `filter.rs:41` calls `filter_input_keydown(&e)`, and the test at
`:191` calls **that same function** — not a copy of its logic. Deleting the call
from the shipped function is what fails the test. That is the distinction §3 of
the handoff asked for, and it is the reason §2 above is a real gap rather than a
stylistic preference: the pattern is proven, it is just not applied where it
counts.

## 4. `ModalState` instead of `&Modal` — correct call

You declined the handoff's sketch because `Modal`'s recovery variants carry full
`SettingsRuntimeResolution`/`SessionRuntimeResolution` structs, and building one
to exercise a keyboard branch buries the test's intent in unrelated fixture
noise. Agreed. **And you closed the hole that argument opens** — a 3-value
summary can drift from the thing it summarises — with
`from_modal_classifies_settings_recovery_as_recovery` and its session twin,
which do pay the fixture cost, once, exactly where it buys something.

`app.rs` is now a genuine dispatcher: read state, call the function, match the
action. No decision left in the closure.

## 5. Your `settings/modal.rs` note — conclusion right, mechanism wrong

You flagged this yourself and asked for it to be examined, so: **the conclusion
holds and the stated reason does not.**

You wrote that `app.rs` closes the settings modal *"regardless of what that
modal's own handler does"*. It could not have. The old handler called
`e.stop_propagation()` **before** setting `Modal::None`, so the event never
reached the app root — the local handler was doing the work, not app.rs.

The redundancy is real, but it is *"either mechanism produces the same
outcome"*, not *"app.rs was doing it all along"*. That distinction matters for
the next person: someone deleting the handler on your stated reasoning gets the
right result, but for a reason that was not true. Recorded so the note in the
code does not outlive the accuracy of its explanation.

Leaving it converted rather than deleting it was the right scope call.

## 6. One naming point, not blocking

`swallow_when_typing` is now called on `settings/modal.rs`'s scrim `div` — which
is not a text input and involves no typing. The behaviour is right; the name
describes three of its four call sites. Worth a rename when §2 is done, if a
better one is obvious. Not worth a commit on its own.

## 7. Verified independently

Full workspace suite re-run here: 109 ui-lib / 109 ui-bin / 200 ui-logic, green.
17 `#[test]` in `keyboard.rs` plus one in `filter.rs` — your "+18" is exact.
`rfcs/done/060-*.md` was not moved, per scope.

**RFC-060's deferred note stays open until §2 lands**, then it closes.
