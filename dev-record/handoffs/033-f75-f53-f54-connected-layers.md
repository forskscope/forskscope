# Handoff 033 — F75, F53, F54: nothing left unconnected, and a check that keeps it so

**From:** architect. **Release:** `0.171.0` (`ROADMAP.md` § *Release plan*).
**Register:** F75 (remainder), F53, F54. **Medium.**

## 1. Why this comes before RFC-080

The owner decided (RFC-080 Q4) that F75 lands before RFC-080. The concrete
reason recorded then — `DigestState` — is already gone. The reason that remains
is stronger: this project has shipped **five** defects of one shape, *built,
tested, documented, never connected* (F52, F75, F84, F88a, and RFC-085's
restoration, which my own handoff 022 would have left unreachable). RFC-080 adds
new `ui-logic` state. Without a check, it is the likeliest sixth.

## 2. The candidate list — verify it, do not trust it

A rough scan of `forskscope-ui-logic/src/lib.rs`'s `pub use` list found these
with **no word-boundary reference anywhere in `forskscope-ui/src`**:

```
build_palette  clamp_font_size  ConflictNavView  ConflictRailRow
DeepCompareSummary  density_choices  find_active  font_family_choices
guard_for_sizes_with_limits  MatchPosition  MatchSide  PaletteRow
ProfileChoice  profile_presets  RecoveryButton  SelectChoice
StartupArgError  StatusRow  theme_choices  validate_context_lines
validate_font_size
```

**That list is an overcount, and I know it.** Every symbol I spot-checked also has
5–20 references *inside* `ui-logic` — some from tests, some from exports the UI
does consume. `guard_for_sizes_with_limits`, for instance, sits behind
`guard_for_sizes`, which the UI calls. A symbol used only as a return type of a
live function is not dead either. Establish each one from the call graph.

## 3. The decision rule

For each candidate, one of:

- **Used by something the UI consumes** → keep it, and if it need not be part of
  the crate's public surface, stop re-exporting it at the root. Being `pub` is
  what makes it look like an unconnected layer.
- **Genuinely unused, describing behaviour the UI implements inline** → one
  source of truth. Wire the UI to it if it is the better home and its values are
  correct; otherwise delete it.
- **A view-model for deferred post-v1 UI** — `ConflictNavView`/`ConflictRailRow`
  (the conflict workspace), `build_palette`/`PaletteRow` (the command palette) —
  → **delete it.** It returns from git history with its feature. *This is a
  design decision and it is mine:* the alternative is an allowlist in §5's
  check, and an allowlist is exactly how *"what stops the sixth?"* stayed
  unanswered. The owner may overrule it; do not decide it the other way yourself.

**Say which rule each symbol fell under** in the review request. A table is
fine.

## 4. F53, specifically — three sources, not two

`settings/modal.rs:85` clamps font size to **8–32** inline. The unused
`clamp_font_size` clamps **6–50**. `font_family_choices` duplicates the modal's
own match at `:94`. And there is a `validate_font_size` too, which may be what
persisted settings use.

**Find every place a font-size bound lives** — the modal, the helpers, and
settings persistence — and leave exactly one, with the value the product
actually ships (8–32). If persistence accepts something the modal would clamp,
say so; that is a finding, not a detail.

## 5. F54 — the check

A CI check, no allowlist, that fails when a public export of
`forskscope-ui-logic`'s crate root has no consumer in `forskscope-ui`. An
`xtask` subcommand is the natural home; wire it into `ci.yml`.

**My scan's failure is the design brief.** A plain word search overcounts
(internal and test references) and can undercount (a glob import would hide
everything). So:

- decide what counts as a consumer, and state the limits in the check's own doc
  comment;
- **fail loudly if `forskscope-ui` ever gains a glob import of
  `forskscope_ui_logic`** — that would blind the check silently, and a check that
  can go blind without failing is the F42 defect again;
- ignore references in test code on the `ui-logic` side when deciding whether
  something is *consumed*.

## 6. Falsification

1. A planted, unused public export makes the check **fail**, naming the symbol.
2. A planted glob import of `forskscope_ui_logic` in `forskscope-ui` makes the
   check **fail**, rather than passing everything.
3. After the cleanup, the check **passes with no allowlist**.
4. For F53: changing the single remaining font-size bound fails a test that
   exercises the shipped path — the modal or persistence, not the helper alone.

## 7. Scope

**In:** `forskscope-ui-logic` exports, the settings modal's font-size and family
handling, `xtask` and `ci.yml` for the check, tests.
**Out:** RFC-080 (`0.172.0`); building any deferred UI; `forskscope-core`'s
public surface — this check is about `ui-logic` only.

## 8. Gates

The usual set, plus the new check itself green in CI.
