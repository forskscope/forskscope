# Handoff 105 — *Settings* at the far end of the tab bar

**From:** architect. **Date:** 2026-11-21. **Release:** `0.199.0`.
**RFC-089, amended by the owner on 2026-11-21:** *Settings* takes the far
end of the bar, after *? Shortcuts*. The order had been carried over from
the removed header and never weighed. **This is the only change.**

**It can run alongside the 192b triage.**

## Evidence plan first

- **A markup test** for the order.
- **The layout gate** for geometry.
- **One capture.**

**Say the plan in one line at the top of your reply.**

## Do

- **`ui/layout/tabs.rs`:** swap the two buttons, so the bar reads
  `[tabs] … [? Shortcuts] [Settings]`. Nothing else about them changes:
  - labels;
  - accessible names;
  - actions;
  - Ctrl+/;
  - styles;
  - the narrow-window wrap rule, which keeps both on the first line at
    its end.
- **The Tab order follows the DOM:** tabs, then *? Shortcuts*, then
  *Settings*.
  - Any keyboard test that walks those stops is updated deliberately.
  - **`keyboard.md`** is updated too, if it names the order.
- **A markup test:** *Settings* is the tab bar's last button.
- **The guide:** `quick-start.md` and `settings.md` already say "at the
  right end of the tab bar", which becomes exactly true. Check that
  nothing else describes the old order.
- **Screenshots in the docs, metainfo or Flathub material:** list any
  that show the bar.
  - **Don't re-take them for this alone.** The next refresh picks it up.
  - Say which ones they are.

## Verification

- **The layout gate is green,** with nothing expected.
- **One capture:** 1000 × 700, 100 %, with a comparison tab open. Look at
  it in full.
- **Test names diffed.**

## Gates

The standard set. Reply in
`.git-exclude/review-request/193-settings-at-the-end.md`.

## Closed (2026-11-21, review 227)

Approved and complete.
