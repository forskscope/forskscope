# Platform Evidence — `macos-aarch64`

**Artifact filename:** `forskscope-v0.167.0-macos-aarch64.dmg`
**SHA-256:** `2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099`
**Source commit:** `cb6f5b6`
**Test date (UTC):** 2026-08-15
**Tester role:** implementer (M5-A), automated via CI (`macos-latest`) — no
manual macOS host exists in the owner's stated execution model
(`matrix-plan.md` §1)
**Host OS and version:** macOS 26.5.2 — **resolved runner image**
`macos26`, image version `20260728.0273.1` (recorded per review 057
§4.3's rolling-label caveat: `macos-latest` itself is not reproducible,
this resolved image is; `Darwin ... RELEASE_ARM64_VMAPPLE arm64` confirms
the runner is aarch64, matching this row's architecture)
**Architecture:** aarch64
**Display server / WebView runtime:** real macOS window server (the
runner is a full GUI-session VM, not headless) + WKWebView
**Install source and prerequisites:** published GitHub Release asset,
downloaded and digest-verified by CI before every case run; `.dmg`
mounted with `hdiutil attach`, `ForskScope.app` copied out, its
`Contents/MacOS/forskscope` binary launched directly

## Cases

| Case | Result | Evidence |
|---|---|---|
| P01 — Install and cold launch | **Pass** | CI run [`31853094639`](https://github.com/forskscope/forskscope/actions/runs/31853094639) |
| P02 — CLI file compare | **Pass** | CI run [`31853203173`](https://github.com/forskscope/forskscope/actions/runs/31853203173) |
| P09 — Mergetool | **Pass** | CI run [`31853204009`](https://github.com/forskscope/forskscope/actions/runs/31853204009) |
| P10 — Binary/XLSX fail-closed policy | **Pass** | CI run [`31853205096`](https://github.com/forskscope/forskscope/actions/runs/31853205096) |
| P04 — Merge, undo/redo, safe save | **Pass** (mouse path only — see §3 resolution below) | CI run [`31879443866`](https://github.com/forskscope/forskscope/actions/runs/31879443866) |
| P05 — External modification | **Pass** | CI run [`31879848097`](https://github.com/forskscope/forskscope/actions/runs/31879848097) |
| P06 — Async identity | **Pass** (sequential two-launch variant — see case notes) | CI run [`31883446824`](https://github.com/forskscope/forskscope/actions/runs/31883446824) |
| P08 — Persistence: Exit | **Pass** | CI run [`31879559456`](https://github.com/forskscope/forskscope/actions/runs/31879559456) |
| P08 — Persistence: Continue with defaults | **Pass** | CI run [`31879563832`](https://github.com/forskscope/forskscope/actions/runs/31879563832) |
| P08 — Persistence: Reset and back up | **Pass** | CI run [`31879567945`](https://github.com/forskscope/forskscope/actions/runs/31879567945) |
| P08 — Persistence: legacy migration (filesystem) | **Pass** | CI run [`31879767594`](https://github.com/forskscope/forskscope/actions/runs/31879767594) |
| P12 — Session/settings restart | **Fail** (real product defect found — session not persisted; see below) | CI run [`31881394791`](https://github.com/forskscope/forskscope/actions/runs/31881394791) |
| P03 — Compare layout and scrolling | **Pass** (basic layout observation, per RFC-078's macOS depth — see M5-C notes) | CI run [`31937970972`](https://github.com/forskscope/forskscope/actions/runs/31937970972) |
| P07 — Explorer and directory report | **Pass** (two real product defects and a harness/technique limitation found and registered — see M5-C notes) | CI run [`31941623347`](https://github.com/forskscope/forskscope/actions/runs/31941623347) |
| P11 — Keyboard and modal safety | **Pass** (modal-focus sub-case only — see M5-C notes) | CI run [`31937264321`](https://github.com/forskscope/forskscope/actions/runs/31937264321) |

Harness: `packaging/evidence/macos_harness.py` + `macos_ui.applescript`,
driven by `.github/workflows/m5-evidence-macos.yml` (`workflow_dispatch`,
one case per run). Every case downloads and digest-verifies the
**published** artifact itself — nothing here was built from source.

### P01

`--diagnostics` exited 0 and reported `ForskScope 0.167.0`; the harness's
own assertions (not just a printed summary line) confirmed the `OS:` line
contains `macos` and, when a `Home:` line is present, that it is redacted
to `***` rather than a literal path — the harness's stdout only echoes a
one-line summary of these checks, not the full diagnostic report text
verbatim, so this record states what was *asserted and passed*, not a
literal captured string. A plain cold launch (no args) produced a real,
non-blank 1024×677 window, confirmed via macOS's Accessibility API
(System Events).

### P02

`forskscope <left> <right>` rendered the F34 fixture pair
(`left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`) as 14 `AXRow`
accessible elements — `hunk.rs`'s `RowLeft`/`RowRight` each independently
carry `role="row"`, so the pinned 7-rows-per-pane fixture produces 14
total once both panes render. This asserts rendered content, not merely a
zero exit code.

### P09

Applied the fixture's first hunk via the "Use this change" button
(clicked through System Events' `AXPress`, addressed by its accessible
description, not synthetic keyboard/mouse input), then clicked the
toolbar's "Save merge result" button the same way. `<merged>`, pre-seeded
with an unmistakable placeholder, was confirmed overwritten with real
content (46 bytes) afterward.

### P10

`forskscope <left.xlsx> <right.xlsx>` (arbitrary bytes; classification is
by extension only, `core::file_kind::classify`) produced the fail-closed
message **"Spreadsheet comparison is temporarily disabled for security."**,
found by walking the accessible tree for that exact text — the message
reaching the user was asserted directly, not merely "no crash occurred."

## M5-B — the interaction cases (P04, P05, P06, P08, P12)

The five cases below all need real in-app interaction, not just launch
and CLI-argument checks — the harder slice, per the M5-B handoff. Same
artifact, same digests, same harness family (`macos_harness.py` +
`macos_ui.applescript`, extended with new commands and case functions,
not restructured).

### §3 resolution — P04's "keyboard and mouse"

Per the M5-B handoff §3, this was settled *before* execution, not during:
**the mouse path is CI-verifiable and was executed; the keyboard path is
structurally not CI-verifiable on any platform and was not attempted.**

The hunk's "Use this change" button (`aria_label: "Use this change (apply
left to right)"` in `hunk.rs`) has a real `onclick` handler. Invoking it
via `AXPress` (System Events' `perform action "AXPress"`, or the
equivalent `click` verb once the accessible element is found) calls the
exact same handler a real mouse click would — the same equivalent-path
argument M5-A already used for Save. The keyboard shortcut
(`Key::Enter => apply_focused_hunk(...)` in `app.rs`'s global
`onkeydown`) is a raw keydown listener with no accessible action to
invoke — there is no accessibility mechanism to exercise it, on any
platform, not just this one. Recorded as **manual-outstanding**, not
silently skipped and not claimed covered — see P04's own report below.

### P04

`forskscope <left> <right>` (plain 2-arg compare, F34's fixture pair —
*not* mergetool mode this time, so Save writes to `<right>` per
`compare.rs`'s `SaveDestination::RightInput`). Applied the first hunk via
the "Use this change" button's `AXPress`; the dirty marker ("unsaved",
`statusbar.rs`) appeared. Clicked Undo (toolbar) — the marker
disappeared. Opened the advanced panel ("More ▼") and clicked Redo — the
marker reappeared. Clicked Save — the marker disappeared again, `<right>`
was confirmed overwritten with real content (differs from its pre-edit
bytes), a `.bak` sibling existed whose bytes equaled the file's *pre-save*
content (the original `right` fixture, matching `save.rs`'s
`SiblingBak` policy), and no leftover `.right.txt.fsk-tmp` sidecar
remained after the successful save. **Keyboard-Enter apply path: NOT
executed** — see §3 resolution above.

### P05

Three fresh launches, each opening F34's fixture pair and applying the
first hunk (mouse/`AXPress`, same technique as P04) to make the tab dirty
before Save's toolbar button becomes clickable:

1. **Cancel.** While the app had `<right>` open, it was overwritten
   *outside the app* with different bytes. Clicking Save triggered
   `OverwriteModal` ("File changed on disk") — the file still held the
   externally-written bytes (Save did not silently win the race).
   Clicking Cancel closed the dialog; the file was still untouched.
2. **Overwrite.** Same setup; this time clicking "Overwrite" let the
   write proceed. The `.bak` sibling's bytes were confirmed to equal the
   bytes that were *just overwritten* — the externally-modified content,
   not the original pre-edit fixture content. This is the detail the
   handoff called out as the one that makes the check meaningful, not
   just "a `.bak` exists".
3. **Save As.** Same setup; this time "Save As" to a different path
   (`type_into`'s click-to-focus-then-keystroke on the plain `<input
   type="text">` path field — `set_value`'s direct `AXValue` write was
   tried first and also worked in this run, since a plain text field
   behaves differently from the font-size spinner that confirmed
   `set_value` a no-op; see the M5-B technique notes below). The original
   target file was confirmed still holding the externally-modified bytes
   (untouched by Save As), and the new path held the app's own merge
   output.

### P06

RFC-078 describes this case as opening a second compare in-app while the
first is still loading, in one process. That is **not what this run
does** — see the case's own extensive docstring in `macos_harness.py` for
the full account of why, but summarized: three shapes were tried, in
order, against real CI:

1. **In-app** (RFC-078's literal description): switch to the Explorer tab
   while tab 0 loads, pick a second pair's files there, open it as tab 1,
   close tab 0 mid-load. Hung 45s+ in accessibility queries against
   Explorer whenever another tab's load was in progress.
2. **Two concurrent processes** (the handoff's own pre-approved
   fallback). Still unreliable even after ruling out process-identity
   ambiguity outright via unambiguous PID-based addressing.
3. **What actually ran**: two launches that never overlap. Process A
   launches, gets a short real load window, and is terminated before
   process B ever starts; process B is then interacted with alone, using
   the same single-process pattern every other passing case in this file
   already relies on.

The real root cause of why (1) and (2) failed was found only after
retreating to (3): a generated diff pair's content simply never reached
the accessibility tree once the file crossed a size threshold between 30
and 100 lines (binary-searched in isolation, with nothing else running:
10 and 30 lines render correctly; 100 and 400 do not, consistently) — not
a process-concurrency problem at all. (1) and (2) were most likely
failing for this same reason. Revisiting the in-app design with correctly
-sized fixtures is a reasonable follow-up not attempted in this slice.

**What this run actually verifies**: process A exited cleanly
(`terminate()`) after a short real load window; process B, launched only
afterward with a distinct 30-line synthetic pair, showed its own correct
sentinel content (not blank, not crashed) — the falsifiable content check
the handoff's §6 requires (a bare "the process is still running" check
would be vacuous). Reloading twice in quick succession (rewriting the
right-hand file with a different sentinel between clicks) resulted in the
*second* reload's content being displayed, not the first's — `reload_tab`'s
`LoadToken` machinery exercised for real. **This is materially weaker
than RFC-078's description**: it cannot exercise concurrent-process
coexistence, let alone in-process async-task-identity confusion. Recorded
here plainly, not hidden behind the original case description. Per
review 056 §5.3's standing rule referenced in the handoff, this reduced
scope should be treated as informative context for any future P06 spot-
check on this platform, not a clean pass on the original case.

### P08

The highest-value case in this slice per the handoff §4 — all three
recovery-dialog choices, each on its own fresh, isolated `$HOME`
(`dirs_next::config_dir()` = `$HOME/Library/Application Support` on
macOS, confirmed against `dirs-next`'s own source before relying on it).

- **Exit** (future-schema session fixture, `Incompatible` outcome,
  actions Exit/Continue with defaults). Clicking Exit
  (`dioxus_desktop::window().close()`) — the process was confirmed to
  actually terminate: `proc.poll()` (the direct, non-racy child-reap
  check, reading this exact child's own wait status) returned a real
  return code, corroborated by `os.kill(pid, 0)` reporting no such
  process. This is the single most important assertion in this slice per
  the handoff — "an Exit that dismisses the dialog and leaves a zombie is
  a failure that looks like a pass" — and it was checked directly, not
  inferred from the dialog closing.
- **Continue with defaults** (same future-schema fixture). The app kept
  running, the dialog dismissed, and `session.json`'s bytes were
  confirmed byte-for-byte unchanged afterward — the write-disabled
  behaviour `session_resolve_future_version_disables_writes_and_preserves_bytes`
  already covers at the unit level, exercised here through the real
  dialog and a real subsequent wait.
- **Reset and back up** (corrupt session fixture — literal invalid JSON,
  `CorruptPreserved` outcome, actions Continue with defaults/Reset — no
  Exit offered, matching `corrupt_shows_continue_and_reset_but_not_exit`).
  The dialog dismissed, `session.json` was reset away from the corrupt
  bytes, and a `session.json.reset.bak` sibling was confirmed to hold the
  *original corrupt bytes* exactly (`ensure_reset_backup` in
  `persist/schema/repository.rs`).
- **Legacy migration (filesystem-only)**: the shipping v0 settings/session
  fixtures (`crates/forskscope-core/src/tests/fixtures/persistence/
  settings-v0.json`/`session-v0.json`, already sanitized `/tmp/fixtures/…`
  paths) migrated without loss — theme, language, font size, and both tab
  pairs were all confirmed present in the migrated payload — and both
  files got a versioned envelope plus a `.pre-v2.bak` matching the
  original v0 bytes exactly.

### P12

**Result: Fail — a real product defect was found (registered here, not
fixed; see the dedicated section below).** Settings (Theme → Night,
Language → 日本語, both via `select_popup_item`'s `AXPress`-on-the-
opened-dropdown's-`AXMenuItem` — see the technique notes below) *did*
survive a normal close (the window's own close button) and restart: a
Japanese label ("設定") rendered post-restart, and re-opening Settings
confirmed both values read back correctly. **Tab restoration did not
happen** — a tab opened via CLI at the first launch was not restored on
the no-args relaunch, because `session.json` was never written for it in
the first place (the product defect below). Font size was **not**
changed via the UI in this run: every accessibility technique tried
against the font-size spinner (direct `AXValue` write, `AXIncrement`,
keystroke after `set focused`, keystroke after a UI-element click,
keystroke after a real `click at {x,y}` with the process made frontmost)
was a confirmed no-op on readback — recorded as manual-outstanding,
mirroring F45's shape, not silently skipped.

The explicit-CLI-args-does-not-restore-old-tabs half of P12 (relaunching
with `<left> <right>` must show only the new pair, not the session's old
tab) was implemented and is exercised by the same case, but was not
separately re-verified once the tab-restore defect was found, since there
was nothing to restore in this run's session file to begin with — this is
flagged as an open item, not asserted as passing.

## M5-B — accessibility technique findings worth recording

Several real dead ends and their eventual fixes, useful for any future
case built on this harness:

- **Buttons with no `aria_label`** (Redo, More ▼, Save As, Exit, Continue
  with defaults, Continue without saving, Reset and back up, Cancel,
  Overwrite) *are* findable via `click_button`'s existing description
  -then-title fallback — WebKit does expose an accessible name derived
  from a button's inner text when no `aria-label` is set. The one
  apparent counter-example (the header's "Settings" button returning
  `NOT_FOUND` on a probe fired immediately after the window appeared)
  turned out to be the accessibility tree lagging the DOM right after
  initial mount, not a real gap — a second probe ~0.4s later in the same
  run found it fine, and `click_wait`'s poll loop already tolerates this.
- **`<select>` elements are `AXPopUpButton`, not `AXComboBox`**, but a
  direct `AXValue` write (`set value of e to "Night"`) and
  `perform action "AXIncrement"` are both confirmed silent no-ops against
  them (readback never changes). What works: click the popup (`AXPress`,
  opens its dropdown), then find the opened dropdown's option as a plain
  `AXMenuItem` element via an ordinary tree walk (title match) and invoke
  `perform action "AXPress"` on *that* — not the generic `click` verb,
  which structurally succeeds but does not change the underlying value
  for this control. The dropdown's options are not reachable via the
  standard `menu item X of menu 1 of popUpButton` reference syntax at all
  (`Can't get menu 1 of pop up button N ... Invalid index`) — this
  control is not a true native `NSPopUpButton`, only styled to look like
  one.
- **Text fields**: a plain `<input type="text">` (Save As's path field)
  accepted a direct `AXValue` write in the run that exercised it. The
  font-size `<input type="number">` spinner never did, under any
  technique tried (see P12 above) — text vs. number inputs appear to
  behave differently for this purpose, not yet fully explained.
- **`entire contents of w` and per-property queries can wedge or fail
  outright** against certain views/element combinations for reasons never
  fully root-caused (`dump_roles`, an internal-only debugging aid, was
  abandoned after exhausting several hypotheses — see
  `macos_ui.applescript`'s own comments for the investigation). The
  commands actually used by every real case (`count_rows`, `find_text`,
  `click_button`, `click_row`, `click_row_side`, `click_any`) all remained
  reliable throughout.
- **Two same-named processes**: addressing a specific one by PID
  (`"pid:<n>"`, `first process whose unix id is n`) is supported and
  works correctly, but did not turn out to be the fix for P06's
  concurrent-process attempt — that was the fixture-size issue above, not
  process-identity ambiguity. Kept as infrastructure since it's a real,
  useful capability regardless.

## M5-B — product defect found (registered, not fixed)

**Session state opened via CLI startup is never persisted if the tab is
never closed before the app quits — the reactive tabs-changed
`use_effect` meant to auto-save it does not actually write, while direct
(non-reactive) save call sites do.**

Found while developing P12, isolated via a dedicated diagnostic
(`recon_session_save`, not a scored case) that strips away every other
variable:

1. `forskscope <left> <right>` opens a tab. `session.json` is confirmed
   **absent** (not even created) 1.5s after the tab fully renders, even
   though `app.rs` registers:
   ```rust
   // Persist the session whenever the tab list changes.
   use_effect(move || {
       let _tabs = store.tabs.read();
       save_session(&store);
   });
   ```
2. Clicking the tab's own Close button — `close_tab` in `state/session.rs`,
   which calls `save_session(&store)` **directly**, not through the
   reactive effect — and `session.json` appears correctly right
   afterward, in the same process, same environment.

Two earlier hypotheses were investigated and ruled out before finding
this:

- *Missing config directory* (`dirs_next::config_dir()/forskscope` never
  created by anything in the shipped binary — confirmed true by grepping
  the whole crate, and a real gap on its own: neither `persist_session`
  (`state/session.rs`) nor `persist_settings` (`ui/view/settings.rs`)
  check the `Result` of `repo.save(...)`, both discard it with `let _ =`,
  so any write failure — directory-related or otherwise — is silent by
  construction). Pre-creating the directory did **not** fix the missing-
  session-file symptom on its own, ruling this out as the sole cause,
  though the silent-discard pattern remains real and worth fixing
  independently of this specific defect.
- *App Sandbox entitlements* — ruled out via `codesign -d --entitlements`,
  which came back empty (unsigned, no sandbox container restricting
  writes).

**Practical impact**: any user who runs `forskscope <left> <right>` from
the command line and quits without otherwise touching the tab list (the
single most ordinary CLI/`git difftool`-style workflow this application
exists for) loses that session silently — nothing is written to disk, no
error is shown, and the next launch has nothing to restore. Settings
changes made via the Settings dialog are unaffected (they persist through
a separate, synchronous, non-reactive call path) — this is specific to
session/tab-list persistence.

Reported here precisely, per the handoff's instruction, and **not fixed**
in this pass.

## Accessibility approach — a deviation from the Linux harness worth noting

The Linux harness (`linux_harness.py`) drives AT-SPI's `Action.do_action`
via `pyobjc`-equivalent Python GTK bindings (`gi.repository.Atspi`), which
are pre-installed via `apt`. macOS has no equivalent pre-installed Python
binding for the Accessibility API (`pyobjc` is not on the `macos-latest`
image and would need a `pip install` step — a new tooling dependency to
manage, and still needs its own accessibility grant separate from
whatever System Events already has). This harness instead drives macOS's
built-in `osascript`/AppleScript against System Events, invoking the same
kind of direct accessible action (`click`/`AXPress`) that AT-SPI's
`do_action` provides on Linux — see `macos_ui.applescript`'s header
comment for the full reasoning. This worked without any special CI setup:
`macos-latest`'s System Events UI scripting was already usable with no
permission wall encountered on any of the eight runs in this pass (see
"Falsifiability" below) — worth flagging as **not fully proven robust**
for a wider ongoing use, since it was observed to work on this specific
resolved image (`macos26`/`20260728.0273.1`) only.

## Falsifiability

Every case's `--break` mode was run and confirmed to fail for the
expected reason, per handoff §7:

| Case | Break-mode run | Result |
|---|---|---|
| P01 | [`31853183079`](https://github.com/forskscope/forskscope/actions/runs/31853183079) | Fail — `--diagnostics output does not start with 'ForskScope 0.0.0-impossible': ForskScope 0.167.0` |
| P02 | [`31853264441`](https://github.com/forskscope/forskscope/actions/runs/31853264441) | Fail — `compare view showed 14 AXRow elements, expected 99, within 45s` |
| P09 | [`31853265706`](https://github.com/forskscope/forskscope/actions/runs/31853265706) | Fail — real merge content (`alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n`) correctly rejected against a value real Save output can never produce |
| P10 | [`31853267021`](https://github.com/forskscope/forskscope/actions/runs/31853267021) | Fail — `could not find text containing 'this message cannot appear' within 45s` |
| P04 | [`31880080196`](https://github.com/forskscope/forskscope/actions/runs/31880080196) | Fail — `saved content b'alpha\nold-line\ngamma\nepsilon\nzeta\ninsert-line\n' != impossible expected b'this exact string can never appear in real merge output'` |
| P05 | [`31880082912`](https://github.com/forskscope/forskscope/actions/runs/31880082912) | Fail — `.bak content b'EXTERNALLY MODIFIED WHILE APP WAS OPEN\nsecond line\n' != impossible expected b'this exact string can never be the pre-overwrite content'` |
| P06 | [`31883530193`](https://github.com/forskscope/forskscope/actions/runs/31883530193) | Fail — `expected final sentinel 'ASYNC-IDENTITY-SENTINEL-PAIR-B-RELOAD-V1' never appeared` (the impossible expectation — the stale reload's content — was correctly not satisfied; real behaviour shows the second reload's content) |
| P08 — Exit | [`31880085680`](https://github.com/forskscope/forskscope/actions/runs/31880085680) | Fail (expected) — `process (pid 27125) exited with returncode 0 within 20s of clicking Exit - the real behaviour --break's impossible expectation ('still running') was checked against` — the exact falsifiability demonstration handoff §6 requires for Exit specifically |
| P08 — Continue with defaults | [`31880089230`](https://github.com/forskscope/forskscope/actions/runs/31880089230) | Fail (expected) — `session.json is unchanged, matching write-disabled behaviour - --break's impossible expectation ('bytes changed') was correctly not satisfied` |
| P08 — Reset and back up | [`31880092086`](https://github.com/forskscope/forskscope/actions/runs/31880092086) | Fail — `backup content b'{not valid json' != impossible expected b'this exact string can never be the original corrupt bytes'` |
| P08 — legacy migration | [`31880096494`](https://github.com/forskscope/forskscope/actions/runs/31880096494) | Fail — `migrated settings payload language 'ja' != expected 'impossible-language'` |
| P12 | [`31883545108`](https://github.com/forskscope/forskscope/actions/runs/31883545108) | Fail — `restored Theme 'VALUE: Night' != expected 'VALUE: Light'` |

## F46 — Gatekeeper: Blocked, not Pass, not Waived

Per `matrix-plan.md` §3 and the M5-A handoff, out of scope for this slice
to resolve and not attempted here. `gh release download` and CI's
checkout/download path never apply the quarantine extended attribute a
real browser/`curl`-to-Finder download would, so Gatekeeper never engages
regardless of what this workflow does — the `.dmg` mounted and the
binary launched cleanly in every run above, which is expected and
uninformative about signing/notarization posture, not evidence Gatekeeper
was satisfied. **No manual macOS host exists in the current execution
model, so F46 has no resolution path under current resourcing** — recorded
here as an explicit, unresolved Gate D input, exactly as `matrix-plan.md`
directs.

## Failures and issue links

None found. All four cases pass functionally on `macos-latest`
(`macos26`/`20260728.0273.1`, aarch64) — no P01/P02/P09/P10 product
defect was observed in this pass. F46 (Gatekeeper) remains open per
above, and is not a P01-vs-Pass/Fail case result — it is a separate,
structurally-unverifiable posture question.

## M5-B — Failures and issue links

One real product defect found, registered, not fixed: session state
opened via CLI startup is never persisted if the tab is never closed
before the app quits — see "M5-B — product defect found" above for the
full writeup. P12 is recorded as **Fail** for this reason, distinct from
a harness bug; Theme/Language restoration (the working half of P12) was
confirmed passing within the same run. P06 runs as a materially reduced-
scope variant of RFC-078's description (sequential, non-overlapping
launches instead of concurrent ones) — see P06's own section above for
why and what this does and doesn't verify; recorded as **Pass** for what
it actually checks, not for RFC-078's original description. P04's
keyboard-Enter apply path and P12's font-size UI change are both
recorded as manual-outstanding (structurally unreachable via
accessibility on any platform, for the former; five independent
accessibility techniques all confirmed no-ops, for the latter) — neither
silently skipped nor claimed covered.

## M5-B — Waivers

None.

## M5-C — visual/navigation cases and evidence assembly (P03, P07, P11)

Same artifact, same digests, same harness family (`macos_harness.py` +
`macos_ui.applescript`, extended with new commands and case functions, not
restructured). This slice's first, primary deliverable — per the M5-C
handoff's explicit instruction — was resolving F63 (whether macOS content
above a certain file size genuinely never reaches the accessibility tree,
or whether M5-B's harness simply gave up too soon) *before* gathering any
P03/P07 evidence against it.

### F63 resolution — harness artifact, not a product accessibility defect

M5-B found a diff pair's content stopped reaching the macOS accessibility
tree above a file-size threshold between 30 and 100 lines. Three real
dispatches, run in order, resolve this conclusively:

1. **`recon_f63_investigation`** ([`31936174801`](https://github.com/forskscope/forskscope/actions/runs/31936174801)) — a 200-line fixture with sentinels near the top and at line 150. A 90-second, zero-interaction poll loop (far longer than any case's normal timeout) never found the deep sentinel, and every call after it — `list_roles`, a keyboard-scroll attempt, follow-on `count_rows`/`find_text` — started hitting this harness's 20s per-call subprocess timeout. That is a different symptom from M5-B's own finding (a prompt, repeatable "0 rows"/"NOT_FOUND", never a timeout) — consistent with this investigation's own repeated heavy `entire contents of window` queries degrading the WebProcess/accessibility server's responsiveness over the run, not with genuinely empty content.
2. **`recon_f63_v2_single_call`** ([`31936446822`](https://github.com/forskscope/forskscope/actions/runs/31936446822)) — isolates the confound: one fresh launch, one `find_text` call for a line-60 sentinel in a 100-line fixture, given a 150-second timeout and nothing run before it. Result: **`FOUND` after 95.8 seconds.** The content reaches the accessibility tree well past the ~30–100 line threshold M5-B recorded — it is simply far slower to enumerate via this harness's bulk `entire contents of window` AppleScript technique than any case's default 15–45s timeout allows. The same run's `count_rows` call (issued first) returned `'0'` in only 1.3s, a real asymmetry worth resolving on its own.
3. **`recon_f63_v3_count_rows_alone`** ([`31936590400`](https://github.com/forskscope/forskscope/actions/runs/31936590400)) — isolates `count_rows` specifically, with a 150s timeout and nothing else run first. First call: `'0'` in 0.8s (fast, but wrong). A **second** `count_rows` call in the *same launch*, immediately after: **`'82'` in 24.0s** — a large, real, correct-shaped row count. The first call's bulk `entire contents of window` AppleEvent evidently returns fast but incomplete before WebKit's own accessibility-tree computation has caught up; a second call, benefiting from whatever computation the first triggered, returns far more complete (and still slow) results.

**Conclusion: F63 is a harness artifact, not a product accessibility
defect.** Content does reach the macOS accessibility tree for files well
past the previously-recorded threshold — this was never a case of content
being invisible to assistive technology. The real, evidenced cause is that
WebKit's own accessibility-tree computation for a sizeable view is
measurably slow to complete via this harness's bulk `entire contents of
window` AppleScript technique (tens of seconds, scaling with content), and
this program's default per-call timeouts (15–45s) were too short to let a
correct enumeration finish — the first query after a fresh window
sometimes returns fast-but-incomplete rather than blocking until genuinely
done.

**Practical consequence for this slice:** P03/P07 do not need this
large-fixture workaround at all. P03's multi-hunk requirement is already
satisfied by F34's small `all_hunk_kinds` fixture (14 rows, already proven
fast and reliable across P02/P04/P05/P09); P07's Explorer/directory-report
fixtures are small by nature. Retrofitting M5-B's own P06 (which uses a
much larger generated pair) with this understanding is a reasonable
follow-up, not attempted in this slice (out of scope, and P06 is not one
of this slice's assigned cases).

### P03 — Compare layout and scrolling

RFC-078 requires this case in full only on WebKitGTK; macOS WebKit gets "a
basic layout observation," matching `matrix-plan.md`'s Spot-check depth
for this row. What was actually attempted and found, real dispatches
throughout:

- **Multi-hunk rendering**: F34's `all_hunk_kinds` fixture renders 14
  `AXRow` elements across multiple hunks, matching P02's own established
  count.
- **Word wrap**: the toggle (`aria_pressed`-carrying, inside the toolbar's
  "Advanced" disclosure) toggles on and off without breaking the view —
  rows remain present and correctly counted after each toggle. A real
  dispatch found this control does not respond to `click_button`'s
  `AXButton`-role-filtered search via either its `aria_label` or its inner
  text — `aria_pressed` plausibly maps it to a different accessibility
  role, the same class of ARIA-state-driven role change already seen
  elsewhere in this program (`<select>` → `AXPopUpButton`, not
  `AXComboBox`). `click_any` (no role filter) was used instead.
- **Narrow window**: resizing the window to 480×500 (well below the
  fixture's natural content width) still renders all 14 rows — the view
  stays usable, not blank.
- **Horizontal-scroll-mirror — attempted for real, not skipped.** This is
  the item with no precedent anywhere in this program (per the handoff),
  so full effort went into it before falling back to "basic." Two
  dedicated recon rounds (`recon_p03_scroll`,
  [`31936939543`](https://github.com/forskscope/forskscope/actions/runs/31936939543)) against a wide-line fixture (forcing `.diff-col-left`/`.diff-col-right`'s `overflow-x:auto` to genuinely overflow) found: a plain child-tree walk (`list_roles`) reports `AXScrollArea=1` but `AXScrollBar=0` — no ordinary `AXScrollBar` children at all, and only **one** `AXScrollArea` total (not two, so even distinguishing per-pane scroll state structurally is unclear from this alone); the standard NSAccessibility fallback — a scroll area's bars exposed as attribute-valued references (`AXHorizontalScrollBar`/`AXVerticalScrollBar`), not ordinary children — resolved to `missing value` for both orientations on the one `AXScrollArea` that does exist (`scroll_area_bars`, a new command added for this). **No accessibility-exposed scroll-position property was found for this content on macOS, via any technique this harness's established AppleScript/System Events approach can reach.** This is recorded as a genuine, evidenced platform/technique limitation — the horizontal-scroll-mirror assertion itself is **not executed** on macOS, consistent with the handoff's own framing that this item may or may not be solved on every platform yet.

`--break`: asserts an impossible row count (99) immediately after the
word-wrap toggle — real behaviour shows 14, so this correctly fails. CI
run [`31941710391`](https://github.com/forskscope/forskscope/actions/runs/31941710391) — `FAIL: after toggling word wrap, row count is '14', expected 99`.

Normal-mode CI run: [`31937970972`](https://github.com/forskscope/forskscope/actions/runs/31937970972) — **Pass.**

### P07 — Explorer and directory report

Mostly automatable through the accessibility-action approach established
by M5-A/M5-B, but this slice's most extensively iterated case by far —
real, extensive CI iteration (dozens of dispatches) was needed to separate
real defects from harness technique problems, exactly as the handoff
anticipated. Three genuine findings resulted, all disclosed in full below
and in the harness's own inline comments (`packaging/evidence/
macos_harness.py`'s `p07` docstring and case body).

**Navigation/history.** Executed via `PathBar`'s "↑" (Go up one directory)
and the Back/Forward toolbar buttons — ordinary, single `AXPress` clicks —
but **only reliably on the LEFT Explorer pane.** Directory-descent via a
directory row (a real double-click, two positioned `click at {x,y}`
events) did not trigger `tree.rs`'s `ondoubleclick` at all in a real
dispatch; `PathBar`'s edit-path mode (type a path into the revealed
`<input>`, submit via Return or blur) also never produced real navigation
despite `find_focused` confirming the field genuinely had focus, unlike
Save As's outwardly-identical field in P05, where the same techniques
demonstrably worked. Separately — see **Finding 3** below — the RIGHT
pane's own button clicks were found unreliable via three further real
dispatches. Only the left pane's button clicks, and row picks
(`click_row_side`) on *either* pane, proved reliable throughout.

**Equal/different/one-sided statuses, deep-compare stats, and filters.**
All confirmed via a five-file fixture (`aaa-changed.txt` Changed,
`equal.txt` Equal, `left-only.txt` LeftOnly, `right-only-1.txt`/
`right-only-2.txt` RightOnly ×2): all five entries findable, the summary
stats line present, and the Different (default)/All/Equal-only filters
each showing the expected subset (`equal.txt` absent under the default
filter, present under "All"; `left-only.txt` absent under "Equal only").

**"Focused-pane keyboard behaviour" was NOT executed** — it hits the same
structurally-not-CI-verifiable limitation as P04's keyboard-Enter path and
P11's items 1/3/4 (handoff §6): F6 pane-toggle and the aligned tree's
arrow-key navigation are raw `onkeydown` handling with no accessible
action to invoke. Recorded here, not silently skipped.

**Per-file and batch copy: manifest CONTENTS and backup BYTES, not just
"it reported success"** (handoff §5 / F62's lesson) — and where **Finding
2** below was actually discovered. A per-file copy's real `.bak` backup
and overwritten content were read and verified against the true pre-copy
bytes; a batch copy's manifest JSON was read from disk (with a real
dispatch catching and fixing a bug in the harness's own manifest lookup,
which initially picked up the per-file copy's own earlier manifest
instead of the batch's new one) and all 3 entries verified by name,
outcome, and real backup/content bytes.

**Finding 1 — PRODUCT DEFECT, registered, not fixed: Back destroys
Forward history.** Clicking Back correctly returns to the previous
directory, but Forward is then permanently disabled — not "nothing to go
forward to yet." Root cause, traced in source: `NavHistory::back()`
(`dir_pane.rs`) only decrements an index, leaving `entries` untouched, so
`can_forward()` should read `true` afterward. But `explorer.rs`'s
`on_back` handler calls **both** `history.write().back()` (moves the
index) **and then** `navigate_to()` with the popped path — and
`navigate_to` *unconditionally* calls `history.write().push(path)` too.
`push`'s own re-entrancy guard (`if entries.last() == path { return }`)
does not save this, since `entries.last()` is still the not-yet-truncated
forward entry, not equal to the path Back just navigated to — so `push`
truncates `entries` and re-appends, destroying the forward entry. Any
Back click destroys that pane's Forward history. Not macOS-specific —
shared `dir_pane.rs`/`explorer.rs` code, so this almost certainly affects
every platform. Confirmed via real dispatch (Forward reports `DISABLED`
immediately after a correct Back); `--break` asserts the impossible
opposite (Forward enabled after Back) and correctly fails.

**Finding 2 — PRODUCT DEFECT, registered, not fixed: per-row copy buttons
use Explorer's remembered pane directory, not the compare root.**
`DeepRow`'s per-row copy buttons (`deep_compare.rs`) compute their src/dst
paths from `store.settings.read().last_left_dir`/`last_right_dir`
(Explorer's own "remembered pane directory" setting) — **not** from the
deep-compare view's own `left_root`/`right_root` props (the actual roots
being compared). `BatchCopyButtons` does **not** share this defect — a
real dispatch confirmed its manifest entries' `src` genuinely came from
`right_root`, not `last_right_dir`, so only the per-row path is affected.
Concretely demonstrated: with `last_right_dir` unable to track the actual
right compare root (Finding 3 means the right pane can never be
navigated there), a per-row "Copy to right" click landed at
`$HOME/aaa-changed.txt` — verified via a real backup and overwrite —
while `root-b/aaa-changed.txt`, the file the deep-compare view was
actually showing as "Changed," was verified completely untouched. **No
error surfaces to the user** — the per-row copy simply writes to the
wrong place, silently. This is exactly the class of silent, destructive
mismatch the handoff's F62-lesson emphasis on real backup/manifest
verification exists to catch, and it would not have been caught by a
check that only confirmed "the operation reported success."

**Finding 3 — HARNESS/TECHNIQUE LIMITATION, not a product defect: the
right Explorer pane's controls do not respond to this harness's `click`
technique.** Three real dispatches — two occurrence indices (`↑` at
positions 2 and 3), both click orderings (right-pane-first and
left-pane-first) — conclusively found the right pane's own PathBar
buttons report `CLICKED` (the AppleScript `click` verb structurally
succeeds) but never produce the expected navigation effect, while the
identical technique against the left pane's buttons works every time.
This is recorded as a genuine, evidenced technique limitation specific to
this harness's approach on this platform, not a product defect — it rules
out an entire technique family (right-pane button-driven navigation) for
any future work on this control. Worked around by design: `last_left_dir`
is kept genuinely correct via the left pane's own reliable navigation
(exercising Finding 1 for real in the process); `last_right_dir` is seeded
to `$HOME` and never navigated, which is exactly what exercises Finding 2
for real rather than working around it.

**Open item, not resolved here (review 068 §6):** this harness cannot
currently distinguish "AppleScript's synthesized `click` doesn't reach the
right pane's buttons" from a genuine accessibility gap in that pane — if a
real VoiceOver user also cannot operate those controls, that is a product
defect in an area this project makes explicit accessibility claims about,
not a harness quirk. (Separately, and for the same underlying reason —
this program's AppleScript/System Events techniques exercise a different
query/interaction path than VoiceOver's own incremental navigation model —
F63's resolution above, based on bulk `entire contents of window` fetches,
says nothing about whether a real screen-reader user experiences
comparable enumeration latency on a large view; that is likewise an
unmeasured question, not a finding either way.) Neither is something this
slice's constraints allow resolving; both are recorded so a future macOS
slice does not read "harness/technique limitation" as "product is fine."

Normal-mode CI run: [`31941623347`](https://github.com/forskscope/forskscope/actions/runs/31941623347) — **Pass** (for what this case's decomposition actually verifies — both defects and the technique limitation are registered, not hidden inside a passing result). `--break`: flips the batch's existing-destination backup-bytes check to an impossible expected value; CI run [`31941706567`](https://github.com/forskscope/forskscope/actions/runs/31941706567) correctly failed on the *earlier*, more fundamental Forward-disabled-after-Back assertion first (the same real defect Finding 1 registers) — `FAIL (expected, --break): Forward is disabled after Back ('DISABLED: →') - the real (defective) behaviour --break's impossible expectation ('enabled') was checked against.`

### P11 — Keyboard and modal safety (the CI-verifiable sub-case only)

RFC-078's P11 has four sub-items (handoff §6). Decomposition, as executed:

| Item | CI-verifiable? | Executed? |
|---|---|---|
| 1. Execute the maintained keyboard checklist | **No** | Manual-outstanding |
| 2. Modal focus starts on the safe/cancel action for destructive operations | **Yes** | **Executed — this case** |
| 3. Global shortcuts do not affect the background view while a modal is open | **No** | Manual-outstanding |
| 4. Escape behaviour is consistent | **No** | Manual-outstanding |

Items 1, 3, and 4 all require a real keystroke to test — the same
structurally-not-CI-verifiable limitation M5-B's P04 already established
for the keyboard-Enter apply path: no accessibility API can invoke a raw
global `onkeydown` handler bound to no actionable UI element. Recorded
here as owner-executed/manual-outstanding, mirroring F45's shape — not
silently skipped, not claimed covered.

Item 2 is CI-verifiable because focus **position** is exposed through the
accessibility tree without synthesizing any input at all. Two techniques
were tried: `focused_element` (the aggregate `AXFocusedUIElement` pointer,
queried at the process level, then window level) resolved to `missing
value` at the process level and errored outright at the window level, for
this WKWebView-hosted content — a real dispatch confirmed this. `find_focused`
(a per-element `AXFocused` boolean walk over a role's matching elements,
not dependent on any aggregate pointer) is what actually works. Using
P05's technique (external modification + Save) to open `OverwriteModal`
("File changed on disk") — a genuinely destructive-operation-adjacent
modal, since confirming it discards the externally-changed file's content
— `find_focused` confirmed the "Cancel" button (the safe action, which
carries `autofocus: true` in `ui/overlay/modals/file.rs`) reports
`AXFocused=true`, and "Overwrite" (the destructive action, no autofocus)
does not.

**Consequence, stated plainly per the handoff:** the documented keyboard
interface has no automated runtime coverage on any platform. Keyboard
operability is a claim this project makes in its README and its
accessibility RFCs; only the one sub-item with a genuine data-safety
consequence (destructive-modal focus position) has real, automated,
CI-verified coverage anywhere in this program.

Normal-mode CI run: [`31937264321`](https://github.com/forskscope/forskscope/actions/runs/31937264321) — **Pass** — `OK: destructive-operation modal ('File changed on disk') opened with keyboard focus on the safe/cancel action ('Cancel'), not the destructive one ('Overwrite'), confirmed via a per-element AXFocused boolean walk with no input synthesized.`

`--break`: asserts focus is on the destructive "Overwrite" action instead
of "Cancel" — real behaviour shows "Cancel," so this correctly fails. CI
run [`31941714393`](https://github.com/forskscope/forskscope/actions/runs/31941714393) — `FAIL: focused element 'FOCUSED: Cancel' does not name the expected safe/cancel action 'Overwrite'`.

## M5-C — Falsifiability

| Case | Break-mode run | Result |
|---|---|---|
| P03 | [`31941710391`](https://github.com/forskscope/forskscope/actions/runs/31941710391) | Fail (expected) — `after toggling word wrap, row count is '14', expected 99` |
| P07 | [`31941706567`](https://github.com/forskscope/forskscope/actions/runs/31941706567) | Fail (expected) — `Forward is disabled after Back ('DISABLED: →') - the real (defective) behaviour --break's impossible expectation ('enabled') was checked against` |
| P11 | [`31941714393`](https://github.com/forskscope/forskscope/actions/runs/31941714393) | Fail (expected) — `focused element 'FOCUSED: Cancel' does not name the expected safe/cancel action 'Overwrite'` |

## M5-C — Failures and issue links

**F63 is resolved: harness artifact, not a product accessibility
defect** — see the dedicated section above for the full, three-dispatch
investigation. No case result depends on F63 remaining open.

**Two real product defects found via P07, registered, not fixed:**

1. Explorer's Back button destroys that pane's Forward history —
   `explorer.rs`'s `on_back` handler's `navigate_to()` call unconditionally
   re-pushes onto the same history `NavHistory::back()` just rewound,
   truncating the forward entry. Not macOS-specific.
2. `DeepRow`'s per-row copy buttons use Explorer's remembered pane
   directory (`last_left_dir`/`last_right_dir`), not the deep-compare
   view's own compare roots — a mismatch (trivially easy to trigger, since
   nothing about the aligned-view picker requires navigating into a root
   first) silently writes to the wrong location with no error shown.
   `BatchCopyButtons` does not share this defect.

**One harness/technique limitation found via P07, registered:** the right
Explorer pane's PathBar buttons do not respond to this harness's `click`
(AXPress) technique the way the left pane's do, confirmed via three real
dispatches. Rules out button-driven navigation on the right pane for any
future work on this harness; row picks (`click_row_side`) remain reliable
on both panes.

P03's horizontal-scroll-mirror assertion is **not executed** on macOS — a
genuine, evidenced platform/technique limitation (no accessibility-exposed
scroll-position property was found for this content via any technique
tried), not a skipped or weakened check.

P11 items 1/3/4 (keyboard checklist, global-shortcut inertness behind a
modal, Escape consistency) are **not executed** — structurally not
CI-verifiable on any platform, recorded as owner-executed/manual-
outstanding, mirroring F45's shape. P07's "focused-pane keyboard
behaviour" hits the same limitation.

## M5-C — Waivers

None.

## Waivers

None.
