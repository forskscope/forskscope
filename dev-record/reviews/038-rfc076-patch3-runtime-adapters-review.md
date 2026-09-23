# RFC-076 patch 3 — runtime adapters review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/034-rfc076-patch3-runtime-adapters.md`
**Baseline:** `9abafab` (`persist: resolve N1 and add RFC-076 patch 3 runtime adapters`)
**Governing documents:** RFC-076 §"User-facing behavior", §"UI integration"; handoff §4.4 patch 3; audit finding B2; review 037 (N1, §4.4)
**Review mode:** Independent verification against the repository; code read, not
description. No implementation changes made.

## 1. Verdict

**Conditionally Approved.** Two corrections required before patch 4 begins.

Review 037's N1 is resolved exactly as recommended, and the failure-window test
is precisely the one I described — obstructing the sibling temp path with a
directory so the backup succeeds and the write fails, then asserting the backup
holds the original and the target is untouched. That is real evidence of the
real window.

The two corrections are in the same seam: what happens when a migration commit
fails. They are not independent — they compound into a concrete data-loss path
that N1's fix was meant to close. Both are reachable only once patch 4 wires
this to startup, which is why they must land first.

B2 remains open until patch 4; B3 and B4 remain open; v1/public release stays
**No-Go**.

## 2. Corrections required

### C1 — The failure cause is discarded, and silence is justified by an assumption true of only one cause

`commit_migrated` collapses every failure into one untyped case
(`settings/runtime.rs:98`):

```rust
Some(Err(_)) | None => (None, false),
```

`PersistenceCommitError` has two variants and they are not alike:

- **`Conflict`** — the file changed between load and commit. Benign,
  self-healing, retried next run.
- **`Io(String)`** — permission denied, read-only config directory, full disk.
  **Persistent.** It will fail identically on every launch, forever.

Three places then treat `committed: false` as though only the first exists:

1. The doc comment on `SettingsRuntimeOutcome::Migrated` states `committed` is
   `false` "only if the durable rewrite lost a race with an external change".
   That is inaccurate — an I/O failure produces the same value.
2. `SettingsRecoveryView` shows a notice only when `committed` is true, so a
   failed commit produces **no output at all**.
3. The test asserting that silence justifies it as "an uncommitted migration
   will simply retry next run" — true for `Conflict`, false for `Io`.

The result inverts RFC-076's own requirement that "failed saves are visible and
do not masquerade as success". Here a *successful* migration produces a notice
and a *failed* one produces silence, so silence is the failure signal. A user
with a read-only config directory has their legacy file re-migrated every launch,
never backed up, and is never told.

**Required:** carry the failure cause into the outcome, and surface the
persistent case. The benign race may legitimately stay silent — the distinction
between the two is the whole point. Correct the doc comment and the test's
justification to match.

### C2 — A refused migration leaves the file writable without verification

When `commit_migration` refuses, `commit_migrated` returns
`write_disabled: false` (`settings/runtime.rs:100-107`), and
`SettingsRepository::save` performs no verification of any kind — it builds the
envelope and calls `atomic_write_envelope`.

So the application declines to migrate a file because it changed underneath it,
then remains free to overwrite that same file the moment any setting changes.

### The two combined

```text
1. load_with_raw reads legacy bytes B0        → migrated value V
2. something external writes B1
3. commit_migration verifies → Conflict       → refused    ← N1's guard, working
4. committed: false, cause discarded, no notice shown       ← C1
5. write_disabled: false
6. user changes any setting → save() → V' overwrites B1
   no verification, no backup                               ← C2
7. B1 is gone. The user was told nothing at any point.
```

N1's guard buys protection until the first settings change, and nothing tells
anyone the guard fired. This is the same shape as audit finding B3 — act on a
stale snapshot, overwrite the real target — which is exactly what N1 was raised
to prevent.

**Required (stated as a property, not a mechanism):** once a migration commit has
been refused because the on-disk file did not match what was read, the
application must not subsequently overwrite that file without re-establishing
that doing so is safe.

Setting `write_disabled: true` on a refused commit is the smallest change that
satisfies it and is coherent with the flag's existing meaning — we could not
establish ownership of this file, so we do not write to it this run. It costs the
user the ability to persist settings changes for one session in a case where
something else is concurrently writing their config, which seems the right trade.
Verifying inside `save()` is the alternative; RFC-076 does not require it and it
is heavier. Choose either, but the property must hold.

## 3. Verified

- **N1 fix.** `verify_unchanged` runs first in `commit_migration`, before the
  backup and the write. It treats `NotFound` as `Conflict` rather than an I/O
  error, which is right — a vanished file is a change, not a read failure.
