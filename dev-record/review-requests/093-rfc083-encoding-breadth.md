# Review Request 093 — RFC-083: UTF-16, BOM wiring, and encoding override

Handoff: `rfcs/handoffs/083-text-encoding-breadth/023-rfc083-encoding-breadth.md`
Commit: `bc3fd7f` (pushed to `main`)

## §6's four falsifications, run for real

### 1 — UTF-16 BOM classification

Falsified by restoring the NUL sniff ahead of BOM detection in `classify`:

```
thread 'tests::file_kind_tests::classify_utf16be_bom_file_returns_text' panicked:
assertion `left == right` failed
  left: Binary
 right: Text
thread 'tests::file_kind_tests::classify_utf16le_bom_file_returns_text' panicked:
assertion `left == right` failed
  left: Binary
 right: Text
```

Both fail, both restored, both green again.

### 2 — the BOM line-1 lie

Falsified by reverting `document.rs`'s `Text` branch to decode raw bytes
directly (no `detect_bom` call):

```
thread 'tests::document_tests::a_bommed_file_diffs_identically_to_its_non_bommed_twin' panicked:
assertion `left == right` failed: a BOM'd file and its non-BOM'd twin must produce
identical comparable text — otherwise the diff reports a line-1 change with nothing
visibly different
  left: "\u{feff}hi\n"
 right: "hi\n"
```

