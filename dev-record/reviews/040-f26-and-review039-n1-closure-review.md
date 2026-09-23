# F26 closure and review-039 N1 — verification review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/036-f26-and-review039-n1-closure.md`
**Baseline:** `014debf` (`persist: close F26 (schema-enum wire-format coverage) and review-039 N1`)
**Responds to:** review 036 (F26), review 039 (N1)
**Review mode:** Independent verification by mutation testing; all mutations
reverted. No implementation changes retained.

## 1. Verdict

**Approved.** F26 and N1 are both closed.

F26 is verified closed by the same mutation that exposed it. Two new findings of
the same family surfaced while checking how far the guard now reaches — both are
cheap fixture/copy changes, neither blocks patch 4, and both are registered.

B2 remains open until patch 4; B3 and B4 remain open; v1/public release stays
**No-Go**.

## 2. F26 — verified closed by mutation

The new `persist_v2_schema_enum_wire_format_tests.rs` asserts both directions per
variant (`to_value` against a literal string, then `from_value` back), one test
per enum, 32 variants across all ten — including the five that were already
covered by the fixture's profile list. Making the file a complete reference
rather than a patch over the five that were broken is the right call: a reader
should not have to know which half was historically at risk.

I re-ran the exact mutation from review 036, plus a second on another
previously-unpinned enum:

| Mutation | Before F26 | Now |
|---|---|---|
| `ThemeId::Dark` → `"darkx"` | all 969 passed | `theme_id_variants_have_exact_wire_strings` **FAILED** |
| `NewlinePolicy::ForceCrlf` → `"force-crlfx"` | (unpinned) | `newline_policy_variants_have_exact_wire_strings` **FAILED** |

The property review 036 actually wanted — *a variant rename fails a test* — now
holds, and holds independently of what any fixture happens to contain.

## 3. Review-039 N1 — closed

`RecoveryDialogAction::ContinueWithoutSaving` is added and used by the
`Migrated(Failed)` arm; `Incompatible` and `CorruptPreserved` keep
`ContinueWithTemporaryDefaults`, which remains accurate for them because their
resolved value genuinely is defaults. Tests assert both the new action's presence
and the old one's absence, which is the right pair — asserting only presence
would not catch a dialog offering both.

## 4. New findings

While checking how far the guard now reaches, I mutated the two field-shaped
cases the enum tests do not cover. One is protected; one is not.

### F27 — six defaulted fields are still unpinned (verified)

A serde-only rename of a **required** field is caught: `PerformanceLimits` has no
`#[serde(default)]`, so a renamed field goes missing, deserialization fails, the
payload becomes `Corrupt` instead of `Current`, and the golden fixture test
fails. Verified:

| Mutation | Result |
|---|---|
| `PerformanceLimits::max_eager_lines` → `"max_eager_lines_x"` | `current_v2_golden_fixture_parses_to_the_exact_expected_struct` **FAILED** |

That is the N2 strictness decision from patch 1 paying off — declining to add
`#[serde(default)]` to required fields is what makes a rename loud rather than
silent. Two decisions reinforcing each other, which is worth noticing.

But a `#[serde(default)]` field whose fixture value **equals its default** is not
caught, because the renamed key is ignored and the field falls back to a value
identical to what the test expects:

| Mutation | Result |
|---|---|
| `show_line_numbers` → `"show_line_numbers_x"` | all 709 core tests **passed** |

Six fields are in this position — `show_line_numbers`, `wrap_long_lines`,
`remember_explorer_dirs`, `restore_session`, `enable_binary_comparison`, and
`recent_limit` — each carrying a fixture value identical to its default. Every
other defaulted field already differs (`density` spacious vs comfortable,
`diff_font_family` consolas vs system-mono, `explorer_compact` true vs false, and
so on) and is therefore pinned.

The consequence is milder than the enum case: a renamed field silently resets one
stored preference to its default rather than making the file unreadable. It is
still the silent-degradation class RFC-076 exists to remove.

**Fix:** give every defaulted field a non-default value in the golden fixture and
the expected struct — flipping six booleans and one number. That restores the
property for the whole payload rather than most of it.

**Registered as F27.** Cheap enough to ride with patch 4; not a blocker for
starting it.

### F28 — two dialogs disable writes without saying so

`write_disabled` is `true` for three outcomes: `Migrated(Failed)`,
`Incompatible`, and `CorruptPreserved`. Only the first tells the user:

| Outcome | Body mentions changes will not be saved? |
|---|---|
| `Migrated(Failed)` | yes — "Changes will not be saved until this is resolved." |
| `Incompatible` | no — "The file has not been modified." |
| `CorruptPreserved` | no — "The settings file is preserved but could not be parsed." |

A user who picks *Continue with temporary defaults* on a future-version file gets
a working application, changes a setting, and nothing persists — with no
indication that would happen. That is the same visibility principle as review
038's C1, one layer over: the consequence is real and the user is not told.

**Fix:** extend the `Incompatible` and `CorruptPreserved` bodies to state the
same consequence the `Failed` body already does.

**Registered as F28**, to land before patch 5 renders these dialogs.

## 5. Answers to the requested review focus

### 5.1 Does the enum reuse pattern itself need to change?

**No — the test gap was the whole problem, and it is closed.** Reusing core types
as the canonical wire form is what RFC-076 asks for, and the coupling it creates
is now defended in three complementary layers:

1. schema-invariant doc comments on each type, so the intent is visible where a
   change would be made;
2. per-variant wire-format assertions, so an enum rename fails a test;
3. the deliberate absence of `#[serde(default)]` on required fields, so a
   required-field rename fails a test too.

I verified layers 2 and 3 by mutation rather than by reading. F27 is a gap in
layer 3's reach, not a reason to revisit the pattern.

### 5.2 Is `ContinueWithoutSaving` the right name and granularity?

**The name is right; the axis it names is inconsistent with its sibling.**

`ContinueWithTemporaryDefaults` names *which value you continue with*.
`ContinueWithoutSaving` names *what will not happen to your changes*. Both facts
are true of both actions — all three write-disabled outcomes continue without
saving — so a user comparing them could reasonably infer that
`ContinueWithTemporaryDefaults` does save.

I would not rename anything. The cleaner fix is F28: put the shared consequence
in the dialog bodies, where it applies to all three cases, and let the action
labels keep naming the thing that actually differs between them — defaults versus
your own settings. That keeps the enum small and puts the common fact in one
place instead of encoding it into two of four variant names.

So: no further granularity needed in `RecoveryDialogAction`. Good instinct to ask
at the cheapest point, but the adjustment belongs in the copy rather than the
enum.

## 6. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **1023**, exactly 1013 + 10 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30775978671` | `success` on `014debf` |
| Mutation: `ThemeId::Dark` wire form | **Caught** — was the review-036 gap |
| Mutation: `NewlinePolicy::ForceCrlf` wire form | **Caught** |
| Mutation: `PerformanceLimits` required-field rename | **Caught** via `Corrupt` |
| Mutation: `show_line_numbers` defaulted-field rename | **Not caught** — F27 |
| `ContinueWithoutSaving` used only by `Migrated(Failed)` | Confirmed |
| Tests assert absence of the old action | Confirmed |
| Working tree after four mutations | Clean; all reverted |

## 7. Recommended next action

1. Treat F26 and review-039 N1 as closed.
2. Begin patch 4 — the production switch, where B2 finally closes. F27 can ride
   with it; neither new finding blocks it.
3. F28 before patch 5 renders the dialogs.
4. F9, F25 remain M4's.
