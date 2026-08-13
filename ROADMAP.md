# ForskScope Roadmap

**Last updated:** 0.166.0 cut (2026-08-08); M2 and M3 complete
**Current phase:** v1 release stabilization — release-baseline reconciliation,
then correctness workstreams, then runtime/platform acceptance and a new
architecture go/no-go review.
**Planning basis:** ordered tasks, dependencies, and exit gates. Milestones
carry no calendar windows or effort estimates; a milestone completes when its
gate evidence exists.

---

## Current state

The `forskscope-core` and `forskscope-ui-logic` crates are feature-complete for
the v1 two-way diff/merge workflow. The current observed workspace gate passes
**1094 tests** with zero failures.

The UI crate (`forskscope-ui`) has the v1 two-way workflow implemented:
two-pane diff with independent pane labels and shared horizontal scroll;
English/Japanese translation-key coverage enforced by `cargo xtask i18n`
(223 `t(...)` keys); per-file and batch copy in the directory report view;
F3/Shift+F3 search navigation; compare profiles; session restore; patch export;
and release-gate CSS freshness checks.

Release-readiness hardening completed after v0.140.0:

- XLSX parsing was security-disabled: `.xlsx` files are recognized but
  comparison fails closed until the `sheets-diff -> calamine -> quick-xml`
  dependency path is remediated.
- Dioxus desktop network-capable transitive dependencies were reviewed. Default
  Dioxus features/devtools are disabled, and the accepted loopback WebSocket IPC
  path is enforced by `cargo xtask audit-deps`.
- The project no longer builds its own source archive (F43); `PKGBUILD` fetches
  GitHub's automatic per-tag archive directly instead.
- CI and release preflight now run the documented gates: format, CSS, audit,
  dependency-path audit, version sync, i18n coverage, tests, and clippy. Release
  tags are checked against the workspace version before artifacts are created.

The 2026-07-15 architecture audit approved continued development but issued a
v1/public-release No-Go over four blockers. **Three are now closed:** B1 (stable
async tab/load identity, RFC-075, released in `0.165.0`), B2 (versioned
production settings/session persistence, RFC-076) and B3 (a distinct
Git-mergetool save-target model, RFC-077), both released in `0.166.0`.

**B4 remains open** — platform runtime evidence, RFC-078 — so the v1/public
release decision stays **No-Go**. GTK/WebKitGTK and cross-platform package
verification are M5, gated behind M4's integrated stabilization. Three-way merge
conflict workspace UI, command palette, and editor adapter work remain
post-v1.

---

## v1 release stabilization program

[RFC-074](rfcs/proposed/074-v1-release-stabilization-program.md) is the
authoritative program design. RFC-075 through RFC-078 define the detailed
workstreams. Milestones are ordered by dependency and completed by gate
evidence. No milestone carries a target date or effort estimate.

| # | Milestone | Scope | Depends on | Exit gate | Release |
|---|---|---|---|---|---|
| — | M0 — Design approval | RFC-074–078 designs and compatibility decisions | — | Owner and architect accept detailed designs | — |
| — | M1 — Async identity | Stable tab IDs, load generations, deterministic race tests | M0 | RFC-075 acceptance complete | — |
| — | R0 — Stabilization baseline | Version/CHANGELOG reconciliation for the unreleased delta; release-trigger reconciliation; documentation-truth fixes; version-sync tag check | M1 | Gate B plus an observed release-workflow run and owner release approval | 0.165.0 |
| 1 | M2 — Release mechanics and persistence convergence | **M2-A:** release-notes composition, release policy documentation, threat-model currency (F19–F22). **M2-B:** canonical schema v2 plus UI-v0/core-v1 migrations | R0 | M2-A content review plus verification at the next real release cut; RFC-076 acceptance complete | level decided at release time |
| 2 | M3 — Mergetool target safety | Separate remote input/output identity and explicit match/absence preconditions | M1 (hard); sequenced after M2 | RFC-077 acceptance complete | shipped in 0.166.0 |
| 3 | M4 — Integrated stabilization | **M4-A:** residual correctness. **M4-B:** gate integrity. **M4-C:** truth reconciliation, advisory dispositions, frozen `matrix-plan.md` | M2, M3 | Gate C — release-core candidate approved for QA | level decided at cut time |
| 4 | M5 — Platform acceptance | Linux Wayland/X11, Windows, macOS runtime matrix | M4 | Gate D — RFC-078 evidence complete | candidate re-cuts as needed |
| 5 | M6 — Handoff and go/no-go | Refresh handoff and independent architecture review | M5 | Gate E — explicit v1 Go or continued No-Go | — |

M3's only hard dependency is M1; it is sequenced after M2 because a single
developer owns the overlapping UI state files. If ownership ever separates,
M2 and M3 may run concurrently without changing any gate.

**Progress (2026-08-01):** M0, M1, and R0 are complete. RFC-075 guards
asynchronous compare completion with stable tab IDs and per-load generations,
resolving audit finding B1.

R0 released `0.165.0`. It reconciled the version and CHANGELOG with the
26-commit delta that had accumulated past the published `0.164.0` tag, repaired
the release-workflow trigger, added a published-tag check to
`cargo xtask version-sync`, and corrected five documentation-truth defects. The
first real release-workflow run surfaced F17, a Windows build failure in the
`app-json-settings` dependency that no amount of configuration review would
have found; it was reported upstream, fixed in 2.4.1, and re-verified before
the tag. All six release jobs are green and the four artifacts are digest-
recorded. Reviewed and conditionally approved with six documentation-currency
follow-ups carried into `0.166.0`.

`0.165.0` is published (2026-08-02). The working tree is at `0.165.1` — the
post-release patch default, not a claim that the next release is a patch. M2's
content will decide its level; RFC-076's persistence schema change is expected
to promote it to `0.166.0` at release time. The N1–N6 documentation-currency
follow-ups from review 032 (registered as F19–F21) ride with M2 rather than
shipping separately.

**Progress (2026-08-04):** RFC-076 is implemented, resolving audit finding B2.
The running app now reads and writes settings/session exclusively through
core's versioned schema-v2 repositories; legacy UI-v0 files migrate with a
durable backup; future-version and corrupt files are preserved untouched and
reported via a blocking recovery dialog (Exit/Continue/Reset) rather than
silently collapsed to defaults. M2-B's exit criterion is met.

M2-A's **content** is complete and approved (`896f2c6`, C1 fix `fe9940e`,
review 034): F19–F22 are resolved. What keeps M2 open is the rest of its exit
gate — F23 (`actionlint`), which must land before the cut, and the gate's
requirement that the release mechanics be *verified at a real release cut*,
which has not happened since `0.165.0`.

RFC-078 host access for Linux, Windows, and macOS is confirmed available, so M5
is schedulable once M4 completes. M2–M6 remain outstanding and R0 closed no
audit blocker, so v1/public release remains **No-Go**.

**Progress (2026-08-08): M3 is complete, resolving audit finding B3.** RFC-077
is in `rfcs/done/` with its implementation outcome recorded; the compared right
input and the save destination are now distinct typed values, mergetool
preparation fingerprints the actual merged target, and a target expected to be
absent is committed with no-clobber semantics. F38, the only register entry
tagged against M3, is resolved.

**Release column correction.** M3's row said `0.167.0` and M4's said
`0.168.0 candidate`. Both pre-committed a version level before the content
existed — the mechanical rule `release.md` removed at F21, reappearing in this
table. M3 in fact shipped inside `0.166.0`, and later levels are decided at the
cut. The column now says so rather than predicting.

M3 closed **out of table order**, before M2. This is permitted — M3's only hard
dependency is M1, and the "sequenced after M2" note is a single-developer
resource constraint rather than a gate — but the recorded sequence and the tree
have diverged, so the table above describes the plan, not what happened.

**M4 remains blocked**, since it depends on M2 as well as M3. M2's remaining
work is **F23** and then **the release cut itself**, which its exit gate
requires as verification. So the critical path is F23 → cut → M2 closes → M4,
and M3's `0.167.0` release column is contingent on that cut landing first.

