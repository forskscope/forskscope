# Assessment — Independent audit, 2026-09-01

**Reviewer:** architect
**Audit:** `.git-exclude/tmp/audit/report/AUDIT-2026-09-01.md`, commit `099e2c0`
**Verdict:** **Accepted, with one evidence correction and one context correction.**
Its two Criticals and three of its Highs are real; I reproduced them.

## 1. What I verified, and how

The owner's standing instruction is that an outside reviewer may not know this
project's rules and may be wrong. So nothing here was accepted on report.
Everything that drives a schedule change was re-reproduced:

| Finding | How verified | Result |
|---|---|---|
| F86 `is_dirty` depth-based | ran the scenario against the real `MergeSession` | saved `"a\nb\nc\nD\n"`, buffer `"A\nb\nc\nd\n"`, `is_dirty() == false` |
| F87 lossy encode | called `encode_text` directly | wrote literal `"hi &#128512;\n"`, flag `false` |
| F89 insecure temp file | pre-created the temp name as a symlink | unrelated file overwritten; `doc.txt` became a symlink to it |
| F90 UTF-16 | `classify` vs `decode_bytes` on the same bytes | `Binary` from one, correct `UTF-16LE` text from the other |
| F85 swap/save_target | read `swap_sides` and `build_request`'s `None` arm | `save_target` untouched; it is the sole source of path, precondition and encoding |
| F88 `EditabilityClass` | grep for call sites | zero outside `lib.rs`'s re-export and its own tests |
| F91 CRLF patches | read `write_lines`; counted CRLF cases in `patch_apply.rs` | unconditional `push('\n')`; zero CRLF cases |

Spot-checked and also correct: no crate-root `build.rs` (so `Rust:` is
permanently `unknown`), `external_tool` has no UI call site, `--diagnostics` is
matched anywhere in the argument list.

## 2. Where the audit is wrong

**Finding #14's evidence is stale, and the real defect is worse.** It lists
`compare::scroll_sync`, `compare::summary`, `compare::command_bar` and
`compare::tab_state` as modules with "zero references in `crates/forskscope-ui/src`".
All four were **deleted** in `d69c83b`, an ancestor of the commit audited. The
finding — that `architecture.md`'s module table is wrong — **stands and is
sharper than stated**: the document does not overstate wiring, it documents four
modules that do not exist, with API descriptions.

That defect is ours and it is new. Handoff 010 required annotating the three
`done/` RFCs whose view-models were removed and **said nothing about
`architecture.md`**, which is the document a reader consults first. Registered as
**F93**.

**Finding #13 lacks project context.** It cites `ui-logic`'s tests falling
241→199 as evidence of drift going unnoticed. That fall is deliberate: F75(b)
part 1 deleted four obsolete modules and their 58 tests, decided by the
architect, implemented, reviewed at review 081, and recorded. `testing.md` is
genuinely stale — that half is right — but the shrink is not a regression, and
the count must not be "fixed" by restoring deleted tests.

I could not fault anything else. The `patch/build.rs` trap — a source file, not a
Cargo build script — was navigated correctly.

## 3. Where the audit is right in a way that matters more than its severity

**The reason for No-Go was wrong, and that is the finding of the audit.** This
project recorded one open blocker, B4, and B4 is about *evidence*. That framing
implied the software was correct and only its platform record was missing. Two
Criticals and three Highs in the write path say otherwise.

I record that as a failure of this register, not of the dev team. Every one of
these defects sat in code the register describes in detail, and the register's
own summary line read "Gate D blocked on F44 and F60" — which I wrote, repeatedly,
and twice had to correct upward for smaller reasons.

**And the pattern it names is the one we keep finding.** *"ForskScope repeatedly
builds the right abstraction, tests it well, documents it — and then does not
connect it."* `EditabilityClass::requires_save_guard()` would have prevented F87.
`BomPolicy` would have prevented the phantom BOM diff. That is now the **fourth**
shipped defect traced to an unwired layer (F51, F52, F76 were the others), and it
is the strongest argument yet for F75's no-allowlist gate.

## 4. What I did with it

- **B5 opened**, mapped to **RFC-082**, exactly as B1–B4 map to RFC-075–078.
- **M7 added**, running *in parallel* with M5's upstream wait — these are
  correctness and security defects in shipped code and do not queue behind F44.
- **Release-blocking outcomes** gained five entries. The audit's remark that the
  list "previously listed five outcomes of which four were met, which reads as
  nearly there" was fair.
- **RFC-083** (encoding breadth) and **RFC-084** (patch conformance) for the
  non-blocking clusters, both post-v1 — with the audit's proviso adopted:
  shipping a limitation is fine, shipping it while the docs claim otherwise is
  not. Those documentation corrections are immediate.
- **F85–F93 registered.**

**RFC-082 is deliberately self-contained**, because the audit report lives under
`.git-exclude/` and is untracked. If it is lost, the analysis survives — which is
the same gap F66 recorded for correspondence and that every review in
`dev-record/reviews/` still has.

## 5. What I did not do

I did not fix anything. Every finding is recorded, scheduled, and waiting on the
owner's acceptance of RFC-082 — which carries three open questions, one of them
whether B5 blocks at all.
