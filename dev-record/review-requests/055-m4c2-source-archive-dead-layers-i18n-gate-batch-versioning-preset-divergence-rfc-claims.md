# Review Request: M4-C2 (§5–10) — Source Archive Removal, Dead Contract Layers, i18n Gate Scope, Batch/Report Versioning, Preset Divergence, RFC/Claim Corrections

**Date:** 2026-08-13
**Reviewer stance:** implementer-level decisions, documented with reasoning — none of these were owner-scoped
**Repository baseline:** `7fd6b66` (six commits: `3a4b560`, `8f1af77`, `4d5e14f`, `e367dd9`, `0a48649`, `7fd6b66`)
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c2-documentation-and-code-truth-handoff.md` §5–10

## 1. Implementation summary

Six commits, one per handoff item, following the established one-item-per-commit
pattern (unlike M4-C1, which was documentation-only and landed as one act).
Every item in §5–10 required a decision the handoff explicitly delegated to
me rather than the owner (unlike F49 in the companion request, #054) — each
is documented below with the reasoning, per the operating instructions'
requirement that a delegated decision be recorded, not just asserted.

- F43 (`3a4b560`) — removed the source archive entirely (owner already
  decided 2026-08-11: drop it).
- F48 (`8f1af77`) — deleted the unwired `fs-line-*`/`hunk_decorations`
  layers rather than wiring them through.
- F39 (`4d5e14f`) — narrowed the i18n gate's documented scope rather than
  translating five passthrough error paths.
- F31 (`e367dd9`) — batch manifests and reports stay explicitly unversioned.
- F25/F25b (`0a48649`) — documented the two divergent preset sets rather
  than converging them.
- F11/F12/F16 (`7fd6b66`) — RFC-058 security-suspension note, RFC-062
  `proposed/` → `done/`, and a full public feature-claim audit.

Three new findings surfaced while resolving these and are registered, not
fixed, as out of scope for this slice: **F51** (RFC-034's ConflictNavigator
has the same unwired-to-DOM shape F48 fixed for RFC-024), **F52**
(RFC-017's `AppError`/`SaveErrorView` taxonomy is fully built and unused —
the real fix for F39's bypass sites), **F53** (`settings_view.rs`'s font
helpers are dead and diverge from the shipped UI's real values).

## 2. F43 — source archive removal

**Not a decision — the owner decided 2026-08-11 to drop it** (`ROADMAP.md`
F43, recorded before this slice). This item was mechanical execution with
one structural risk to manage: the handoff's explicit warning that "other
jobs may depend on the source job; removing it must not orphan them," and
that `archive-layout` "may be referenced... beyond the obvious call sites."

- Removed the entire `source` job from `.github/workflows/release.yml`.
  Confirmed `GITHUB_REF_NAME` (used already, unconditionally, in the
  `release` job's "Compose release notes" step) is available in every job
  of a tag-triggered run without the `source` job's `outputs.version`
  indirection — this let me delete the whole job, not just its
  archive-building steps, and re-point `linux`/`macos`/`windows` at
  `needs: preflight` directly.
- Removed `run_archive_layout_check` and its dispatch/usage lines from
  `xtask/src/main.rs`; the source-archive section from
  `packaging/build-release.sh`; `PKGBUILD`'s `source=` now points at
  GitHub's own tag-tarball URL, `build()`/`package()`'s
  `cd "$srcdir"` → `cd "$pkgname-$pkgver"` (GitHub's own extraction
  directory naming).
- Swept for stray references beyond the obvious call sites (grep across
  the whole repo, excluding `target/`/`docs/book/`): found and fixed three
  I would otherwise have missed — `ROADMAP.md`'s "Current state" section,
  `packaging/README.md` (two places: the Arch-install instructions, and a
  whole "Source archive" section describing the now-removed script
  behavior), and `docs/src/users/installation.md`'s Arch section (which
  told the user to "place the release source archive beside" the PKGBUILD
  — no longer necessary since `makepkg` now fetches it itself).
- Left untouched, deliberately: `CHANGELOG.md`'s historical entries and
  `dev-record/reviews/`'s past review records (dated historical record,
  not current-state claims), and `xtask/src/main.rs`'s F24 comment, which
  narrates *why* a past guard was added at a specific point in the old job
  graph — updated with a one-clause note that the source archive it
  references is now gone, rather than rewritten.

## 3. F48 — decision: delete both layers

`assets/css/30-contract-diff-decorations.css` styled
`.diff-row.fs-line-*`/`.fs-inline-*` classes `hunk.rs` never emitted, fed by
`forskscope-ui-logic::compare::hunk_decorations`, which nothing in
`forskscope-ui` called. **Decided: delete, per the handoff's own steer
("prefer (2) unless you can name what (1) buys") — I could not name what
wiring it through would buy.**

`hunk.rs`'s actual, live rendering already has a separate, working,
simpler contract: hunk-level (not per-row) backgrounds via
`.hunk-del`/`.hunk-ins`/`.hunk-rep`/`.pane-gutter`/`.in-del`/`.in-ins`
(`11-view-diff.css`), computed directly from `HunkKind`. Wiring the richer
per-row RFC-024 contract through would be a real visual change — it
distinguishes `Conflict`/`MergeApplied` states the live UI currently
doesn't color at all — not a documentation fix, and nothing named a reason
the product needs that distinction today.

Deleted: the `.fs-line-*`/`.fs-inline-*` CSS block (kept the RFC-034
`.fsk-conflict-*` block in the same file — separate contract, see F51
below); `hunk_decorations.rs` entirely, including its `pub mod`/re-export
in `compare.rs`/`lib.rs`; the two now-inapplicable `css_coverage.rs` tests
asserting those classes existed in `main.css`. **Kept**
`forskscope-core::diff_decoration` (`DiffDecorationSet`/
`LineDecorationKind`/`InlineDecorationKind`) untouched — it is
self-contained, tested independently of any renderer, and is RFC-024's
actual "core complete" deliverable; nothing about F48's decision requires
removing it, and a future renderer change wanting the richer contract can
still build on it.

RFC-024's status corrected: the acceptance criterion "Dioxus UI consumes
decoration sets without recomputing diff semantics" is now stated as **not
met and not expected to be** under the current renderer design, replacing
the previous, disproven "deferred to UI layer" framing (F48's own source —
review 055 — is what found the DOM claim was false: `hunk.rs` never had a
`.fs-line-*` DOM, at any point, not just "not yet").

**F51, registered not fixed:** while resolving F48, found RFC-034's
ConflictNavigator (`fsk-conflict-*`, `conflict_nav.rs`,
`conflict_nav_view.rs`) has the identical shape — a stylesheet and a
tested view-model with zero consumers in `forskscope-ui` (confirmed:
`crates/forskscope-ui/src/` has no reference to `conflict_nav`,
`ConflictNav`, or any `fsk-conflict-` string at all). Out of scope for
F48's decision, which named only the RFC-024 chain.

## 4. F39 — decision: narrow G-006 rather than translate

Five `store.notify(...)` sites bypass `t()`:
`diff_actions.rs`'s `handle_result` and `describe_block`, `recovery.rs`
(×2), `state/compare.rs`. **Decided: narrow the gate's documented scope,
not translate — the handoff's explicit condition ("do not do (1) without
addressing the detection gap") made this the defensible choice once I
traced what these sites actually carry.**

All five pass `CoreError`'s `Display` output. For `Io`/`Decode`/
`Unsupported`/`InternalInvariant`, that `message` field is generated by the
OS or a dependency at the moment of the error (confirmed:
`CoreError::io`'s constructor stores `err.to_string()` directly from
`std::io::Error`) — not authored copy in this codebase, so there is no
fixed string a translation map could hold. `describe_block`'s two literal
arms (`Binary`/`Spreadsheet`) are the one genuine exception, but
translating only those two without a working bypass-detector would recreate
exactly the gap this decision documents — a *partial* fix with an unchanged
gate is the specific failure mode the handoff warned against — so they stay
matched to the established `e.to_string()` precedent (review 048 C2) rather
than split silently.

`docs/src/maintainers/testing.md` and `run_i18n_audit`'s new doc comment
now state precisely what `cargo xtask i18n` checks (call sites that already
reach `t()`) and name the exempt classes and why, replacing an unqualified
"zero gaps" framing that G-006's own wording implied.

**F52, registered not fixed:** while assessing whether these five sites
*could* be translated, found that `AppError`/`AppErrorKind`/`SaveErrorView`
(RFC-017) is a complete, tested, translatable error taxonomy built for
exactly this problem — mapping `CoreError` into UI-presentable categories
with detail kept separate — but `forskscope-ui` never uses it anywhere
(confirmed: `grep` for `SaveErrorView`/`AppError`/`AppErrorKind` across
`crates/forskscope-ui/src/` returns nothing; the only `CoreError` awareness
in the UI is one `Err(CoreError::Conflict {..})` match arm). This
materially informed the F39 decision: a *real* fix routes through this
taxonomy, which is substantially more work than wrapping the current
passthrough text in `t()` — not something to attempt inside a
documentation-and-truth slice. RFC-017's own status line already says
"error toast component deferred to UI layer," which is honest as far as it
goes but understates the gap (it's not just a toast nicety — the save-error
*dialog* path has zero callers); left RFC-017 as-is rather than editing it,
since it isn't making a false claim the way RFC-024's was, and the fuller
picture now lives in F52.

## 5. F31 — decision: stay unversioned

`dir::batch::BatchManifest::to_json` and `report::{file,dir}`'s `to_json`
hand-roll JSON with no schema envelope. **Decided: stay explicitly
unversioned, no schema envelope added.**

Traced the actual read path (or lack of one) before deciding, rather than
defaulting to "add versioning is always safer": `restore_from_manifest`
restores from the **in-memory** `BatchManifest` the batch just produced,
not from a parsed JSON file (confirmed: no `from_json`/deserialization path
exists anywhere for `BatchManifest`, `FileComparisonReport`, or
`DirComparisonReport` — `report_tests.rs`'s own tests construct reports via
`from_diff`/`from_entries`, never by parsing exported JSON back).
Settings/session (RFC-076) genuinely reload across upgrades, which is what
justified `VersionedEnvelope` there; batch manifests and reports are
write-only exports for the user (something to keep, share, or attach to a
bug report), so a schema version would protect a read path that does not
exist. `BatchManifest` already carries `app_version`, which is enough for a
human or future tool to tell an old export's shape apart from a current
one without a dedicated field.

Documented in `dir::batch`'s and `report`'s module docs (new), and
`persist.rs`'s existing one-line gap note updated to name the decision
rather than just the gap.

## 6. F25/F25b — decision: document as legacy, don't converge

`CompareProfile::all_presets()` (core, RFC-028) and
`persist::schema::settings::ui_builtin_profiles()` (RFC-076, schema v2) are
two different four-item "built-in profile" sets with different names.
**Decided: document which is authoritative and why they diverge, rather
than converging them.**

Traced *why* two sets exist rather than assuming drift: RFC-028's own
status line already says its toolbar profile selector was "deferred
post-v1" — confirmed no `forskscope-ui` file calls
`CompareProfile::all_presets()` or its ui-logic bridge
(`settings_view::profile_presets()`) at all. `ui_builtin_profiles()`'s four
are what the shipped Settings dialog actually persists — the feature that
exists. Converging them now would mean designing for a UI that hasn't been
built, which is not this slice's mandate. Documented both sets' doc
comments, the unreached bridge function, and RFC-028's status line to state
plainly which set is authoritative today and that the two are deliberately
not kept in sync, so a future toolbar-picker author has to actively choose
rather than assume either already matches.

## 7. F11 — RFC-058 security-suspension note

Added a note to RFC-058's status documenting that `.xlsx` structural
comparison currently fails closed (`sheets-diff -> calamine -> quick-xml`'s
active XML DoS advisories — traced to `xlsx.rs`'s `diff_xlsx`/
`Unsupported` return), **without rewriting** the v0.57.0 implementation
record it sits below, per N4's explicit instruction to preserve the
historical record — that migration did happen and ship; it's simply not
reachable at runtime right now.

## 8. F12 — RFC-062 move

Moved `rfcs/proposed/062-safe-batch-copy-ux-and-restore-manifest.md` to
`done/`, changed its status to "Implemented (v0.145.3)", and added an
`## Implementation outcome` section per RFC-000's template. Verified all
four acceptance criteria against the actual code rather than trusting the
`rfcs/README.md` table's existing "Shipped" annotation — traced the
implementing commit (`99542b0`) and confirmed B1–B4 in
`crates/forskscope-ui/src/ui/overlay/modals/copy.rs`. Also resolved the
RFC's own open question ("in-app restore, or manifest-path-only?") against
what actually shipped: lighter than either alternative it posed — the
manifest path is shown, but there is no in-app restore action of any kind,
not even the "Restore this batch" button the RFC leaned toward.
`rfcs/README.md` counts updated (Implemented 51→52, Proposed 16→15), row
moved between tables.

