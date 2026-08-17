# Gate D Input List — 0.167.0

The single place everything that bears on the Gate D go/no-go decision is
recorded, per the M5-C handoff §7. This is the thing Gate D is assessed
against — not a summary of case results (see each row file and `README.md`'s
verdict for those) but a record of what is *true right now*, dated, not
predicted. If an input's status changes, this file is updated and re-dated;
it is not retroactively rewritten to read as if the current status always
held.

**Assembled:** 2026-08-16, after M5-A, M5-B, and M5-C's CI-verified rows all
landed and were reviewed (reviews 063–068).

## Un-waivable blockers

RFC-078's waiver policy names five categories no waiver may turn into a
release pass. Three separate, independent findings land in that territory:

| Input | Status |
|---|---|
| **F44** | **Fail, un-waivable.** The published Linux artifact does not launch on any libxdo-4 distribution (a supported platform — no per-distribution floor is claimed). Upstream schedule dependency: fixed in `dioxus-desktop` upstream (merged 2026-08-10), not yet in a released version this project can pick up. |
| **F61** | **Un-waivable** (silent settings/session loss — one of the five named categories). A tab opened via CLI startup args was not reliably persisted to `session.json`. **Fixed on `main` for real** (review 066, 2026-08-16) — the reactive `use_effect` that never fired for non-event-driven writes was removed in favor of explicit `save_session` calls, verified on a real desktop process. **Not in the published `0.167.0` candidate.** A new candidate build is required before M5's P12 rows can be re-run against the fix and this input clears. |
| **F73** | **Un-waivable** (wrong-file/stale-load behavior — one of the five named categories), per review 068 §4. `DeepRow`'s per-row copy buttons source their destination from Explorer's remembered pane directory instead of the deep-compare view's actual compare root, silently writing to the wrong location with no error surfaced — confirmed with a real backup/overwrite, not a reported "success." Found **independently on both Windows and macOS** during M5-C's P07 work — shared `deep_compare.rs` code, not a platform quirk. **Not fixed in this candidate.** Shares one root cause with F68 (`DeepRow` is never passed `left_root`/`right_root`) — one fix (passing the roots in) closes both. |

**Gate D cannot pass while any of the three above is open**, regardless of
how clean the rest of this evidence is — this is expected, not a failure of
the M5 evidence-gathering effort itself (per the M5-A handoff §1: a case or
input that fails, recorded accurately with its cause, is a successful
outcome for this milestone).

## Other Gate D inputs

Not un-waivable on the current reading, but each bears on the decision and
belongs in front of the owner before a go/no-go, not discovered later inside
a row file's fine print.

