# RFC-074 M4-C2 Developer Handoff: Documentation and Code Truth

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md), with [RFC-078](../../proposed/078-platform-runtime-acceptance.md) for §3–§4
**Milestone.** M4-C2 — the last slice before Gate C is assessable
**Register items.** F49, F43, F48, F39, F31, F25/F25b, F11, F12, F16, plus review 056's N1 corrections
**Baseline.** `main` at `0335abc`

This handoff directs execution of one slice. It does not redefine RFC-074 or
RFC-078. If implementation evidence contradicts a decision below, amend the RFC
first, then update this handoff to match.

## 1. Summary

Gate C requires "docs/RFC status synchronized." This slice is that, plus two
things that came out of M4-C1's review: reconciling the platform version claims
before the matrix can freeze, and amending RFC-078's execution model to match
the resources the project actually has.

**It is large but shallow** — mostly the project stating what is true. Split
your review request at the §4/§5 boundary if it gets unwieldy: §2–§4 gate the
matrix freeze and are worth reviewing on their own; §5 onward is
documentation-truth cleanup.

No audit blocker closes here. B4 remains open; v1/public release stays
**No-Go**.

## 2. Corrections carried from review 056

Small, do them first:

- **`docs/src/maintainers/testing.md:93`** names `AtomicSaveStrategy`, a type
  absent from `forskscope-core`. The M4-C1 sweep fixed `architecture.md` and
  missed this one. **Leave `rfcs/notes/core-completion-summary-v0.72.md` alone**
  — same token, but a dated historical note, and RFC-074's N3 is explicit that
  the archive is not revised.
- **`advisories.md`**: record that `cargo tree -i rand@0.7.3 -p forskscope-ui`
  prints "nothing to print" without `--target all`, so a future reviewer running
  the obvious command does not conclude the advisory lapsed. And restate the
  `glib` finding as **"`VariantStrIter::new` is `pub(crate)`, so no external
  construction path exists"** — that survives a dependency bump better than
  "nothing currently calls the one public constructor."
- **`matrix-plan.md`**: record the P06 axis question (async identity is timing,
  and the completion path runs through `tao`'s Wayland/X11 event loop, not the
  web engine) and adopt the rule: **any P06 defect on any row upgrades every
  P06 spot-check on that platform to Required before the matrix closes.**

## 3. F49 — reconcile the platform version claims

**This gates the matrix freeze.** Freezing a plan on top of contradictory
version claims bakes the contradiction into M5's evidence.

### 3.1 macOS — three numbers, none authoritative

| Source | Claim |
|---|---|
| `packaging/macos/build-dmg.sh` → `Info.plist` | `LSMinimumSystemVersion` **13.0** |
| Built Mach-O (`0.166.0` artifact) | `minos` **11.0** |
| RFC-078 §277 | "the conflict between macOS **12** and 13.0" |

`MACOSX_DEPLOYMENT_TARGET` is set nowhere, so `minos` is whatever the runner's
SDK yields. And **"macOS 12" appears nowhere else in the repository** — RFC-078
preserves one half of a conflict whose other half is gone, while omitting the
live one.

Required:

1. Set an explicit deployment target so `minos` stops being incidental.
2. Make `LSMinimumSystemVersion` agree with it.
3. Correct RFC-078 §277 to describe the real conflict, or delete it if resolved.
4. **The owner has asked for `macos-latest`** in place of `release.yml`'s pinned
   `macos-14`. Do this *together with* (1) — switching runners changes the SDK
   and therefore `minos`, so doing it alone swaps one undefined number for
   another.

The owner decides which macOS version the project claims; ask if (1) is not
implied by their answer. Do not pick a floor unilaterally.

### 3.2 Windows — four minimums

| Source | Claim |
|---|---|
| `AppxManifest.xml` | `MinVersion=10.0.17763.0` → Windows 10 **1809** |
| `AppxManifest.xml` | `MaxVersionTested=10.0.19041.0` → Win10 2004, predating Windows 11 |
| RFC-078 §109 | Windows 10 **1903+**, plus a full Windows 11 row |
| `docs/src/users/installation.md` | "Windows 10 or later" |

