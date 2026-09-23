# Review 082 — Request 080: F84, wiring the large-file guard

**Reviewer:** architect
**Date:** 2026-08-27
**Reviewed:** `8242a61`, against baseline `9c44883`
**Verdict:** **Approved. F84 is resolved.**

## 1. The measurement is the most valuable thing here, and it corrects me

You opened two 150 MB files on a real build and watched:

- **~19.36 GB RSS** — roughly **65× the 300 MB of input**
- **9.5 minutes**, then killed manually; it never finished
- the window stayed responsive and stayed on *"Loading…"* forever

**I recorded F84's severity as "not a Gate D input — it needs a deliberately
large input and degrades rather than corrupting."** That judgement was made on an
assumption I explicitly flagged as unverified, and the measurement contradicts
it. A 150 MB text file is not exotic — logs, CSV exports and database dumps are
exactly what someone points a diff tool at — and the behaviour is not degradation
but an unbounded hang with no error and no way out.

Had this not been fixed in the same commit, it would have been a blocker. The
register now says so.

Disclosing that your fixture was adversarial — nearly every line differing —
rather than presenting it as typical is what makes the number usable. It is an
upper bound, honestly labelled.

## 2. Verified independently

I re-ran both falsifications rather than accepting them:

- **Defeating `suppress_inline` alone** fails **exactly one** test —
  `confirm_prompt_suppresses_inline_diff_on_the_resumed_options`, 81 passed 1
  failed. Precise, not incidental.
- **Defeating `decide_load`** fails **three**, including
  `both_load_call_sites_stop_at_the_guard_for_a_large_pair`.

Gates clean. `PerformanceLimits` untouched.

## 3. Both call sites are covered for real

`open_compare_request` asserts no tab is allocated; `reload_tab` asserts the
existing tab's state is untouched. Both against a real 5 MiB fixture, both landing
on `Modal::ConfirmLargeLoad` with the right `LargeLoadTarget`.

That is the requirement met literally. The reload path was the one I expected to
be missed — it is where a user re-opens the file they were just warned about.

## 4. The unused field, and why leaving it is right

`ConfirmPrompt.too_large` is threaded through and never read. You said so plainly
rather than dropping it or pretending it was used.

Leaving it is correct: the distinction it carries is already conveyed by the
differing `title`/`body`/`confirm_label` that `guard_for_sizes` produces, and a
future change that wants to special-case *too large to diff at all* — disabling
confirm, or routing to a metadata-only view — has it without another trip through
`load_guard`. An unread field that is documented as unread is not debt; an unread
field nobody mentions is.

## 5. Status

F84 resolved. Three of F75(b)'s four KEEPs remain unwired; `save_error` is
review 083.
