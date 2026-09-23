# Review Request: M4-C2 (§2–4) — Review-056 Corrections, F49 Version Reconciliation, RFC-078 Execution Model

**Date:** 2026-08-13
**Reviewer stance:** focused documentation/evidence review, plus two owner-scoped questions
**Repository baseline:** `ea46991` (this slice), `6ce1b94` (F50, registered mid-slice)
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c2-documentation-and-code-truth-handoff.md` §2–4, RFC-078

## 1. Implementation summary

One commit (`ea46991`) covering the handoff's §2–4 — the half that gates the
matrix freeze, per the handoff's own suggested split point. A second,
unplanned commit (`6ce1b94`) registers F50, a new real vulnerability
discovered mid-slice via two independent CI runs, unrelated to any change
made here.

- §2 — review-056's three corrections (`testing.md`, `advisories.md`,
  `matrix-plan.md`).
- §3 (F49) — reconciled macOS's three and Windows's four disagreeing
  version claims.
- §4 — amended RFC-078's "Required platform matrix" and added an
  "Execution model" section describing the real CI/manual-test resources.
- F50 — registered, not fixed (dependency changes are out of scope for
  every M4 handoff).

## 2. F49 — the owner-scoped decision, and how it was actually handled

This item is called out separately because it is the one piece of this
slice that was **not** mine to decide. Per the handoff's explicit
constraint ("Version *values* in §3 are the owner's call. Ask; do not
choose a floor."), I did not choose macOS's or Windows's version floors
myself. I asked in chat; the answer routed back was: relay the report and
question to you as a review request rather than have me act on a decision
made in that conversation. I'm doing that here, together with what the
owner had already told the user directly:

- **macOS runner:** `macos-latest` (not the `macos-14` `release.yml` had
  pinned). Applied: `.github/workflows/release.yml`'s `macos` job now runs
  `macos-latest`.
- **Windows runner:** `windows-latest`. Already what `release.yml`'s
  `windows` job used — no change needed there.
- **Manual test host:** "Windows 11 Desktop when manual tests are done
  occasionally" — recorded verbatim in RFC-078's new "Execution model"
  section and in `matrix-plan.md`.
- **macOS deployment target:** switching to `macos-latest` changes the SDK
  and therefore the compiled binary's `minos` field, so I set
  `MACOSX_DEPLOYMENT_TARGET: "13.0"` explicitly on the `macos` job's build
  step rather than leaving it to whatever the new runner's SDK produces
  incidentally — 13.0 matches `Info.plist`'s already-enforced
  `LSMinimumSystemVersion`, so it doesn't change any real, already-shipped
  behavior. This one specific value (a safe default matching a fact already
  true today) I did apply directly rather than leaving open, since the
  handoff's constraint was about *choosing a floor*, not about mechanically
  keeping a newly-selected runner's incidental SDK output from silently
  drifting from a value already fixed elsewhere in the repo. Flagging this
  judgment call explicitly in case it should have been asked instead.
- **Windows version floor:** left **open**. `AppxManifest.xml`'s
  `MinVersion=10.0.17763.0` (Windows 10 1809) may back a live Microsoft
  Store submission (`docs/src/users/installation.md` links
  `https://apps.microsoft.com/detail/9p63f7npc3mh`) not automated anywhere
  in this repo — the raw-zip Windows release job never touches the
  manifest. I did not touch the manifest's actual values; I only aligned
  `installation.md`'s doc prose to state the manifest's *current* value
  precisely. **Question:** should this floor change, and if so, does that
  need to happen in lockstep with an actual Store resubmission this repo
  cannot see the state of?

## 3. F50 — new vulnerability, reported not fixed