- **Failure-window test.** `settings_commit_migration_survives_failure_between_backup_and_replace`
  pre-creates the `.{name}.fsk-tmp` sibling as a directory so `atomic_replace`
  fails after the backup succeeds, then asserts the backup content and an
  untouched target. This is the test recommended in review 037 §4.4, implemented
  as described.
- **`load_with_raw`.** Returning `(load, Option<Vec<u8>>)` removes the second
  racy read that pairing bytes with `commit_migration` would otherwise need.
  `load()` delegating to it keeps one code path.
- **`resolve_and_commit` write discipline.** Writes only for the two migration
  outcomes; `Missing`, `Current`, `FutureVersion`, and `Corrupt` never write.
  `write_disabled` is true for exactly `Incompatible` and `CorruptPreserved`,
  matching RFC-076's `persistence_write_disabled`.
- **Recovery action sets.** `Incompatible` offers Exit and
  ContinueWithTemporaryDefaults but never Reset — with a test whose message
  explains why ("a future file may be valid to a newer build"). `CorruptPreserved`
  offers Continue and Reset but never Exit. Both match RFC-076.

## 4. Answers to the requested review focus

### 4.1 Scope of the N1 fix

**Inside `commit_migration` is right.** It makes the guarantee unconditional
rather than dependent on each caller remembering, and it mirrors how `save_text`
owns its own fingerprint preflight instead of delegating it. There is one caller
today; patches 4 and 5 may add more, and a guarantee that lives in the callee
does not decay as callers multiply.

### 4.2 Silently reporting `committed: false`

**No — see C1.** The reasoning would hold if the race were the only cause, and
review 037 did describe that race as microseconds. But `Io` reaches the same
outcome and is not a race at all: it is a standing condition that recurs every
launch. Conflating a self-healing event with a permanent one under a single
untyped `false` is what makes the silence wrong, not the decision to stay quiet
about the race itself.

You were right to flag this as the one outcome with no explicit RFC-076
behaviour to match — but RFC-076 does give the governing principle, in
§"UI integration": failed saves are visible and do not masquerade as success.

### 4.3 The core / ui-logic split

**Right boundary, and consistent with the precedent you cite.** Core does I/O and
decides what happened; `ui-logic` maps that to renderable content with no
Dioxus or GTK dependency, exactly as `SaveErrorView` does for `AppError`. Moving
the mapping into core would put dialog copy — including user-facing English
strings needing i18n — inside the crate that is meant to be presentation-free.

The new top-level `session` module is also correctly placed: persisted-session
recovery is a different concern from `compare::tab_state`/`load_identity`, which
are runtime tab identity.

### 4.4 The rewritten non-overwrite test

**It does not under-prove it — arguably the reverse.** Pre-seeding a backup whose
content is distinguishable from anything the commit would write, then asserting
that content survives a legitimate commit, tests `ensure_pre_v2_backup`'s
`if !backup.exists()` guard directly. The old formulation proved the same thing
only as a side effect of a caller pattern the new contract correctly forbids.

Flagging it as a test change driven by a fix rather than new coverage was the
right disclosure.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **1010** (698+27+16+2+253+6+7+1), exactly 978 + 32 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30770696155` | `success` on `9abafabe` |
| `verify_unchanged` precedes backup and write | Confirmed |
| `NotFound` treated as `Conflict` | Confirmed |
| Failure-window test obstructs the real temp path | Confirmed |
| Non-overwrite test proves the guard | Confirmed |
| `resolve_and_commit` writes only for migration outcomes | Confirmed |
| `write_disabled` true for `Incompatible`/`CorruptPreserved` | Confirmed |
| `save()` verification | **None** — C2 |
| Commit failure cause preserved | **No** — `Err(_)` discards it — C1 |
| No `forskscope-ui` production change | Confirmed |

## 6. Notable quality observations

- Implementing review 037's §4.4 suggestion as an actual test rather than
  treating it as optional is what turned a reasoned argument into evidence.
- Disclosing the rewritten test under its own heading, with the reason the old
  formulation became illegal, is the kind of change that is easy to slip through
  silently and expensive to discover later.
- Declining to add `RecoveryDialogAction::ChooseAnotherLocation` for a capability
  that does not exist is correct. A dialog button with nothing behind it is a
  bug, not a placeholder.

## 7. Recommended next action

1. Apply C1 and C2 as a patch 3 follow-up.
2. Close F26 in the same batch — it is also required before patch 4, and both are
   pre-patch-4 work.
3. Then begin patch 4, where B2 finally closes.
4. F9, F25 remain M4's.

The design is settled. Neither correction requires amending RFC-076 — C1
implements a principle it already states, and C2 preserves a property N1
established.
