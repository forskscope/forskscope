# Review Request 064: M5-C Windows row — P03, P07, P11

**Governing RFC.** [RFC-078](../../rfcs/proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../rfcs/proposed/074-v1-release-stabilization-program.md)
**Governing handoff.** [`rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`](../../rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md)
**Scope of this request.** The **Windows row only** (`windows-11`/`windows-10`,
one shared CI host per `matrix-plan.md`) — P03, P07, P11, extending the
existing M5-A/M5-B `windows_harness.py`. The handoff also covers Linux,
macOS, and full evidence assembly (README.md verdict, Gate D input list,
artifacts.md); those are separate, parallel efforts this request does not
speak for — see §6 below for exactly what that means for this document's
scope.
**Candidate.** `0.167.0`, same published artifacts/digests as M5-A/M5-B.
**Baseline.** `main` at `b27545d` at handoff time; this row's work landed as
the commit series cited throughout, on top of a `main` that other parallel
M5-C rows (Linux, macOS) were also committing to concurrently — see §6's
note on that.

## 1. Prerequisite B's outcome — resolved favorably, before any P03/P07 evidence

Settled first, per the handoff's explicit instruction, via two committed,
real-CI diagnostics (`rowprobe`, not an evidence case — no `matrix-plan.md`
case ID, no `--break`, no assertion, pure diagnostic dump):

