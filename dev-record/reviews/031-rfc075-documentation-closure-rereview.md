# RFC-075 documentation closure rereview

**Review date:** 2026-08-01  
**Requested baseline:** `be5d28e45abc49f21ca07fa42f0527965c89e921` (`ui: guard async compare completion with load tokens`)  
**Actual repository baseline:** `93b586887e5449af7c1baeffa063c2c600b1b157` (`Update .gitignore`) with the RFC closure unstaged  
**Prior review:** `dev-record/reviews/030-rfc075-documentation-closure-review.md`  
**Review scope:** the corrections requested by review 030, preservation of the full RFC-075 closure, lifecycle counts/paths/references, evidence wording, and focused documentation gates.  
**Review mode:** Independent focused rereview; no source or durable-document implementation changes.

## 1. Verdict

**Accept.**

Review 030's blocking lifecycle contradiction is resolved, both non-blocking
recommendations were applied, and no new closure issue was found. RFC-075 may
be treated as Implemented, audit finding B1 and milestone M1 may be treated as
complete, and the documentation closure may be committed separately before
RFC-076 begins.

This verdict does not change the overall v1/public-release **No-Go**. RFC-076
through RFC-078, integrated gates, platform evidence, the refreshed release
handoff, and the final architecture review remain outstanding.

## 2. Blocking findings

None.

Review 030 B1 is closed: RFC-075 is no longer present under `ROADMAP.md`'s
“Remaining proposed RFCs” table (`ROADMAP.md:279-296`). Its sole indexed table
entry is now in the Implemented section and points to the lifecycle-authoritative
`done/` path (`rfcs/README.md:9-63`).

## 3. Non-blocking findings

None newly identified.

Review 030 N1 is closed: the roadmap now records the independently observed
headless inventory as 943 total tests, including 241 UI-logic unit tests, with
the unchanged component arithmetic (`ROADMAP.md:11-14`). No stale 930/228 text
remains.

Review 030 N2 is closed as a clarity issue: the handoff labels reviews 028 and
029 as “Ignored workspace-local review evidence (not committed/public links)”
and renders their paths as code rather than Markdown links
(`rfcs/handoffs/075-async-compare-identity-and-generation/implementation-handoff.md:98-104`).

## 4. Lifecycle and evidence assessment

- The new RFC exists only at
  `rfcs/done/075-async-compare-identity-and-generation.md`; the old proposed
  path is absent.
- The RFC status remains `Implemented (post-v0.164.0 stabilization)`, accurately
  avoiding an invented shipped version (`done/075-...md:1-6`).
- The index and filesystem agree on 49 implemented and 18 proposed RFCs.
- No Markdown reference to the old `proposed/075-...` path remains.
- RFC-074 and the roadmap consistently record M1/B1 complete while retaining
  B2-B4 and the final release decision as outstanding
  (`rfcs/proposed/074-v1-release-stabilization-program.md:96-103` and
  `ROADMAP.md:65-68`).
- The completed handoff remains self-contained: it records actual files,
  decisions, commands/results, implementation commits, limitations, and the
  separate RFC-076 next-workstream boundary.
- The accepted exhaustion wording remains unchanged and correctly distinguishes
  generation exhaustion on a live tab from tab-ID exhaustion before append
  (`done/075-...md:150-161`).

The request named `be5d28e` as the repository baseline, but current `HEAD` is
`93b5868`. The intervening commit only adds `*.local.json` to `.gitignore`; it
does not overlap the RFC closure, change `.git-exclude/`'s already-ignored
status, or alter the accepted Rust implementation. The closure still modifies
only the five durable Markdown files listed in the request.

## 5. Missing evidence

No evidence is missing for this documentation closure.

Rust tests and Clippy were appropriately not rerun for the two Markdown-only
corrections. Their implementation evidence was observed before `be5d28e` and
preserved in the completed handoff. Runtime/platform acceptance and later
program gates are intentionally missing at this milestone and remain explicit
release blockers, not RFC-075 closure requirements.

## 6. Observed checks in this rereview

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `git diff --check HEAD` | Pass for tracked closure changes |
| Trailing-whitespace scan of all five durable closure documents, including the untracked new done-path RFC | Pass |
| RFC lifecycle filesystem counts | Pass; 49 done, 18 proposed |
| New done path exists / old proposed path absent | Pass |
| Search for the old proposed RFC-075 path | Pass; no references remain |
| RFC-075 search in RFC tables | Pass; only the implemented `rfcs/README.md` entry remains |
| Roadmap inventory search | Pass; 943/241 present, 930/228 absent |
| Handoff evidence-label search | Pass; explicitly ignored, workspace-local, and non-public |

Ordinary `git diff --check` does not inspect an untracked file, so the separate
trailing-whitespace scan explicitly included the new `rfcs/done/` file.

## 7. Recommended next action

The project owner may commit the five-file RFC-075 documentation closure as a
standalone change. After that closure lands, begin RFC-076 in a separate
reviewed workstream, preserving the rule that legacy persisted IDs must never
be installed as runtime `CompareTabId` values.

Do not combine RFC-076 implementation into the closure commit, and do not infer
an overall v1 Go decision from RFC-075/M1 completion.
