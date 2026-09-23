# Review Request 069: M5 re-verification against published 0.167.1

**Governing instruction.** Architect message, 2026-08-17: *"re-run the affected
M5 rows against 0.167.1 — P12 (F61), P07 (F73), P11 (F69). F68, F72 and F73
stay open in the register until that verification lands; F69 until the
Windows P11 check flips. The fixes exist, but nothing has confirmed them
against a published artifact yet."*
**Responds to.** `dev-record/reviews/070-m5-defect-fixes-review.md` (the
review that approved the F73/F68, F72, F69 fixes and recommended this exact
next step, §6 items 2–3).
**Candidate.** `0.167.1`, published 2026-08-17, source commit `f65c74b`.
**Scope.** Evidence work only — no product code changed here. Three commits:
`da002bd` (workflow re-pointing), `8b0a2d5` + `2abaaec` (a real Linux harness
bug found and fixed along the way), `3f5da6a` (macOS harness assertions
flipped to match the fix), `771a7ce` (register + Gate D input list updates).

## 1. Verdict this request asks for

**All four fixes (F61, F72, F73, F68) are confirmed working for real against
the published `0.167.1` artifacts, on every platform where the case
architecture allows it — including F69's specific required evidence (the
Windows P11 item-2 check flipping Fail → Pass on real CI).** Gate D's
un-waivable blocker count drops from three to one: F44 alone remains.

## 2. What was done

1. **Digests verified locally**, independent of the release page — a fresh
   `gh release download 0.167.1` + `sha256sum`/`shasum` before touching
   anything:
   ```
   0d2b2fdcb185b8d9cdfe89f320474898a4c53eca7f144d2307a7a63e9a071787  forskscope-v0.167.1-linux-x86_64.tar.gz
   1e86f6640f4f43145076266011acbf8dab6ef2a9a3c3670b86aaabf5eaac4de6  forskscope-v0.167.1-macos-aarch64.dmg
   3513def1bcaba69dfda4399350b5013b231b0ca7aec8a1ab973d4e66e7e101d0  forskscope-v0.167.1-windows-x64.zip
   ```
2. **All three M5 evidence workflows re-pointed at `0.167.1`** (tag, filename,
   digest) — `da002bd`. Confirmed the tag's ancestry includes all four fix
   commits (`git merge-base --is-ancestor`) before dispatching anything.
3. **Dispatched P12, P07, P11 in both normal and `--break` mode, on Linux,
   Windows, and macOS** — 18 real CI runs. Two genuine problems surfaced and
   were fixed mid-flight (§3, §4) rather than reported as-is; both were
   re-dispatched afterward to confirm the fix.
4. **Register and Gate D input list updated** (`771a7ce`) with the specific
   CI run citing every claim below.

## 3. Finding 1 — a real, independent Linux harness bug (not a product issue)

Linux P12 (F61's case) failed reproducibly (two independent dispatches,
`32002931729` and `32003260710`, identical failure): `"no-args relaunch did
not restore the compare tab"`.

Windows and macOS P12 both passed cleanly on the *first* dispatch, with
explicit confirmation of the real mechanism (`"a CLI-opened tab restored
automatically on a no-args relaunch"` — Windows run `32002963437`), which
made a genuine Linux-specific product regression implausible: F61's fix is
platform-agnostic Rust code, and 2/3 platforms confirmed it immediately.

Added a temporary diagnostic (`8b0a2d5`) to dump `session.json`'s actual
state around the relaunch. It showed the file **never existed under the
scratch config directory, even right after the first launch** — narrowing
this to a write-location problem, not a restore-logic problem.

**Root cause:** P12's sub-test 2 used the shared `launch()` helper for its
first (CLI-args) launch, and `launch()` does not accept an `env` parameter at
all — so that launch silently ran against the CI runner's real, default
`XDG_CONFIG_HOME`, never the test's isolated scratch profile. The second
(relaunch) and third (explicit-args) launches in the same sub-test *did*
correctly pass `env=env` via a raw `subprocess.Popen` call — this was an
inconsistency within the same function, not a systemic pattern (checked:
every other `env`-isolated case in the file already uses the raw-`Popen`
pattern; this was the one place still going through `launch()`).

Fixed (`2abaaec`) by switching both affected launches to the same raw-`Popen`
pattern the second launch already used. Re-dispatched: Linux P12 now passes
(`32004014491`), `--break` correctly fails (`32004093574`).

## 4. Finding 2 — macOS's P07 harness had the old defects' shapes baked in as "expected"

