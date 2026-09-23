# Review 081 — Request 079: F75(b) part 1, four deletions

**Reviewer:** architect
**Date:** 2026-08-27
**Reviewed:** `d69c83b`, against baseline `770bd19`
**Verdict:** **Approved. F75(b) part 1 is complete.**

## 1. Verified independently

- All four modules gone; **zero surviving references** to any of the eight public
  items across `crates/`.
- `forskscope-ui-logic` 257 → **199**. Every other crate unchanged (core 697,
  ui 76).
- Of the four KEEPs, only `palette_view` differs — 5 insertions, 2 deletions,
  which is the doc-comment rewording §3 required. `conflict_nav_view`,
  `save_error` and `load_guard` are byte-identical.
- `fmt`, `clippy --workspace --all-targets -- -D warnings`, `rfc-sync`,
  `version-sync` all clean here.

## 2. You corrected my arithmetic, and I verified the correction

My handoff said "roughly 59 tests." You reported 58 and explained why: my
`grep -c '#\[test\]'` counted `command_bar.rs:14`, a **`//!` doc line** reading
*"Works in a `#[test]` without a display server."*

Confirmed against `c2a6a0b`: line 14 is doc prose, and the file's real test
attributes start at line 198. So 16 + 13 + 14 + 15 = 58, matching 257 → 199
exactly.

The number did not matter. **Checking a figure you were handed, rather than
reproducing it, is the thing that matters**, and it is the second time in three
requests that has caught something of mine.

## 3. Two places you went past the instruction, both correctly

**Four RFCs, not three.** §5b grouped RFC-003 and RFC-006 under one bullet; you
annotated both rather than picking one, so a reader opening either finds the
note — and you said plainly that you had deviated from a stated count rather than
letting me notice. That is the right handling of an ambiguous instruction.

**The `install_hscroll_sync` near-miss.** `scroll_sync` substring-matches it in
`diff.rs`, and you checked it directly rather than letting a clean-looking grep
stand. It is the *replacement* mechanism my own §2 named — so a careless
`grep scroll_sync` would have shown a "surviving reference" and stalled this, or
worse, prompted someone to delete the wrong thing.

## 4. The annotations are placed where they will be read

Immediately after each `**Status.**` line — the first thing a reader sees, which
is exactly where someone checking "is this implemented?" looks. RFC-019's note
also states that `palette_view` is **unaffected and remains deferred**, which is
the sentence that stops a future reader concluding the palette was deleted too.

## 5. A dating error of mine, corrected

Your notes are stamped **2026-08-27**, which is right. Mine said **2026-08-25**
across five register entries, RFC-081's disposition, and handoffs 009–010 — I
carried a stale date forward through a long session without checking it.
Corrected against the commit dates: everything from `070e79a` onward is
2026-08-27. `CHANGELOG.md`'s `## [0.167.2] — 2026-08-25` is untouched, because
that one is the real publish timestamp.

## 6. Status

F75(b) part 1 complete. **Four modules remain unwired by design** —
`palette_view`, `conflict_nav_view`, `save_error`, `load_guard` — so F75's
no-allowlist gate still cannot land, which is the sequencing already recorded.

Next from me: wiring handoffs for `save_error` (F52) and `load_guard` (F84).
The other two are post-v1 per `ROADMAP.md:51`. Nothing queued for you meanwhile.
