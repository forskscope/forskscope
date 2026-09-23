# RFC-075 documentation closure review

**Review date:** 2026-07-15  
**Repository baseline:** working tree over `be5d28e45abc49f21ca07fa42f0527965c89e921` (`ui: guard async compare completion with load tokens`)  
**Review scope:** `dev-record/review-requests/026-rfc075-documentation-closure.md`, RFC lifecycle policy, the RFC-075 move and finalized text, completed handoff, RFC index, RFC-074 progress record, roadmap, local review evidence, and observed documentation checks.  
**Review mode:** Independent lifecycle/evidence review; no source or durable-document implementation changes.

## 1. Verdict

**Needs changes.**

The RFC move, final status, accepted exhaustion policy, implementation outcome,
handoff evidence, RFC index counts, M1/B1 progress wording, and continued v1
No-Go boundary are otherwise accurate. One lifecycle contradiction remains in
the roadmap, so the documentation closure is not yet internally consistent.
The accepted RFC-075 runtime implementation at `be5d28e` is not reopened by
this verdict.

## 2. Blocking findings

### B1 — The roadmap still classifies implemented RFC-075 as “Remaining proposed”

This patch moves RFC-075 to `rfcs/done/`, marks it Implemented, removes it from
the RFC index's Proposed section, and records M1/B1 complete. However,
`ROADMAP.md` still lists:

```text
| 075 | Milestone M1 | Stable async compare identity and load generations |
```

under `## Remaining proposed RFCs` (`ROADMAP.md:279-297`). This directly
contradicts the same file's M1-complete statement (`ROADMAP.md:65-68`), the
implemented index entry (`rfcs/README.md:9-63`), and RFC-000's rule that the
`done/` folder is lifecycle-authoritative
(`rfcs/done/000-rfc-lifecycle-policy.md:91-130`).

**Required for acceptance:** remove RFC-075 from the “Remaining proposed RFCs”
table (or move it to an explicitly delivered/history table if the roadmap needs
to retain the row). The current placement cannot remain in an RFC lifecycle
closure.

## 3. Non-blocking findings

### N1 — The roadmap's “current” headless test inventory is stale after RFC-075

`ROADMAP.md:11-14` still claims 930 headless tests, including 228 UI-logic unit
tests. The accepted RFC-075 work added 13 UI-logic tests. A current inventory
observed during this review reports 943 tests for the same core + UI-logic
headless set; the workspace run in review 029 reported 241 UI-logic unit tests.
The arithmetic remains otherwise unchanged:

```text
643 core unit + 45 core integration + 241 ui-logic unit
+ 6 CSS integration + 8 doctests = 943
```

RFC-074 assigns a broader count reconciliation to M4, so this stale number does
not invalidate RFC-075's implementation evidence. Nevertheless, because this
patch already updates `ROADMAP.md` with a dated “current” M1 progress record,
refreshing 930/228 to 943/241 now is the smallest durable-source correction and
avoids knowingly carrying false current-state numbers into RFC-076.

### N2 — Checkpoint review links are valid only in the originating workspace

The completed handoff uses Markdown links to review 028 and 029 under
`dev-record/reviews/` (`implementation-handoff.md:98-104`). Those files exist
locally, but `.git-exclude/` is ignored and therefore the links will not resolve
in a clean clone or hosted repository view. The handoff is sufficiently
self-contained—the verdicts, commits, commands, decisions, and limitations are
also recorded in durable text—so this does not block closure under RFC-074's
local audit-source convention.

For clarity, consider rendering these as backticked workspace-local evidence
paths rather than ordinary Markdown links, or explicitly label them as ignored
local links at the point of use. Do not imply that the review artifacts are
committed/public evidence.

## 4. Lifecycle and evidence assessment

- **Move/status:** Correct. The old proposed path is absent, the new done path
  exists, and `Implemented (post-v0.164.0 stabilization)` states lifecycle and
  release timing without inventing a shipped version
  (`rfcs/done/075-async-compare-identity-and-generation.md:1-6`).
- **Exhaustion policy:** Correct. The RFC now distinguishes existing-tab
  generation exhaustion from pre-tab ID exhaustion and records fail-closed
  behavior for both (`done/075-...md:150-161`), closing review 029 N1.
- **Implementation outcome:** Correct. Commits `ad2e98e` and `be5d28e`, the
  token boundary, deterministic race coverage, B1/M1 closure, and remaining
  release block are accurately summarized (`done/075-...md:222-241`).
- **Handoff:** Substantively complete. It lists the actual implementation and
  closure files, accepted decisions, observed mandatory and advisory gates,
  implementation commits, limitations, and next-workstream boundary
  (`implementation-handoff.md:1-136`).
- **Index:** Correct. Filesystem and index both report 49 implemented and 18
  proposed RFCs; RFC-075 links to `done/`, and no reference to its old proposed
  path remains (`rfcs/README.md:9-94`).
- **Program boundary:** Correct. RFC-074 and the roadmap mark only M1/B1
  complete, retain B2-B4 and later gates, and keep v1/public release No-Go
  (`rfcs/proposed/074-v1-release-stabilization-program.md:96-103` and
  `ROADMAP.md:65-68`).
- **Next workstream:** Correct. The handoff directs work to RFC-076 separately
  and preserves fresh runtime identity during persistence migration
  (`implementation-handoff.md:131-136`).

## 5. Missing evidence

- No new runtime evidence is required for this documentation-only closure. The
  implementation evidence was observed and independently accepted in reviews
  028 and 029 before baseline `be5d28e`.
- Runtime/platform acceptance, integrated Gate C, RFC-076-RFC-078, the refreshed
  release handoff, and final architecture Go/No-Go remain intentionally absent
  and are correctly described as outstanding.
- The optional Store-level exhaustion/startup integration tests noted in review
  029 remain absent and correctly remain a documented limitation rather than a
  retroactive RFC-075 blocker.

## 6. Observed checks in this review

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `git diff --check be5d28e45abc49f21ca07fa42f0527965c89e921` | Pass for tracked closure changes |
| Trailing-whitespace scan of all five durable Markdown outputs, including the untracked new RFC path | Pass |
| RFC lifecycle filesystem counts | Pass; 49 done, 18 proposed |
| New done path exists / old proposed path absent | Pass |
| Search for references to `proposed/075-async-compare-identity-and-generation.md` | Pass; none remain |
| Review 028/029 local targets | Pass in this workspace; see N2 |
| `git show --check be5d28e` | Pass; implementation commit has no whitespace errors |
| Core + UI-logic test inventory (`cargo test ... -- --list`) | 943; confirms N1 |

Rust tests and Clippy were not rerun for this documentation-only working-tree
patch. Their accepted implementation results are preserved accurately in the
handoff; listing the test inventory executes no test cases.

## 7. Recommended next action

Make a minimal documentation correction before closing RFC-075:

1. remove RFC-075 from `ROADMAP.md`'s “Remaining proposed RFCs” table;
2. preferably refresh the same roadmap's headless inventory from 930/228 to
   943/241 while it is already being updated; and
3. optionally clarify that the review 028/029 links are ignored local evidence.

Then rerun `cargo fmt --check`, `git diff --check`, the RFC path/count checks,
and the old-path/reference scan, and request a focused re-review. Keep the patch
documentation-only and do not start RFC-076 until this lifecycle closure is
accepted.