**A real regression I introduced and caught before it shipped, worth stating
plainly rather than folding into "it works now."** My first version of the
BOM-aware decode stripped the BOM, then handed the *remainder alone* to
`chardetng` via unchanged `decode_bytes`. For a bare invalid byte after a
UTF-8 BOM (F88a's own fixture, `[EF BB BF FF 61 0A]`), that let `chardetng`
re-guess a single-byte legacy encoding that decodes `0xFF` without error —
turning a previously-guarded decode-substitution case into a silently
saveable one. The pre-existing F88a test caught it immediately
(`SaveCapability::Saveable` where `SaveableWithGuard` was expected). Fixed
by having a BOM select its encoding *directly* (`decode_body`'s `Utf8` arm
now decodes as UTF-8 unconditionally, matching what `encoding_rs::Encoding::decode`'s
own BOM-sniffing already did before this RFC, for the same reason: a BOM is
an explicit encoding declaration, and re-running detection on the
BOM-stripped remainder alone throws that declaration away). Both the F88a
test and the two BOM tests I added pass together now; I re-ran the full
suite after the fix, not just the one test that caught it.

### 3 — BOM save round-trip

```
thread 'tests::save_tests::a_document_loaded_with_a_bom_is_saved_with_a_bom' panicked:
assertion `left == right` failed
  left: [104, 105, 10]
 right: [239, 187, 191, 104, 105, 10]
```

(Falsified by dropping the `BomPolicy::Preserve` prepend in `save_text`.)

### 4 — encoding override updates the save label

```
thread 'state::tab::tests::set_encoding_redecodes_and_updates_the_save_label' panicked:
assertion `left == right` failed: the save label must follow the chosen encoding
  left: "windows-1252"
 right: "Shift_JIS"
```

(Falsified by skipping `set_encoding`'s `refresh_save_target` call — content
re-decodes correctly, but the tracked save label stays stale, exactly the
failure mode the acceptance criterion names.)

## Design decisions disclosed

**Overriding the encoding clears the tracked BOM to `Absent` rather than
carrying it forward.** Not addressed by the RFC directly, but load-bearing:
a BOM is an assertion about one *specific* encoding (a UTF-16LE BOM implies
UTF-16LE content). If a file loaded with a UTF-16LE BOM is overridden to
Shift_JIS, preserving the old BOM bytes would prepend two bytes that no
longer describe what follows them — a genuinely corrupt file, not a
degraded one. No UI in this handoff lets a user choose a BOM explicitly
(only `BomPolicy::Preserve` is ever exercised in production), so `Absent`
is the only safe default post-override.

**The encoding control is gated on `right_is_text`, not `can_save`.** They
usually coincide but not always: a missing right side can be `can_save`
(F88b: saving creates it) while having no real decoded text to fix, and
conversely a text-vs-non-mergeable-left pair can have real text worth
correcting on the right side while `can_save` is false for an unrelated
reason (the left side's kind). Gating on the more precise condition avoids
both a spurious control (missing side) and an unnecessary omission
(mismatched-kind pair).

**One control, the right side only** — not left, not (in Git mergetool
mode) the merged output. `current_encoding_label`/`right_label()` already
established "right" as the encoding-relevant side for save purposes before
this handoff; `set_encoding` and the toolbar control follow that precedent
rather than introducing a second one. Overriding the remote ("right" in
mergetool mode) still flows into `save_target`'s fallback the same way
`current_encoding_label` already does for Save As — I did not special-case
mergetool mode differently, matching `swap_sides`'s own established
mergetool-target-independence.

## A scope line I crossed by one line, disclosed

Handoff §7 lists "anything in `xlsx.rs`" as out of scope. `TextDocument`
gained two new fields (`bom`, `raw_bytes`), and `xlsx.rs`'s `excel_doc`
helper is one of the four construction sites across the codebase — the
compiler requires a value for both fields regardless of scope. I added
`bom: BomPresence::Absent, raw_bytes: Vec::new()` there, nothing else:
no behavior in `xlsx.rs` changed, and an Excel side has no encoding
concept for the override control to apply to. Flagging it rather than
letting a mechanical, compiler-forced two-line addition pass as if it
weren't a line in a file the handoff named explicitly.

## F93, corrected against the tree rather than trusted

`architecture.md`'s `ui-logic` table said 15 modules and listed four
deleted ones (`command_bar`, `scroll_sync`, `summary`, `tab_state`, all
`d69c83b`/F75(b) part 1) as if they still existed. Before fixing just
those four, I ran `find crates/forskscope-ui-logic/src -name '*.rs'`
against the table entry-by-entry rather than assume the finding was
complete. Two more things fell out:

- **A fifth stale entry F93 didn't catch**: `compare::hunk_decorations` was
  also deleted (`8f1af77`, F48 — a different commit than the other four),
  and nothing in the codebase references `DecorationIndex` anymore. F93's
  own count ("the tree has ten module files") happens to equal the number
  of entries that were both real *and* already correctly listed — which is
  why the discrepancy stayed hidden inside a finding about undercounting.
- **Three real modules the table never listed at all**: `compare::startup`,
  `session::persistence_recovery`, `settings::persistence_recovery` all
  exist, are wired, and were simply absent from the table before and after
  the four-module deletion. Added with one-line descriptions matching the
  table's existing style.

Corrected count: 13 modules, not 15 or 10.

## Verified live, with one honest gap

Built the desktop binary and drove two of the three features for real
(niri/wtype, screenshots): a UTF-16LE pair with a genuine line-2 difference
opened as `Text`, diffed correctly, showed `UTF-16LE` in the status bar,
and merge/save stayed available (clean UTF-16 decode → `ReadWrite`). A
UTF-8-BOM'd file compared against an otherwise-identical non-BOM'd file
reported **"Files are identical"** — the exact defect this RFC exists to
fix, confirmed working end to end, not only in the core-level test.

**I could not click-test the toolbar's Encoding `<select>`.** This
environment has `wtype` (keyboard only, Wayland virtual-keyboard protocol)
but no mouse-automation tool (`ydotool` is not installed, and none of the
available agents/tools drive a native GTK/WebKit window's mouse). Saying so
rather than skipping it silently or claiming a click I didn't make. What I
have instead: `set_encoding`/`change_encoding` are tested directly
(including the required falsification above) with a real misdetection
fixture, and the toolbar wiring (`onchange: move |e| change_encoding(&mut
store, index, e.value())`) is structurally identical to the diff-algorithm
`<select>` immediately above it in the same panel, which is already shipped
and working. If a mouse-capable tool becomes available, this is the one
piece I'd still want to see rendered and clicked for real.

## Scope

Touched exactly what §7 named (`file_kind.rs`, `encoding.rs`, the diff
toolbar's encoding control, the three documents) plus what compiling and
wiring those changes required: `document.rs`, `compare_prep.rs`, `save.rs`,
`lib.rs` (re-exporting `BomPresence`/`BomPolicy`/`COMMON_ENCODING_LABELS`
at the crate root, matching how `NewlineStyle`/`TextEncoding` already are),
`state.rs`/`state/tab.rs` (the new `Modal::ConfirmEncodingChange` variant
and `set_encoding`/`change_encoding` pair, mirroring
`change_diff_options`/`set_diff_options` exactly), `diff_actions.rs`
(`SaveRequest`/`SaveTargetState` threading `bom` alongside `encoding_label`
throughout), the modal dialog and its dispatcher wiring, and `xlsx.rs`'s
one mechanical fixture completion (above). Untouched: merge/save semantics
beyond the label, `save_capability`'s block sites, `build_side_text`,
RFC-084's patch work.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (732 core / 118 ui-lib / 118 ui-bin /
200 ui-logic, all green — +8 core tests, +2 ui-lib tests over the
pre-handoff baseline), `cargo xtask css --check`, `version-sync`, `i18n`
(243 keys, +5 new — the encoding-change confirmation dialog and toolbar
control strings, all translated), `rfc-sync`, `audit-deps`, `git diff
--check`, `mdbook build docs` — all green. `cargo audit` re-run: exit 0,
the same 14 pre-existing, unrelated warnings as before this handoff (no
new dependency added). CI run 33931845502 for `bc3fd7f` confirmed green.
