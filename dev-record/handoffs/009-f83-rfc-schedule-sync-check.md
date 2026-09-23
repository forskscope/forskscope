# Developer Handoff 009 — F83: make the RFC schedule and folder verifiably agree

**From:** architect
**Date:** 2026-08-27
**Register:** F83. Also discharges part of **F13** — see §7a.
**Gate:** Not a Gate D blocker. Small, self-contained, and it lands clean.

---

## 1. Task title

A `cargo xtask` check that `ROADMAP.md`'s *"Remaining proposed RFCs"* table and
`rfcs/proposed/` contain exactly the same RFC numbers, wired into CI.

## 2. Purpose

The table is the project's only statement of *when* each RFC happens. On
2026-08-27 it had drifted from the folder **in both directions**:

- it listed **020** and **077** as remaining, when both are in `rfcs/done/` —
  implemented, moved, and still advertised as outstanding;
- **seven** RFCs sat in `proposed/` with no row at all, three of them accepted
  commitments (079, 080, 081) — invisible in the only table that schedules
  anything.

I reconciled it by hand. **That is the whole problem**: something that drifted by
hand has been re-synced by hand, and nothing stops the next drift. This is
F54's *"what stops the sixth?"* in a different folder.

## 3. Why this one lands clean, unlike F75's

**The check passes as of the reconciliation, so it needs no baseline and no
allowlist.** That matters because I argued against exactly that for F75, where
nine unwired modules would have forced a baseline — and a baseline is the
mechanism that lets the tenth through unnoticed.

Here there is nothing to grandfather. Land it green, and it is green because the
tree is correct rather than because the check was taught to ignore things.

## 4. Applicable requirements

- **RFC-000 lifecycle policy** (`.git-exclude/rules/000-rfc-lifecycle-policy.md`):
  the **folder is the source of truth** for an RFC's state. This check enforces
  that the schedule agrees with it, not the reverse — if they disagree, the
  folder is right and the table is wrong.

## 5. Change scope

- **New:** `xtask/src/rfc_sync.rs`
- `xtask/src/main.rs` — one dispatch arm
- `.github/workflows/ci.yml` — one step

## 6. Explicit non-change scope

- **Do not edit `ROADMAP.md` or any RFC to make the check pass.** It passes
  today. If it does not, that is a finding — stop and report it rather than
  adjusting the data.
- **Do not fix F13 generally.** §7a is the narrow part only.
- **Do not touch `css`, `i18n`, `audit-deps` or `version-sync`.**
- No RFC content changes, no lifecycle moves.

## 7. Required implementation

### 7a. A new module, not more of `main.rs`

`xtask/src/main.rs` is **655 lines** and `xtask/src/` contains nothing else.
**F13 names this exact file as the largest above the 300-ELOC soft threshold**,
with disposition *"opportunistic, when touched"* — and this handoff touches it.

So: **the check goes in `xtask/src/rfc_sync.rs`**, with `main.rs` gaining only a
dispatch arm. That honours F13's disposition at the moment it applies without
turning a small check into a refactor. **Do not** restructure the existing four
checks; that is a separate decision.

### 7b. What the check verifies

A new subcommand — `cargo xtask rfc-sync` — failing with a clear message when any
of these holds:

1. An RFC file exists in `rfcs/proposed/` with **no row** in the table.
2. A table row names an RFC that is **in `rfcs/done/` or `rfcs/archive/`** —
   the `020`/`077` shape, and the one a naive "does the file exist?" check
   misses.
3. A table row names an RFC that exists **nowhere**.
4. A file in `rfcs/proposed/` has **no `**Scheduling.**` line.** Every one carries
   it as of 2026-08-27; without this, a new RFC lands scheduled in the table and
   silent in its own file.

Report **every** violation, not the first — a maintainer who moved three RFCs
wants all three, not three runs.

### 7c. The parsing hazard, which is the real difficulty

**`ROADMAP.md` contains several pipe tables**, including the findings register
(`| F83 | … |`) and the milestone table (`| 1 | M2 … |`). A global regex for
`^| NNN |` will match things that are not RFC rows and miss nothing loudly.

**Anchor the parse to the `## Remaining proposed RFCs` heading** and stop at the
next heading. If that heading is absent, **fail** — do not silently find zero
rows and pass, which is the vacuous-green failure this project has repeatedly
caught elsewhere.

## 8. Required tests

Demonstrated failing, per the standing requirement — and here the four conditions
in §7b are each their own falsification. **Create the condition, observe the
failure, restore.**

1. Add a throwaway `rfcs/proposed/099-scratch.md`; the check must name `099`.
2. Add a table row for an RFC that lives in `done/` (`077` is the real
   historical case); the check must say it is not in `proposed/`.
3. Add a table row for a number that exists nowhere.
4. Remove the `**Scheduling.**` line from one proposed RFC.

Plus one that must **not** fail:

5. **The tree as it stands passes.** If it does not, stop — see §6.

And one that guards §7c:

6. **A `ROADMAP.md` with the heading renamed or missing fails**, rather than
   passing with zero rows found.

Unit tests over fixture strings are fine and preferable to driving real files
where you can; the parse is the part worth testing, and it is a pure function
over text.

## 9. Required documentation updates

None. **Do not edit `ROADMAP.md`** (§6).

## 10. Acceptance criteria

- `cargo xtask rfc-sync` passes on the tree as it stands.
- Each of §8.1–8.4 makes it fail, with a message naming the offending RFC number.
- §8.6 fails rather than passing vacuously.
- The check runs in CI beside the other four.
- `xtask/src/main.rs` grew by roughly a dispatch arm, not by the check.
- Gates green: `cargo fmt --manifest-path xtask/Cargo.toml --check`, plus the
  workspace suite as usual.

## 11. Prohibited shortcuts

- **Do not make the data fit the check.** The tree is correct; if the check
  disagrees, the check is wrong.
- **Do not let a missing section heading pass.** See §7c.
- **Do not report only the first violation.**
- **Do not add the check without wiring CI.** A check nobody runs is the thing
  this project has now found nine times; adding a tenth deliberately would be
  its own finding.
- **Do not report a falsification you did not run.**

## 12. Relevant boundaries

`xtask` is not a workspace member (DEC-005/F18) and builds with its own manifest
— note how CI invokes it. No new dependency: this is file listing and string
parsing, and `xtask` already reads and parses files for `version-sync`.

## 13. Known risks

- **The heading is load-bearing.** Renaming that section silently disables the
  check unless §7c's guard exists. That guard is the difference between a check
  and a decoration.
- **RFC numbers are three digits today** (`004`…`083`). Do not hard-code a range;
  parse what is there.

## 14. Required evidence

- Observed failure output for each of §8.1–8.4 and §8.6, quoted.
- Confirmation that §8.5 passes unmodified.
- The CI step as added.

## 15. Required review-request format

Short — this is a small change. Lead with the falsifications.

State plainly:
- whether `main.rs` grew by more than a dispatch arm;
- whether the check reports all violations or stops at the first;
- what happens when the section heading is missing.
