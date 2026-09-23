# Review Request: M4-A Residual Correctness — F40, F8, F35, F10

**Date:** 2026-08-11
**Reviewer stance:** focused implementation review
**Repository baseline:** `db8a95b`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4a-residual-correctness-handoff.md`

## 1. Implementation summary

All four M4-A items, each its own commit:

- `8213945` — F40: diff-option toggles no longer silently discard merge work.
- `b8139e2` — F8: clarified as not a new defect (docs-only).
- `383ebf8` — F35: blank counterpart rows no longer announce a bare "Changed".
- `db8a95b` — F10: VCS "outside repo" tests verify their own precondition.

## 2. Addressed items

### F40 — diff-option toggles silently destroyed merged work

**Chose design (b): confirm, like `swap_sides`** — not (a) preserve-and-reapply.

Investigated (a) first. `HunkId` is `hash(diff_id, ordinal, kind, left_range,
right_range)`, and `diff_id` comes from a process-global `AtomicU64` counter
incremented on every `compute_diff` call
(`forskscope-core::diff::engine::DIFF_COUNTER`). So hunk identity is not
merely unstable across a diff-*options* change — it is unstable across
*any* recompute, even one that changes nothing (identical content, identical
options, back-to-back calls). Reapplying transactions against the new hunk
set would need a rebasing rule keyed on something other than `HunkId`
(content or position matching), which is a materially larger design than
this slice's scope, and the handoff's own framing ("your call, justify it")
left room for (b).

Implemented `change_diff_options`/`set_diff_options`
(`crates/forskscope-ui/src/state/tab.rs`) and `Modal::ConfirmDiffOptionChange`:
the three toolbar controls (Ignore WS, Ignore case, algorithm select) now
compute the candidate `DiffOptions` and route through a dirty-check-then-
confirm gate, mirroring `swap_sides`'s existing `ConfirmSwap` guard exactly,
instead of mutating `tab.diff_options` and calling `recompute_diff` inline.
`is_dirty()` never goes silently `false` while work is discarded — a dirty
tab always shows the confirm dialog first.

**RFC-015 §8 rule 4** ("Recomputing diff after an edit must not erase undo
history") is marked **Not met**, with a dated note (`rfcs/done/015-...md`)
explaining precisely why (the `DiffId`-instability argument above) and what
*is* guaranteed instead (ask-first, not silent). Did not leave the rule
asserting something the code doesn't do.

### F8 — the captured save fingerprint is unused

**Outcome 3, with a correction, not a clean "misreading."** `SaveOutcome.new_fingerprint`
is genuinely consumed: `diff_actions.rs::handle_result` (line 312) stores it
as the tab's next `TargetExpectation::MustMatch` — RFC-077's own save-target
model. Had this consumer not existed, that would have been outcome 1, a real
defect.

What's actually true, and what I believe the original audit N1 (register
entry F8) meant: the `digest: Option<u64>` field *inside* `FileFingerprint`
is captured (`FileFingerprint::capture` hashes the written bytes,
`save.rs:87`) but never read — `check_external_state` (`document.rs`), the
function that evaluates a `MustMatch` conflict, compares only `len` and
`modified_unix_nanos`. RFC-074 already tracks exactly this under its own
label, N1, with two remediation options ("use the digest when metadata is
inconclusive, or document the limitation"), scoped to M4 broadly — not this
slice specifically, and not something the handoff's "no product behaviour
changes beyond the four items above" constraint covers.

Corrected `ROADMAP.md`'s F8 entry to say precisely this rather than leave the
old, now-inaccurate "digest is captured but unused at save time" (which reads
as the whole struct being dead weight — it isn't). No code changed.

### F35 — blank counterpart rows announced a bare "Changed"

**Decision: leave blank counterpart rows unlabelled** — not "label only the
first row of a run" or "label the run once at the hunk level".

A row with real content keeps announcing `Changed: <line>` (useful when a
screen-reader user is navigating row by row through a multi-line change); a
row with nothing on that side gets no label at all, instead of a bare
"Changed" with nothing following it.

**What a screen-reader user hears now**, for a 4-line-left/1-line-right
Replace hunk (fixture used for AT-SPI verification, §5): navigating the left
column, four distinct announcements — "Changed: old-a", "Changed: old-b",
"Changed: old-c", "Changed: old-d". Navigating the right column: one
"Changed: new-a", then three rows with nothing announced at all (previously:
three bare "Changed" announcements with no content).

Implemented as a pure `wants_replace_label(kind, has_content)` predicate in
`hunk.rs`, called from both `RowLeft` and `RowRight`; unit-tested directly
(content-present, content-absent, non-Replace kinds) since the components
themselves are `Store`-dependent (F36).

Recorded the decision in `rfcs/proposed/061-...md` per the handoff's
explicit direction ("RFC-061 is the accessibility track"), with a note
flagging that RFC-061's own §"Cross-references" says RFC-019 "owns row
ARIA" — which reads like the more precise home for this specific decision.
Did not move it there unilaterally since the handoff named RFC-061
specifically; said so instead, so it can be corrected if RFC-019 is right.

### F10 — VCS discovery tests assumed the temp dir sits outside any repo

Both "outside any repo" tests (`vcs_tests.rs`) now check an independent
precondition — `ancestor_has_git`, a from-scratch ancestor walk, deliberately
*not* reusing `find_git_root`/`detect()` — before trusting the "must be
`None`" assertion, and print a loud `eprintln!` skip reason if the OS temp
directory's own ancestry already contains a `.git` (mirroring
`save_target_tests.rs`'s existing `0o000`-restriction precedent exactly: verify
the assumed condition actually holds, rather than assume it).

Deliberately independent rather than calling `detect()` itself for the
precondition check: if it reused `detect()`, a genuine bug in `detect()`
and a contaminated environment would be indistinguishable — both would
report "confounded" and skip, silently hiding the very bug the check exists
to protect against.

Did not move the fixtures to a different location (the handoff explicitly
said not to — no location is portably guaranteed clean).

## 3. Changed files

`crates/forskscope-ui/src/state/tab.rs`, `state.rs`,
`ui/overlay/modals.rs`, `ui/overlay/modals/file.rs`,
`ui/view/diff/toolbar.rs`, `ui/view/settings.rs`, `i18n.rs`,
`state/tab/tests.rs` (F40); `rfcs/done/015-undo-redo-transaction-log.md`
(F40); `ROADMAP.md` (F8's entry corrected, F40/F35/F10 marked `**Resolved.**`);
`crates/forskscope-ui/src/ui/view/hunk.rs` (F35);
`rfcs/proposed/061-explorer-pane-focus-and-keyboard-completeness.md` (F35);
`crates/forskscope-core/src/tests/vcs_tests.rs` (F10).

## 4. Difference from the handoff

- **F35**: recorded the decision in RFC-061 exactly as directed, but flagged
  in both the RFC and the register that RFC-019 may be the more precise
  home. Not a scope difference, a documented uncertainty.
- **F10**: the handoff's first option ("establish the condition explicitly —
  create a barrier the walk cannot cross") wasn't implemented. I couldn't
  identify a portable way to make the upward filesystem walk itself
  unable to cross a boundary without changing `find_git_root`'s production
  behavior (e.g. a `GIT_CEILING_DIRS`-style mechanism it doesn't currently
  have), which would be a product change beyond this slice's constraints.
  Went with the handoff's second option (detect-and-skip) instead, which the
  handoff's own phrasing ties to the `save_target_tests.rs` precedent.

## 5. Runtime evidence (F40, F35 — `Store`-dependent, F36)

Both rebuilt the debug binary and drove it via AT-SPI (`gi.repository.Atspi`),
screenshots under `.git-exclude/tmp/`. One environment note: this session's
shell had `NO_AT_BRIDGE=1` set, which silently prevented the app from
registering on the accessibility bus at all (`Atspi.get_desktop(0)` simply
didn't list it) — had to launch with `NO_AT_BRIDGE=0` explicitly. Noting
this in case it recurs.

**F40**, `.git-exclude/tmp/f40-diffoption-guard/`: applied a hunk (tab dirty),
clicked "Toggle ignore whitespace" — `Modal::ConfirmDiffOptionChange` appeared
(`01-confirm-modal.png`), naming what would be lost, *before* any option
changed. Clicked Cancel — dialog dismissed, applied hunk and dirty state
untouched, `Ignore WS: off` unchanged (`02-after-cancel.png`). Re-triggered,
clicked "Discard and Change" — `Ignore WS: on` applied, tab dirty-dot gone,
Undo disabled, applied hunk reverted to pending (`03-after-confirm.png`).
Toggled "Ignore case" on the now-clean tab — applied immediately, no dialog
(`04-clean-immediate-apply.png`). All four branches (dirty→dialog,
cancel→no-op, confirm→apply-and-discard, clean→immediate) observed.

**F35**, `.git-exclude/tmp/f35-sr-blank-rows/`: 4-line-left/1-line-right
Replace fixture (`same1/old-a/old-b/old-c/old-d/same2` vs
`same1/new-a/same2`). Walked the AT-SPI accessible tree and read each row
cell's text directly (`Atspi.Text.get_text`): left column's four rows each
read `Changed: old-a` / `old-b` / `old-c` / `old-d`; right column's real row
reads `Changed: new-a`, its three blank-counterpart rows read empty string
(no `Changed:` prefix at all) — confirmed against the pre-fix build first
(rebuilt binary was stale on the first attempt; caught via the tree still
showing bare `Changed:` three times, rebuilt, re-verified). Screenshot
`01-unequal-replace-hunk.png`.

## 6. Not verified — stated rather than implied

- **F10**: did not verify the `ancestor_has_git` skip path end-to-end against
  a live `TMPDIR` pointed into a real repo — that specific command (creating
  a scratch git repo and re-running the two tests under a custom `TMPDIR`)
  was declined by this session's sandbox permissions, and I did not retry a
  variant per this session's own instructions on tool denials. What stands
  instead: three direct unit tests of `ancestor_has_git` itself (a `.git`
  directly present, one several levels up, and a clean tree producing
  `false`), which exercise the same walk logic the skip path calls, just not
  through the actual two "outside repo" tests taking the skip branch.
- **F23's actionlint local falsifiability gap** (review 050/052/053, prior
  round) remains as previously reported — unrelated to this slice, noting
  only that the same class of sandbox limitation recurred here.

## 7. Tests and gates run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1105 (+11 over the M4-A-start baseline of 1094: F40 +2 — one new forskscope-ui test, counted once each for its lib and bin targets; F35 +6 — three new forskscope-ui tests, same lib/bin doubling; F10 +3 — three new forskscope-core tests, counted once, no lib/bin doubling for that crate)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 227 keys (+4: F40's diff-option-change strings)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.166.1
cargo xtask audit-deps                                           pass
git diff --check                                                 pass
```

CI run `31460472535`: success, on `db8a95b` (covers all four commits pushed
together).

## 8. Unresolved issues and known limitations

- F8's remediation (use digest as a fallback, or document the same-size/
  same-mtime limitation in threat-model/user docs) remains open as RFC-074's
  own N1, unchanged — not resolved by this investigation, deliberately.
- F35's RFC placement (RFC-061 vs RFC-019) is unsettled; see §4.
- F10's end-to-end skip-path verification gap; see §6.
- Per the handoff's constraints: no Rust dependency added/removed/changed,
  no product behavior changed beyond the four named items, no fifth defect
  found.

## 9. Requested review focus

1. F40 — is confirm-over-preserve the right call given the `DiffId`
   instability argument, or does RFC-015 rule 4's "not met" status need to
   become an actual M-something follow-up rather than just a recorded gap?
2. F8 — does the correction read as accurate, or did I misread what audit
   N1/F8 originally meant?
3. F35 — RFC-061 vs RFC-019 for where this decision should actually live.
4. F10 — is direct unit-testing of `ancestor_has_git` sufficient evidence
   given the end-to-end skip path wasn't exercised, or does that gap need
   closing before F10 stands as resolved?