## 9. F16 — public feature claim audit

Audited every bullet in `README.md` §Features (17) and every claim in
`docs/src/users/features.md`, against `crates/forskscope-ui/src/` — the
live app, not core/ui-logic, given how many core/ui-logic layers this
slice found unreached by it (F25, F48, F52, F53). **Classification: all 17
README bullets and every features.md claim are user-reachable and
accurate.** Full classification and per-claim verification detail is in
`ROADMAP.md`'s F16 entry (kept there rather than duplicated here, since
that's where a future reader will look for it). One correction: README's
"GitHub Actions gates" bullet still listed "archive layout" as a checked
gate, stale since F43 (§2 of this request) removed it — dropped from the
list. One item already met the standard the handoff asked the rest to
match: the three-way-merge bullet's "core model shipped; conflict
workspace UI deferred post-v1" phrasing.

**F53, registered not fixed:** while verifying the font-size/font-family
claims, found `forskscope-ui-logic::settings_view`'s `clamp_font_size`
(6–50pt) and `font_family_choices()` (3 families) have zero callers in
`forskscope-ui` and diverge from what the live Settings dialog actually
hardcodes (8–32pt, 5 families, in `ui/view/settings/modal.rs`) — the
documented numbers are correct because they match the live UI, not the
unreached ui-logic helper of the same apparent purpose.