macOS P07 (F72/F73's case) failed on first dispatch (`32002982532`) with:
`"FAIL: Forward is enabled after Back... either it has been fixed (update
this case) or this run's state differs from what was confirmed via real
dispatch."`

This is the harness correctly reporting a real change: this case's original
M5-C investigation (against `0.167.0`) wrote its Forward-history and
per-row-copy assertions to encode F72's and F73's *exact defect shapes* as
the expected, passing outcome — reasonable at the time, since both were then
open, registered product defects. Now that both are fixed, those same
assertions correctly flag the mismatch rather than silently passing.

Updated (`3f5da6a`) to assert the corrected behavior instead:

- Forward-history check now requires Forward enabled after Back (`--break`
  requires the impossible opposite).
- Per-file copy check now requires the copy land at `root-b/aaa-changed.txt`
  with a real backup, and the *old* wrong location (`$HOME/aaa-changed.txt`)
  verified untouched (previously the reverse).
- Docstring, fixture-setup comments, and the final `OK` summary rewritten
  throughout — including one internally-inconsistent leftover comment block
  from an earlier investigation phase that no longer matched the actual
  fixture code, discovered while making these edits.

Batch copy's own verification needed no changes — never affected by F73
either way. Re-dispatched: macOS P07 now passes (`32003819705`) with an `OK`
message explicitly confirming both fixes; `--break` correctly fails
(`32003821373`).

## 5. Final results, per case and platform

| Case (register item) | Linux | Windows | macOS |
|---|---|---|---|
| **P12** (F61) | Pass [`32004014491`](https://github.com/forskscope/forskscope/actions/runs/32004014491) / break Fail [`32004093574`](https://github.com/forskscope/forskscope/actions/runs/32004093574) | Pass [`32002963437`](https://github.com/forskscope/forskscope/actions/runs/32002963437) / break Fail [`32002965148`](https://github.com/forskscope/forskscope/actions/runs/32002965148) | Pass [`32002978390`](https://github.com/forskscope/forskscope/actions/runs/32002978390) / break Fail [`32002980324`](https://github.com/forskscope/forskscope/actions/runs/32002980324) |
| **P07** (F73/F68, F72) | Pass [`32002935565`](https://github.com/forskscope/forskscope/actions/runs/32002935565) / break Fail [`32002937478`](https://github.com/forskscope/forskscope/actions/runs/32002937478) | **Fail (F70, pre-existing, unrelated)** [`32002967151`](https://github.com/forskscope/forskscope/actions/runs/32002967151) | Pass [`32003819705`](https://github.com/forskscope/forskscope/actions/runs/32003819705) / break Fail [`32003821373`](https://github.com/forskscope/forskscope/actions/runs/32003821373) |
| **P11** (F69) | Pass [`32002939138`](https://github.com/forskscope/forskscope/actions/runs/32002939138) / break Fail [`32002940976`](https://github.com/forskscope/forskscope/actions/runs/32002940976) | **Pass** [`32002971000`](https://github.com/forskscope/forskscope/actions/runs/32002971000) **— the required Fail→Pass flip** / break Fail [`32002972760`](https://github.com/forskscope/forskscope/actions/runs/32002972760) | Pass [`32002985498`](https://github.com/forskscope/forskscope/actions/runs/32002985498) / break Fail [`32002987450`](https://github.com/forskscope/forskscope/actions/runs/32002987450) |

Windows P07's failure is F70 (Explorer never lists directories on Windows
CI) — already registered, undetermined product-vs-environment, entirely
unrelated to this fix release. Confirmed unchanged, not investigated further
here (out of this request's scope).

## 6. Register and evidence updates

- **ROADMAP.md**: F61, F68, F69, F72, F73 entries each updated to record
  resolution with the specific verifying CI runs, appended to (not replacing)
  their existing history. A new progress note summarizes the pass.
- **New `docs/src/maintainers/release-evidence/0.167.1/gate-d-input-list.md`**
  — a per-candidate document (matching the established convention each
  candidate gets its own evidence directory), recording the updated blocker
  count and pointing back to `0.167.0`'s own input list for full M5-A/B/C
  detail. `0.167.0`'s input list itself was left untouched, as an accurate
  historical record of what was true for that candidate — not retroactively
  rewritten.

## 7. Executed gates

Harness/doc-only changes. `python3 -m py_compile` on both harness files
before every commit; `mdbook build docs` clean after every doc change;
`git diff --check` clean before each commit. Full `CI` workflow (fmt,
clippy, test, i18n, actionlint) confirmed green via `gh run view` on the
final commit, [`771a7ce`](https://github.com/forskscope/forskscope/actions/runs/32004756104)
— not assumed from the push alone.

## 8. Unresolved issues

- **This was a targeted 18-run dispatch (P12/P07/P11 × 3 platforms × 2
  modes), not a full M5-A/B/C re-execution against `0.167.1`.** Every other
  case result recorded against `0.167.0` is assumed to still hold (the same
  Rust source outside the four fixed defects) but was not re-dispatched to
  confirm.
- **F72 is confirmed only on macOS**, not independently re-exercised via
  Linux/Windows P07 (neither drives Back/Forward the same way that case
  does). Backed by a dedicated `dir_pane.rs` unit test (confirmed to fail
  before the fix) covering the shared mechanism directly, but no cross-
  platform CI confirmation beyond that.
- **F70 remains open and un-investigated here** — Windows P07 still fails on
  it, unchanged from before this fix release. Out of this request's scope
  (the cheapest next step, per its own ROADMAP entry, is still the owner
  opening Explorer on the Windows 11 manual host).
- **The matrix is still not formally complete** — `linux-wayland` and F45's
  Windows manual sub-case remain outstanding, unaffected by this pass.

## 9. Requested review focus

1. **Is the Linux `launch()`/`env` bug (§3) correctly diagnosed and fixed**,
   or does its narrow scoping (checked every other `env`-isolated call site;
   found only these two) risk missing a sibling instance elsewhere in the
   harness?
2. **Is flipping macOS P07's assertions (§4) the right response**, or should
   a case that encoded a since-fixed defect's exact shape have been
   rewritten more substantially (e.g., a fresh fixture) rather than
   inverted in place?
3. **Is per-candidate `gate-d-input-list.md` filing (§6) the right pattern**
   going forward for a fix release that doesn't warrant a full M5 re-run, or
   should this have updated `0.167.0`'s document directly with a dated
   amendment instead?
4. **Does this request close the loop the architect's instruction opened**,
   or is there a further verification step expected before F61/F68/F69/
   F72/F73 can be considered fully settled in the register?
