# RFC-076 patch 2 — repositories and safe-write review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/033-rfc076-patch2-repositories.md`
**Baseline:** `b710e7f` (`persist: add explicit-path repositories and safe migration writes (RFC-076 patch 2)`)
**Governing documents:** RFC-076 §"Repository API" and §"On first durable rewrite"; handoff §4.4 patch 2; audit finding B2
**Review mode:** Independent verification against the repository; code read, not
description. No implementation changes made.

## 1. Verdict

**Approved.** Proceed to patch 3.

No mandatory corrections. The `save.rs` extraction is a genuine pure move, the
migration commit order is crash-safe, the backup non-overwrite semantics are
right and are proven by a test that asserts a negative, and — the item with the
most user-visible consequence — the new defaults match the shipping ones exactly,
so patch 4 will not silently change what a first-run user sees.

Four non-blocking findings, one of which (N1) should be resolved in patch 3 when
a caller first exists.

B2 remains open until patch 4; B3 and B4 remain open; v1/public release stays
**No-Go**.

## 2. Verified

### The `atomic_replace` extraction is safe

Disclosing this explicitly was the right instinct — it is the one change outside
`persist/v2/` and it touches an S-005 path. The diff confirms it is a pure move:
the same four statements, the same error mapping, the same temp-cleanup on
rename failure, with `save_text` calling the extracted function. Nothing about
fingerprinting, backup policy, or `SaveRequest`/`SaveOutcome` moved. All 11
`save_tests` pass unmodified, which I re-ran independently.

### Migration commit order is crash-safe

```rust
let backup_path = ensure_pre_v2_backup(&self.path, original_bytes)?;
self.save(value)?;
```

Backup precedes replacement, which is the ordering that matters. Walking the
crash windows:

| Crash point | On-disk state | Retry behaviour |
|---|---|---|
| After backup, before write | original intact, backup present | backup preserved (not overwritten), write proceeds |
| During temp write | original intact, stray temp | temp overwritten on retry |
| During rename | original intact (rename is atomic) | retry succeeds |

No window loses the original. `ensure_pre_v2_backup`'s `if !backup.exists()`
guard is what makes retry safe, and
`settings_commit_migration_does_not_overwrite_existing_backup` proves it by
calling twice with different "original" bytes and asserting the first call's
bytes survive. Writing a test that asserts a negative is the right instinct here.

### Defaults match the shipping application

This is the finding with the most user-visible consequence at patch 4, so I
checked it field by field rather than accepting the claim.
`PersistedSettingsV2::default()` agrees with `AppSettings::default()` on every
UI-owned field, including the two that route through `#[derive(Default)]` and so
are not obvious from the constructor:

- `ThemeId::default()` → `Dark`, matching `Theme::Dark`
- `DiffFontFamilySetting::default()` → `SystemMono`, matching `DiffFontFamily::Monospace`

plus `diff_font_size` 14, `context_lines` 3, `active_profile` 0, empty ignore
strings, `remember_explorer_dirs` true, and the same four built-in profiles. A
first-run user after patch 4 gets exactly what they get today. Had either derived
default differed, patch 4 would have shipped a silent behaviour change with no
test to catch it.

## 3. Non-blocking findings

### N1 — `commit_migration` trusts caller-supplied bytes it cannot verify

The doc comment states the problem honestly: "`original_bytes` should be exactly
what was read to produce `value` via `load()` — the caller owns that pairing,
since this method has no way to verify it."

The failure mode: `load()` reads bytes B0 at T0; something changes the file to B1
at T1; `commit_migration(value, B0)` at T2 writes B0 into `.pre-v2.bak` and
atomically replaces B1 with the v2 envelope. B1 is gone and the backup does not
contain it. The window is small in the real flow — likely microseconds during
startup — and RFC-076 does not require a guard.

I am raising it anyway because of what it is, not how likely it is. "Guard the
actual target rather than a stale snapshot" is precisely audit finding B3, which
RFC-077 exists to fix in the save path. Reproducing that shape in the persistence
path while fixing it next door would be an unfortunate symmetry.

**Recommendation:** re-read the file inside `commit_migration` and compare
against `original_bytes`; on mismatch, return a conflict rather than proceeding.
This mirrors the project's existing S-006 external-modification posture and costs
one read.

**Resolve in patch 3**, when the runtime adapter that calls this first exists and
the right conflict behaviour can be decided together with the recovery states.

### N2 — Step order deviates from RFC-076's literal sequence

RFC-076 §"On first durable rewrite" orders it: write `.migration-tmp` → flush →
copy original to `.pre-v2.bak` → atomically replace. The implementation does
backup → (temp write → rename), because the temp write is inside the shared
`atomic_replace`.

Both are safe and the observable constraint (backup before replacement) is
honoured. The only consequence is that a failure of the subsequent write — disk
full, permissions — leaves a `.pre-v2.bak` beside a still-unmigrated file. That
self-heals on retry and never loses data, but it is a state the RFC's ordering
would not produce.

