# Review Request: RFC-077 Patch 4a+4b — Startup and Save-Path Migration (Closes B3)

**Date:** 2026-08-04
**Reviewer stance:** focused implementation review, file-safety-critical path
**Repository baseline:** `5be4086` (`core+ui: RFC-077 patch 4b - save path
routed through save_target; closes B3`), on top of `b387f59` (`ui: RFC-077
patch 4a - startup and request migration`)
**Governing documents:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`;
RFC-077; review 047 (`dev-record/reviews/047-rfc077-patch3-normal-compare-migration-review.md`)

## Why 4a and 4b landed together, not as two separately-reviewed checkpoints

Review 047 §3.2 asked for two patches instead of one, and I agree with the
reasoning — but implemented them as two separate **commits** landed in the
same working session, not two separately-reviewed stopping points. Reason:

4a alone changes `right_path` to correctly always mean the compared input
(never the mergetool output) — that's the whole point of the migration. But
`diff_actions.rs`'s `build_request` was still unmigrated at that point, and
it reads `tab.right_path` as the save target. Landing 4a alone would mean a
mergetool save writes to `<remote>` — the file being *compared against* —
instead of `<merged>`. That's strictly worse than the bug RFC-077 exists to
fix (today's bug writes to the *right* file with the *wrong* fingerprint
check; 4a-alone would write to the *wrong* file). Committing that as a
standalone reviewable state, even briefly, seemed like exactly the kind of
window this project's conventions guard against elsewhere (RFC-076's
`Force`-never-implicit rule, the fail-closed no-clobber requirement here).

So: implemented both without stopping in between, gates run once at the end
covering both, two commits kept separate for review granularity (4a is pure
restructuring; 4b is the file-safety-relevant change), and this single
review request covers both together with the runtime evidence 4b needs.

## Summary

**4a** (`b387f59`): `main.rs` parses `StartupRequest` via
`parse_startup_args`, rejecting unsupported arity with a non-zero exit.
`app.rs`'s startup `use_hook` converts it to a `CompareRequest` and calls
`open_compare_request` — one call, no post-spawn mutation. `CompareTab`
gains `launch_mode: CompareLaunchMode`; `reload_tab` derives its request
from it. `open_compare(store, left, right)` keeps its existing signature
(thin wrapper) so the eight in-app call sites (Explorer, deep compare,
session restore) are untouched.

**4b** (`5be4086`): `SaveRequest.expected_fingerprint: Option<FileFingerprint>`
→ `precondition: TargetPrecondition`; `save_text` routes `MustBeAbsent`
through `persist_noclobber`. `diff_actions.rs`'s `build_request` uses only
`tab.save_target` for path/precondition/encoding; `handle_result`/`save_as`
update `tab.save_target`, never `right_doc.fingerprint_at_load`, never
`right_path`. This is the change that actually closes B3.

## Report: §3.4 (`CompareRequest` placement) — resolved

**`CompareRequest` never crosses into core.** `open_compare_request`
destructures it in `forskscope-ui` (`state/compare.rs`) to decide
`launch_mode` and to build the tab; `load_and_diff` destructures it further
to call core's `save_target_from_loaded`/`inspect_save_target` — but those
core functions take plain `&Path`/`&LoadedDocument`/`&str`, never
`CompareRequest` or a growing piecemeal parameter list standing in for it.
`grep -rn CompareRequest crates/forskscope-core/` returns nothing.

Per review 047 §3.4's own test ("if patch 3 finds itself passing
`CompareRequest`'s fields into a core function one at a time... that is the
signal"): that didn't happen. The type is consumed wholesale by
`forskscope-ui`'s own orchestration (`open_compare_request`, `load_and_diff`,
`reload_tab`) and reconstructed from `CompareTab.launch_mode` on reload — it
never needed to become core's vocabulary. I'm treating this as the placement
question settled in favor of the RFC-075-precedent placement (ui-logic), per
the reasoning in the 043 checkpoint request.

## Files Changed

**4a:** `main.rs`, `app.rs`, `state.rs` (re-export), `state/compare.rs`
(`open_compare_request`, `reload_tab`, `load_and_diff` take `CompareRequest`),
`state/compare/tests.rs` (+3 mergetool tests), `state/tab.rs`
(`CompareLaunchMode`, `CompareTab.launch_mode`).

**4b:** `save.rs` (`SaveRequest.precondition`, `save_text` routing, the
permission fix below), `tests/save_tests.rs` (11 mechanical
`expected_fingerprint`→`precondition` conversions, +2 new `MustBeAbsent`
tests), `tests/save_target_tests.rs` (doc comment update, +1 permission
test), `ui/view/diff_actions.rs` (`build_request`/`handle_result`/`save_as`
migration, +2 `to_precondition` tests).

## An issue found and fixed while gathering runtime evidence

**`persist_noclobber`-created files were `0600`, not `~0644`.**
`tempfile::NamedTempFile` defaults to narrow permissions (it's designed for
scratch files that often hold sensitive data). But a no-clobber-committed
mergetool output is a *permanent* file, not scratch — `ls -la` on the
freshly-created `merged2.txt` during runtime testing showed `-rw-------`
where a normally-saved file (`atomic_replace`'s path) showed `-rw-r--r--`.
Fixed by explicitly setting `0o644` on the temp file (Unix only — no POSIX
mode-bit equivalent on Windows) before `persist_noclobber`, with a test that
asserts the exact mode. This wasn't something I'd anticipated designing the
type model in the earlier checkpoints; it only surfaced by actually running
the binary and checking the file, which is the whole reason the handoff asks
for runtime evidence rather than trusting the types.

## Runtime Evidence

Built the debug binary, ran the three-argument Git mergetool CLI against
real files under `.git-exclude/tmp/rfc077-mergetool/` (repo-relative scratch,
not `/tmp`), driven via AT-SPI (`Atspi.Action.do_action`) since no
pointer/keyboard-injection tool is available in this Wayland session.
Screenshots and stdout logs are in that directory (not committed —
`.git-exclude/` is repo-ignored; happy to reproduce live or attach if you
want them archived elsewhere).

**Setup:**
```
local.txt:  "line one\nline two\nline three\n"
remote.txt: "line one\nline TWO changed\nline three\n"
merged.txt: "line one\nline two\nline three\n(pre-existing merged content)\n"
```

**1. Startup identity.** Launched `forskscope local.txt remote.txt
merged.txt`. Screenshot confirms: left header `.../local.txt`, right header
`.../remote.txt` (never merged), diff shows local-vs-remote (line 2 differs).
Tab's accessible name (via the "Close" button's AT-SPI label, since the
visible tab strip truncates): `"Close local.txt ↔ remote.txt (merge)"` —
confirms the `(merge)` suffix from `open_compare_request` is present in the
underlying string, not just visually cropped.

**2. Existing merged target, full round trip.** Clicked "Use this change"
(AT-SPI), then "Save merge result". Result:
```
merged.txt:     "line one\nline two\nline three\n"           (the merge result)
merged.txt.bak: "line one\nline two\nline three\n(pre-existing merged content)\n"  (exact pre-save bytes)
local.txt:      unchanged, byte-identical
remote.txt:     unchanged, byte-identical
```
This is the exact defect RFC-077 closes, demonstrated end to end: the
compared inputs are untouched, the distinct merged path receives the write,
and the pre-existing merged content is preserved as a backup rather than
silently lost.

**3. Missing merged target.** Relaunched with a merged path that doesn't
exist (`merged2.txt`). "Save" button correctly disabled until a change is
applied (pre-existing product behavior, unaffected by this patch) — clicked
"Use this change" then "Save": `merged2.txt` created with the merge result,
**no `.bak`** (nothing existed to back up), permissions `0644` (confirms the
fix above) once rebuilt.

**4. `stdout.log`** across all three runs: only pre-existing WebKitGTK/EGL
warnings (`libEGL warning: pci id for fd...`), identical to noise seen in
every prior F32/RFC-076 runtime-evidence session — no panics, no errors.

## Not Addressed Here

- **Presentation** (RFC-077 "Result: /path/to/MERGED" quiet output line) —
  patch 5's job per the handoff's own patch sequence. Right now a mergetool
  tab shows no distinct on-screen indicator of the save target beyond the
  `(merge)` title suffix and (once applied) the toast — confirmed via
  runtime evidence, not asserted from the diff.
- **Save As requires confirmation for an existing destination** (RFC-077
  test design section) — `build_request`'s Save-As branch now correctly
  *inspects* the destination and never constructs `Force`, so an external
  race is caught; but there's still no confirmation *dialog* before
  attempting to overwrite a hand-typed existing path in `SaveAsModal` (a
  pre-existing gap — today's code always used `force: true` unconditionally
  for Save As, silently overwriting). Closing the identity bug felt in scope
  for 4b; adding new confirmation UI felt like patch 5's "target-transition
  cases, presentation" territory. Flagging explicitly so it isn't
  mistaken for closed.
- Target-transition integration tests (appeared/deleted/replaced-after-
  preparation races, the no-clobber race at the *save_text* level rather
  than the *persist_noclobber* level already covered) — patch 5 per the
  handoff's own sequencing, though the no-clobber race itself is already
  covered (patch 2's `persist_noclobber_race_via_before_commit_hook...`).
- `CompareLaunchMode`'s title/presentation beyond the `(merge)` suffix.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1090 (685+27+16+2 core, 267 ui-logic, 80 forskscope-ui, 6 css, 7 doctests), exactly 1077 + 13
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 220 keys (unchanged; no new user-visible strings)
cargo xtask css --check                                          pass
cargo xtask audit-deps                                           pass
cargo audit                                                      pass — same 14 pre-existing allowed warnings as the 043 checkpoint; no new advisories
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
future_version_session_stays_byte_identical_through_a_disabled_save  pass (review 047's named acceptance test)
```

