# F61 review — the real mechanism

**Review date:** 2026-08-16
**Request:** `dev-record/review-requests/063-f61-real-mechanism.md`
**Baseline:** `202ab97`
**Responds to:** `dev-record/reviews/065-f61-f62-session-persistence-review.md`
**Review mode:** Independent verification on a real desktop, reproducing review 065's decisive case.

## 1. Verdict

**Approved. F61 is fixed.**

I re-ran review 065's Case B — the one that falsified the previous attempt — on
a real desktop process against the fixed build:

```text
$ XDG_CONFIG_HOME=<fresh, empty> forskscope <left> <right>
  (no interaction, process exits)

session.json? YES
tabs: [{'left': '.../f61-l.txt', 'right': '.../f61-r.txt'}]
```

The case that produced `NO` twice now produces the correct session with no
interaction.

**One finding of my own**, arising directly from your question 2: the fix is
complete for what the session persists, but **what the session persists is
narrower than the product claims** (§3). Registered as F67; not a defect in this
slice.

## 2. Verified

| Check | Result |
|---|---|
| Case B on a real desktop, fresh profile | **Pass** — session written, correct tab |
| `use_effect` on `store.tabs` removed | Confirmed — one `use_effect` left in `app.rs`, the window-title one |
| Explicit `save_session` in `open_compare_request` | Confirmed — `compare.rs:214` |
| `cargo test --workspace` | **1097**, matching your count |
| `clippy --workspace --all-targets -D warnings`, `fmt --check` | Pass |

## 3. Your question 2 — complete for `store.tabs`, and that is not the whole story

I enumerated every site that changes tab identity rather than checking your list
against itself:

```text
tabs.write()  membership changes:
  session.rs:166   remove(index)   → close_tab            (pre-existing save) ✓
  compare.rs:203   push(tab)       → open_compare_request (new save)          ✓
  compare.rs:343   push            → dir_tabs   ← different signal
  compare.rs:350   remove          → dir_tabs   ← different signal

left_path / right_path assignment:
  compare.rs:187-188  construction in open_compare_request ✓
  (swap_sides uses std::mem::swap — not an `=`, and you covered it)
```

**Confirmed: for `store.tabs`, your three sites are the complete set.**

But `store.dir_tabs` is a **separate signal**, and `build_save_payload` never
sees it. Directory comparisons are not persisted at all — open one, quit, and it
is gone. `README.md:96` still says *"open tabs are restored on next launch."*

Following that into `PersistedSession` shows the shape is broader than one
missing tab class. Of its three payload fields:

| Field | State |
|---|---|
| `tabs` | Written and restored — the only live one |
| `active_tab` | Hardcoded `None` in `build_save_payload`; never restored |
| `explorer_roots` | Written `None`, never read (found during F33) |

So two of three fields are permanently dead, and a whole tab class is absent.
**This is F61's shape one level out** — not a write that fails, but a claim
broader than the mechanism behind it — and it is the same limit F16's audit hit:
a claim true on one path and false on another passes a per-claim review.

Registered as **F67**. Explicitly **not** yours to fix here: it is scope beyond
F61, and F61's own fix is correct and complete for what it covers.

## 4. Answers to the other requested focus

### 4.1 "Tied to discrete UI-event dispatch" is sufficient to fix from

Yes, and the investigation that established it is the strongest part of this
submission. Three observations, each ruling something out:

- the effect's entry line never printed, while both writes to `store.tabs` did —
  so not "called and returned early";
- the compare view **rendered fully** (7/7 rows) while the session never
  appeared — so "the view updates" and "the effect runs" are not the same
  guarantee;
- an unrelated re-render (the `?` button, touching only `store.modal`) did not
  wake it — so not "any subsequent render flushes it."

That is a fact established by elimination, not a hypothesis. And you correctly
noted it is **none of review 065 §3's three candidates** — all of which assumed
the effect fired at least once. It never fires at all.

Gate D does not need the `dioxus-core` root cause, because the fix does not
depend on it: you stopped relying on the mechanism rather than tuning it. A root
cause would matter if we were keeping the effect and making it reliable.

### 4.2 §9's other reactive effects — not a follow-up, and here is why

Your grep found `app.rs`'s window-title effect and the Explorer scan/filter
effects, none of which persist data, and settings persistence was already
explicit through `onchange` handlers.

That is sufficient, because of what your own §2 established: the failure needs a
signal write made **outside** a discrete UI event. The Explorer effects are
driven by user interaction; the title effect has no durable consequence. There
is no site left where the dangerous combination — a non-event-driven write plus
a reactive persistence dependency — still exists, because you removed the only
one.

I would not open a follow-up. Record the reasoning in the register entry so the
next person does not re-derive the grep.

## 5. Notable quality observations

- Instrumenting the **real process** rather than the harness, and getting a
  negative result — an entry line that never prints — which is harder to trust
  and harder to fake than a positive one.
- Deleting `app/tests.rs` rather than keeping a passing test for a mechanism
  that no longer exists. A green test guarding removed code is worse than none.
- Proving the new test fails by commenting out the fix, then restoring — the
  standard review 065 §5.2 required, met exactly.
- Fixing `with_test_store`'s runtime-context gap as a strict capability addition
  rather than working around it. That was a real F36 limitation and it is now
  closed for everyone.
- Adding the `swap_sides` call **for consistency with the chosen design** rather
  than only where a gap was proven — "explicit calls everywhere" means
  everywhere, and a design applied only where it was forced is how the next gap
  appears.
- Correcting the earlier "no GUI verification possible in this sandbox"
  conclusion, and saying which invocation actually works. That mistaken belief
  is what let the previous attempt through, and retracting it is worth more than
  the fix.

## 6. Recommended next action

1. **F61 closes.** One Gate D blocker down; **F44 remains**, and it is upstream.
2. **F67** — mine, registered, for the owner to schedule. Not urgent, and not a
   Gate D input on current reading: the un-persisted directory tab is a missing
   convenience, not silent loss of work the user did.
3. **A new candidate is needed** before M5's P12 rows can be re-run against a
   build containing this fix. That is release work, not evidence work.
4. M5-C continues; the dev team is already on P07.
