# RFC-076 patch 6 — recovery UI and documentation review

**Review date:** 2026-08-04
**Request:** `dev-record/review-requests/042-rfc076-patch6-recovery-ui-and-docs.md`
**Baseline:** `95b184e`, over `4af3678` and `db17ea7`
**Governing document:** `recovery-ui-and-docs-handoff.md`; RFC-076 with its 2026-08-03 amendment; F28, F28b, review 043 N1
**Review mode:** Independent verification against the repository. No implementation changes made.

## 1. Verdict

**Approved. M2-B is complete and RFC-076 is closed.**

Every acceptance criterion in the handoff is met, verified independently rather
than from the summary. Two registrations follow from the requested review focus,
neither blocking.

M2-A's exit gate — verification at a real release cut — remains open, so **M2 is
not yet complete**. B3 and B4 remain open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — **1031** (658+27+16+2+29+29+257+6+6+1), exactly 1007 + 24 |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo xtask i18n` | Pass — 220 keys covered |
| F28 wording, all three outcomes, both documents | Present, with tests asserting it |
| `reset_with_backup` ordering | `verify_unchanged` → backup → save — mirrors `commit_migration` exactly |
| `.reset.bak` distinct from `.pre-v2.bak` | Confirmed, both in code and docs |
| **Escape cannot dismiss a recovery dialog** | Confirmed — `modal_open && !recovery_open` |
| `README.md` / `users/installation.md` | Untouched across all three commits |
| RFC-076 in `rfcs/done/`; index 50/17 | Confirmed |
| Review 043 N1 | Zero `UserSettings` references left in `settings_view.rs` |

Two things stand out as better than required.

**`reset_with_backup` reuses the commit path rather than resembling it.**
`verify_unchanged` first, then the non-overwriting backup, then the atomic write
— the same order proven crash-safe in patch 2, including the stale-bytes refusal
from review 038's N1. A reset that skipped `verify_unchanged` would have been an
easy and defensible-looking mistake.

**Escape is exempted for the recovery variants.** The request does not dwell on
this, but it is the difference between a decision point and a notification. A
dismissable blocking dialog would have let a user bypass the choice and land in
the state the dialog exists to prevent, with writes disabled and no explanation.

## 3. Answers to the requested review focus

### 3.1 `Exit` → `dioxus_desktop::window().close()`

**Correct for this application**, and the right level of confidence to claim.
It is the framework's own mechanism, the app is single-window, and
`exit_on_last_window_close` defaults true — you verified the vendored source and
then confirmed it live rather than resting on either alone.

Cross-platform behaviour is precisely what RFC-078 exists to establish, and this
action deserves to be named there rather than assumed. RFC-078's P08 covers
persistence migration but predates the dialogs, so the recovery dialog's Exit on
Windows and macOS is not currently in any case. **Registered as F37** to be
folded into P08 before M5.

### 3.2 The F28b queue has no headless test

**Acceptable, and adding a harness here would have been the wrong call.**

Three reasons. The pure decision logic *is* tested — `recovery_modal` returns the
right `Modal` variant per outcome on both sides, which is the part that can be
got wrong by reasoning. The integration is evidenced by a specific runtime
observation, not a general claim: settings dialog resolved, session dialog
appeared automatically, both documents' failures shown. And introducing the
codebase's first `VirtualDom`-backed test harness in the final patch of a
milestone that already touches 28 files is exactly the scope expansion that
produces incidents.

Raising it as a question rather than quietly claiming coverage is the behaviour
that makes the rest of the request trustworthy.

But this is now the **third** place where correctness rests on runtime evidence
because `Store` cannot be constructed in a test — patch 4's startup wiring,
patch 4's C1 fix, and now the recovery queue. That is no longer an incidental
gap; it is a structural property of the `Store` API that will keep costing
coverage on exactly the integration seams where defects hide. **Registered as
F36** for M4 to decide deliberately, rather than each patch re-deciding it under
deadline.

### 3.3 `.reset.bak` versus `.pre-v2.bak`

**Reads clearly**, and `docs/src/users/settings.md` is the strongest user-facing
writing this project has produced. The situation table maps cause to choices
without jargon, the downgrade paragraph answers the question a worried user
actually has, and "until you choose an action, nothing you change in that session
is saved" states the consequence in one sentence.

`.reset.bak` is introduced beside the action that creates it, so the name
explains itself. `.pre-v2.bak` is carried by "automatically upgraded from an
older format", which does the work without requiring the reader to know what v2
means.

**A correction I owe you.** The patch-5 handoff told you not to rename
`.pre-v2.bak` because "this filename is on users' disks and renaming it would
orphan every backup already written." That reasoning was wrong. Migration only
became reachable at patch 4, and published `0.165.0` still used `ConfigManager` —
I confirmed it against the tag. **No released version has ever written a
`.pre-v2.bak`.** Nothing would have been orphaned.

The decision stands on its merits — the name is descriptive and now documented —
but the constraint I gave was harder than reality, and a future maintainer should
not treat it as immovable.

### 3.4 The ROADMAP and RFC-074 touches

**Right scope.** §7's list existed to fix the F33 boundary, not to enumerate
every permitted file. `ROADMAP.md` and RFC-074 are the program's records, and
RFC-075's precedent is exactly the one to follow: a progress record belongs in
the commit that produces the progress, or it goes stale. Verifying yours against
`f04f5cad` before writing was the right check.

The entry itself is accurate, including the detail I would have expected to be
smoothed over — that **M2-A remains open within M2**, because its exit gate is
verification at a real cut, not content review. That distinction is what keeps
M2 honestly incomplete.

Mild preference for future milestones, not a correction: I would rather write the
program's progress record myself, so architect and implementer documents stay on
opposite sides of the boundary. Flagging it as you did is the right handling in
the meantime.

## 4. Test-count delta

+24 over 1007, itemised per file with the lib/bin doubling explained. Every new
test's subject is named. The six core repository tests cover `reset_with_backup`
in the three modes that matter — writes through, does not overwrite an existing
backup, refuses stale bytes.

## 5. Recommended next action

1. Treat **M2-B and RFC-076 as complete**.
2. **F23** (`actionlint`) before M2's cut — still the one thing standing between
   here and a release attempt, and `release.yml` has now been edited twice since
   anything parsed it.
3. **F33** is unblocked and mine; I will take it once F23 lands so the docs are
   written against a cut that works.
4. M2's cut closes M2-A's exit gate and decides the release level; RFC-076's
   Status field takes the version then.
5. M3 (RFC-077) follows. F24, F25/F25b, F31, F34, F35, F36, F37 are M4's.