CI run `30883973553`: Test & Lint green, on `5be4086`.

**Test-count delta, itemized** (baseline **1077**, per review 047's
correction — not the `1069` this thread previously misreported):

| Source | Delta | Reason |
|---|---:|---|
| `state/compare/tests.rs` (4a) | +3, ×2 targets = +6 raw | mergetool save-target derivation: merged path not remote, `MustBeAbsent` when missing, local-vs-remote diff (not local-vs-merged) |
| `tests/save_tests.rs` (4b) | +2 | `save_text` end-to-end through `TargetPrecondition::MustBeAbsent`: creates a missing target, conflicts and leaves an existing one untouched |
| `tests/save_target_tests.rs` (4b) | +1 | the `0644` permission fix |
| `ui/view/diff_actions.rs` (4b) | +2, ×2 targets = +4 raw | `to_precondition`'s pure mapping (the one piece of the migration not gated behind a live `Store` — F36 again) |
| **Total** | **+13** | 3 core + 10 raw ui (5 distinct ×2 targets) |

## Requested Review Focus

1. Is landing 4a+4b together (rather than pausing after 4a) the right call
   given the reasoning above, or should the intermediate state have been
   surfaced for review even though it regresses mergetool save?
2. The permission fix — `0o644` unconditionally, not derived from umask
   (reading the process umask from safe Rust std requires an awkward
   set-then-restore dance) — acceptable, or does this need to respect the
   user's actual umask before M3 closes?
3. The Save-As-confirmation gap (Not Addressed Here) — confirm patch 5 is
   the right place for it, not a patch 4b omission.
4. Runtime evidence covers existing and missing merged targets. Deleted/
   replaced/externally-modified-after-preparation target transitions are
   still only unit-tested (patch 2's core-level tests), not exercised
   end-to-end through the running binary — acceptable for this checkpoint,
   or does B3's closure claim need broader runtime coverage before M3?