`MaxVersionTested` is the sharpest: the Store manifest says the newest thing
tested is Windows 10 2004, while RFC-078 requires a full Windows 11 matrix.

Converge on one published minimum, make all four agree, and say which is
authoritative. Again the *value* is the owner's call; making them consistent is
yours.

## 4. RFC-078 — amend the execution model

RFC-078 assumes named humans executing 12 cases across 6 rows. The owner has
stated the actual model:

> **Users:** Windows, macOS, Linux.
> **Verification:** GitHub Actions CI on `windows-latest`, `macos-latest`,
> `ubuntu-latest`, plus occasional manual tests on Linux Wayland and Windows 11.

Amend RFC-078 to describe this. It is a better fit than the current text and the
F34 render check already proves headless GUI verification works in CI.

**But two cases are structurally invisible to CI, and the amendment must say so
rather than let a green matrix imply coverage:**

- **F45 — Windows prerequisites.** `windows-latest` runners ship with the VC++
  redistributable and WebView2 preinstalled. A CI launch proves the binary runs
  *on a machine that already has them*; the failure mode is a clean machine,
  which no runner is. **P01 on Windows must be marked manual-only for the
  prerequisite sub-case**, and the owner's Windows 11 manual pass is what covers
  it.
- **F46 — macOS Gatekeeper.** Gatekeeper only refuses files carrying the
  quarantine attribute, which a browser download applies and a CI checkout does
  not. A runner launches an unsigned app happily. With no macOS manual host in
  the stated model, **F46 cannot be verified at all** — record it as an explicit
  open gap with release impact, not as a passing row.

Also settle the macOS row count (RFC-078's table lists macOS twice, the evidence
layout names one file) consistently with §3.1's answer.

State plainly in the amendment what CI evidence does and does not establish.
That distinction is the whole subject of this milestone.

## 5. F43 — remove the source archive (owner decided)

The owner decided on 2026-08-11: **drop it.** It duplicated GitHub's automatic
tarball — same 507 files, differing only in the omitted top-level directory,
which existed to suit `PKGBUILD`'s `cd "$srcdir"`. The checksum-stability
justification never applied, because `sha256sums=('SKIP')` and `source=` is a
bare local filename.

Remove: the source archive from `packaging/build-release.sh`, `cargo xtask
archive-layout`, its CI and release-workflow jobs, and its `release.md` section.
Change `PKGBUILD` to Arch's conventional `cd "$pkgname-$pkgver"` against
GitHub's own tarball.

**Watch the release workflow's job graph** — other jobs may depend on the source
job; removing it must not orphan them. And `archive-layout` may be referenced in
`docs/`, `ROADMAP.md`'s "Current state", or CI beyond the obvious call sites.
Sweep, do not just delete the function.

## 6. F48 — the CSS decoration contract the DOM never receives

`assets/css/30-contract-diff-decorations.css` styles `.diff-row.fs-line-*` and
its header asserts "**Current DOM (v0.162.0+):** `.diff-row[.fs-line-*]`". That
DOM does not exist: `hunk.rs` emits only `"diff-row"` / `"diff-row match"`, and
`fs-line-` appears nowhere in `crates/forskscope-ui/src/`. Core's
`LineDecorationKind::css_class()` feeds
`forskscope-ui-logic::compare::hunk_decorations`, and **no UI code consumes
`hunk_decorations` at all.**

A decision, not a patch. Either:

1. **Wire it through** — apply the decoration classes in `hunk.rs`. This is a
   product change and needs its own justification: what does it buy that the
   current `match` class does not?
2. **Delete both layers** — the stylesheet and the unreached view-model — and
   correct RFC-024's status to record that the contract was designed and not
   adopted.

Prefer (2) unless you can name what (1) buys. Either way, note that `cargo xtask
css --check` passed throughout, because it verifies `main.css` is a current
concatenation, not that its selectors are reachable — the same blindness that
let F32 ship. Say whether that is worth fixing; it is not in this slice's scope.

