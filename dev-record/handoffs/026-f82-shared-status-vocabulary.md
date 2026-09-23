# Handoff 026 — F82: one status vocabulary, and RFC-063's delivery audit

**From:** architect. **Release:** 0.170.0. **Register:** F82, RFC-063.
**Not release-blocking.**

## 1. The divergence, measured — F82's own text is imprecise

F82 says the two views use *"`=`, `≠`, `…`, `!` versus `✓`, `⚠`, `⊙`, `↗`"*. I
checked both tables and it is narrower and stranger than that:

| Concept | Explorer (`RowStatusKind`) | Deep Compare (`RecStatus`) | |
|---|---|---|---|
| Equal | `=` | `✓` | diverges |
| Different / Changed | `≠` | `⚠` | diverges |
| Computing | `…` | `⊙` | diverges |
| Error / Unreadable | `!` | `⊘` | diverges |
| LeftOnly | `←` | `←` | **already agree** |
| RightOnly | `→` | `→` | **already agree** |
| NotCompared | `–` | — | Explorer only |
| Symlink | — | `↗` | Deep Compare only |

So **four** concepts diverge, two already agree, and two exist in only one view.
`↗` is Symlink, not Error — F82 pairs it against `!`, which is wrong.

## 2. The target vocabulary — decided, with reasons per concept

Not "one view wins". Each concept on its merits:

- **Equal → `=`.** `✓` reads as *approval*. Equality is not approval, and in a
  comparison tool a checkmark next to a file invites "this one is fine".
- **Different → `≠`.** **`⚠` is the important one to drop.** A differing file is
  the *normal, expected* result in a diff tool — warning iconography claims
  severity that is not there. Your own code already reasons this way:
  `deep_compare.rs:308` deliberately refuses `⚠` for `Unreadable` because it
  *"would visually claim a comparison was actually made."* The same objection
  applies to using it for the ordinary case.
- **Computing → `…`.** Universally understood; `⊙` is not.
- **Error / Unreadable → `⊘`.** **Deep Compare's wins here.** `!` is generic
  alarm; `⊘` says *prohibited/not available*, which is what an unreadable entry
  is. Explorer's `Error` adopts it.
- **`←` / `→`** unchanged, already identical.
- **`–` (NotCompared)** and **`↗` (Symlink)** stay, each rendered only by the
  view whose enum can produce it.

## 3. The shape, and the trap in it

One shared table — glyph, CSS class, accessible label — that **both** views
render through, in `forskscope-ui-logic`. That shape was proposed by you in
request 076 §3 and adopted.

**The trap, stated plainly:** `ui-logic` is where **nine modules sit unwired**
(F75), and RFC-085's own restoration was nearly dead code for exactly this
reason. **A shared table added there and wired by only one view becomes the
tenth**, and this register entry would then be evidence for F75 rather than a
fix for anything. Both views convert in the same change or the change is not
done.

**Do not merge the two enums.** `RecStatus` is `forskscope-core`;
`RowStatusKind` is `forskscope-ui-logic`. They have different variants because
they describe different things — a recursive scan result versus a row in an
aligned tree. The shared thing is the *presentation vocabulary* both map into,
not the state itself. If the implementation starts to look like it needs the
enums unified, **stop and tell me**.

## 4. Why now, and not folded into RFC-080

F82's register entry says *"fold into RFC-080's implementation."* I am
overriding that: RFC-080 is **Post-Gate-D** with no date, so folding would keep
the divergence indefinitely for a change that is small and independent.

Doing it first also *helps* RFC-080, which adds new states to the Explorer's
vocabulary — it will add them to one table instead of two.

## 5. Second half — RFC-063's delivery audit

RFC-063 is in `done/` **as a triage record**. Its dispositions are complete; per
review 092 §5 I moved it there saying explicitly that **per-item delivery was
never audited**, rather than letting `done/` imply it. This closes that.

Ten items, C1–C10. C8 is **Reject (keep current)** — verify nothing was built
for it. For each of the other nine, establish whether the adopted (or
downscoped-adopted) behaviour actually ships, and report per item:
**shipped / partly shipped / not shipped**, with the evidence.

**Do not implement anything you find missing.** This is an audit. A missing item
becomes a scheduling decision, and scheduling is mine. Report and stop.

Two spot-checks already done, so do not redo them: **C2**'s density variables
(`--control-h`, `--row-h`) and **C5**'s severity model (`ui/component/notice.rs`)
both ship.

## 6. Falsification

For §1–§3:

1. Every concept renders the same glyph in both views — falsify by changing one
   view's mapping and watching a test fail. A test that reads the shared table
   and compares it to itself proves nothing; it must assert what each **view**
   renders.
2. Removing either view's use of the shared table must fail a test. This is the
   F75 guard: it is the one that proves the table is wired twice, not once.
3. Every glyph still carries its accessible label (F74/F80) — the labels must
   move with the glyphs, not be dropped in the consolidation.

## 7. Scope

**In:** `ui-logic/src/explore/status.rs`, `ui/src/ui/view/deep_compare.rs`, the
new shared table, CSS classes if they consolidate, tests, and the RFC-063 audit
report.

**Out:** unifying `RecStatus` and `RowStatusKind` (§3); RFC-080's tiered
comparison; implementing anything the §5 audit finds missing.

## 8. Gates

The usual set. If the CSS classes consolidate, `cargo xtask css --check` must
stay green and the generated `main.css` regenerated rather than hand-edited.
