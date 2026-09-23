# Review Request: RFC-077 Review 050 — Save As Pre-Check Fix and `done/` Move

**Date:** 2026-08-08
**Reviewer stance:** focused implementation + documentation review
**Repository baseline:** `e027c1d`
**Governing documents:** RFC-077 (now `rfcs/done/077-mergetool-save-target-model.md`);
RFC-000 §"Granularity of transitions"; review 050

## Summary

Review 050 approved patch 5 and gave two recommended next actions. Both are
done:

1. **§3.2 — `Path::exists()` → `inspect_save_target` for the Save As
   pre-check** (`7595802`). `SaveAsModal`'s pre-write check previously asked
   "Overwrite existing file?" for any existing path, including one that could
   never be written to (a directory, a binary file) — the user confirmed,
   then was refused one step later. Added `SaveAsPrecheck`
   (`New`/`Overwrite`/`Blocked(String)`) and `precheck_save_as_target`,
   reusing the same `inspect_save_target` classification `build_request`
   already applies, so a `Blocked` destination reports immediately with no
   intervening confirmation dialog.
2. **Move RFC-077 to `rfcs/done/`** (`765f60c` + `e027c1d`). Added an
   `## Implementation outcome` section following RFC-075/076's shape: the
   patch/correction commit list, each acceptance criterion's status
   (including the "partially" on target-transition tests, explained in my
   own words per §3.1), and Windows `persist_noclobber` semantics deferred
   to RFC-078 explicitly. Status line: `Implemented (Milestone M3)`.
   `rfcs/README.md`'s Implemented/Proposed counts and RFC-077's row moved in
   the same logical change.

## Files Changed

`ui/view/diff_actions.rs` (+`SaveAsPrecheck`, +`precheck_save_as_target`),
`ui/view/diff.rs` (re-export), `ui/overlay/modals/file.rs` (`SaveAsModal`'s
onclick now matches on `SaveAsPrecheck` instead of `target.exists()`),
`rfcs/done/077-mergetool-save-target-model.md` (moved from `proposed/`,
Status line, `## Implementation outcome` section), `rfcs/README.md` (counts,
row move, updated summary paragraph).

## A commit-hygiene mistake, corrected in the same round

The `done/` move landed as two commits instead of one: `git add` was given a
stale path (`rfcs/proposed/077-...md`, already renamed by the preceding
`git mv`) alongside the real targets in one invocation; `git add` aborted on
the bad pathspec and staged nothing at all, but `git mv`'s own staged rename
(rename-only, no content) had already gone into the commit. So `765f60c`
committed a pure rename with a message describing content it didn't contain.
Caught immediately via `git status`/`git show --stat` after the commit;
`e027c1d` carries the actual content (Status line, outcome section,
`README.md` changes) with a commit message that says explicitly what
happened and that it completes `765f60c`, rather than silently amending.
Flagging this so it's visible rather than only discoverable by reading two
commits closely — no data or design content was lost, but the split wasn't
intentional and the first commit's message doesn't match its diff.

## Design Notes

- `precheck_save_as_target` and `describe_block` share `SaveTargetBlockReason`
  → user-facing string mapping — no new i18n boundary, consistent with the
  established precedent (`describe_block` is untranslated; F39 already tracks
  the general gap).
- No new tests — same F36 exemption as the rest of this workstream
  (`Store`-dependent Dioxus rendering with no separable pure logic beyond
  what `inspect_save_target` itself already tests at the core level).
- The `## Implementation outcome` section folds review-driven correction
  commits (N1, C1/C2, §3.2) into the patch bullets they correct, mirroring
  RFC-076's shape rather than listing them as a separate un-narrated list.
- Explicitly stated in the outcome section: the folder move records that the
  design shipped, not that Milestone M3 closes — F38 (permission/umask)
  remains open and registered against M3, unchanged by this commit.

## Runtime Evidence

Rebuilt the debug binary, ran under `.git-exclude/tmp/rfc077-review050/`
(`left.txt`, `right.txt`, `a-directory/`), driven via AT-SPI.

Launched `forskscope left.txt right.txt a-directory` (mergetool mode, merged
path = a directory, so Save As's default path is the Blocked target without
needing to type into the AT-SPI-inaccessible input field — see the AT-SPI
`EditableText`-not-exposed limitation noted in prior review requests).
Confirmed via `Atspi.Text.get_text` that the pre-filled path read the
directory. Clicked Save via `Atspi.Action.do_action`. Screenshot
(`01-blocked-saveas-immediate.png`) shows: `SaveAsModal` still open, and a
toast reading **"Cannot save here: not a regular file."** appearing
immediately — no `ConfirmSaveAsOverwriteModal` shown first. This is the exact
defect review 050 §3.2 described, now closed.

Directory target verified untouched after the run (`ls -la`, still an empty
directory) before killing the process.

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1094 (unchanged — no new tests, see Design Notes)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 223 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
mdbook build docs                                                pass, no broken-reference warnings
```

CI run `31228349781` (covering both this round's pushed commits): success,
on `e027c1d`.

## Not Addressed Here

- **F38** (permission/umask) — review 050 named this "the last item before
  M3 can close." Not started; the fix approach (umask-derived vs.
  preserve-existing-target's-mode) is still an open choice per review 048,
  not one I've made unilaterally.
- F23 — still gates M2's cut, untouched by this work.
- F39 (i18n gate blindness) — deferred to M4.
- RFC-078's platform-specific runtime matrix, including Windows
  `persist_noclobber` replacement semantics — explicitly named in the new
  outcome section as deferred there.

## Requested Review Focus

1. Does the `## Implementation outcome` section satisfy §3.1's three
   requirements (patch commits, acceptance criteria with the "partially"
   explained, Windows semantics deferred to RFC-078) and match RFC-075/076's
   shape closely enough, or does it need restructuring?
2. Is the two-commit split for the `done/` move (and its disclosure above)
   handled adequately, or should history be cleaned up some other way given
   the mismatched first commit message?
3. Whether F38 should now be assigned to me with a specific design direction
   (umask-derived vs. preserve-existing-mode), given review 050 called it
   the last item before M3 can close.