**F23 and F41 are resolved** (`0573de5`, review 053). `actionlint` now runs on
every push and pull request, discovering workflow files from the directory
rather than a list, and the file-permission tests additionally run under
`umask 077` so a regression to a hardcoded mode is observable. Both were
verified by mutation in review 053, including the demonstration the implementer
could not run in their sandbox, and the pinned `actionlint` checksum was
confirmed against the published artifact.

**Progress (2026-08-08): `0.166.0` is cut.** Promoted from the post-release
patch default to a minor level per `release.md`'s content-driven rule: RFC-076's
persistence schema change and RFC-077's save-target behaviour are both
user-visible. The full pre-release checklist passed, including MSRV 1.91 and the
source-archive layout contract; `version-sync` caught a fourth version carrier
(`xtask/Cargo.toml`) that the other three bumps had missed.

**M2 closes with this cut** — the verification its exit gate required. With M3
already closed, **M4 is now unblocked**: Gate C, advisory dispositions, the
frozen `matrix-plan.md`, and the accumulated register.

The release is **tagged and drafted, not published**. Per `release.md`,
publication out of draft is an explicit owner action and is the point after
which the version is immutable.

**Progress (2026-08-11): M4-A is complete.** F40 (diff-option toggles silently
discarded applied merges and the undo stack), F35 (blank counterpart rows
announced a bare "Changed") and F10 (VCS discovery tests assumed the OS temp
directory sits outside a repository) are resolved; F8 was investigated and found
not to be a defect, and now points at RFC-074's advisory N1, which tracks the
same thing.

F40 shipped ask-first rather than preserve-and-reapply, for a reason worth
keeping: `HunkId` is not a stable identifier across recomputes at all — `diff_id`
comes from a process-global counter incremented on every `compute_diff` — so
preserving history needs stable identity or a rebasing rule, registered as F47
for post-v1. RFC-015 §8 rule 4 is recorded **Not met** rather than left
asserting something the code does not do.