`RUSTSEC-2026-0257` (`webbrowser` 1.2.1, "Unix `BROWSER` handling allows
browser argument injection") appeared in the RustSec advisory database
between two CI runs on `main` minutes apart (the concurrent architect
session's `413f482` and my own `ea46991`), with no dependency change in
either commit — `cargo audit`'s loaded-advisory count grew from 1207 to
1216 in that window. `webbrowser` is a direct dependency of
`dioxus-desktop`, one hop from `forskscope-ui`. This is not one of the 14
previously-dispositioned advisories in `advisories.md` — it is a genuine
`cargo audit` failure (exit 1), and it is currently failing every push to
`main` at the "Security advisory audit" step, including every commit in
this slice and the next (M4-C2 §5–10, submitted separately).

A fix is available (`webbrowser >= 1.2.2`), but every M4 handoff this
program has run under explicitly forbids dependency changes in-slice — "a
disposition records the decision; changing the graph is separate work with
its own review." Registered as F50 in `ROADMAP.md` (`6ce1b94`).
**Question:** how should this be handled — a dedicated minimal-diff
dependency-bump slice ahead of the rest of M4-C2/C3, or folded into
whichever slice next touches `Cargo.lock`?

## 4. Review-056 corrections (§2)

- `testing.md` line 93: `AtomicSaveStrategy` (a type that doesn't exist)
  replaced with the real `TargetPrecondition`.
  `rfcs/notes/core-completion-summary-v0.72.md`'s identical stale token left
  untouched per review 056's explicit instruction — dated historical note,
  RFC-074 N3 says the archive isn't revised.
- `advisories.md`'s `glib`/`RUSTSEC-2024-0429` section restated: the finding
  is now framed as "`VariantStrIter::new` is `pub(crate)`, no external
  construction path exists" — a compiler-enforced, one-line-grep fact,
  cheaper and more certain than a full-source text search (which risks
  missing a macro-expanded call site). The full-source grep from M4-C1
  stays as the second, independent confirmation.
- `advisories.md`'s `rand`/`RUSTSEC-2026-0097` section: added a note that
  `cargo tree -i rand@0.7.3 -p forskscope-ui` silently prints nothing
  without `--target all` (verified locally, exact output starts "warning:
  nothing to print...") — could otherwise read as the advisory having
  lapsed.
- `matrix-plan.md`'s P06 axis question and upgrade rule folded in from
  review 056's N1: async identity is about *timing* (event-loop), not
  rendering engine, so `tao`'s Wayland-vs-X11 backend differs even though
  both share WebKitGTK; adopted backstop: any P06 defect on any row
  upgrades every P06 spot-check on that platform to Required before the
  matrix closes.

## 5. RFC-078 execution model amendment (§4)

Added a `## Execution model (M4-C2 amendment, 2026-08-11)` section quoting
the owner's stated model verbatim: GitHub Actions CI on
`windows-latest`/`macos-latest`/`ubuntu-latest`, plus occasional manual
tests on Linux Wayland and Windows 11. Explains what CI actually covers per
platform (`ubuntu-latest`+Xvfb ≈ X11-family, not real Wayland;
`windows-latest` ≈ a Server image, not literal retail Windows 10/11;
`macos-latest` is the only macOS host — no manual pass exists) and states
two CI blind spots explicitly rather than as passing rows:

- **F45** (Windows prerequisites) — CI runners already carry the VC++
  redistributable and WebView2, so CI cannot exercise the "these are
  missing" case at all; P01's prerequisite sub-case is manual-only by
  construction.
- **F46** (macOS Gatekeeper) — cannot be verified under this model at all;
  no macOS manual host exists to test signing/notarization/quarantine
  behavior against.

`matrix-plan.md` updated to match: a new "Verification method" column per
row (CI vs. Manual), macOS collapsed to one row (F49's answer above),
`linux-wayland` marked Manual, the rest CI, with F45/F46 restated to match
the new RFC-078 language.

## 6. Changed and created files

`.github/workflows/release.yml` (F49: `macos-14` → `macos-latest`,
explicit `MACOSX_DEPLOYMENT_TARGET`), `docs/src/maintainers/testing.md`,
`docs/src/maintainers/release-evidence/0.167.0-rc1/advisories.md`,
`docs/src/maintainers/release-evidence/0.167.0-rc1/matrix-plan.md`,
`docs/src/users/installation.md` (F49 doc-prose alignment),
`rfcs/proposed/078-platform-runtime-acceptance.md` (execution-model
section, macOS row collapse, Windows row annotation), `ROADMAP.md` (F50
entry, in `6ce1b94`).

## 7. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass
cargo xtask i18n                                                 pass
cargo xtask css --check                                          pass
mdbook build docs                                                pass
git diff --check                                                 pass
```

CI run `31674712122` on `ea46991`: fails at "Security advisory audit"
only — the F50 advisory that appeared independently of this change (also
observed failing on the concurrent architect session's `413f482`, which
touched none of these files either). Every step before it — including
`actionlint`, format, CSS, version-sync, i18n — passes. No dependency
added, removed, or version-changed by this slice.

## 8. Difference from the handoff, RFC-074, or RFC-078

None in scope beyond the one judgment call flagged in §2
(`MACOSX_DEPLOYMENT_TARGET` applied directly rather than left as an open
question, since it's a safe default matching an already-enforced fact, not
a new floor choice).

## 9. Unresolved issues and known limitations

- F49's Windows version floor (§2) — open, owner question.
- F50 (§3) — open, owner question on how/when to schedule the fix.
- `matrix-plan.md`'s other owner questions (executor/host-access per manual
  row) remain open from M4-C1, unaffected by this slice.

## 10. Requested review focus

1. Whether applying `MACOSX_DEPLOYMENT_TARGET: "13.0"` directly (rather
   than asking) was the right call given the "ask; do not choose a floor"
   constraint, or should have been routed as a question too.
2. The two open questions in §2 and §3.
3. Whether the RFC-078 execution-model section correctly and completely
   captures the real CI/manual-test resourcing, or omits a blind spot.
