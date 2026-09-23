# RFC-076 patch 1 — settings/session schema v2 design review

**Review date:** 2026-08-02
**Request:** `dev-record/review-requests/031-rfc076-patch1-schema-v2-migration.md`
**Baseline:** `5054000` (`persist: add settings/session schema v2 routing and migration (RFC-076 patch 1)`)
**Governing documents:** RFC-076; handoff §4.4 (mandatory pause after patch 1); audit finding B2
**Review mode:** Independent verification against the repository; code read, not
description. No implementation changes made.

## 1. Verdict

**Conditionally Approved.** The design is accepted. Two corrections must land
before patch 2 begins; both are in patch 1's own schema layer.

This is a strong patch. The routing decision tree matches RFC-076's table
branch for branch, the legacy DTOs are correctly anchored so arbitrary JSON
cannot be mistaken for a legacy file, all nine of RFC-076's applicable
core-schema test cases are present, and the two hardest modelling judgements —
which built-in profile set is canonical, and how far v1 session recovery can
honestly reach — were both resolved the right way with reasoning I can check.

Both corrections come from the same place: validation and format stability got
less attention than routing and migration did.

Patch 1 correctly closes nothing. B2 closes at patch 4; B3 and B4 remain open;
v1/public release stays **No-Go**.

## 2. Corrections required

### C1 — `diff_font_size` is never validated

`normalize()` clamps `appearance_font_size` to 6..=50 and `context_lines` to
..=20, but leaves `diff_font_size` untouched
(`persist/v2/settings.rs:382-392`).

RFC-076 §Validation requires "font size: clamp to the supported UI range". The
supported range is defined in `forskscope-ui-logic`:

```rust
validate_font_size:    const MIN: u32 = 6;  const MAX: u32 = 50;
validate_context_lines: MIN 0, MAX 20   // matched correctly by CONTEXT_LINES_MAX
```

So `context_lines` is right and `appearance_font_size` is right, but the
asymmetry runs the wrong way: `diff_font_size` is the **more** user-visible of
the two fonts — it is the one the Settings dialog exposes and the one that sizes
the diff panes. A file carrying `diff_font_size: 0` or `9999` — hand-edited,
partially corrupted but still parseable, or written by a future version whose
range differs — survives `normalize()` intact and renders the diff view
unusable.

**Required:** clamp `diff_font_size` to the same 6..=50 range, and add a test
alongside `out_of_range_appearance_font_size_clamps`.

### C2 — The v2 wire format is not pinned by anything

`current_v2_envelope_round_trips` builds `sample_v2()` as a Rust struct,
serializes it, and reads it back. Both directions use the same definitions, so
the test passes no matter what those definitions serialize *to*. It proves
self-consistency, not format stability.

That matters specifically because of the decision in review question 1.
`PersistedSettingsV2` embeds `ThemeId`, `Density`, `FontFamilySetting`,
`LocaleId`, `NewlinePolicy`, `WhitespaceMode`, `NewlineCompareMode`,
`CaseSensitivity`, `InlineMode`, `DiffAlgorithm`, and `PerformanceLimits`
directly. Every one of those is now part of the on-disk contract. A rename of a
single enum variant — the sort of change nobody would think of as a schema
change, because the type lives in `diff/options.rs` next to diff logic — silently
makes every existing user file unreadable. Under this design that surfaces as
`Corrupt`, which by intent preserves the file and shows an error: the user's
settings stop loading and stay stopped until someone intervenes.

The asymmetry is stark and worth stating plainly:

| Format | Pinned by | Status |
|---|---|---|
| v0 | hand-authored `settings-v0.json` / `session-v0.json` | pinned |
| v1 | fixtures generated from the real v1 writer | pinned |
| **v2** | **nothing** | **unpinned** |

v0 and v1 are historical and will never be written again. v2 is the one format
this project will actually write to users' machines from patch 4 onward, and it
is the only one with no golden file.

**Required:** commit a `settings-v2.json` and `session-v2.json` golden fixture
containing a literal envelope with every enum variant that appears in the
canonical payload, and assert `load_settings_v2`/`load_session_v2` produce the
expected struct from them. That test fails on a variant rename; the round-trip
cannot.

**Timing:** before patch 2. Both corrections are schema-layer and belong with
the patch that created the schema. Once patch 4 ships, real v2 files exist and
this guard becomes retroactively necessary rather than cheap.

## 3. Non-blocking findings

### N1 — A UI default is being sourced from a core default

`migrate_from_v1` fills `diff_font_size: default_appearance_font_size() as u32`
(`settings.rs:305`). The two happen to coincide at 14 today, but they are
different settings with different owners — the field matrix itself lists them on
separate rows. If the core appearance default ever moves, the UI diff-font
default silently follows it. Introduce `default_diff_font_size()` even though it
returns the same number.

### N2 — Strict required fields are correct but undocumented

`PersistedSettingsV2` leaves `theme`, `language`, `diff_font_size`,
`context_lines`, `profiles`, `active_profile` without `#[serde(default)]`, and
`PersistedSessionV2` does the same for `tabs`. A v2 payload missing any of them
is therefore `Corrupt` rather than silently defaulted.

That is the right behaviour and consistent with the whole point of B2 — but it
is exactly the kind of strictness a future maintainer "fixes" by adding
`#[serde(default)]` to make a bug report go away, reintroducing the silent
reset. State the intent in the module doc so the next person has to argue with a
comment rather than an absence.

### N3 — Two built-in profile sets now diverge permanently