## 7. F39 — the i18n gate is blind to strings that bypass `t()`

Five user-visible error paths reach the toast without `t()`:
`diff_actions.rs:285`, `recovery.rs:142` and `:252`, `state/compare.rs:154`, and
`describe_block`. G-006 requires all user-visible strings routed through the
translation layer with zero gaps, and `cargo xtask i18n` reports pass throughout
because it compares `t(...)` call sites against the Japanese map and is
structurally blind to strings that never reach one.

**Decide, and make the gate match the decision:**

1. **Translate them** — then the gate should be able to detect a *new* bypass,
   or it will drift straight back. That detection is the harder half.
2. **Narrow G-006** to what is actually guaranteed, and say which string classes
   are exempt and why.

Do not do (1) without addressing the detection gap; a fixed instance with an
unchanged gate is how this recurs.

## 8. F31 — batch manifests and reports are unversioned

`persist.rs`'s module doc has already been corrected and now states the gap
plainly, naming `dir::batch::BatchManifest::to_json` as hand-rolled with no
schema envelope. That half is done.

What remains is the decision RFC-076's convergence deferred: **do batch
manifests and reports adopt schema versioning, or are they explicitly
out of scope?** They are files ForskScope writes and a user could keep. If they
stay unversioned, say so where someone reading `persist.rs` will find it, and
record what happens when a future version changes their shape.

## 9. F25/F25b — two divergent built-in preset sets

Core's `CompareProfile::all_presets()` returns four profiles and is consumed by
`ui-logic::settings_view::profile_presets()`, which is re-exported but which no
`forskscope-ui` file calls. The UI's own four are canonical for persisted schema
v2.

So there are two built-in sets, and the divergence is invisible only because the
bridge is unreached. Converge them, or document core's as legacy with an
explicit note that schema v2 never produces it. State which set is authoritative
either way.

## 10. F11, F12, F16 — RFC and claim status

- **F11 (advisory N4)** — RFC-058's status line reads "Implemented (v0.57.0) —
  migrated to sheets-diff v2.2.1" and says nothing about the fail-closed
  security suspension that removed the runtime XLSX path. Add a
  security-suspension note linking the current decision, **preserving the
  historical implementation record** rather than rewriting it, per N4.
- **F12** — RFC-062 is fully shipped and still filed under `proposed/` with
  Status "Proposed". Move to `done/`, add the `## Implementation outcome`
  section per RFC-000, update `rfcs/README.md`'s counts.
- **F16** — public feature claims are not systematically audited for
  core-complete versus user-reachable. Concretely: walk every bullet in
  `README.md` §Features and `docs/src/users/features.md`, classify each as
  user-reachable, core-only, or partial, and correct anything that overclaims.
  The three-way merge bullet already does this correctly ("core model shipped;
  conflict workspace UI deferred post-v1") — make the rest match that standard.
  Report the classification, not just the diff.

## 11. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed.
- Product behaviour changes only where a decision above explicitly selects one
  (§6 option 1, §7 option 1). Everything else is documentation, packaging, and
  RFC status.
- Existing gates must keep passing, including M4-B's new ones.
- Version *values* in §3 are the owner's call. Ask; do not choose a floor.

## 12. Required review-request content

Submit under `.git-exclude/review-request/` — split at the §4/§5 boundary if
large — with:

1. implementation summary;
2. addressed items;
3. changed files;
4. **§3's reconciliation: the final authoritative version per platform, every
   source now agreeing, and what the owner decided** — main focus;
5. **§4's amendment, with the two CI blind spots stated explicitly** and what
   F46's unverifiable status means for Gate D;
6. **each decision item's choice and reason** — §6, §7, §8, §9;
7. **§10's F16 classification table**;
8. any difference from this handoff, RFC-074, or RFC-078;
9. executed gates with observed output;
10. anything found and registered rather than fixed;
11. unresolved issues and known limitations;
12. requested review focus.
