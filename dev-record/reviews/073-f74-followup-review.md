# Review 073 — Request 071: F74 follow-up, real-path coverage and accessible labels

**Reviewer:** architect
**Date:** 2026-08-18
**Reviewed:** `afac676`, against baseline `a462f31`
**Verdict:** **Approved. F74 is resolved.**

## 1. The acceptance criterion — verified independently, not accepted on report

Handoff 003 §10 made one thing the criterion: reintroducing the original defect
must fail a test. I restored it inside `classify_entry` myself and ran the suite:

```
test ui::view::explorer::tests::real_directories_with_the_same_name_and_differing_contents_are_not_compared ... FAILED
assertion `left == right` failed: a same-named directory whose contents differ
must never be classified Equal - see 9f355c6's original defect
  left: Final(Equal)
 right: Final(NotCompared)
test result: FAILED. 61 passed; 1 failed
```

**Met.** The suite now detects the defect that shipped, which is what review 072
said it could not. Gates re-run here independently: `fmt` clean, `clippy
--workspace --all-targets -D warnings` clean, `xtask i18n` 231 keys covered,
`xtask css --check` up to date.

## 2. The extraction is the right one

`classify_entry` is the real call site rather than a parallel implementation —
the `use_effect` closure is now a `match` and holds no classification logic. The
`Final`/`NeedsDigest` split is a good shape: it names the one case that cannot
resolve synchronously instead of hiding it, and it kept the async digest path
byte-for-byte equivalent.

Keeping the signal reads in the closure and passing plain values in is exactly
the boundary §14 asked for, and citing the `apply_navigation` precedent in the
comment is the right instinct — the next person does not have to re-derive it.

## 3. Your three questions

**Q1 — does `classify_entry` reach the exact call site, or is there still a gap?**
It reaches it; §1 is the proof, and it is proof rather than argument. What
remains outside the test is the closure's key choice (`DigestKey::Common(rel)`)
and the async result mapping. Neither was the defect and neither needs covering
now — but you asked precisely, so: the answer is "no gap for F74", not "no gap".

**Q2 — is the fixture sufficient?** Yes, and one property of it is worth stating
so nobody misreads it later: **`classify_entry` never reads file contents, so the
differing content in your fixture is representational, not load-bearing.** The
test would pass identically with the two files byte-equal. That is correct — the
Explorer's honest answer is *not compared* either way — but it means the fixture
demonstrates the reported scenario rather than testing content-sensitivity, and
a future reader should not conclude otherwise. What no unit test can cover is the
end-to-end behaviour the owner saw on Windows; that is P07's job, not this
fixture's shortfall.

**Q3 — is deferring the `aria_label` verification acceptable?** Yes — I
authorised it in handoff 003 §14 and nothing here changes that. **Do not hold the
request.** But "when the Explorer's Windows rows next run" is a plan, not a
tracker, so I have written it into F74's register entry as a named pending P07
assertion. Deferred is fine; deferred and unrecorded is how these disappear.

## 4. An observation on the ARIA pattern — no action, and my mis-citation

Handoff 003 §7b sent you to `role="img"` + `aria_label`, citing eight existing
uses. That was accurate but drawn from the wrong neighbourhood: modals and the
pane region. **The closest analogue is `hunk.rs`, which implements RFC-009 §7
itself and uses a different pattern** — `span { class: "sr-only", "{label}: " }`
beside `span { class: "diff-mark", aria_hidden: "true", "{mark}" }`.

Both are correct ARIA and yours follows the more widespread of the two
(`aria_label` 8 uses, `sr-only` 2, both in `hunk.rs`). **I am not asking you to
change it** — the churn would buy nothing. Recording it because whoever writes
P07's accessibility assertion must handle **both** patterns, and would otherwise
find one and assume it was the convention.

## 5. Second finding, folded into F76 rather than raised against you

Not caused by this change; noticed while reading the path it preserved.

`file_digest_equal(&left_abs, &right_abs).unwrap_or(false)` maps an **error** to
`false`, and `false` renders as `DigestState::Different`. So a file that cannot
be read — permissions, a race, an I/O fault — is reported to the user as
**differing** when nothing established that it does. `DigestState` has no error
state; core's `EqualityEvidence::Error { message }` exists and is unreachable
from here.

This is F74's own family — a status asserting more than was measured — but the
**safe** direction, unlike F74: a false *different* makes the user look, a false
*equal* makes them stop. Low severity, no action now. Added to **F76**, which is
already the entry for status states that overstate their evidence and is already
scheduled to be fixed by the same change (F75's wiring brings `TypeMismatch` and
`Error` across together).

## 6. Why F74 closes without runtime evidence, when F73 did not

Worth stating, because the two were handled differently within a week and the
difference should not look arbitrary.

F73 was a wrong-destination **file write**; the only thing that could confirm it
was observing real filesystem state after a real copy, so it stayed open until a
published candidate was exercised. F74 is a pure **classification** — a
`DigestState` chosen from two filesystem predicates — and it is now covered by a
falsifiable test driving the real call site against real directories. Runtime
re-verification would re-run the same predicates through more machinery.

The residual — whether `aria_label` reaches the accessibility tree — is an
**evidence item, tracked in F74's entry**, not an open defect. **Gate D's blocker
count returns to one: F44 alone.**

## 7. Process note

Requests 070 and 071 are the same team a day apart, and the difference is worth
naming. 070 reported five tests "each confirmed to fail against a temporarily
reintroduced pre-fix version" — true as written, and two of them tested the wrong
thing. 071 led with the one falsification that mattered, pasted the failure, and
§7 flagged its own unresolved gaps before I could.

Nothing about your care changed between them. What changed is that the handoff
named the acceptance criterion as a single falsifiable sentence instead of a
standard to uphold. That is a lesson about how I write handoffs, and it is
recorded as such.

No further work. F75 and F76 stay untouched and unscheduled until after Gate D.