**Recommendation:** amend RFC-076's sequence to match the implementation, since
sharing `atomic_replace` is the more important property and it fixes the internal
ordering. Do not restructure the code to match the prose.

### N3 — "flush" is not literally implemented

RFC-076 step 3 says "flush according to the selected save contract".
`atomic_replace` performs `fs::write` then `fs::rename` with no `sync_all` on
either the file or the parent directory — identical to `save_text`, so the
selected contract is honoured as it actually exists.

This is already governed by audit finding N2 (registered as **F9**, "atomic and
power-loss durability wording exceeds the implementation"). No action here beyond
ensuring patch 5's documentation makes no durability claim this does not support.
Cross-referencing so the two are not resolved independently.

### N4 — `existing_created_unix` reads the target on every save

`build_envelope_json` reads and parses the target file to preserve
`created_unix`, so an ordinary settings save is now read-parse-write rather than
write. It fails open at every step and the file is small, so this is
inconsequential — recorded only so it is a known property rather than a surprise
if save latency is ever measured.

The behaviour itself is a good call: silently resetting a file's creation
timestamp on every preference change would have been an avoidable regression.

## 4. Answers to the requested review focus

### 4.1 Sharing `atomic_replace` versus a repository-local implementation

**Sharing is right.** RFC-076 explicitly says to "reuse or generalize core
safe-file primitives rather than creating an unrelated unsafe writer", and a
second hand-rolled temp-then-rename would be exactly that — a copy that can
drift from the tested one, in a project where two of the last three defects came
from things that looked correct but were never exercised.

The extraction is also the right *size*: only the primitive moved, and the
document-save policy that surrounds it — fingerprint preflight, `.bak`,
encoding — stayed in `save_text` where it belongs. A larger extraction would have
coupled preference writes to document-save semantics that do not apply to them.

### 4.2 The two-method split

**Matches RFC-076's intent.** The reasoning in the request is the correct one:
an ordinary save has no legacy source to protect, and always backing up would
accumulate a `.pre-v2.bak` that stops describing anything real. The RFC scopes
the backup to "the first durable rewrite", which is what `commit_migration` is.

Nothing else ordinary saves should protect against — with the one caveat in N1,
which is a load/commit pairing concern rather than a save-policy one.

### 4.3 `PersistenceError::Io` under `Corrupt`

**Right fit, and well reasoned.** RFC-076's `PersistenceLoad<T>` taxonomy has six
variants and deliberately no I/O case. The caller's obligation for an unreadable
file is identical to a corrupt one — preserve, report, never treat as absent — so
a seventh top-level variant would add a branch every caller handles the same way.
Distinguishing at the `PersistenceError` level is exactly the right granularity:
the taxonomy stays as the RFC defines it, and the message still names the real
problem.

The doc comment explaining why this must not be `Missing` — because treating it
so would mean writing over content that is still there — is the sentence a future
maintainer needs.

### 4.4 A more direct crash-safety test

The reasoning is sound and the order is right, so this is an improvement rather
than a gap. A more direct test is available without fault injection: induce a
failure in the window between backup and replacement by making the write fail
while the backup succeeds — for example, pointing the repository at a path where
a *directory* already exists, so `rename` fails — then assert that the backup
holds the original bytes and the pre-existing target is untouched.

That exercises the actual failure window rather than proxying it, using only
ordinary filesystem behaviour. Worth adding in patch 3 alongside N1's guard,
since both concern the same commit path.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **978** (678+27+16+2+241+6+7+1), exactly 969 + 9 |
| `cargo test -p forskscope-core save_tests` | Pass — 11/11, unmodified |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30752989766` | `success` on `b710e7ff` |
| `save.rs` extraction is a pure move | Confirmed by diff — same statements, same error mapping |
| `commit_migration` order | Backup precedes replacement; every crash window preserves the original |
| Backup non-overwrite | `if !backup.exists()`; proven by a negative-assertion test |
| `PersistedSettingsV2::default()` vs `AppSettings::default()` | Matches on every UI-owned field, including both derived defaults |
| No `forskscope-ui` production change | Confirmed |
| Repositories resolve no config directory | Confirmed — both take a bare `PathBuf` |

## 6. Notable quality observations

- Disclosing the `save.rs` touch under its own heading, with the reasoning for
  sharing over duplicating, is exactly the escalation the role boundary asks for
  on a security-critical path.
- The drive-by clippy work is correctly scoped and disclosed: six findings the
  patches introduced were fixed, eight pre-existing ones were left to F6 rather
  than quietly swept in.
- Naming what was deliberately *not* implemented — who decides when to call
  `commit_migration`, and config-directory resolution — keeps the patch boundary
  legible and made this review faster.

## 7. Recommended next action

1. Proceed to patch 3 — runtime adapters, recovery states, write-disable
   protection.
2. Resolve N1 in patch 3, with the §4.4 failure-window test alongside it.
3. Amend RFC-076's rewrite sequence per N2 rather than changing the code.
4. F26 remains required before patch 4. F9 and F25 remain M4's.