## 10. Changed and created files

By commit:

- `3a4b560` — `.github/workflows/release.yml`, `xtask/src/main.rs`,
  `packaging/build-release.sh`, `packaging/linux/PKGBUILD`,
  `docs/src/maintainers/release.md`, `packaging/README.md`,
  `docs/src/users/installation.md`, `ROADMAP.md`.
- `8f1af77` — `crates/forskscope-ui/assets/css/30-contract-diff-decorations.css`,
  `crates/forskscope-ui/assets/main.css` (regenerated),
  `crates/forskscope-ui-logic/src/compare.rs`,
  `crates/forskscope-ui-logic/src/compare/hunk_decorations.rs` (deleted),
  `crates/forskscope-ui-logic/src/lib.rs`,
  `crates/forskscope-ui-logic/tests/css_coverage.rs`,
  `rfcs/done/024-diff-visual-semantics-decoration-contract.md`, `ROADMAP.md`.
- `4d5e14f` — `xtask/src/main.rs`, `docs/src/maintainers/testing.md`,
  `ROADMAP.md`.
- `e367dd9` — `crates/forskscope-core/src/dir/batch.rs`,
  `crates/forskscope-core/src/persist.rs`,
  `crates/forskscope-core/src/report.rs`, `ROADMAP.md`.
