# RFC-076 patch 1 — C1/C2 follow-up review

**Review date:** 2026-08-02
**Request:** `dev-record/review-requests/032-rfc076-patch1-c1c2-followup.md`
**Baseline:** `f899193` (`persist: clamp diff_font_size and pin the v2 wire format (review 035 C1/C2)`)
**Responds to:** review 035, mandatory corrections C1 and C2
**Review mode:** Independent verification, including an empirical mutation test
of the new guard. No implementation changes retained.

## 1. Verdict

**Approved.** C1 and C2 are closed, and patch 2 may begin.

C1 is complete. C2 is closed as specified and verified to work — but the
verification also exposed a residual hole that my specification did not ask for
and that a golden fixture structurally cannot close. It does not block patch 2;
it must close before patch 4. Registered as F26.

B2 remains open until patch 4; B3 and B4 remain open; v1/public release stays
**No-Go**.

## 2. C1 — closed

`normalize()` now clamps `diff_font_size` to `FONT_SIZE_MIN..=FONT_SIZE_MAX`,
widened to `u32` correctly, alongside the existing `appearance_font_size` clamp
(`persist/v2/settings.rs:390-395`). The range matches
`forskscope-ui-logic::validate_font_size` (6..=50). New test covers both
directions. N1 applied: `default_diff_font_size()` is split out and used at the
v1 migration site, so the two defaults can now move independently.

## 3. C2 — closed as specified, with a residual gap

### The guard works

I verified this by mutation rather than by reading. Adding
`#[serde(rename = "nightx")]` to `ThemeId::Night` — a wire-format change that
compiles cleanly and that no other test targets:

```text
test tests::persist_v2_settings_tests::current_v2_golden_fixture_parses_to_the_exact_expected_struct ... FAILED
test result: FAILED. 25 passed; 1 failed
```

The golden fixture catches it. The round-trip test alone would not have. The
fixtures are genuinely literal — hand-written kebab-case strings, not derived
from a serialized struct — which is what makes this work.

### The residual gap

Repeating the experiment on `ThemeId::Dark`, a variant the fixture does not
carry:

```text
all suites pass — 969/969
```

Nothing fails. And `Dark` is not an arbitrary choice: it is the default theme in
`AppSettings::default()`, so it is the value sitting in the majority of real
users' settings files. Renaming it would silently make most existing
configurations unreadable, which is precisely the failure C2 exists to prevent.

Coverage divides cleanly by field shape:

| Enum | Variants | Covered | Missing |
|---|---:|---:|---|
| `WhitespaceMode` | 4 | 4 | — |
| `DiffAlgorithm` | 4 | 4 | — |
| `InlineMode` | 3 | 3 | — |
| `NewlineCompareMode` | 2 | 2 | — |
| `CaseSensitivity` | 2 | 2 | — |
| `ThemeId` | 3 | 1 | `Dark`, `Light` |
| `Density` | 3 | 1 | `Comfortable`, `Compact` |
| `FontFamilySetting` | 3 | 1 | `SystemMono`, `SystemSerif` |
| `DiffFontFamilySetting` | 5 | 1 | `SystemMono`, `SystemSans`, `SystemSerif`, `CourierNew` |
| `NewlinePolicy` | 3 | 1 | `Preserve`, `ForceCrlf` |

The profile enums reach full coverage because `profiles` is a list and the
fixture carries four entries. The scalar-field enums cannot: one payload holds
exactly one theme. **Twelve variants are unpinned, and no additional fixture can
change that.**

### This is my specification's fault, not the implementation's

Review 035 asked for "a golden fixture containing every enum variant that
appears in the canonical payload." The delivered fixture does contain every
variant that appears in a payload — a payload holds one theme, and that theme is
covered. The request was satisfied as written.

What I actually wanted was the property "a variant rename fails a test," and I
named a mechanism that cannot deliver it for scalar fields. That is the third
time in this milestone that my acceptance wording, not the implementation, was
the limiting factor — `test -s` in M2-A, and now this. The lesson is specific
enough to carry forward: **state the property to be guaranteed and let the
implementer choose the mechanism**, rather than naming the artifact and assuming
it implies the property.

### The fix

Not more fixtures. A per-variant serialization assertion, which is exhaustive,
compact, and independent of payload shape:

```rust
assert_eq!(serde_json::to_string(&ThemeId::Dark).unwrap(), "\"dark\"");
```

one line per variant across the ten schema enums, plus a matching deserialize
direction if you want symmetry. The golden fixtures stay — they pin structure,
field names, and nesting, which per-variant assertions do not.

**Registered as F26, required before patch 4.** Not before patch 2: patch 2 adds
repositories and file I/O but still writes no v2 file to a real config
directory. Exposure begins at patch 4. Blocking patch 2 over this would be
disproportionate to what is now in place.

## 4. Remaining items — all verified

| Item | Status |
|---|---|
| N1 — `default_diff_font_size()` split | Applied, used at the v1 migration site |
| N2 — strictness documented | `persist/v2.rs:30-40` explains why the required fields have no `#[serde(default)]`, and names the silent-reset regression that adding one would reintroduce |
| §4.1 — schema-invariant comments | Present on all schema enums across `settings/display.rs`, `diff/options.rs`, `encoding.rs`, `job/limits.rs` |
| §4.4 — frozen-input note | Present on both `from_payload_json` implementations, stating they are migration inputs and not parsers to evolve |

The N2 note is the one I would have been most worried about being written
mechanically, and it is not — it names the specific future bug report that would
tempt someone to add a default, and why that would undo B2.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **969** (669+27+16+2+241+6+7+1), exactly 966 + 3 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30749636074` | `success` on `f8991935` |
| `diff_font_size` clamp and range | Matches `validate_font_size` 6..=50 |
| Golden fixtures are literal, not derived | Confirmed by inspection |
| **Mutation test — covered variant (`Night`)** | **Golden test FAILS as intended** |
| **Mutation test — uncovered variant (`Dark`)** | **All 969 pass — gap confirmed** |
| Working tree after experiments | Clean; both mutations reverted via `git checkout` |

## 6. Recommended next action

1. Proceed to patch 2 — repositories, explicit paths, safe migration writes.
2. Close F26 before patch 4, when the first v2 file could reach a user's config
   directory.
3. F25 remains M4's.

Patch 1 is complete. Its design was settled at review 035 and nothing since has
required amending RFC-076.