- CI run [`31936262847`](https://github.com/forskscope/forskscope/actions/runs/31936262847)
- CI run [`31936284361`](https://github.com/forskscope/forskscope/actions/runs/31936284361) (encoding-fixed re-run)

**Finding:** `hunk.rs`'s `div.diff-row[role="row"]` maps to UIA control type
**`DataItem`** — the same control type the content-cell child also gets, so
disambiguated by `ClassName`. `ClassName` on this WebView2 build mirrors the
DOM `class` attribute **directly and literally** (`class="diff-row"` reads
back as `class_name='diff-row'`, `class="cell"` as `class_name='cell'`,
`id="app-root"` as `automation_id='app-root'`) — confirmed empirically
against the real published artifact, not assumed. Every row's accessible
child count is exactly 2 (gutter `DataItem`, then content-cell `DataItem`;
the `aria-hidden` +/− mark span is correctly absent from the tree).

**Outcome:** row count and per-row geometry are both checkable on Windows,
matching (not merely approximating) Linux F34/`render_check.py`'s
`check_pane` — actually a *more* precise filter than Linux's AT-SPI "table
row" role match, since it keys on the literal CSS class rather than an
ARIA-role-derived label. P03 (§below) does the fuller Linux-parity geometry
check rather than stopping at RFC-078's stated "basic layout observation"
floor for this row.

## 2. Cases executed per row, with results

Both `windows-11` and `windows-10` share one CI host
(`windows-latest`) per `matrix-plan.md`; both evidence files record the
same runs.

| Case | Result | Evidence (CI run) |
|---|---|---|
| P03 — Compare layout and scrolling | **Pass** | [`31937225763`](https://github.com/forskscope/forskscope/actions/runs/31937225763) |
| P07 — Explorer and directory report | **Fail — candidate product defect, blocks the whole case** | [`31938459272`](https://github.com/forskscope/forskscope/actions/runs/31938459272) |
| P11 — Keyboard and modal safety | **Fail — candidate product defect** | [`31938755692`](https://github.com/forskscope/forskscope/actions/runs/31938755692) |

Full narrative for each case is in `docs/src/maintainers/release-evidence/0.167.0/windows-11.md`'s
new "M5-C" section (`windows-10.md` cross-references the same runs, per
that row's established convention).

**P03** — three separate launches (mirrors `p05`'s multi-launch shape):
row shape/alignment against `left_all_hunk_kinds.txt`/`right_*` (7 rows/pane,
uniform child count, content-cell x-origin, and row-extent uniformity — the
last one is the Windows-observable proxy for "short-row backgrounds span
the full widest-line area"); horizontal-scroll-mirror against
`left_long_line.txt`/`right_long_line.txt` (§4 below has the full story);
narrow-window usability (480px resize, content still present).

**P07 and P11** — both genuinely reached, genuinely executed, and both
found a real, reproducible defect rather than passing or hitting a harness
bug. See §5.

## 3. P11's decomposition as executed, and the keyboard-coverage statement

| Item | CI-verifiable? | What happened |
|---|---|---|
| (1) Keyboard checklist | **No** — manual | Not attempted; manual-outstanding, mirroring F45's shape |
| (2) Modal focus starts on safe/cancel | **Yes** — this is what P11 checks | **Executed. Found a real defect** — see §5 |
| (3) Global shortcuts inert behind a modal | **No** — needs a real keystroke | Not attempted; manual-outstanding |
| (4) Escape behaviour | **No** — needs a real keystroke | Not attempted; manual-outstanding |

(1)/(3)/(4) all need a real keystroke dispatched at the OS/window level and
observed to (not) do something — the same structural gap M5-B §3 already
established for P04's Enter-shortcut path and P06's double-reload: no
accessibility API can invoke a global `onkeydown` listener bound to no UI
element, on any platform. This is scoping, not a shortcut — the
decomposition itself, not a whole-case manual mark, is what the handoff
asked for, and it's what's in `windows_harness.py`'s `p11` docstring
verbatim.

Item (2) is what has a real data-safety consequence and is genuinely
checkable: `HasKeyboardFocus` is a plain UIA property, readable with
nothing synthesized. Reused P05's own conflict setup (apply a hunk, modify
the target externally, Save) to reach `OverwriteModal` — every one of this
app's destructive-action modals carries `autofocus: true` on its
Cancel-equivalent button (`OverwriteModal`, `BatchCopyModal`,
`ConfirmDirOpModal`, `ReloadModal`, `SwapModal`,
`ConfirmDiffOptionChangeModal`, `ConfirmSaveAsOverwriteModal` — confirmed
by reading every one, not sampled), so this one modal is representative of
the pattern, not a case picked to pass.

**Keyboard-coverage statement, stated plainly per the handoff's
instruction:** across items (1)/(3)/(4) here, P04's Enter-apply path
(M5-B), and P06's double-reload, **the documented keyboard interface has
no automated runtime coverage on any platform this program has evidence
for.** Keyboard operability is a claim this project's README and
accessibility RFCs make; this decomposition makes that gap explicit rather
than leaving it implied by an unqualified "Pass."

## 4. Falsifiability demonstrations, with observed output

| Case | Break-mode run | Result |
|---|---|---|
| P03 (geometry branch) | [`31937274686`](https://github.com/forskscope/forskscope/actions/runs/31937274686) | Fail (expected) — `FAIL(geometry --break): row geometry required an impossible misalignment to be present, and correctly found none - the real check above is not vacuous` |
| P07 | Not reached — the case fails before any `--break`-gated assertion (the initial-listing blocker, §5) | N/A |
| P11 | [`31938879519`](https://github.com/forskscope/forskscope/actions/runs/31938879519) | Fail — but for the same real-defect reason as normal mode, not `--break`'s own impossible-value branch (see §5's note) |

**The geometry branch specifically (handoff §3's Prerequisite A callback,
Windows side):** P03's row-geometry check (`check_pane_geometry`) is the
direct Windows analogue of Linux F34/`render_check.py`'s `check_pane`
(reused via Prerequisite B's resolution, §1). Its falsifiability is
demonstrated the same way every other case in this harness demonstrates it
against a black-box published artifact this slice cannot rebuild: assert
against a value the real, correct app can never produce (an impossible
non-empty misalignment list), rather than injecting a real defect the way
Linux's `inject_geometry_defect.py` can against a locally-built binary.
Confirmed failing correctly on real CI, cited above.

**Horizontal-scroll-mirror's own falsifiability chain is worth spelling out
in full, since building the check *was* a falsifiability investigation in
itself** — five committed diagnostic CI runs (`scrollprobe` v1–v5) before
any assertion could be written:

1. Locate the scroll containers by `AutomationId`/`ClassName` — **zero
   matches**, on real CI
   ([`31936784577`](https://github.com/forskscope/forskscope/actions/runs/31936784577)/[`31936787742`](https://github.com/forskscope/forskscope/actions/runs/31936787742)).
2. Fall back to a whole-tree `IScrollProvider` capability scan
   (`CurrentHorizontallyScrollable == True`) — **zero matches anywhere**,
   on real CI ([`31936889265`](https://github.com/forskscope/forskscope/actions/runs/31936889265)).
3. Fall back to real input synthesis: a plain vertical wheel — no effect;
   Shift+wheel and a native `WM_MOUSEHWHEEL` (raw `SendInput`, stdlib
   `ctypes`) — **both moved a row's rectangle**, but the very first attempt
   used an off-screen target point (a bug in the diagnostic itself: the
   cell's own accessible rectangle is *not* clipped to the viewport, so its
   naive midpoint landed at screen x=7743 on a ~1044px-wide window) — fixed
   and re-run.
4. With a correctly-targeted point: `WM_MOUSEHWHEEL` moved the left row's
   rectangle by a clean 1,000px, **and the right pane's row rectangle moved
   by the identical 1,000px at the same time** — confirmed twice
   (cumulative to 2,000px total) on real CI
   ([`31936958119`](https://github.com/forskscope/forskscope/actions/runs/31936958119)):
   `(33,15453)`/`(572,15991)` → `(-967,14453)`/`(-428,14991)` →
   `(-1967,…)`/`(-1428,…)`.

This is real, positive proof `install_hscroll_sync`'s mirror works — not
inferred, not assumed from the JS source alone. P03's actual check
(§2 above) reuses exactly this mechanism, then polls for settling (six
samples after the match) per RFC-078's "without feedback/jitter" wording —
an oscillation is itself a failure, so a single post-scroll sample would
not have been sufficient evidence.

## 5. Product defects found, registered not fixed

Both found while genuinely trying to make the corresponding case pass, not
invented to justify a Fail — both required real, patient, repeated CI
investigation before being confirmed rather than dismissed as harness bugs.

### Candidate defect A — Explorer's directory listing never renders any row on this Windows CI environment (P07)

Confirmed across **five real CI runs** and **four independent trigger
mechanisms**:

1. Restoring `last_left_dir`/`last_right_dir` from `settings.json` at
   launch — [`31937800743`](https://github.com/forskscope/forskscope/actions/runs/31937800743).
2. A real PathBar "edit path, commit" re-navigation to the *identical*
   path (ruling out a mount-time-vs-live-navigation race) —
   [`31938065443`](https://github.com/forskscope/forskscope/actions/runs/31938065443).
3. Clicking Home (`⌂`) to navigate to the real `C:\Users\<runner>`
   directory (ruling out anything specific to the freshly-created fixture
   directories) — [`31938290605`](https://github.com/forskscope/forskscope/actions/runs/31938290605).
4. Polling for up to 150s (ruling out mere slowness rather than
   never-happens — P06's own precedent that this app+runner combination
   can be surprisingly slow) — [`31938459272`](https://github.com/forskscope/forskscope/actions/runs/31938459272),
   final/authoritative: the accessible-text-node count never moved off the
   empty state's 54, not once, across the full budget.

`aligned.is_empty()` (`explorer/tree.rs`) is what actually renders the
empty-state message seen in every run's diagnostic dump ("Choose a file or
folder on each side to compare"). `compute_aligned_rows` genuinely receives
zero rows from both `DirectoryTree`s, on every attempt, for directories
proven to have real content by `mkdir` (the fixture) and by simply existing
(the real home directory). Root cause narrowed no further than: something
in `dioxus-swdir-tree`'s lazy-scan pipeline (`on_toggled` → `ScanRequest` →
`use_scan_driver`'s executor → `on_loaded` merge) never completes or never
delivers a result back into `tree_l`/`tree_r`, specifically on this
Windows CI environment. Could be a genuine cross-platform product defect
Windows CI simply never exercised before (no M5-A/M5-B case touched
Explorer at all), or an environment-specific limitation (a background
scan thread/executor this sandboxed runner blocks or never schedules).
Distinguishing the two needs a rebuild with added instrumentation, which
this slice's constraints forbid. Linux's equivalent P07 pass is recorded
elsewhere as CI-confirmed working, which is why this reads as a real,
platform-specific finding rather than a harness artifact common to all
three platforms.

**Consequence:** every P07 sub-check past the initial listing (statuses,
filters, deep-compare progress, per-file/batch copy — including the
wrong-base-directory defect the harness code is written to demonstrate,
per the handoff's F62-lesson emphasis on verifying manifest/backup bytes)
is unreached and unverified on Windows. The code exists in
`windows_harness.py`, written and believed correct against a *rendered*
listing, but nothing past the initial listing has been, or currently can
be, verified on this platform.

### Candidate defect B — `autofocus` does not move keyboard focus to a destructive modal's Cancel button on this environment (P11)

Confirmed on two independent CI runs — normal mode
([`31938755692`](https://github.com/forskscope/forskscope/actions/runs/31938755692))
and `--break` mode
([`31938879519`](https://github.com/forskscope/forskscope/actions/runs/31938879519)) —
**both fail identically**, for the same underlying reason. After polling
the full readiness budget, a whole-tree scan for any element reporting
`CurrentHasKeyboardFocus` found exactly two: the top-level OS window
itself, and the toolbar's **"Save merge result"** button — the control
that was focused *before* the modal opened, from clicking it to trigger
the conflict. Focus never moved to either modal action button (`Cancel` or
`Overwrite`).

Linux's equivalent check is recorded elsewhere as CI-confirmed working
both directions (Cancel focused, the destructive control not) — this looks
like a real Chromium/WebView2-specific difference in how the HTML
`autofocus` attribute is handled for an element mounted *after* initial
page load (a dynamically-inserted modal), not a harness defect: the check
is corroborated by finding a real, different, specific element (Save)
holding focus, not an absence of any signal.

**`--break`'s limitation, noted rather than hidden:** because the real
app's actual state is "neither button focused" (not "Cancel focused,
correctly"), both modes fail at the same earlier branch for the same
reason, before `--break`'s own "requires the impossible" branch is ever
reached — so `--break` does not currently demonstrate this specific case's
falsifiability in isolation *while the defect persists*. The check's
liveness is still evidenced a different way: it reads real, specific,
corroborated state and reports a genuine mismatch against what's required,
not a tautological or unconditional failure.

**Why this matters beyond a test result:** this is exactly the shape
RFC-078's own P11 text warns about — "a destructive modal whose focus
starts on the destructive action, not Cancel, is a hazard a screen-reader
user hitting Enter/Space immediately after the modal announces itself
would hit for real." Here focus doesn't even land on the destructive
action — it stays on the *background* Save button, behind the modal. That
raises a related, unconfirmed question worth flagging for the reviewer
even though this slice cannot test it (item (3), keyboard-not-verifiable):
if keyboard focus remains on a covered background control while a modal is
open, that's suggestive that global shortcuts might *not* be inert behind
a modal on this platform either — not confirmed, since testing it needs a
synthesized keystroke this program has established it cannot do reliably,
but worth the reviewer's attention given how directly the focus evidence
points at it.

Neither defect is fixed here, per the handoff's and this slice's explicit
"no product behaviour changes" constraint. Registered for the reviewer to
disposition — whether either becomes a new tracked ROADMAP finding is a
judgment call this evidence doesn't make for them.

## 6. Differences from the handoff, RFC-078, or the frozen plan

- **`matrix-plan.md` was not edited.** No difference from the frozen plan
  in scope or case-to-row mapping.
- **P03 exceeds RFC-078's stated Windows floor** ("a basic layout
  observation") by doing the full Linux-parity geometry check plus the
  first-of-its-kind scroll-mirror check, since Prerequisite B resolved
  favorably and the handoff explicitly invited attempting the fuller check
  when achievable. Not a deviation from the plan — the plan's own text
  frames "basic" as a floor, not a ceiling.
- **A genuine, unplanned operational difference worth reporting plainly:**
  this row's work happened on a `main` that other parallel M5-C efforts
  (Linux and macOS rows) were committing to **concurrently, in the same
  working directory** — not a separate clone or worktree. This surfaced
  twice, concretely:
  - One commit made by this effort (`d108e91`, "ci: M5-C P03 - probe
    Windows UIA horizontal ScrollPattern support") inadvertently bundled
    in another effort's concurrently-staged, uncommitted changes to
    `packaging/evidence/linux_harness.py` and
    `.github/workflows/m5-evidence-linux.yml` (227/2 lines), because a
    bare `git commit -m ...` with no pathspec commits everything staged
    in the shared index, not just the files this effort had just `git
    add`ed. No work was lost — the content is correctly in the repository,
    just under a commit message that doesn't mention it. Caught, and every
    commit from that point on in this effort used an explicit trailing
    `-- <pathspec>` on `git commit` (which commits only the named paths'
    current on-disk content, regardless of what else is staged) to prevent
    a repeat.
  - Separately, and more substantively: `packaging/evidence/windows_harness.py`
    itself was found, more than once, to already contain working P03/P07/P11
    implementations this effort had not written — matching this effort's
    own Prerequisite B/scroll-mechanism findings closely enough (including
    citing this effort's own CI run IDs) that the most coherent
    explanation is a second, concurrent process working the same
    assignment against the same shared repository state, rather than an
    unrelated contributor. This effort treated that code as collaborative
    output once verified: read it, dispatched real CI runs to confirm or
    refute it independently (never trusted by inspection alone — several
    fixes in the commit list above, e.g. the window-resize `move_window`
    fix and the scroll-probe coordinate-clamping fix, came from exactly
    this verification), and fixed what real CI showed was broken.
  - **Recommendation for future parallel M5-C-style slices:** either
    isolate concurrent per-row agents in separate git worktrees, or make
    the row assignment to a single agent explicit and exclusive before
    work starts. Sharing one working directory across concurrent agents
    is what produced both issues above; neither caused data loss here, but
    both were avoidable.

## 7. Executed gates

No Rust source was touched — this slice is harness-only
(`packaging/evidence/windows_harness.py`,
`.github/workflows/m5-evidence-windows.yml`, and the two Windows evidence
docs). `cargo fmt`/`cargo clippy`/`cargo test` were not run and are not
applicable to this change set. Python syntax was checked
(`python3 -m py_compile packaging/evidence/windows_harness.py`) before
every commit that touched the harness; no Python linter is established
elsewhere in this repo's harness files to run additionally.

Every CI run cited in this document was dispatched for real via
`gh workflow run` and its result read from the actual run log via
`gh run view --log` — none are inferred from reading the harness code
alone, per this program's standing "run it for real" discipline.

## 8. Unresolved issues and known limitations

- **Both candidate defects (§5) are open, unfixed, and their root cause is
  narrowed but not fully determined** — neither can be resolved further
  without a rebuild this slice's constraints forbid.
- **P07 is Fail as a whole on Windows**, not partially passing — every
  sub-check past the initial listing is unreached, not merely unverified
  in detail.
- **P11's `--break` mode does not currently demonstrate falsifiability in
  isolation** while candidate defect B persists (§5's note) — an honest
  limitation of the evidence, not a silently-accepted weaker check.
- **Items (1)/(3)/(4) of P11, and P07's "focused-pane keyboard behaviour,"
  remain manual-outstanding** — no automated runtime coverage exists for
  the documented keyboard interface on any platform this program has
  evidence for (repeating the keyboard-coverage statement, §3, because
  it bears on more than one case here).
- **Full M5-C evidence assembly (README.md verdict, Gate D input list,
  `artifacts.md`) is out of scope for this request** — it depends on the
  Linux and macOS rows' own results, which this effort does not own or
  speak for. What this request contributes toward that eventual assembly:
  two new Gate D input candidates (§5's defects A and B) and Prerequisite
  B's resolution (§1), for whoever assembles the final list to fold in.
- **This row's own manual sub-case (F45's Windows-11 prerequisite
  sub-case) is unaffected by this slice** — still owner-executed,
  outstanding, exactly as `matrix-plan.md` already records.

## 9. Requested review focus

1. **Are candidate defects A and B (§5) real product defects, or is there
   a plausible environment-specific explanation this evidence hasn't
   ruled out?** Both were investigated as thoroughly as this slice's
   constraints (no rebuild, published artifact only) allow; a rebuild
   with instrumentation is the natural next step for whichever the
   reviewer judges more likely to be a real, fixable defect versus a CI
   environment quirk.
2. **Is the horizontal-scroll-mirror check's methodology (§4) sound as a
   Gate D input**, given it relies on real input synthesis
   (`WM_MOUSEHWHEEL`) rather than an accessibility pattern, because no
   pattern exists for this container on Windows? This is a genuine
   first-of-its-kind check in this program with no precedent to compare
   its rigor against.
3. **The shared-working-directory concurrency issue (§6)** — whether the
   bundled-commit incident needs any further remediation beyond what's
   already recorded, and whether the recommendation (isolate parallel
   row agents) should become a standing instruction for future M5-style
   slices.
4. **Whether either candidate defect should become a new tracked ROADMAP
   finding now, or wait for the reviewer's own reproduction** — this
   evidence deliberately doesn't make that call.