Choosing the UI's four as v2 canonical is correct (see §4.2), but it leaves
`CompareProfile::all_presets()` — core's four, `Default` / `Code Review` /
`Loose Text` / `Large File Safe` — as a set that no UI reaches and that v2 will
never produce, while `is_core_preset_name` still references it for `built_in`
tagging. That is a pre-existing inconsistency this patch inherits rather than
creates, but convergence is the moment it should be resolved or explicitly
documented as legacy. **Registered as F25 against M4.**

## 4. Answers to the requested review focus

### 4.1 Reusing core types instead of parallel `*V2` types

**Right call, and it is what RFC-076 asks for** — "one public canonical domain
type plus private version-specific payload DTOs". Parallel `*V2` duplicates plus
hand-written conversions would add a whole class of transcription bugs to
eliminate a coupling that is, in the end, intentional.

But the coupling risk you identified is real, and C2 is its mitigation. The
invariant this creates should be written down where the types live, not only
here: **these enums are the settings disk schema; changing a variant name or
its serde representation is a schema change and requires a version bump.** A
one-line comment on each affected enum would do it.

### 4.2 The UI's built-in profile set as canonical

**Correct, and for the right reason.** Migration exists to preserve what users
have; core's presets have never been reachable through any control, so no user
has one. Preferring the richer-but-unseen set would have been modelling
elegance at the cost of the actual goal.

Verified the mechanics: v0 carries every profile with `built_in` flags and
`active_profile` intact, mapped up into the richer v2 shape; v1 places the one
selected profile at index 0 and appends UI built-ins by name without
duplicating. That matches RFC-076's "recreate canonical built-ins, then append
valid custom profiles without duplicate names."

The residue is N3 — resolve the divergence at M4, do not carry it silently to
v1.0.

### 4.3 v1 session bounded recovery

**Correct, and forced rather than chosen.** Verified in the source: v1's
`from_payload_json` carries its own note that `tabs` is always empty because
restoration was never wired up. Recovering `root` only is therefore not a
narrowing decision — there is nothing else in the file. `active_tab = Some(0)`
when a tab was recovered is the only defensible value.

Do **not** synthesize anything richer. Inventing tab state from a root would
manufacture data the source never contained, which is the opposite of what a
migration should do.

### 4.4 `pub(crate)` v1 parsers as the trust boundary

**Right call.** Re-deriving the v1 shape with serde would have had to reproduce
`compare_profile`-serialized-as-a-name-then-looked-up, and a silent divergence
there would corrupt migrations of exactly the files the RFC calls immutable
inputs. Reusing the frozen parser removes that risk entirely, and the diff
confirms only visibility and doc comments changed.

The boundary is better protected than the question implies: the v1 fixtures were
generated from the real v1 writer and are committed, so any future change to v1
parsing or serialization breaks those tests. That is a genuine regression guard,
not just a convention. Worth noting on the parser itself that it is a frozen
migration input, since `pub(crate)` alone does not communicate "do not evolve
this".

### 4.5 The two ELOC files

**Both accepted.**

`persist/v2/settings.rs` at 327 already has the legacy DTOs extracted; splitting
routing from migration would trade a soft-threshold overrun for indirection
across a boundary that is read together. Not worth it at this size.

`core/session.rs` at 322 was over the threshold before this patch, and a
visibility change plus a doc comment is not the substantive edit the handoff's
"split as you touch it" was aimed at. Your reading is the right one. It stays on
the F13 opportunistic list for M4.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **966** (666+27+16+2+241+6+7+1), exactly 943 + 23 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| `cargo xtask audit-deps` | Pass after the serde addition |
| No `forskscope-ui` production change | Confirmed — diff against the crate is empty |
| Routing decision tree vs RFC-076 table | Matches branch for branch |
| Legacy v0 anchoring (settings) | `theme`, `language`, `diff_font_size` required — arbitrary JSON cannot match |
| Legacy v0 anchoring (session) | `tabs` required — same |
| `FutureVersion` precedes payload parsing | Confirmed — a future file with an odd payload still reports `FutureVersion` |
| Unknown payload fields tolerated | Confirmed, no `deny_unknown_fields` |
| v1 parser edit is visibility + docs only | Confirmed by diff |
| `persist_tests.rs` untouched | Confirmed — the temporary fixture generator left no trace |
| Fixture sanitization | All paths `/tmp/fixtures/…`; no real user data |
| RFC-076 core-schema test coverage | All nine patch-1-applicable cases present |
| `diff_font_size` clamped | **No** — C1 |
| v2 wire format pinned | **No** — C2 |
| Threat-model dependency table | serde/serde_json added; `app_json_settings` corrected to 2.4.1 |

## 6. Notable quality observations

- Generating the v1 fixtures from the real writer rather than hand-authoring
  them is the right instinct, and deleting the generator afterwards with a clean
  `git diff` is the right hygiene. Hand-typed fixtures for a format with
  name-lookup transforms would have been a latent trap.
- Correcting the stale `app_json_settings` version in the threat-model table
  while editing next to it was proportionate — small, adjacent, disclosed.
- Flagging both ELOC decisions for review rather than resolving them silently is
  the behaviour that makes a design pause worth having.
- The request separates what the handoff specified from what the implementer
  chose, and names its most consequential decision explicitly. That is why this
  review could focus on judgement rather than archaeology.

## 7. Recommended next action

1. Apply C1 and C2, with their tests, as a short follow-up to patch 1.
2. Add the schema-invariant comments from §4.1 and the frozen-input note from
   §4.4; adopt N1 and N2.
3. Then proceed to patch 2 (repositories and safe writes). Do not begin it
   before C1 and C2 land — they are corrections to the schema this patch
   defines, and patch 2 builds directly on it.
4. F25 is registered against M4 and is not patch 2's concern.

The design is settled. Nothing here requires amending RFC-076.