| Input | Status |
|---|---|
| **F45** | Manual-only, owner-executed, outstanding. Windows artifact carries two undeclared runtime dependencies (VCRUNTIME140/140_1, WebView2 Runtime) invisible to CI since `windows-latest` already has both preinstalled. |
| **F46** | Blocked — cannot be verified under current resourcing. The macOS artifact is neither Developer ID-signed nor notarized; no manual macOS host exists to confirm Gatekeeper's actual behavior against a downloaded, quarantined DMG. |
| **F60** | Windows floor unevidenced; owner decision open. `AppxManifest.xml` declares a Windows 10 1809 floor, but M5's CI rows ran on a Server-2025-based image (NT 10.0.26100) — the oldest supported Windows has never been observed running the app, and nothing currently planned changes that. |
| **F63** | **Resolved, not an input** — closed as a harness artifact (review 068 §3), not a product accessibility defect. Content reaches the macOS accessibility tree well past the previously-recorded threshold; the harness's default timeouts were simply too short for a slow-but-correct enumeration. Listed here only for completeness against the handoff's own template — carries no weight in the decision. One open, unmeasured caveat carried forward (not resolved, not blocking): whether real VoiceOver navigation experiences comparable latency was not tested either way — see `macos-aarch64.md`'s Finding 3. |
| **F69** | New (M5-C Windows, review 067). `autofocus` never moves keyboard focus into a destructive confirmation modal on WebView2 — focus stays on the background control that triggered the modal. Verified independently on Linux/WebKitGTK, where the same modal pattern correctly focuses Cancel — a genuine engine-specific difference, not a harness artifact. Worse than RFC-078's P11 warns about (focus-on-destructive-action): here focus never enters the modal at all. Not fixed in this candidate. |
| **F70** | New (M5-C Windows, review 067). Explorer's directory listing never renders a single row on the Windows CI environment — confirmed across 5 CI runs, 4 independent trigger mechanisms, and a 150s patient poll. **Product defect or CI-environment limitation is undetermined**, and that undetermined status is itself what belongs here — recorded as an open input rather than waiting for resolution. Cheapest next step: the owner opens Explorer on the Windows 11 manual host (seconds of work, not requiring a rebuild). |
| **F72** | New (M5-C macOS, review request 066). Explorer's Back button destroys that pane's Forward history via an unconditional re-push in `navigate_to` that its own duplicate-guard cannot catch. Shared code (`explorer.rs`/`dir_pane.rs`), found on macOS but not macOS-specific — almost certainly reproduces on every platform. Not fixed in this candidate. |
| **Keyboard interface** | **No automated runtime coverage on any platform this program has evidence for.** Across M5-B and M5-C: P04's Enter-apply shortcut, P06's double-reload, and three of P11's four items (checklist, shortcuts-inert-behind-modal, Escape) all need a real keystroke dispatched at the OS/window level that no accessibility API on any of the three platforms can synthesize. The one item that *is* CI-verifiable — modal focus starting on the safe/cancel control — passes on Linux and macOS and is exactly what surfaced F69's failure on Windows. Keyboard operability is a claim this project's accessibility RFCs make; this gap should be visible at the decision point, not only inside each row's fine print. |
| **`linux-wayland`** | Owner's manual row — outstanding. Outside what CI automation can cover; `linux-x11` (Xvfb) stands in for the CI-verified Linux row per `matrix-plan.md`. |

## Considered and not included above

Recorded here so their absence from the tables reads as a deliberate
judgment, not an oversight:

- **F67** (session persists less than `README.md` claims — `active_tab`/
  `explorer_roots` dead, directory tabs not persisted) — review 066: "not a
  Gate D input on current reading... a missing convenience, not silent loss
  of work the user did." Owner decision, post-Gate-D.
- **F68** (turning off "remember explorer directories" silently removes the
  per-file copy buttons) — same root cause and fix as F73 (see F73's row
  above); not listed separately here to avoid double-counting one blocker as
  two inputs. Still its own tracked ROADMAP entry.
- **F66** (outbound correspondence drafts tracked in a public repo) and
  **F71** (concurrent-agent working-directory hazard, adopted as standing
  practice) — process findings about how this evidence was produced, not
  about the product being evidenced. Neither changes what Gate D is
  assessing.
- **F65** (`sheets-diff` 2.3.0 clears the blocker that disabled `.xlsx`
  comparison) — a dependency-adoption decision explicitly sequenced to run
  *after* Gate D clears ("adopt when your acceptance matrix clears, not
  before" — their own words), not an input to the decision itself.
- **F49b** (`AppxManifest.xml`'s `MaxVersionTested` stays `10.0.19041.0`
  pending M5's Windows evidence) — that evidence now exists (M5-A/B/C ran
  on NT 10.0.26100), so this is **an action M5's evidence just enabled**
  (bump `MaxVersionTested` to the build actually tested), not a question
  bearing on the go/no-go itself. F60 (the Windows floor) is listed above
  because it *is* a decision input; F49b is the ceiling half of the same
  manifest and does not carry the same weight.

## Case-result summary (context, not a separate input)

Every CI-verified case executed across M5-A, M5-B, and M5-C passes on every
row it was run against, **except**:

- **P12** (Session/settings restart) — fails on every row, for real, because
  of F61 (already listed above as a blocker in its own right).
- **P07** (Explorer and directory report) — passes on Linux and macOS as a
  case (its own assertions hold); fails outright on Windows because of F70
  (also already listed above). The Windows/macOS P07 passes/fails both carry
  real product defects (F68/F70/F72/F73) registered separately per
  review 068 §4 — a case result says what the checks found, not what the
  finding means for Gate D, which is exactly why this list exists as a
  distinct document.
- **P11** (Keyboard and modal safety, CI-verifiable item only) — passes on
  Linux and macOS (modal focus correctly lands on Cancel); fails on Windows
  because of F69 (already listed above).

See `README.md`'s verdict and each row file for the full per-case detail.
