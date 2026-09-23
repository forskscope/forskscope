# RFC-076 patch 3 — C1/C2 follow-up review

**Review date:** 2026-08-03
**Request:** `dev-record/review-requests/035-rfc076-patch3-c1c2-followup.md`
**Baseline:** `6956b79` (`persist: distinguish deferred vs failed migration commits (review 038 C1/C2)`)
**Responds to:** review 038, corrections C1 and C2
**Review mode:** Independent verification; no implementation changes made.

## 1. Verdict

**Approved.** C1 and C2 are closed.

One non-blocking finding on a user-facing label, to fix before patch 5 renders
it. Both of the requested judgement calls were made correctly.

B2 remains open until patch 4; B3 and B4 remain open; v1/public release stays
**No-Go**.

## 2. Verification

### C1 — closed

`MigrationCommitOutcome` now carries the cause, and `commit_migrated` matches
`PersistenceCommitError` explicitly rather than discarding it:

| Cause | Outcome | User-visible |
|---|---|---|
| `Ok` | `Committed { backup_path }` | one-time migration notice |
| `Err(Conflict)` | `DeferredByConflict` | silent — benign, self-healing |
| `Err(Io(detail))` | `Failed { detail }` | blocking dialog naming the cause |
| `None` (defensive) | `Failed` | surfaced, not swallowed |

Mapping the defensive `None` to `Failed` rather than silence is the right
instinct — an unexpected case should be loud.

The doc comments now state the real distinction, including *why* the two
non-committed cases differ ("this recurs on every launch until the underlying
cause is fixed"). The dialog body is accurate about the consequence: settings are
in use this session, and changes will not persist.

### C2 — closed

```rust
let write_disabled = !matches!(commit, MigrationCommitOutcome::Committed { .. });
```

True for both non-committed cases. The compound path traced in review 038 is
closed: a refused commit no longer leaves the file writable for the next settings
change.

### The renamed test

The disclosure is accurate and worth acknowledging. The old
`uncommitted_migration` runtime test obstructed the `.fsk-tmp` sibling with a
directory, which produces an **I/O failure**, not a conflict — so it had always
been exercising the `Failed` path while its name claimed otherwise. The rename to
`settings_resolve_surfaces_a_failed_commit_and_disables_writes` and the added
`write_disabled` assertion make the test say what it does. Finding and correcting
a mislabeled test that nobody asked about is the kind of thing that quietly
prevents a future misreading.

### The untested `Conflict` arm

The reasoning holds. `resolve_and_commit` calls `load_with_raw()` and
`commit_migration()` back-to-back inside one synchronous function, so no
single-threaded test can modify the file in between without a test seam. The
guard itself is covered directly at the repository level
(`settings_commit_migration_rejects_stale_bytes_after_external_change`), and the
`DeferredByConflict` mapping is covered at the view-model level by constructing
the variant.

That is the right split. Adding a seam to `resolve_and_commit` purely to reach an
arm already proven one layer down would buy coverage of the plumbing, not of the
behaviour.

## 3. Non-blocking finding

### N1 — `ContinueWithTemporaryDefaults` misdescribes the `Failed` case

The action enum is shared across three outcomes whose resolved values differ:

| Outcome | `resolution.value` | Label accurate? |
|---|---|---|
| `Incompatible` | `PersistedSettingsV2::default()` | yes |
| `CorruptPreserved` | `PersistedSettingsV2::default()` | yes |
| `Migrated(Failed)` | **the migrated value** | **no** |

For `Failed`, the user's settings were read and migrated correctly — they are not
on defaults at all. The dialog body says exactly that ("Your settings were read
and are in use for this session"), and then the button underneath offers to
"continue with temporary defaults". The two contradict each other, and the label
is the more pessimistic of the two: it implies the user has lost their settings
for this session when they have not.

**Recommendation:** add a distinct action for this case — `ContinueWithoutSaving`
conveys both what is true (your settings are fine) and what is not (they will not
persist). `RecoveryDialogAction` is a three-variant enum with no implementation
behind it yet, so this is a cheap change now and a more awkward one after patch 5
wires behaviour to each variant.

Fix before patch 5. Not before patch 4, which does not render dialogs.

## 4. Answers to the requested review focus

### 4.1 Should `Failed` offer something reset-shaped?

**No — the current action set is right.** Nothing is corrupt in this case; the
file is perfectly valid and the problem is that it cannot be written. A reset
would attempt a write, which is precisely the operation that is failing, so the
button could not succeed. Offering it would be the "button with nothing behind
it" you correctly declined to add for `ChooseAnotherLocation`.

`Exit` lets the user fix permissions or free disk space and relaunch; continuing
lets them work for the session. That is the complete set of things that can
actually happen.

### 4.2 Is `write_disabled: true` for `DeferredByConflict` an overcorrection?

**No — `Conflict` is the case C2 was actually about.** The compound path traced
in review 038 ran through the conflict arm specifically:

```text
something external writes B1
commit_migration verifies → Conflict → refused
...
user changes a setting → save() overwrites B1
```

A conflict means something else demonstrably wrote to that file. That is the
strongest reason to stop writing to it, not the weakest. `Failed` inherits the
flag as a bonus — if the filesystem refused the migration write it will refuse
ordinary saves too, so disabling them just makes the failure honest rather than
per-save.

Your reasoning — "we could not establish overwriting is safe holds equally for
both, even though only `Failed` needs to be *visible*" — is exactly the right
separation of the two concerns.

## 5. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test -p forskscope-core -p forskscope-ui-logic` | Pass — **1013** (699+27+16+2+255+6+7+1), exactly 1010 + 3 |
| `cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` | Pass |
| CI run `30771769968` | `success` on `6956b79a` |
| Cause preserved through to the outcome | Confirmed — explicit match, no `Err(_)` |
| `write_disabled` derived from commit outcome | Confirmed — true for both non-committed cases |
| Defensive `None` surfaces as `Failed` | Confirmed |
| Renamed test exercises the I/O path and asserts `write_disabled` | Confirmed |
| View-model has three distinct arms | Confirmed |
| No `forskscope-ui` production change | Confirmed |

## 6. Recommended next action

1. Treat C1 and C2 as closed and patch 3 as complete.
2. Fix N1 before patch 5. It can ride with any pre-patch-4 work if convenient.
3. Close **F26** — still required before patch 4.
4. Then begin patch 4, where B2 finally closes. That patch switches the
   production call sites, so it is the first time any of this milestone's work
   becomes reachable by a user.
5. F9, F25 remain M4's.
