# Handoff 110 — RFC-101 stage A: measure the catalog format, list the assembled messages

**From:** architect. **Date:** 2026-11-21. **The `0.200.0` cycle,
measurement only.** Stages B and C are `0.201.0`.
**RFC:** `rfcs/accepted/101-translation-groundwork.md`, accepted by the
owner on 2026-11-21. **Read §1, §3 and *Stages*.**

**No product change in this handoff.**
- A measurement branch or worktree is fine, kept out of `main`.
- **Adding Fluent to `main` waits for the decision.**

## 1. The catalog format: the numbers

**Fluent:**
- **In a scratch worktree,** add `fluent-bundle` (and whatever it needs to
  format one message with a placeholder and a plural) to `forskscope-ui`.
- **Report:**
  - every new crate in `Cargo.lock`, with version, licence and
    maintainer;
  - `cargo audit` and `cargo deny` (if configured) on that tree;
  - the release binary's size on Linux against `0.199.0`'s, built the
    same way.
- **Migrate ten real messages,** including one assembled message and one
  count, and show what the `.ftl` file and the call sites look like.

**Our own format:**
- **Sketch it in the same ten messages:**
  - the file;
  - the build-time parser that turns it into Rust tables;
  - its size in lines, and the tests it would need.
- **No runtime dependency,** by design.

**Both:**
- **How the audit checks would work** (RFC-101 §3's four checks).
- **What a later conversion from ours to Fluent would take,** in a
  paragraph.

**Your recommendation, with the numbers.** RFC-101 states mine, our own
format, **before** the numbers. Disagree if the numbers say so.

## 2. The assembled messages: the list

**Every place where two or more translated results are joined in code.**
For each:
- **where** (file:line);
- **the English and Japanese as shown today;**
- **the proposed whole sentence in both,** with named placeholders.

**The Japanese wording goes to the owner for review** in stage B. Mark any
that you think changes meaning, not just order.

**Also list every count shown to the user** (RFC-101 §2's call sites),
with its current English and Japanese.

## Reply

**Measurement only:**
- the two-format comparison;
- the two lists;
- your recommendation.

**Then stop for my answer.** Reply in
`.git-exclude/review-request/198-translation-groundwork-stage-a.md`.

## Closed (2026-11-23, review 232)

**Approved.** The format is our own. RFC-101 is amended with stage A's
decisions. Stage B gets its own handoff for `0.201.0`.