- `0a48649` — `crates/forskscope-core/src/diff/options.rs`,
  `crates/forskscope-core/src/persist/schema/settings.rs`,
  `crates/forskscope-ui-logic/src/settings/settings_view.rs`,
  `rfcs/done/028-preferences-profiles-and-compare-options.md`, `ROADMAP.md`.
- `7fd6b66` — `README.md`, `rfcs/README.md`,
  `rfcs/done/058-spreadsheet-xlsx-structural-diff.md`,
  `rfcs/proposed/062-...` → `rfcs/done/062-...` (renamed, edited),
  `ROADMAP.md`.

## 11. Executed gates, with observed output (every commit)

Ran the full local gate suite before every commit in this slice; identical
pass pattern each time:

```text
cargo fmt --check                                              pass
cargo fmt --manifest-path xtask/Cargo.toml --check              pass (F43 only, xtask touched)
cargo build --manifest-path xtask/Cargo.toml                    pass (F43 only)
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass
cargo xtask css --check                                          pass (F43, F48 — CSS touched)
cargo xtask i18n                                                 pass — 227 keys, unchanged
mdbook build docs                                                pass
git diff --check                                                 pass
```

Test counts moved as expected and only where a real change explains it:
`css_coverage` integration tests 6→4 (F48 removed two now-inapplicable
tests); `forskscope_ui_logic` doctests 1→0 (F48 removed
`hunk_decorations.rs`'s doc example). No other suite's count changed.

CI runs, one per commit, all on `main`:

| Commit | Run | Result |
|---|---|---|
| `3a4b560` | `31681974056` | fails at "Security advisory audit" only (F50, pre-existing) — `actionlint` on the restructured workflow job graph passes |
| `8f1af77` | `31682740841` | same pattern |
| `4d5e14f` | `31683055862` | same pattern |
| `e367dd9` | `31683216934` | same pattern |
| `0a48649` | `31683439950` | same pattern |
| `7fd6b66` | `31683985246` | fails at "Security advisory audit" only (F50, pre-existing) |

No dependency added, removed, or version-changed anywhere in this slice.
No product-visible behavior change except F48's CSS/view-model deletion,
which removes dead code with zero live consumers — confirmed via grep
before deletion, and via a clean `cargo build --workspace` after.

## 12. Difference from the handoff, RFC-074, or RFC-078

None in scope. Every §5–10 decision was delegated to me by the handoff's
own wording ("Decide...", "State which set is authoritative either way.");
none required owner input the way F49 (companion request #054) did.

## 13. Unresolved issues and known limitations

- F51, F52, F53 — registered, not fixed, all explicitly out of scope for
  the items that surfaced them. F52 in particular is the real fix for
  F39's underlying gap and should probably be prioritized before another
  i18n-adjacent audit re-finds the same five bypass sites.
- F16's classification found no overclaims beyond the one stale "archive
  layout" mention (already an F43 side-effect, not a pre-existing defect) —
  worth noting since a "systematically audited" finding usually turns up
  more.

## 14. Requested review focus

1. F48 and F39's "delete/narrow rather than build" pattern, repeated three
   times in F51/F52/F53's discovery — whether this is the right ongoing
   posture for M4-C3/Gate C, or whether one of F51/F52/F53 should be
   pulled forward rather than left for "next slice."
2. F31's reasoning (no read path → no schema version) — whether this
   should also apply to any other write-only export in the codebase not
   audited here.
3. F16's classification — whether "all bullets accurate, one stale
   mention" is a plausible result for a first systematic audit, or whether
   a second pass with different sampling is warranted.
