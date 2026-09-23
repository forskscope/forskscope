# Review Request 066: M5-C macOS row — P03, P07, P11

**Governing RFC.** [RFC-078](../../rfcs/proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../rfcs/proposed/074-v1-release-stabilization-program.md)
**Governing handoff.** [`rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`](../../rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md)
**Scope of this request.** The **macOS row only** (`macos-aarch64`, per
`matrix-plan.md`) — P03, P07, P11, extending the existing M5-A/M5-B
`macos_harness.py`/`macos_ui.applescript`. The handoff also covers Linux,
Windows, and full evidence assembly (README.md verdict, Gate D input list,
`artifacts.md`); those are separate, parallel efforts landing on the same
`main` concurrently with this work (see review requests 064 and 065) — this
request does not speak for them.
**Candidate.** `0.167.0`, same published artifacts/digests as M5-A/M5-B.
**Baseline.** `main` at `b27545d` at handoff time; this row's work landed as
the commit series cited throughout, on top of a `main` other parallel M5-C
rows were also committing to concurrently.

## 1. F63's resolution — harness artifact, not a product accessibility defect

Resolved first, per the handoff's explicit instruction, before any P03/P07
evidence was gathered. Three real dispatches:

1. **`recon_f63_investigation`** — CI run [`31936174801`](https://github.com/forskscope/forskscope/actions/runs/31936174801). A 200-line fixture, sentinels near the top and at line 150. A 90-second, zero-interaction poll never found the deep sentinel, and every call *after* it (`list_roles`, a keyboard-scroll attempt, follow-on `count_rows`/`find_text`) started hitting the harness's 20s per-call timeout — a different symptom from M5-B's original finding (a prompt, repeatable "0"/"NOT_FOUND," never a timeout), consistent with this investigation's own repeated heavy queries degrading the WebProcess's responsiveness over the run, not with genuinely empty content.
2. **`recon_f63_v2_single_call`** — CI run [`31936446822`](https://github.com/forskscope/forskscope/actions/runs/31936446822). Isolates the confound: one fresh launch, one `find_text` call, a 150s timeout, nothing run first. Result: **`FOUND` after 95.8 seconds** — content reaches the tree well past the ~30–100 line threshold M5-B recorded, just far slower to enumerate than any case's default timeout allows. The same run's `count_rows` (issued first) returned `'0'` in 1.3s — a real asymmetry.
3. **`recon_f63_v3_count_rows_alone`** — CI run [`31936590400`](https://github.com/forskscope/forskscope/actions/runs/31936590400). Isolates `count_rows` with a 150s timeout. First call: `'0'` in 0.8s (fast, wrong). A second call, same launch, immediately after: **`'82'` in 24.0s** — a large, correct-shaped count. The first bulk AppleEvent returns before WebKit's own accessibility-tree computation catches up; a second call, benefiting from that computation, returns far more complete results.

**Conclusion: F63 is a harness artifact.** Content does reach the macOS
accessibility tree for files well past the previously-recorded threshold —
this was never content invisible to assistive technology. The real cause:
WebKit's own accessibility-tree computation for a sizeable view is
measurably slow to complete via this harness's bulk `entire contents of
window` AppleScript technique (tens of seconds), and this program's
default per-call timeouts (15–45s) were too short to let a correct
enumeration finish — the first query after a fresh window can return
fast-but-incomplete rather than blocking until done.

**One honest caveat, not verified here:** VoiceOver's own real navigation
model queries the tree incrementally, not via a blanket bulk fetch the way
this harness's technique does — this program has no direct evidence about
whether a real screen-reader user experiences comparable latency on the
same content. Left as an open, unverified question, not a finding either
way.

**Practical consequence:** P03/P07 never needed the large-fixture
workaround at all — P03's multi-hunk requirement is satisfied by F34's
small, already-proven `all_hunk_kinds` fixture (14 rows); P07's fixtures
are small by nature. Retrofitting M5-B's own P06 (which uses a much larger
generated pair) with this understanding is a reasonable follow-up, **not
attempted here** — P06 is not one of this slice's assigned cases.

## 2. Cases executed, per case, with results

| Case | Normal-mode result | Break-mode result |
|---|---|---|
| P03 | **Pass** (basic layout observation) — [`31937970972`](https://github.com/forskscope/forskscope/actions/runs/31937970972) | **Fail (expected)** — [`31941710391`](https://github.com/forskscope/forskscope/actions/runs/31941710391) |
| P07 | **Pass** (two product defects + a harness/technique limitation found and registered) — [`31941623347`](https://github.com/forskscope/forskscope/actions/runs/31941623347) | **Fail (expected)** — [`31941706567`](https://github.com/forskscope/forskscope/actions/runs/31941706567) |
| P11 | **Pass** (modal-focus sub-case only) — [`31937264321`](https://github.com/forskscope/forskscope/actions/runs/31937264321) | **Fail (expected)** — [`31941714393`](https://github.com/forskscope/forskscope/actions/runs/31941714393) |

Full narrative for every case is in
`docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md`'s new
`## M5-C` sections — this request summarizes, that file is the record.

**P07 was, by a wide margin, this slice's most extensively iterated case**
— dozens of real CI dispatches were needed to separate genuine product
defects from harness-technique problems, exactly the kind of real
iteration the handoff anticipated. That iteration is preserved in
`packaging/evidence/macos_harness.py`'s `p07` docstring and inline
comments (and in this repository's commit history) rather than silently
smoothed over, since it rules out entire technique families (a real
double-click via two positioned clicks; `PathBar`'s edit-path-mode typing
via `type_into`/`set_value`/`Return`/blur; `perform_action`'s explicit
AXPress against a checkbox) for any future work on this control family.

### P03 — Compare layout and scrolling

Mandatory in full only on WebKitGTK; macOS gets "a basic layout
observation" per RFC-078 and `matrix-plan.md`'s Spot-check depth. What was
actually attempted (not the minimum): F34's 14-row multi-hunk fixture
renders correctly; the word-wrap toggle (found via `click_any` after
`click_button`'s `AXButton`-filtered search failed — the toggle's
`aria_pressed` plausibly maps it to a different accessibility role, same
class as `<select>` → `AXPopUpButton`) toggles on/off without breaking the
view; a narrow (480×500) window still renders all 14 rows.

**Horizontal-scroll-mirror was attempted for real** — the item with no
precedent anywhere in this program — via a dedicated recon
(`recon_p03_scroll`, [`31936939543`](https://github.com/forskscope/forskscope/actions/runs/31936939543)) against a wide-line fixture. Result: a plain child-tree walk found `AXScrollArea=1`, `AXScrollBar=0` (no ordinary scrollbar children, and only one scroll area total — not two); the standard NSAccessibility fallback (`AXHorizontalScrollBar`/`AXVerticalScrollBar` as attribute-valued references on the scroll area) resolved to `missing value` for both orientations. **No accessibility-exposed scroll-position property was found for this content on macOS, via any technique tried.** This is recorded as a genuine, evidenced platform/technique limitation — the assertion itself is not executed on macOS, not silently skipped.

### P07 — Explorer and directory report

Navigation/history, equal/different/one-sided statuses, deep-compare
stats and filters, and per-file/batch copy (manifest CONTENTS and backup
BYTES verified, not just reported success, per handoff §5/F62's lesson)
all executed against a five-file fixture. "Focused-pane keyboard
behaviour" was **not executed** — same structural limitation as P04's
keyboard-Enter path and P11 items 1/3/4 (§3 below).

Full decomposition and all three findings (two product defects, one
harness/technique limitation) are in §5 and §6 below.

### P11 — Keyboard and modal safety

Decomposed per handoff §6 — see §3 below for the executed decomposition.
Only item 2 (modal focus on the safe/cancel action) is CI-verifiable and
was executed, using `find_focused` (a per-element `AXFocused` boolean
walk) after `focused_element` (the aggregate `AXFocusedUIElement`
pointer) was found to resolve to `missing value`/error out for this
WKWebView-hosted content on a real dispatch.

## 3. P11's decomposition as executed, and the keyboard-coverage statement

| Item | CI-verifiable? | Executed? |
|---|---|---|
| 1. Execute the maintained keyboard checklist | No | Manual-outstanding |
| 2. Modal focus starts on the safe/cancel action | **Yes** | **Executed** |
| 3. Global shortcuts inert behind a modal | No | Manual-outstanding |
| 4. Escape behaviour is consistent | No | Manual-outstanding |

Items 1/3/4 all require a real keystroke — the same structurally-not-
CI-verifiable limitation M5-B's P04 already established (no accessibility
API can invoke a raw global `onkeydown` handler bound to no actionable UI
element). Recorded as owner-executed/manual-outstanding, mirroring F45's
shape — not silently skipped, not claimed covered. P07's "focused-pane
keyboard behaviour" hits the identical limitation and is recorded the same
way.

**Keyboard-coverage statement, per the handoff:** the documented keyboard
interface has no automated runtime coverage on any platform. Keyboard
operability is a claim this project makes in its README and its
accessibility RFCs; only the one P11 sub-item with a genuine data-safety
consequence (destructive-modal focus position) has real, automated,
CI-verified coverage anywhere in this program.

## 4. Falsifiability demonstrations with observed output

| Case | Break-mode run | Observed output |
|---|---|---|
| P03 | [`31941710391`](https://github.com/forskscope/forskscope/actions/runs/31941710391) | `FAIL: after toggling word wrap, row count is '14', expected 99` |
| P07 | [`31941706567`](https://github.com/forskscope/forskscope/actions/runs/31941706567) | `FAIL (expected, --break): Forward is disabled after Back ('DISABLED: →') - the real (defective) behaviour --break's impossible expectation ('enabled') was checked against.` |
| P11 | [`31941714393`](https://github.com/forskscope/forskscope/actions/runs/31941714393) | `FAIL: focused element 'FOCUSED: Cancel' does not name the expected safe/cancel action 'Overwrite'` |

P07's break-mode run is worth calling out specifically: it flips the
batch copy's backup-bytes expectation to an impossible value, but the run
correctly failed *earlier*, on the Back/Forward defect assertion (Finding
1, §6) — itself a real, working falsifiability demonstration, since real
(defective) behaviour never satisfies the break-mode's "Forward enabled
after Back" expectation either. Both assertions in this case are
independently falsifiable; the break-mode run happened to exercise the
first one it reaches.

The geometry/alignment-branch falsifiability item (handoff §3
Prerequisite A: `check_pane`'s content-cell x-origin comparison) is
**out of scope for this request** — it belongs to the Linux row (review
065), not macOS; RFC-078 requires P03 in full only on WebKitGTK, and
macOS's own P03 depth is "basic layout observation" per §2/§9 above, which
does not include per-row geometry precision.

## 5. Product defects found, registered, not fixed

**Defect 1 — Explorer's Back button destroys that pane's Forward
history.** Root cause, traced in source (`crates/forskscope-ui/src/ui/
view/explorer.rs`, `crates/forskscope-ui/src/ui/view/dir_pane.rs`):
`NavHistory::back()` only decrements an index, leaving `entries`
untouched — `can_forward()` (`idx+1 < entries.len()`) should read `true`
afterward. But `explorer.rs`'s `on_back` handler calls **both**
`history.write().back()` **and then** `navigate_to()` with the popped
path, and `navigate_to` *unconditionally* calls `history.write().push()`
too. `push`'s own guard (`if entries.last() == path { return }`) does not
save this, since `entries.last()` is still the not-yet-truncated forward
entry — `push` truncates and re-appends, destroying the forward entry.
**Any Back click destroys that pane's Forward history.** Not
macOS-specific — shared code, almost certainly affects every platform.
Confirmed via real dispatch (Forward reports `DISABLED` immediately after
a correct Back); `--break` asserts the impossible opposite and correctly
fails.

**Defect 2 — `DeepRow`'s per-row copy buttons use Explorer's remembered
pane directory, not the deep-compare view's own compare root.**
`crates/forskscope-ui/src/ui/view/deep_compare.rs`'s `DeepRow` computes
its per-row copy buttons' src/dst from `store.settings.read().
last_left_dir`/`last_right_dir` (Explorer's own "remembered pane
directory" setting) — **not** from the deep-compare view's own
`left_root`/`right_root` props (the actual roots being compared).
`BatchCopyButtons` does **not** share this defect (confirmed: its
manifest entries' `src` genuinely came from `right_root`). Concretely
demonstrated: with `last_right_dir` unable to track the actual right
compare root (Finding 3, §6, means the right pane can never be navigated
there), a per-row "Copy to right" click landed at `$HOME/aaa-changed.txt`
— verified via a real backup and overwrite — while `root-b/aaa-
changed.txt`, the file actually shown as "Changed," was verified
untouched. **No error surfaces to the user** — the copy silently writes to
the wrong place. This is a real, silent, potentially destructive
mismatch: any user whose Explorer pane's remembered directory differs
even slightly from the roots they are deep-comparing (trivially easy,
since picking a compare root via the aligned-view picker never requires
navigating into it first) would have a per-file copy silently land
somewhere other than intended.

Neither defect was fixed, per this slice's constraints.

## 6. Any difference from the handoff or the frozen plan

- **Finding 3 (§6-equivalent in the evidence doc) — a genuine HARNESS/
  TECHNIQUE limitation, not a product defect, and not anticipated by the
  handoff**: three real dispatches (two occurrence indices for `"↑"`,
  both click orderings) conclusively found the **right** Explorer pane's
  PathBar buttons report `CLICKED` (the AppleScript `click` verb
  structurally succeeds) but never produce the expected navigation
  effect, while the identical technique against the **left** pane's
  buttons works every time. This is disclosed precisely because it
  materially shaped how P07 had to be designed (last_left_dir is kept
  genuinely correct via real left-pane navigation; last_right_dir is
  seeded and never navigated, which is what exercises Defect 2 for real)
  and rules out an entire technique family for any future work on the
  right-hand pane.
- P03's horizontal-scroll-mirror assertion is not executed on macOS — a
  disclosed platform/technique limitation (§2), consistent with the
  handoff's own framing that this item may not be solved on every
  platform yet.
- No dependency added, removed, or version-changed.
- No product behaviour changed — both defects are registered, not fixed.
- `matrix-plan.md` was not touched.

## 7. Executed gates

- No Rust source was touched this slice (harness-only work in
  `packaging/evidence/macos_harness.py` and `macos_ui.applescript`, plus
  the evidence doc and this request) — `cargo fmt`/`clippy`/tests were not
  run, per the handoff's own expectation that this is harness-only work.
- Every case (P03, P07, P11) and every recon diagnostic was dispatched via
  `gh workflow run` against the real, published, digest-verified 0.167.0
  macOS artifact on `macos-latest` CI, in **both** normal and `--break`
  mode, and results were read from the actual run logs — never inferred
  from code inspection alone. Dozens of real dispatches total across this
  slice's development (P07 alone needed well over a dozen iterations); see
  the git log for the full iteration history.

## 8. Unresolved issues and known limitations

- **P03's horizontal-scroll-mirror is unverified on macOS** (§2) — a
  genuine technique gap, not a pass or a fail. Whether this can ever be
  closed on macOS via accessibility introspection alone, or whether it
  needs a fundamentally different technique (e.g. pixel-level screenshot
  comparison), is open.
- **The right Explorer pane's button-driven navigation is unreliable**
  (Finding 3) — established via real, repeated dispatch, but the *root
  cause* of why the left pane's identical technique works and the right's
  does not was never identified (multiple hypotheses — ordering races,
  occurrence-index miscounting, WebKit-side element staleness — were each
  tested and ruled out; no further hypothesis was tested given time
  already spent). A future slice revisiting Explorer automation on macOS
  should treat this as a known, real constraint, not attempt to rediscover
  it from scratch.
- **Defect 2's real-world severity depends on how often `last_left_dir`/
  `last_right_dir` actually diverge from a user's chosen compare roots in
  practice** — this slice demonstrated the mechanism and a concrete
  silent-wrong-write consequence, but did not attempt to characterize how
  common the divergence is for typical workflows.
- **F63's "does a real VoiceOver user experience comparable latency"
  question is explicitly left open** (§1) — not verified either way.
- P06 (M5-B, not one of this slice's assigned cases) was not retrofitted
  with F63's resolved understanding, even though it plausibly could be
  restored closer to RFC-078's original in-app/concurrent-process
  description now that fixture size is understood not to be a hard
  content-visibility wall. Flagged as a reasonable follow-up, not
  attempted here.

## 9. Requested review focus

1. **F63's resolution (§1)** — is "harness artifact, confirmed via three
   independent real dispatches" convincing, or does the unverified
   VoiceOver-latency caveat warrant further investigation before Gate D
   treats this as fully closed?
2. **Defect 2's severity** (§5) — silent, potentially destructive
   mismatches for per-file copy specifically warrant a priority
   assessment; does this need its own tracked issue ahead of Gate D, given
   its "no error surfaces" character matches exactly what F62's lesson was
   about?
3. **Finding 3, the right-pane technique limitation** (§6) — is recording
   this as a harness/technique limitation (not a product defect) the
   right call, given the root cause was never pinned down? Should this be
   escalated to check whether it reflects a real accessibility gap in the
   product itself (i.e. would a real VoiceOver user also have trouble
   operating the right Explorer pane), rather than assumed to be purely a
   harness/AppleScript artifact?
4. **P03's reduced macOS scope** (§2) — RFC-078 only requires "a basic
   layout observation" here, and this slice went beyond that where
   possible (word wrap, narrow window, and a real attempt at horizontal-
   scroll-mirror) — is the horizontal-scroll gap acceptable to leave open
   for this row, or does Gate D need it closed before macOS can be
   considered fully assessed?
5. Whether P07's overall **Pass** verdict is the right Gate D input given
   it carries two real product defects and a technique limitation inside
   it, or whether Gate D would prefer these tracked as separate,
   explicitly-weighted inputs rather than folded into one case's Pass.
