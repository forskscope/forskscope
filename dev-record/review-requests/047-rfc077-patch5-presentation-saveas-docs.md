# Review Request: RFC-077 Patch 5 — Presentation, Save As Confirmation, Docs

**Date:** 2026-08-07
**Reviewer stance:** focused implementation + documentation review
**Repository baseline:** `c789636` (`ui+docs: RFC-077 patch 5 - presentation,
Save As confirmation, docs`)
**Governing documents:** `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`;
RFC-077; review 049

## Summary

Per RFC-077's implementation sequence step 7 ("Add integration tests and
documentation") and the handoff's patch 5 ("target transitions, presentation,
Save As confirmation UX, docs"):

1. **Presentation** — a quiet `Result: <merged>` line under the diff file
   header for a Git mergetool tab, RFC-077 §"Tab state": "Mergetool
   presentation shows a quiet, explicit output line... it must not expose a
   destructive control." It's plain text, not a button.
2. **Save As confirmation** — `SaveAsModal` now checks whether the typed
   destination exists *before* attempting any write, and shows
   `Modal::ConfirmSaveAsOverwrite` if so — closing the one RFC-077
   test-design item review 048 explicitly flagged as unmet ("select an
   existing Save As destination: overwrite confirmation is required").
3. **A bug found while wiring (2)** — the toolbar's Save As button defaulted
   its initial path to `tab.right_path`. That was correct before this RFC
   (mergetool mode aliased `right_path` to the merged output), but patch 4a
   made `right_path` always mean the compared input — so this default was
   quietly pointing Save As at the *remote*, not the merge target, for every
   mergetool tab. Fixed to default to `tab.save_target`'s path.
4. **Documentation** — `cli.md`, `git-integration.md`, `merging.md`,
   `gtk-smoke-test.md` updated; two pre-existing inaccuracies corrected
   (below), not just RFC-077-related additions.

## Files Changed

`state.rs` (+`Modal::ConfirmSaveAsOverwrite`), `ui/overlay/modals/file.rs`
(+`ConfirmSaveAsOverwriteModal`, `SaveAsModal`'s pre-write existence check),
`ui/overlay/modals.rs` + `ui/view/settings.rs` (registration/dispatch),
`ui/view/diff.rs` (`DiffHeader`'s Result line), `ui/view/diff/toolbar.rs`
(Save As default-path fix), `i18n.rs` (+3 keys), `assets/css/11-view-diff.css`
+ regenerated `main.css`, and the four doc files.

## Two pre-existing doc inaccuracies corrected, not introduced by RFC-077

- **`cli.md`'s exit-code table** claimed `forskscope <left> <right>` exits
  non-zero for "path not found." It doesn't, and never did:
  `LoadOptions::allow_missing` means a missing side loads as empty content,
  not an error. What patch 4a actually made non-zero is unsupported
  *argument arity* (`parse_startup_args` rejecting anything but 0/2/3 args)
  — the table now describes that instead.
- **`merging.md`'s "Saving the result"** claimed Save always writes to "the
  right-side file path." That was never fully true (mergetool mode wrote to
  `$MERGED` before this RFC too, just via the buggy `right_path` alias), and
  is unambiguously wrong now that `right_path` always means the compared
  input. Rewritten to name `save_target` for both modes explicitly.

Flagging these as their own line item because they weren't things this patch
broke — they were latent, and the RFC-077 documentation pass is what
surfaced them.

## Design Notes

- **`ConfirmSaveAsOverwrite`'s existence check is a plain `Path::exists()`,
  not the fresh `inspect_save_target` inspection `build_request` still does
  when the user actually confirms.** This dialog is a pre-write UX gate, not
  a safety boundary — the real precondition check happens inside `save_as`
  regardless of which path led to it, so a race between this check and the
  eventual write still surfaces correctly as `ConfirmOverwrite` (review 048
  C1's fix), not a silent overwrite.
- **The Result line reads `tab.launch_mode` directly, not `save_target`.**
  `save_target` is `None` while `Loading`, and the tab title/launch mode are
  already known synchronously from `open_compare_request` — showing the
  merged path doesn't need to wait for the async load to complete the way
  the save-target-dependent UI (Save/Save As button `can_save` gating) does.
- **No new tests.** `SaveAsModal`'s existence check and `DiffHeader`'s Result
  line are both `Store`-dependent Dioxus rendering (F36) with no pure logic
  beyond what's already tested — `Path::exists()` and a `match` on
  `CompareLaunchMode` aren't meaningfully unit-testable in isolation from the
  component.

## Runtime Evidence

Rebuilt the debug binary, ran both new UI paths under `.git-exclude/tmp/rfc077-patch5/`,
driven via AT-SPI.

**Result line.** `forskscope local.txt remote.txt merged.txt` — screenshot
confirms `Result: .../merged.txt` renders under the file header, alongside
the unchanged `.../local.txt ↔ .../remote.txt` compared-input header.

**Save As default-path fix + pre-write confirmation, together.** Opened Save
As on the mergetool tab above: the path field read the corrected default
(`merged.txt`, confirmed via `Atspi.Text.get_text` since the field visually
truncates — not `remote.txt`, which is what the pre-fix code would have
shown). Clicking Save (destination exists) showed
`ConfirmSaveAsOverwriteModal` with the exact path, *before* any write
occurred. Confirmed "Overwrite":
```
merged.txt:     the current merge-buffer content       (written)
merged.txt.bak: the pre-write merged.txt content        (backed up)
local.txt:      unchanged, byte-identical
remote.txt:     unchanged, byte-identical
```
Also verified the negative case earlier in this same session (a Save As
Cancel followed by process kill) never touched `remote.txt`, confirming the
old wrong-default bug's blast radius was contained to what I was testing,
not a persistent risk in the fixture.

## Open Question: Is RFC-077 Ready to Move to `rfcs/done/`?

Not doing this without asking, unlike RFC-076 patch 6 where the lifecycle
move was explicitly in that patch's scope — RFC-077's handoff doesn't assign
the move to a specific patch, and the RFC's own "Dependencies" section says
"Runtime migration and target save behavior are **accepted under RFC-078**,"
which could mean formal acceptance deliberately waits for RFC-078's
cross-platform verification.

Where I believe things stand against RFC-077's acceptance criteria:

| Criterion | Status |
|---|---|
| Compared right input and save output cannot share one ambiguous field | Met — typed `CompareRequest`/`SaveDestination`, `CompareTab.launch_mode` |
| Mergetool preparation fingerprints the actual merged target | Met — `inspect_save_target` |
| Existing/missing/appeared/deleted/changed/replaced target tests pass | **Partially** — see below |
| A path expected to be absent is committed with no-clobber semantics | Met — `persist_noclobber`, tested + runtime-verified |
| Save As never bypasses conflict checks merely because a path was selected | Met |
| Normal compare continues saving to the right input with its fingerprint | Met |
| Save As and reload preserve compared input identity | Met — `reload_tab` derives from `launch_mode`, never touches `left_path`/`right_path` |
| Git/JJ documentation matches observed behavior | Met, this patch |

The "partially" on target-transition tests: every named transition
(missing→appeared, existing→deleted, existing→changed, existing→replaced) is
covered by `check_precondition`'s own tests (`save_target_tests.rs`) — and
`check_precondition` is exactly what `build_request`/`save_text` call, not a
parallel implementation — plus two of the four have live runtime evidence
(existing-target round trip in patch 4b's review, blocked/replaced-by-directory
in review 048's C2 evidence). What's *not* independently re-verified is each
transition specifically through a live mergetool process with a real
concurrent external actor (vs. the core-level tests' direct function calls).
I believe this is sufficient given the shared code path, but it's a
judgment call, not a fact I can point at as thoroughly proven the way the
patch 4b/048 runtime evidence is.

Requesting a decision rather than making one: move to `done/` now with that
caveat stated, hold until RFC-078, or is there a specific transition you
want runtime-verified before either?

## Tests And Gates Run

```text
cargo fmt --check                                              pass
cargo test --workspace                                          pass — 1094 (unchanged from review 049's baseline — no new tests, see Design Notes)
cargo clippy --workspace -- -D warnings                          pass
cargo xtask i18n                                                 pass — 223 keys (+3: "Result:", "Overwrite existing file?", "A file already exists at this path.")
cargo xtask css --check                                          pass
cargo xtask audit-deps                                           pass
cargo xtask version-sync                                         pass — v0.165.1
git diff --check                                                 pass
mdbook build docs                                                pass, no broken-reference warnings
```

CI run `31226885116`: success, on `c789636`.

## Not Addressed Here

- F38 (permission/umask), F39 (i18n gate blindness) — both explicitly
  deferred by review 048/049 to before-M3/M4 respectively.
- RFC-078's platform-specific runtime matrix (Windows/macOS mergetool
  behavior, Windows file-replacement semantics for `persist_noclobber`).
- F23 — still gates M2's cut, untouched by this work.

## Requested Review Focus

1. The `rfcs/done/` question above.
2. Is a plain `Path::exists()` pre-check acceptable for
   `ConfirmSaveAsOverwrite`'s gating, given the real safety check happens
   downstream regardless — or should this check also use
   `inspect_save_target` (classifying Blocked destinations *before* showing
   "Overwrite?" rather than after confirming, which currently routes through
   the existing C2 `describe_block` toast one step later)?
3. The two corrected doc inaccuracies (§"pre-existing... corrected") — confirm
   they read as fixes to latent gaps rather than something this patch should
   have avoided touching.