**Progress (2026-08-11): M4-B is complete.** F42 (actionlint/shellcheck and
the F41 filter can no longer silently degrade), F24 (the CHANGELOG guard
moved to preflight), F18 (`xtask` no longer escapes `cargo fmt --check`), F6
(`clippy --all-targets` run for the first time since M1, 12 mechanical
findings, gate added), F36 (decided a lightweight `Store` test harness is
worth it, adopted `with_test_store`, proved it against F40's guard directly)
and F34 (a rendering geometry check that independently rediscovered F32's
own root-cause diagnosis when the historical defect was reintroduced) are all
resolved. Every item was demonstrated failing on a deliberately broken input
before being accepted as working, per review 053/054's standard. One open
limitation: F34's Xvfb/AT-SPI CI wiring in `release.yml` could not be
dry-run outside GitHub Actions and is unverified until a real tag-triggered
release run.

**Progress (2026-08-11): M4-C1 is structurally complete.** F7/N5
(`advisories.md` — both unsoundness advisories dispositioned individually,
twelve `unmaintained` advisories under one policy statement, two suppressed
advisories restated in the same form), F9/N2 (durability wording narrowed
everywhere it appeared — `save.rs`, README, features/merging/architecture/
threat-model docs — to state visibility-atomicity without implying
power-loss durability, which nothing in `forskscope-core` provides), and F37
(RFC-078's P08 amended to require Exit/Continue/Reset on every platform row,
not narrowed by a row's otherwise-lower required level) are resolved.
`matrix-plan.md`'s case-to-row mapping (P01–P12 × five platform rows) is
decided and justified, with F44/F45/F46 folded in as explicit P01
sub-requirements — but the plan is **not yet frozen**: exact OS versions,
executor owner/role, and host-access status per row are owner-dependent and
recorded as open questions rather than guessed. M5 cannot begin until those
are answered.

### M4 slicing

M4 opens carrying **20 open register items** — three times M2's load, and too
many to review as one change. It is split by what each item is *for*, so a slice
can be reviewed under the attention its content needs:

| Slice | Purpose | Items |
|---|---|---|
| **M4-A** | Residual correctness — real defects in shipped behaviour | F40, F8, F35, F10 |
| **M4-B** | Gate integrity — make each gate measure what it is credited with | F6, F18, F24, F34, F36, F42 |
| **M4-C1** | Advisory dispositions and the platform-matrix freeze — the Gate C and M5 prerequisites | F7 (N5), F9 (N2), F37, `matrix-plan.md` |
| **M4-C2** | Documentation and code truth | F11, F12, F16, F25/F25b, F31, F39, F43, F48 |

**M4-A leads because F40 is the most user-harmful item in the register** — a
diff-option toggle silently discards every applied merge *and* the undo history,
then clears the dirty flag so the close guard stops warning about the work it
just destroyed. Nothing else open costs a user their work.

**M4-B is the theme of this whole program.** The single most repeated finding
here is a green gate credited with more than it measures — `version-sync` blind
to published tags, `css_coverage` blind to layout, a release workflow that had
never fired, `i18n` blind to strings bypassing `t()`, and a permission test that
cannot fail under CI's umask. M4-B is where that stops being a pattern.

**M4-C is last** because freezing `matrix-plan.md` and disposing of advisories
both require knowing what is actually true, which A and B settle.

Architect-owned in parallel, not blocking the dev team: **F33** (README
installation path and screenshots, unblocked now that F32 is resolved — and
`0.166.0` publishes four artifacts while the docs still say "build from source")
and a recommendation on **F43**, which is an owner decision rather than an
implementation task.

### R0 rationale

`0.164.0` is a published, immutable tag. The working tree has advanced well
beyond it — including an MSRV change, the fail-closed XLSX decision, the
Dioxus dependency-path constraint, release-archive and CI gate alignment, and
the RFC-075 correctness fix — while `Cargo.toml` still declares `0.164.0`.

A source build therefore reports a version whose published artifact behaves
differently, and `PlatformInfo` propagates that version into `--diagnostics`
and the About panel, so defect reports would be misattributed. `cargo xtask
version-sync` does not detect this because it compares the workspace version
against PKGBUILD and the MSIX manifest, not against published tags.

R0 closes that gap before any further production change lands, and pulls the
version/CHANGELOG reconciliation and the documentation-truth subset forward
out of M4 so M4 is not a single large batch.

R0 also repairs the release trigger. `.github/workflows/release.yml` fires on
`v[0-9]+.[0-9]+.[0-9]+`, but this project tags without a `v` prefix and none of
its published tags match that pattern. The workflow's gates and artifact jobs
were aligned after the last tag was pushed, so the trigger has never been
exercised. Until it is corrected, tagging a release runs no gates, builds no
artifacts, and publishes nothing — so R0's own release evidence would not
exist. The correction follows the documented tag rule rather than changing the
convention.

### Workstream dependencies

```text
RFC-074 program
└── RFC-075 async identity ......... done (M1)
      └── R0 release baseline ...... 0.165.0
            ├── RFC-076 runtime persistence ──┐
            └── RFC-077 mergetool target ─────┤
                      requires RFC-075        │
                                              └── integrated Gate C
                                                    └── RFC-078 platform matrix
                                                          └── refreshed handoff
                                                                └── architect Gate E
```

RFC-075 is complete, so the remaining single-developer sequence is
R0 → RFC-076 → RFC-077 → integrated gates → RFC-078 → refreshed handoff.

### Release cycle

The program releases one unit per resolved workstream rather than batching to
a single pre-v1 cut. This matches the project's logical-breaking-point rule and
keeps the CHANGELOG, version metadata, and packaging inputs continuously true.

| Element | Policy |
|---|---|
| Release unit | One release per resolved workstream — an RFC disposition or a completed hardening theme |
| Numbering | `docs/src/maintainers/release.md` is authoritative and content-driven: `PATCH` for bug fixes and documentation updates within a stable feature set, `MINOR` for new user-visible features or significant internal changes |
| Post-release default | The commit after a release bumps to the next **patch** level. That satisfies the version invariant while claiming nothing about content |
| Promotion | At release time, the accumulated content decides the level. Promoting patch → minor is a rename across the six enforced locations plus the CHANGELOG heading, confirmed by the owner with the content visible |
| Trigger | Workstream gate passes → set the release version → CHANGELOG entry → source tarball delivered to the owner |
| Version invariant | The workspace version must never equal an existing tag on any commit other than that tag's own |
| Approval | Every release and its version level are confirmed by the project owner; no release is automatic |
| v1.0.0 | Reserved. Gate E yields a Go/No-Go verdict only. Whether and when a Go becomes 1.0.0 is the project owner's decision alone |

The version invariant is enforced by `cargo xtask version-sync`'s published-tag
check, added in R0.

The post-release default and promotion rules exist because the level cannot be
known before the content is. An earlier revision of this table specified
`0.MINOR.0` unconditionally, which contradicted `release.md` and caused R0's
post-release bump to pre-commit the next release to a minor level before its
scope existed. Defaulting to patch keeps the number mechanical and the level a
decision made from evidence.

### Release-blocking outcomes

- Obsolete async completions cannot mutate another/newer load.
- Production settings/session files use core-owned versioned schemas and
  migrate current plain JSON and existing core-v1 envelopes without silent
  loss or schema reinterpretation.
- Git mergetool fingerprints and guards the actual merged output, including
  no-clobber creation when the target was initially absent.
- Exact release artifacts pass the retained runtime/platform matrix.
- A refreshed handoff receives an independent architecture Go verdict.

Until all five outcomes are evidenced, v1 remains No-Go.

### Fix and improvement register

Tracked non-blocking work, each assigned to the milestone that owns it. Items
sourced from the 2026-07-15 architecture audit keep their audit finding ID.

| ID | Item | Source | Milestone |
|----|------|--------|-----------|
| F1 | `docs/src/maintainers/testing.md` test counts stale (930/228 versus observed 943/241) | 2026-08-01 review | R0 |
| F2 | `docs/src/maintainers/architecture.md` documents a shim re-export layer that no longer exists | 2026-08-01 review / audit N3 | R0 |
| F3 | `docs/src/maintainers/threat-model.md` settings-persistence section claims no residual concerns, contradicting audit B2 | 2026-08-01 review | R0 |
| F4 | Workspace version and CHANGELOG behind the published `0.164.0` tag | 2026-08-01 review | R0 |
| F5 | `cargo xtask version-sync` cannot detect workspace version equal to a published tag | 2026-08-01 review | R0 |
| F6 | **Resolved.** All 12 `--all-targets` findings were mechanical (chose outcome 1 for every one, no suppressions): two `manual_contains` rewrites, one owned-value comparison (`PathBuf::from(rel)` → `*rel`), a `type` alias for a test fixture's signature (`search_index.rs`'s `index_for`), and four threshold assertions moved into `const { assert!(..) } }` blocks (checked at compile time now, still named `#[test]`s). `ci.yml` adds `cargo clippy --workspace --all-targets -- -D warnings` alongside the mandatory gate. Demonstrated: reintroduced one `manual_contains` finding, the mandatory gate stayed green (exit 0) while the new gate caught it (`error: ... could not compile`) — exactly the blind spot RFC-074 N6 named; reverted after | audit N6 | M4 |
| F7 | **Resolved.** `docs/src/maintainers/release-evidence/0.167.0-rc1/advisories.md` (directory name a placeholder pending the owner's actual version/RC identifier). Both unsoundness advisories dispositioned individually with a reachability statement, owner, review date, and upgrade trigger: `glib` 0.18.5 (`RUSTSEC-2024-0429`, `VariantStrIter`'s only constructor never called anywhere in the resolved dependency source, confirmed by grepping every cached crate version in the chain) and `rand` 0.7.3 (`RUSTSEC-2026-0097`, a `[build-dependencies]`-only path through `phf_generator`, never linked into the shipped binary, with the unsound preconditions also unmet at build time). The twelve `unmaintained` advisories get one recorded policy (N5: "keep policy-pass distinct from a clean advisory set") rather than twelve essays, enumerated by crate. The two suppressed advisories restated from `.cargo/audit.toml`'s code-comment rationale into the same four-field form, cross-referenced against `cargo xtask audit-deps`'s enforcement | audit N5 | M4 |
| F8 | **Clarified, not a defect** (review 054). `SaveOutcome.new_fingerprint` *is* consumed — `diff_actions.rs:309` stores it as the tab's next `TargetExpectation::MustMatch`. What is unused is the `digest` field *inside* `FileFingerprint`: `check_external_state` compares only `len` and `modified_unix_nanos`, so a same-size, same-mtime external edit is not detected. **This is RFC-074's advisory N1, which is the authority for it** — "use the digest when metadata is inconclusive, or document the same-size/same-mtime limitation" — tracked at M4-C with the other dispositions. This entry carries no independent remediation; two trackers for one item is how something gets closed twice or not at all | audit N1 | see RFC-074 N1 (M4-C) |
| F9 | **Resolved.** Narrowed the wording (option 1 of N2's two — option 2, implementing `fsync`, is explicitly out of scope for this slice and would be a product change with per-platform evidence of its own). Confirmed no `fsync`/`sync_all`/`sync_data` anywhere in `forskscope-core`. Every "atomic" claim found (`save.rs`'s module doc and `atomic_replace`'s doc comment, `README.md`, `docs/src/users/{features,merging}.md`, `docs/src/maintainers/{architecture,threat-model}.md`) now states the actual guarantee explicitly: visibility-atomic (a concurrent reader never sees a partial write), not power-loss durability. The full statement of what this does and does not cover lives in `docs/src/users/merging.md` §"Saving the result", cross-referenced from the shorter mentions. Also fixed, while there: `architecture.md`'s `save` module row named `AtomicSaveStrategy`, a type that doesn't exist in the code (replaced with the real `TargetPrecondition`) — a latent documentation-truth defect this sweep surfaced, not something this patch introduced | audit N2 | M4/M5 |
| F10 | **Resolved.** The two "outside any repo" tests (`vcs_tests.rs`) now check an independent precondition — `ancestor_has_git`, a separate ancestor-walk from `find_git_root`/`detect()` itself, so a genuine `detect()` bug and a contaminated environment can't be conflated into the same silent skip — and print a loud `eprintln!` skip rather than assert if the OS temp directory's own ancestry already contains a `.git`. Same shape as `save_target_tests.rs`'s `0o000`-restriction precedent (verify the assumed condition actually holds before trusting the assertion). `ancestor_has_git` itself is directly unit-tested (direct `.git`, several levels up, and a clean tree) rather than only exercised indirectly. Not verified end-to-end against a live `TMPDIR`-inside-a-repo run — that attempt was declined in this session; the direct unit tests are the evidence | audit N7 | M4 |
| F11 | RFC-058 status does not record the fail-closed security suspension | audit N4 | M4 |
| F12 | RFC-062 is fully shipped but still filed under `proposed/` | RFC-074 | M4 |
| F13 | Source files above the 300-ELOC soft threshold; `xtask/src/main.rs` is the largest | audit N6 | opportunistic, when touched |
| F14 | Release workflow triggers on `v`-prefixed tags that this project never creates, so it has never run | 2026-08-01 review of 001 | R0 |
| F15 | `README.md` describes the three-way conflict workspace UI as "in progress" although it is a deferred post-v1 slice and an explicit RFC-074 non-goal | 2026-08-01 review of 001 / review 001 finding B | R0 |
| F16 | Public feature claims are not systematically audited for core-complete versus user-reachable status | review 001 finding B | M4 |
| F18 | **Resolved.** `ci.yml` adds `cargo fmt --manifest-path xtask/Cargo.toml --check` alongside the workspace `cargo fmt --check` — xtask keeps its own separate `[workspace]` (DEC-005: must build standalone even if the main workspace's `Cargo.toml` is broken), so the bare workspace-scoped check never saw it. Ran `cargo fmt` on `xtask/src/main.rs` to clear existing drift before adding the gate (folded into F24's commit, which already touched the file). Demonstrated failing: reintroduced a single misindented line, `cargo fmt --manifest-path xtask/Cargo.toml --check` reported the diff and exited 1; reverted, re-verified clean. `xtask/src/main.rs` is now 708 lines (up from 577 pre-M4-B, mostly F24's CHANGELOG-emptiness check and its 6 new tests) — still above F13's 300-line soft threshold, which F13 already tracks separately and this slice does not address | review 032 / R0 review question 3 | M4 |
| F19 | **Resolved** (`896f2c6`, review 034). `docs/src/maintainers/release.md` had no re-release or immutability policy, although that policy governed R0's tag re-cut; it exists only in the superseded v0.164.0 handoff bundle | review 032 (N2) | M2 |
| F20 | **Resolved** (`896f2c6`, review 034). Threat-model audit history omitted the RFC-075 integrity fix and retains the superseded v0.148.0 stale-tab-guard claim; section heading still reads v0.164.0 | review 032 (N3, N4) | M2 |
| F21 | **Resolved** (`896f2c6`, review 034). `release.md` now records the corrected release-cycle rules: post-release patch default, promotion at release time, and the definition of "published" as a release out of draft state — aligned with the `version-sync` check, which keys on tag existence | 2026-08-02 owner decision | M2 |
| F22 | **Resolved** (`896f2c6`, C1 fix `fe9940e`, review 034). Release notes were produced by `generate_release_notes: true`, which summarises pull requests; this project commits directly to `main`, so it emits only a compare link and ignores the CHANGELOG. Compose notes in CI from the tag's CHANGELOG section, failing closed when absent, and document the publish step as an explicit owner action | 2026-08-02 owner question | M2 |
| F23 | **Resolved.** `ci.yml` now installs a pinned, checksum-verified `actionlint` (v1.7.12, sha256 recorded in the workflow) and runs it with no arguments — it discovers every file under `.github/workflows/` itself, so a third workflow needs no CI edit to be covered. Runs before the system-dependency install and Rust toolchain setup so a bad workflow file is reported in seconds. Local falsifiability evidence (a deliberate syntax error shown failing) could not be produced: downloading and executing the binary was blocked by this session's sandbox even after explicit approval, so the check's first real execution is its first CI run — see the review request for what was verified instead (manual `shellcheck` pass over every `run:` block, both new and pre-existing, all clean) | review 033 (N1) | M2 |
| F24 | The empty-CHANGELOG-section guard fires in the release workflow's last job, after the source archive and all three platform builds — detectable at preflight from the repository alone, and by then the tag exists so recovery needs a re-cut. Extend `version-sync`'s **release mode only** to require non-whitespace content; dev mode must keep accepting the empty section the post-release bump opens | review 034 (N1) | M4 |
| F25 | Two divergent "built-in" compare-profile sets: the UI's four (now canonical for persisted schema v2) and core's `CompareProfile::all_presets()`, which no UI reaches and v2 never produces, yet which `is_core_preset_name` still consults. Pre-existing; convergence is the point to resolve or explicitly document it as legacy | review 035 (N3) | M4 |
| F25b | **Corrects F25's text.** Core's preset set is *not* unreached: `ui-logic::settings_view::profile_presets()` consumes `CompareProfile::all_presets()` and is re-exported from `ui-logic/src/lib.rs`. What keeps the divergence invisible today is only that no `forskscope-ui` file calls it. Patch 5 removed `is_core_preset_name` — the consultation — but both sets remain and still differ, so wiring the picker in any later settings work would show four preset names unrelated to the four built-in profiles actually persisted | review 043 | M4, with F25 |
| F27 | **Resolved.** Six `#[serde(default)]` fields carried a golden-fixture value identical to their default (`show_line_numbers`, `wrap_long_lines`, `remember_explorer_dirs`, `restore_session`, `enable_binary_comparison`, `recent_limit`), so a serde rename was ignored and the field fell back to a value the test already expected. Flipped all six to non-default values in both `settings-v2.json` and the golden-fixture test's expected struct, restoring per-field wire coverage for the whole payload. | review 040 | RFC-076 pre-patch-4 |
| F29 | After patch 4 the data model carries three settings shapes and two session shapes where RFC-076 asks for one canonical model per document. `UserSettings` and `WorkspaceSession` (with `TabId`, `SessionId`, `WorkspaceRoot` and companions) stay public in core with no runtime consumer, serving only a v1 migration path for a format no released version ever wrote. Remove them with the RFC-031 envelope types | review of patch 4 data model, 2026-08-03 | RFC-076 patch 5 (convergence) |
| F30 | `persist::v2` and the `*V2` type names encode a schema version that will go stale: six of nine files in that module are version-agnostic machinery, so the name is already wrong for most of its contents, and at v3 the only options are to let it lie or to churn. Rename to `persist::schema` with unsuffixed types; keep version names where they are true (v0 DTOs, `SCHEMA_VERSION`, `.pre-v2.bak`) | owner question, 2026-08-03 | RFC-076 patch 5 (convergence) |
| F31 | **Resolved (M4-C2): batch manifests and reports stay explicitly unversioned — no schema envelope.** Both `dir::batch::BatchManifest::to_json` and `report::{file,dir}`'s `to_json` are write-only exports: `restore_from_manifest` restores from the in-memory `BatchManifest` the batch just produced, and `FileComparisonReport`/`DirComparisonReport` are built from live diff data (`from_diff`/`from_entries`) — nothing anywhere parses a written manifest or report JSON file back into this app. A schema version protects a read path; there isn't one here, unlike settings/session (RFC-076), which the app genuinely reloads across upgrades. `BatchManifest` already carries `app_version` (the version that wrote it), enough for a human or future tool to tell an old file's shape apart from a current one without a dedicated field. Documented in `dir::batch`'s and `report`'s module docs, with `persist.rs`'s existing gap note updated to name the decision rather than just the gap; if a future feature ever reads historical manifests/reports back in, it adds its own tolerant parsing at that time | review of patch 4 data model, 2026-08-03 | M4-C2 |
| F28 | `write_disabled` is true for `Migrated(Failed)`, `Incompatible`, and `CorruptPreserved`, but only the first tells the user their changes will not be saved. A user continuing past a future-version or corrupt file gets a working app whose settings changes silently do not persist. Extend the other two dialog bodies to state the same consequence | review 040 (answers review-039 N1's naming question) | RFC-076 patch 6 (recovery UI) |
| F32 | **Resolved (cb6a852).** On WebKitGTK, every changed line (Delete/Replace) in the compare view renders shifted one column right with its content clipped off the pane — the product's core view is unreadable for exactly the lines it exists to show. Cause: `hunk.rs` emits an `.sr-only` span as the first child of a `display: table-row`, and WebKitGTK wraps it in an anonymous table cell, adding a column to only those rows (`sr_label` is `Some` only for Delete/Replace). Introduced by the 0.164.0 table-layout change (`3c01e4d`) and present in the published 0.165.0. Confirmed by mutation: removing the span aligns every row. Fix must preserve the screen-reader label (G-007/RFC-024) — move it inside `.cell`, do not delete it | 2026-08-03 screenshot capture; RISK-002 / RFC-078 P03 materialised | before the next release cut |
| F33 | **Resolved** (review pending). Added `docs/src/users/installation.md` (Linux-first, with Arch/AUR, Microsoft Store, zip, and DMG paths), a README **Install** section replacing "Build from source", a `SUMMARY.md` entry, and two screenshots taken from a clean `0.166.1` build against the real dependency set — a side-by-side diff and the two-pane explorer. Both use a synthetic fixture project rather than a real home directory, so nothing is blurred or redacted. The Linux section states the F44 libxdo limitation plainly and points to building from source; macOS states the build is unsigned and unnotarized with the quarantine workaround; Windows names WebView2 and the VC++ redistributable. `mdbook build docs` clean | 2026-08-04 owner request | done |
| F34 | **Resolved.** Built the full geometry check, not the fallback launch-smoke test. `packaging/render_check.py` runs in `release.yml`'s `linux` job against the actual just-built binary, under `xvfb-run` + `dbus-run-session` (a real virtual display and AT-SPI bus, not a mock) — drives `tests/fixtures/text/{left,right}_all_hunk_kinds.txt` (one fixture, Replace + Delete + Insert together, per review 044; pinned by a new `diff_corpus.rs` test so a future corpus change can't reshape it unnoticed) and asserts every diff row's on-screen geometry (AT-SPI `Component.get_extents`, plus accessible child count) matches its neighbours in the same pane. Chose geometry over a DOM-structure or image-diff assertion: it validates the actual visual outcome WebKitGTK produces, not one specific historical markup pattern, so it would also catch a differently-caused future column shift. Demonstrated failing for real: reintroduced F32's exact defect (moved the `sr-only` span back outside `.cell`), ran the script against the rebuilt binary, got `"a row has 3 accessible children, other rows have 2"` — the check independently rediscovered F32's own diagnosis; reverted, re-verified passing. The Xvfb/AT-SPI-bus CI wiring itself could not be dry-run outside GitHub Actions (no Xvfb in this dev sandbox) — the detection logic is proven, the CI plumbing around it is not, and may need a fast-follow if the first real run surfaces friction | review of F32, 2026-08-03; shape from review 044 | M4 |
| F36 | **Resolved — yes, a lightweight harness is worth it.** Prototyped and adopted `state::with_test_store` (`#[cfg(test)]`): a headless `dioxus_core::VirtualDom` (no renderer, no WebView, no GTK) runs a trivial root component that constructs a real `Store`, captures it via a `thread_local!`, and hands it to the caller's closure — fully readable/writable outside the triggering render as long as the `VirtualDom` stays alive. Proven against a real historical gap: `change_diff_options_defers_to_confirmation_when_the_tab_is_dirty`/`_applies_immediately_when_the_tab_is_clean` (`state/tab/tests.rs`) now test F40's guard directly instead of only through AT-SPI. Documented as policy in `docs/src/maintainers/local-dev.md` §"Testing `Store`-dependent UI logic" alongside the pure-predicate-extraction pattern (F35/F40), which stays the *default* — reach for `with_test_store` only when the logic genuinely needs `Store` state, not for anything a plain function could express. Does not cover rendering/event-dispatch/visual correctness (F34's territory). The other four historically-named occurrences (RFC-076 patch 4 startup wiring, its C1 CLI-mode fix, patch 6's recovery queue, RFC-077's Save As default-path regression) are **not** retroactively converted here — this item was a decision, not a retrofit; each stays on AT-SPI evidence unless a future patch touches it | review 045 | M4 |
| F37 | **Resolved.** RFC-078's P08 amended (2026-08-11) to require all three `RecoveryDialogAction` choices — Exit, Continue (either variant), Reset — on every platform row, not narrowed by a row's otherwise-lower "Required level." Exit named explicitly as the one that matters most: it terminates the process from inside a modal during startup, a WebView-hosted-event-loop path that can behave inconsistently across WebKitGTK/WebView2/WKWebView, and Linux-only evidence says nothing about the other two. A row is not P08-complete until all three are observed leaving the process in the expected state. No matrix execution here — that is M5's work; this slice only amends the case definition | review 045 | M4, for M5 |
| F38 | **Resolved.** `persist_noclobber` requested `tempfile::Builder::permissions(0o666)` before creating the same-directory temp file, replacing the hardcoded `0o644` `set_permissions` call that was correct only under `umask 022`. The kernel applies the process umask to the requested mode the same way it does for `atomic_replace`'s `fs::write`, so no umask query/reset is needed. The permissions test now asserts equality against a same-directory `fs::write`-created reference file's own mode rather than the literal `0o644`, verified locally under both the default umask and `umask 077`. `persist_noclobber` runs only for `MustBeAbsent`, so there is never an existing mode to preserve — that option was inapplicable by construction; F9's overwrite-mode-loss case is adjacent but distinct | review 048, direction review 051 §3.3 | before M3 closes |
| F39 | **Resolved (M4-C2): narrowed the gate's documented scope rather than translating the five bypass sites.** All five (`diff_actions.rs`'s `handle_result` and `describe_block`, `recovery.rs` x2, `state/compare.rs`) route to a toast via `CoreError`'s `Display` output — for `Io`/`Decode`/`Unsupported`/`InternalInvariant`, that text is OS/dependency-generated at the moment of the error, not authored copy, so there is no fixed string to translate. `describe_block`'s two literal arms (`Binary`/`Spreadsheet`) are the one exception but were left matching the established `e.to_string()` precedent rather than split silently — partial translation without a working detector for future bypasses recreates the same gap. `docs/src/maintainers/testing.md` and `run_i18n_audit`'s doc comment now state precisely what `cargo xtask i18n` checks (call sites that already reach `t()`) and name the exempt classes and why, in place of an unqualified "zero gaps" claim. Found while investigating: a structured, translatable error taxonomy already exists for exactly this problem (`AppError`/`SaveErrorView`, RFC-017) but `forskscope-ui` never wires it in — registered separately as F52; a real fix here is routing through that, not wrapping the passthrough text in `t()` | review 049 | M4-C2 |
| F52 | **RFC-017's structured error taxonomy (`AppError`/`AppErrorKind`/`SaveErrorView`) is implemented, tested, and completely unused by `forskscope-ui`.** `grep` for `SaveErrorView`, `AppError`, or `AppErrorKind` across `crates/forskscope-ui/src/` returns nothing; the only `CoreError` awareness in the UI is a single `Err(CoreError::Conflict { .. })` match arm routing to a confirm-overwrite modal. Every other error path calls `.to_string()` on the raw `CoreError` and hands it straight to `store.notify(...)`, bypassing both the recovery-action button set `SaveErrorView` was built to produce and the `t()` translation layer (F39). RFC-017's own status line already says "diagnostics panel UI, copy-diagnostics, error toast component deferred to UI layer," so this isn't a false claim the way RFC-024's was (F48) — but "deferred" undersells it: the save-error *dialog* path (not just a toast/diagnostics-panel nicety) has zero callers. Same shape as F48 (RFC-024) and F51 (RFC-034): a well-designed, tested contract layer that never reached the renderer. Not fixed here — out of scope for F39's decision, which was about the gate's wording, not wiring the taxonomy in | found while resolving F39, 2026-08-13 | M4-C, next slice |
| F50 | **`RUSTSEC-2026-0257` — a new, real (not informational) vulnerability in `webbrowser` 1.2.1: "Unix `BROWSER` handling allows browser argument injection."** `webbrowser` is a direct dependency of `dioxus-desktop`, one hop from `forskscope-ui` (`cargo tree -i webbrowser --target all`). This is not one of the 14 previously-dispositioned advisories in `advisories.md` (all `unmaintained`/`unsound`, informational) — it is a genuine `cargo audit` failure (exit 1, not just a counted warning), appeared between two CI runs on `main` minutes apart with no dependency change in either commit (RustSec's advisory database grew from 1207 to 1216 entries in that window), and is currently failing `main`'s CI for everyone at the "Security advisory audit" step. A fix is available: upgrade to `webbrowser >= 1.2.2`. Not fixed here — every M4 handoff this program has run under explicitly forbids dependency changes in-slice ("a disposition records the decision; changing the graph is separate work with its own review"), and this needs its own reviewed unit of work, not a silent bump riding along with an unrelated slice | discovered mid-M4-C2, CI run `31673380473` | before any further `main` push can go green |
| F49 | **Platform version claims diverge across four sources, and RFC-078 preserves a conflict whose other half no longer exists.** macOS: `build-dmg.sh` writes `LSMinimumSystemVersion 13.0`; the built Mach-O declares `minos 11.0` (no `MACOSX_DEPLOYMENT_TARGET` is set anywhere, so it is whatever the runner's SDK yields); RFC-078 §277 asks to "resolve the documentation conflict between macOS 12 and `LSMinimumSystemVersion` 13.0" — but **no macOS 12 claim exists anywhere in the repo**, so the RFC is preserving a stale half of a conflict while omitting the live one (11.0). Windows: `AppxManifest.xml` sets `MinVersion=10.0.17763.0` (Windows 10 **1809**) and `MaxVersionTested=10.0.19041.0` (Windows 10 2004, predating Windows 11 entirely), while RFC-078 §109 requires a "Windows 10 **1903**+" row and a full Windows 11 row, and `installation.md` says "Windows 10 or later". Four Windows minimums, three macOS minimums, none reconciled. Also: `release.yml` pins `macos-14` while the owner's stated target is `macos-latest` — switching runners changes the SDK and therefore `minos`, so it must be done together with an explicit deployment target or it swaps one undefined number for another | review 057, owner challenge on RFC-078 | M4-C2, before the matrix freezes |
| F48 | **Resolved (M4-C2): deleted both layers, no product justification found for wiring them through instead.** `hunk.rs`'s live rendering was already a separate, simpler, hunk-level (not per-row) contract (`.hunk-del`/`.hunk-ins`/`.hunk-rep`, `.pane-gutter`, `.in-del`/`.in-ins` in `11-view-diff.css`) that doesn't distinguish Conflict/MergeApplied — wiring RFC-024's richer per-row `fs-line-*` contract through would have been a real visual change, not a docs fix, and nothing named a reason the app needs it. Deleted: the `.fs-line-*`/`.fs-inline-*` block from `30-contract-diff-decorations.css` (kept the RFC-034 `.fsk-conflict-*` block in the same file, out of scope here), `forskscope-ui-logic::compare::hunk_decorations` entirely (its only consumer was its own tests), and the two now-inapplicable `css_coverage.rs` tests asserting those classes existed in `main.css`. Kept `forskscope-core::diff_decoration` (`DiffDecorationSet`/`LineDecorationKind`/`InlineDecorationKind`) untouched — self-contained, tested independently of any renderer, and RFC-024's actual "core complete" deliverable; a future renderer change wanting the richer contract can still build on it. RFC-024's status corrected to state the renderer-wiring acceptance criterion is not met and is not expected to be under the current renderer design, rather than the previous, now-inaccurate "deferred to UI layer." Found in passing while resolving this: RFC-034's ConflictNavigator (`fsk-conflict-*`) has the identical unwired-to-DOM shape — registered separately as F51, out of scope for this decision | review 055, found while probing F34's geometry branch | M4-C2 |
| F51 | **RFC-034's ConflictNavigator CSS class contract (`fsk-conflict-*`) has the same unwired-to-DOM shape F48 found and fixed for RFC-024.** `conflict_nav.rs` (core) and `conflict_nav_view.rs` (ui-logic) produce `fsk-conflict-unresolved/-left/-right/-both/-manual/-ignored`, and `30-contract-diff-decorations.css` still styles them, but `crates/forskscope-ui/src/` has no reference to `conflict_nav`, `ConflictNav`, or any `fsk-conflict-` string at all — nothing in the actual Dioxus component tree renders a conflict rail. `css_coverage.rs`'s `conflict_navigator_css_classes_defined_in_main_css` test passes throughout for the same reason F48's did: it checks the class strings are defined in `main.css`, not that anything emits them into the DOM. Not fixed here — out of scope for F48's decision, which named only the RFC-024 chain; this needs its own wire-through-or-delete decision the way F48 got | found while resolving F48, 2026-08-13 | M4-C, next slice |
| F47 | RFC-015 §8 rule 4 ("Recomputing diff after an edit must not erase undo history") is recorded **Not met** as of F40, because `HunkId` is not a stable identifier across recomputes: `diff_id` comes from a process-global `AtomicU64` incremented on **every** `compute_diff` (`engine.rs:20,132`), so every hunk gets a new identity on any recompute — even one that changes nothing. Preserving history therefore needs stable hunk identity or a content/position-based rebasing rule for the transaction log, which is a design, not a patch. F40 shipped ask-first instead, which is safe but not what the rule claims. Sits with RFC-015's other open items (history panel UI, crash-recovery journal) | review 054, from F40 | post-v1 |
| F46 | **The macOS artifact is neither Developer ID-signed nor notarized** — `.github/workflows/release.yml` has no `codesign` or `notarytool` step at all. The `LC_CODE_SIGNATURE` present in the Mach-O is the ad-hoc signature arm64 requires merely to execute; it does nothing for Gatekeeper. A DMG downloaded from the internet carries the quarantine attribute, so Gatekeeper is expected to refuse it ("cannot be opened because the developer cannot be verified"), meaning a normal user may be unable to open the app at all without `xattr -d com.apple.quarantine` or right-click→Open. Separately, the Mach-O declares `minos 11.0` while `Info.plist` declares `LSMinimumSystemVersion 13.0` — the two disagree and neither is verified against a real machine; the bundle also has no `Contents/Resources` or icon. **Evidence level: artifact inspection only, not execution** — no macOS host has run this. Fold into RFC-078's matrix | review of 0.166.0 artifacts, 2026-08-08 | M5 / RFC-078 |
| F45 | The Windows artifact carries two undeclared runtime dependencies. (a) Its PE import table names `VCRUNTIME140.dll` and `VCRUNTIME140_1.dll` — the VC++ 2015–2022 redistributable, which is not guaranteed on a clean Windows install; absent, the app fails at launch with `VCRUNTIME140.dll was not found`. (b) The **WebView2 Runtime** is required to render anything but does not appear in the import table because it is loaded at runtime, so no static check can see it; it is preinstalled on Windows 11 and usually present via Edge on Windows 10, but neither is guaranteed. Nothing bundles or checks either, and the raw zip — unlike the Store MSIX — cannot declare a dependency. Every other import is a stable system DLL. **Evidence level: artifact inspection only, not execution.** Fold into RFC-078's matrix | review of 0.166.0 artifacts, 2026-08-08 | M5 / RFC-078 |
| F44 | **The published `linux-x86_64` binary does not start on any distro shipping libxdo 4** (Arch/CachyOS confirmed): `error while loading shared libraries: libxdo.so.3`. One unresolved dependency out of 146 — GTK 3 and WebKitGTK 4.1 record identical sonames across distro families, `libxdo` does not. Built on `ubuntu-latest` (soname 3); rolling distros ship 4, so **installing `xdotool` does not fix it** and the artifact is Debian/Ubuntu-only while labelled `linux-x86_64`. Root cause: `libxdo` was a **default** feature of both `muda` and `tray-icon`, and `dioxus-desktop` took both with defaults on, so every Dioxus desktop app linked it — for a code path that exists only to serve predefined Copy/Cut/Paste/SelectAll menu items, which this app cannot reach at all (`with_menu(None)`, `main.rs:56`). **Fixed upstream: [DioxusLabs/dioxus#5749](https://github.com/DioxusLabs/dioxus/pull/5749) — merged 2026-08-10** (`b6c258b`), mirroring Tauri's own `linux-libxdo` opt-in. **Not yet released**, so this stays open. On the next `dioxus-desktop` release: bump, confirm `readelf -d` shows no `libxdo` entry, ship. No workspace change needed — we already declare `default-features = false`. Until then the Linux artifact remains Debian/Ubuntu-family and should say so. Found by the owner running the artifact; nothing in CI launches one, which is F34 | owner, post-0.166.0 | **0.166.1**, gated on a dioxus release |
| F43 | **Decided (owner, 2026-08-11): drop the custom source archive.** It duplicated GitHub's automatic one — same 507 files, differing only in that ours omitted the top-level directory to suit `PKGBUILD`'s `cd "$srcdir"`. The checksum-stability justification never applied here (`sha256sums=('SKIP')`, and `source=` is a bare local filename, so nothing fetched or verified it). Remove `build-release.sh`'s source archive, `cargo xtask archive-layout`, its CI/release jobs and its `release.md` section; change `PKGBUILD` to Arch's conventional `cd "$pkgname-$pkgver"` against GitHub's own tarball | owner question 2026-08-08, decided 2026-08-11 | M4-C2 |
| F42 | Both gates F23 added can degrade to no-ops without any signal. (a) `actionlint` runs its bundled shellcheck pass **only if a `shellcheck` binary is on PATH**; when absent it does not warn or fail, it silently skips the rule — verified by experiment in review 053: the same mutant that reports `SC2086` exits 0 with shellcheck off PATH. `ubuntu-latest` ships it today, but nothing here depends on that staying true, and if it goes, `release.yml`'s eleven `run:` blocks stop being checked with CI still green. (b) The F41 step's `"permissions"` substring filter loses its only load-bearing test if that test is renamed, and `cargo test` exits 0 when a filter matches nothing, so the loss is silent. Fix both by asserting the precondition rather than assuming it: fail if `shellcheck` is missing, and fail if the filter stops matching the F38 regression test | review 053 | M4 |
| F41 | **Resolved.** `ci.yml` adds a step that sets `umask 077` and re-runs `cargo test -p forskscope-core permissions` (a name-substring filter — today the F38 regression test plus one unrelated `RecoveryHint` test caught incidentally and harmlessly; a future permission-mode test must include "permissions" in its name to be picked up) in its own shell process, never touching the umask of the rest of the suite. Falsifiability verified locally: reverting `save.rs` to the pre-F38 hardcoded `0o644` and running the same filtered command under `umask 022` still passes (2 passed), but under `umask 077` fails with exactly review 052's predicted message (`expected 600 ..., got 644`); restored immediately after, not committed | review 052 | M2 |
| F40 | **Resolved.** Chose the cheaper of the handoff's two designs (confirm, like `swap_sides`), not preserve-and-reapply: `HunkId` embeds `DiffId`, a process-global counter incremented on every `compute_diff` call, so hunk identity is never stable across a recompute even with identical content/options — reapplication would need a rebasing rule not implemented here. Added `change_diff_options`/`set_diff_options` (`state/tab.rs`) and `Modal::ConfirmDiffOptionChange`: the three toolbar controls (Ignore WS, Ignore case, algorithm) now compute the candidate `DiffOptions` and route through the same dirty-check-then-confirm gate `swap_sides` already used, instead of mutating `tab.diff_options` and calling `recompute_diff` directly. `is_dirty()` never goes silently false while work is discarded. RFC-015 §8 rule 4 marked **Not met** with a dated note explaining why, rather than left asserting something the code doesn't do. Runtime-verified via AT-SPI (dirty→dialog, cancel→no-op, confirm→apply-and-discard, clean→immediate-apply, all four observed) plus a core-level test documenting `recompute_diff`'s destructive contract | review 051 follow-up question, 2026-08-08 | M4 |
| F35 | **Resolved.** Chose "leave blank counterpart rows unlabelled" over labelling only the first row of a run or the hunk as a whole: a row with real content keeps its per-line `Changed: <line>` label (useful when navigating row by row), a row with nothing to say gets none. Implemented as a pure `wants_replace_label(kind, has_content)` predicate in `hunk.rs`, unit-tested directly since `RowLeft`/`RowRight` are `Store`-dependent (F36). AT-SPI-verified against a 4-line-left/1-line-right Replace fixture: the three blank counterpart rows on the shorter side now expose no `Changed:` text at all (previously each said bare "Changed" with nothing after it), while every row with real content on either side still announces `Changed: <line>` correctly. Decision recorded in RFC-019 §"Accessibility Requirements" (review 054 §4.3: RFC-019, not RFC-061, owns row ARIA), with a one-line pointer left in RFC-061 since RFC-061's own F32 AT-SPI work is what surfaced it | review 044 (N1), RFC-061 track | M4 |
| F28b | Both documents can be write-disabled on one launch (corrupt `settings.json` alongside a future-version `session.json`), but the startup notice drops the session one when a settings one exists. Acceptable for toasts; must not survive into the recovery dialogs, where the user would be told about one read-only document and nothing about the other | review 042 §4 | RFC-076 patch 6 (recovery UI) |
| F26 | **Resolved.** Twelve schema-enum variants were unpinned by the v2 golden fixtures — including `ThemeId::Dark`, the default and therefore the value in most real settings files. A single payload holds one value per scalar field, so no fixture could cover them. Added `persist_v2_schema_enum_wire_format_tests.rs`: a literal wire-string assertion for every variant of all ten schema enums (the five scalar-field enums that were unpinned, plus the five list-field enums already covered by the fixture, pinned here too for a complete fixture-independent reference), so a rename of any variant fails immediately regardless of which value a fixture happens to hold. | review 036 | RFC-076 pre-patch-4 |
| F17 | **Resolved.** Windows release build failed: `app-json-settings` 2.3.0/2.4.0 had an out-of-scope `use std::os::windows::ffi::OsStrExt` in `replace_file`'s local scope that did not cover the sibling `wide_null_terminated` function, which also calls `.encode_wide()`. Discovered by R0's first real release-workflow run — this is why R0 required an observed run rather than a configuration review. Reported upstream to `github.com/nabbisen/app-json-settings-rs`; fixed same-day in 2.4.1. Bumped and re-verified (Windows cross-compile, full gate suite) before the 0.165.0 tag. | 2026-08-01 R0 release run | R0 |

Phase 3 candidates are recorded under "Remaining proposed RFCs" and the
post-v1 slices below. They are deliberately unscheduled: post-v1 planning
resumes as a joint discussion after the Gate E verdict.

---

## Delivered milestones

| Milestone | Version | What landed |
|-----------|---------|-------------|
| Core extraction | v0.23 | `forskscope-core` crate, domain model, error taxonomy |
| Diff engine | v0.23 | `similar` v3, normalised diff/inline model |
| Dioxus shell | v0.23 | App shell, tabs, reactive state runtime |
| Explorer | v0.25 | Two-pane explorer, digest status icons |
| Diff/merge workspace | v0.26 | Hunk nav, merge transactions, undo/redo |
| Save safety | v0.27 | Atomic write, backup, dirty-close guard, fingerprint |
| Document buffer | v0.28 | Loaded document + result buffer model |
| Three-way merge | v0.40 | `ThreeWayMergeSession`, diff3 engine, conflict resolution |
| Explorer tree | v0.36 | Tree view, breadcrumb nav, ignore patterns |
| Patch export | v0.39 | Unified-diff export from file/directory diffs |
| Core data layer | v0.40–v0.72 | All RFC data types, 629 tests, clippy clean |
| View-model layer | v0.74–v0.87 | 14 `ui-logic` modules, 189 tests, all 7 slices covered |
| CSS contract | v0.88 | `fs-line-*`, `fs-inline-*`, `fsk-conflict-*` classes; 4 coverage tests |
| CSS bug fixes | v0.89 | `--danger-bg` defined; path.rs tests (16); `cancel_tests`, `file_kind_tests` |
| Test coverage | v0.90–v0.91 | All core modules tested; 26-file diff corpus; 856 tests total |
| UI four-bug fix | v0.92 | Two-pane split, dark theme select colour, ESC modal close, i18n expanded |
| Platform diag | v0.93 | `platform` module, `PlatformInfo`, corpus extended (encoding/binary/large) |
| Scroll fix + i18n | v0.94 | ISSUE-001 resolved (shared scrollbar); modals i18n complete |
| ELOC compliance | v0.115–v0.116 | command, error, session, report, settings, job modules split; zero files over 500 lines |
| Docs + platform | v0.95–v0.96 | Testing/architecture/local-dev docs updated; 4 user docs rewritten |
| CONTRIBUTING + limits | v0.97–v0.98 | ROADMAP/release/features updated; CONTRIBUTING.md; known-limitations.md |
| RFC-041 + v0.100 | v0.99–v0.100 | RFC-041 checklist updated; PlatformInfo wired to About; patch export UI |
| UI polish + i18n | v0.111–v0.139 | Full i18n (158 keys, 0 gaps); CSS cleanup (583→504 lines); keyboard shortcuts; per-file copy; bug fixes |
| Release readiness hardening | v0.164 | XLSX parser path disabled, dependency/network policy enforced, source archive contract fixed, CI/release gates aligned |

---

## UI implementation slices — status at v0.165.0

The remaining work is a series of UI slices that wire the Dioxus components
to the core types. Each slice delivers a testable, usable increment.

### ✓ Slice 1 — Diff view renders and navigates *(shipped)*

**Goal:** A user can open two files, see the diff rendered with correct
colour + gutter symbols, and navigate prev/next hunk with keyboard.

**Core types consumed:**
- `DiffDecorationSet::from_diff` → CSS classes, gutter symbols, aria labels
- `LineMap::from_diff` → aligned row sequence, `ScrollAnchor`
- `cmd::NEXT_DIFFERENCE`, `cmd::PREV_DIFFERENCE` → `CommandRegistry`
- `FileSizeClass::classify` → large-file prompt before diff

**Acceptance criteria:**
- Line diff renders in two synchronised panes with correct decoration classes
- `F7`/`F8` navigate hunks; both panes scroll together
- Large files (> 4 MiB) show the FileSizeClass prompt before diffing

---

### ✓ Slice 2 — Merge actions wire to core *(shipped)*

**Goal:** A user can apply hunks left-to-right, undo, and see the dirty-state
marker in the tab title.

**Core types consumed:**
- `TextEditOperation::Replace` → applied to result buffer
- `TransactionLog::push` / `undo` / `redo`
- `WorkspaceSession::mark_tab_dirty` / `mark_tab_clean`
- `cmd::COPY_HUNK_LEFT_RIGHT`, `cmd::UNDO`, `cmd::REDO`

**Acceptance criteria:**
- Apply-hunk updates the right-pane rendered content
- Ctrl+Z undoes the last merge; Ctrl+Y/Ctrl+Shift+Z redoes
- Tab title shows `*` when dirty; clears after save

---

### ✓ Slice 3 — Save with safety checks *(shipped)*

**Goal:** A user can save a merge result; external modification is detected
and the reconciliation dialog is shown.

**Core types consumed:**
- `save_text` with `AtomicSaveStrategy` and `BackupPolicy`
- `check_external_state` before write
- `AppError::from_core` → `RecoveryAction` → dialog buttons
- `cmd::SAVE`, `cmd::SAVE_AS`

**Acceptance criteria:**
- Save writes atomically and optionally creates a `.bak` backup
- External modification triggers the reconciliation dialog
  (Compare / Reload / Save As / Cancel)
- Failed save preserves dirty state

---

### ✓ Slice 4 — Explorer and directory compare *(shipped)*

**Goal:** A user can browse two directories and see equal/modified/only-left
/only-right status icons.

**Core types consumed:**
- `DirectoryIndex::from_records` + `pair_entries` → `EqualityEvidence`
- `JobRegistry` → progress bar while scanning
- `ConflictFilter` / `AvailabilityRule::SelectedPathExists` → explorer actions
- `ExternalToolCommand::file_manager_reveal` → "Reveal in Finder" action

**Acceptance criteria:**
- Digest icons show ✓ / ⚠ / left-only / right-only correctly
- Progress bar shown while background digest jobs run
- Double-click same-name file opens diff tab (RFC-054 §2-ii)

---

### ✓ Slice 5 — Settings dialog *(shipped)*

**Goal:** A user can change theme, font size, compare profile, and newline
policy from a settings dialog; changes persist across restarts.

**Core types consumed:**
- `UserSettings::to_json` / `from_json` → config file read/write
- `ThemeId::css_var_names` → CSS variable injection
- `CompareProfile::all_presets` → profile dropdown
- `BomPolicy`, `NewlinePolicy` → file settings section

**Acceptance criteria:**
- Settings persist to `~/.config/forskscope/settings.json`
- Theme change applies immediately without restart
- Current v0.164 plain-JSON settings ignore unknown fields; RFC-076 replaces
  this runtime path with schema v2 and explicit future-version handling

---

### ○ Slice 6 — Three-way merge workspace *(core complete; UI deferred post-v1)*

**Goal:** A user can open a three-way merge session, resolve conflicts with
Use Left / Use Right / Edit, and save the merged result.

**Core types consumed:**
- `ThreeWayMergeSession::from_texts`
- `ConflictNavigator::build` → navigator rail
- `resolve_left` / `resolve_right` / `resolve_manual` / `ignore`
- `can_save()` → save-block predicate
- `cmd::USE_LEFT`, `cmd::USE_RIGHT`, `cmd::NEXT_CONFLICT`

**Acceptance criteria:**
- Navigator rail shows `!`/`L`/`R`/`B`/`~`/`-` status for each conflict
- Keyboard: `Alt+L` / `Alt+R` resolve focused conflict
- Ctrl+S disabled while any conflict is unresolved; enabled when all resolved

---

### ○ Slice 7 — Command palette *(deferred post-v1)*

**Goal:** A user can open the command palette (`Ctrl+Shift+P`), type to
filter, and execute any available command.

**Core types consumed:**
- `CommandRegistry::builtin()` + `search(query)`
- `AvailabilityRule::evaluate(ctx)` → disabled-with-reason
- `CommandContext` snapshot from session state

**Acceptance criteria:**
- Palette filters commands by label and description (case-insensitive)
- Unavailable commands show as dimmed with tooltip reason
- Escape closes palette; Enter executes selected command

---

### ○ Slice 8 — Editor adapter prototype *(gated on RFC-004; post-v1)*

**Goal:** Text editing is model-backed; edits flow through
`TextEditOperation` and diff is recomputed on change.

**Gate:** Requires a stable CodeMirror or equivalent editor integration.
This slice is not on the critical path for a functional v1 (the result
buffer can be write-only in v1), but is required for full manual-edit support.

**Core types consumed:**
- `TextEditOperation`, `RevisionId`, `OperationAck`/`OperationReject`
- `EditTransaction` + `TransactionLog`
- `DiffDecorationSet` → editor decoration push

---

## Remaining proposed RFCs

| RFC | When | What |
|-----|------|------|
| 004 | Slice 8 | Editor adapter and CodeMirror bridge |
| 010 | Post-slice-5 | Packaging, diagnostics, QA |
| 016 | Slice 8 | Editor bridge security and contract |
| 020 | Ongoing | CI and architecture test gates |
| 025 | Slice 8 | Editor adapter prototype and kill-switch |
| 026 | Post-slice-3 | Cross-platform WebView compatibility |
| 030 | Post-slice-5 | User documentation and onboarding |
| 040 | Slice 8 | Editor adapter verification harness |
| 041 | Post-v1 | v1.0 product stabilization |
| 042 | Ongoing | Roadmap (this document) |
| 074 | Pre-v1 stabilization | Umbrella schedule, milestones, gates, and final go/no-go package |
| 077 | Milestone M3 | Git mergetool save-target identity and fingerprint safety |
| 078 | Milestone M5 | Platform runtime acceptance and retained release evidence |

---

## Non-goals (unchanged)

ForskScope is not and will not become:
- A full Git GUI
- An IDE
- A cloud diff service
- A file synchronization suite
- A universal document comparator
- An AI auto-merge agent
- A plugin marketplace

See `rfcs/done/001-core-extraction-and-domain-model.md` and
`rfcs/notes/forskscope-non-goals-v0.22.md` for the full non-goals policy.
