# Review Request 094 — RFC-084: patch export conformance (F91), README/patch-export corrected (F92)

Handoff: `rfcs/handoffs/084-patch-export-conformance/024-rfc084-patch-conformance.md`
Commit: `e2dd55d` (pushed to `main`). CI run `33944326130`: green.

## §7's six falsifications, run for real

### 1 — CRLF patch applies with both `git apply` and `patch -p1`

Falsified by reverting `write_lines`'s terminator match back to the shipped
hardcoded `'\n'`:

```
thread 'crlf_patch_applies_with_both_tools' panicked at
crates/forskscope-core/tests/patch_apply.rs:56:5:
patch failed: patching file f.txt
Hunk #1 FAILED at 1 (different line endings).
1 out of 1 hunk FAILED -- saving rejects to file f.txt.rej
```

(`git apply`'s half of the same test failed the same way before I trimmed
the panic to the first failure — both tools reject it, matching the
audit's own reproduction.)

### 2 — mixed-newline files round-trip

Same reversion, same test file, different case (LF file with a CRLF
region introduced on the right side):

```
thread 'mixed_newline_file_round_trips' panicked at
crates/forskscope-core/tests/patch_apply.rs:188:9:
assertion `left == right` failed: GNU patch must round-trip a mixed-newline
file byte-for-byte
  left: [97, 108, 112, 104, 97, 10, 66, 69, 84, 65, ...]   (LF survived)
 right: [97, 108, 112, 104, 97, 13, 10, 66, 69, 84, 65, ...] (CRLF expected)
```

### 3 — a path containing a space applies with both tools

Falsified by reverting `diff_path_header` to plain `{prefix}/{path}`
(dropping the tab-suffix/quoting logic entirely):

```
thread 'space_in_path_applies_with_both_tools' panicked at
crates/forskscope-core/tests/patch_apply.rs:56:5:
patch failed: can't find file to patch at input line 4
Perhaps you used the wrong -p or --strip option?
...
1 out of 1 hunk ignored
```

Exact failure mode as the empirical scratch-repo reproduction I ran before
writing any code.

### 4 — a non-UTF-8 path component is never silently dropped

Falsified by reverting `display_path` to the shipped
`filter_map(|c| c.as_os_str().to_str())`:

```
thread 'tests::patch_tests::non_utf8_path_component_is_never_silently_dropped'
panicked at crates/forskscope-core/src/tests/patch_tests.rs:398:5:
a non-UTF-8 path component must not be silently dropped, leaving a bare
path header:
# forskscope patch: 1 files, 1 additions(+), 1 deletions(-)
--- a/
+++ b/
@@ -1,2 +1,2 @@
 a
-b
+B
```

The dropped component leaves a bare `a/`/`b/` header — a different,
shorter path than the one compared, exactly the failure mode the RFC
names as worse than a visibly-mangled one.

### 5 — exported context matches the user's setting

Falsified by reverting the new `export_patch_options` helper to
`PatchOptions::default()`:

```
thread 'ui::view::diff_actions::tests::export_patch_options_honors_the_context_lines_setting'
panicked at crates/forskscope-ui/src/ui/view/diff_actions.rs:964:13:
assertion `left == right` failed: exported context must follow the user's
setting, not the library default
  left: 3
 right: 1
```

### 6 — exporting with no changes tells the user

Falsified by reverting the `None` branch to the shipped `let _ = tab;
return;` (no notify call):

```
thread 'ui::view::diff_actions::tests::export_patch_notifies_when_there_is_nothing_to_export'
panicked at crates/forskscope-ui/src/ui/view/diff_actions.rs:993:25:
exporting identical files must notify the user, not do nothing
```

All six restored and green afterward; full suites re-run after each
restoration, not just the one test that caught it.

## Design decisions disclosed

**Git's quoting is replicated only to the extent apply-correctness needs
it — not its default cosmetic non-ASCII escaping.** Before writing any
code I built a throwaway scratch git repo and drove real `git diff`/`git
apply`/`patch -p1` against filenames containing a plain space, a double
quote, a literal tab, a backslash, a control byte (`0x01`), and DEL
(`0x7F`), confirming exactly what git emits in each case (trailing tab
for a lone space on `---`/`+++` only; full C-style quoting — `"`→`\"`,
`\`→`\\`, control bytes→`\NNN` octal — of the whole `a/`/`b/`-prefixed
string for the others; the `Binary files ... differ` line untouched in
every case). I did **not** replicate git's separate default behavior of
octal-escaping plain non-ASCII UTF-8 bytes (`core.quotePath=true`):
non-ASCII paths were never reported as failing to apply in the audit, and
the scratch probing confirmed both `git apply` and `patch -p1` accept
plain UTF-8 unquoted. Adding that would be pure cosmetic parity with
git's default, not a conformance requirement — it's the same judgment
call shape as F88's `had_decode_errors` narrowing: implement what the
failure mode actually needs, not everything the reference tool happens to
also do.

**A non-UTF-8 path component is rendered lossily (`to_string_lossy`,
U+FFFD in place of invalid bytes) rather than refused.** The RFC left
this open ("lossily represent it or refuse, and say which"). I chose
lossy over threading a `Result` through `to_unified`/`display_path` and
every caller, because the smaller change still satisfies "must not be
silent" — the component's presence and position survive, only the
specific invalid bytes are replaced with a visible marker — and a refusal
would need `export_patch` to handle a new failure path for a case the
audit never actually observed happening (no report of a real non-UTF-8
path in the wild), whereas the silent-drop *was* observed and reproduced.

**`export_patch`'s signature changed from `&Store` to `&mut Store`.**
Required to call any `notify_*` method (all take `&mut self`) for the
no-op fix. `Store` is `#[derive(Clone, Copy)]` — a cheap Dioxus-signal
handle — and every other action function in `diff_actions.rs` already
takes `&mut Store`, so this brings `export_patch` in line with its
siblings rather than introducing a new pattern. One call site
(`diff/toolbar.rs:188`) updated to match. Smaller than the
`ConfirmEncodingChange` UI-surface latitude the handoff referenced, but
disclosing it under the same grant since it's a public-function signature
change made without being asked for by name.

**Extracted `export_patch_options(&Store) -> PatchOptions`** as the
one piece of `export_patch`'s logic that's genuinely testable without a
live Dioxus runtime driving a native save dialog. `export_patch` itself
reaches `spawn(...)` (the real file-write path) whenever there's a patch
to export, and `rfd::AsyncFileDialog` has no place to run headlessly in a
unit test. Rather than skip the context-lines falsification or attempt a
fragile full end-to-end dialog test (the same gap review 093 disclosed
for the encoding `<select>`), I isolated the two-line struct-construction
step that does the actual wiring into its own function and tested that
directly — the same shape as `to_precondition` being carved out of
`build_request`/`save_tab` for the same reason, noted in this file's
existing test-module comment.

## F92 claims corrected

- **`README.md`** — the "Patch export" bullet claimed export from *"any
  file or directory comparison"*; directory export was never wired (RFC-084
  Q1, below) so this is now *"a file comparison"*. The compatibility half
  of the same bullet (*"compatible with `patch -p1` and `git apply`"*) is
  now true and left as-is.
- **`README.md`'s opening line**, unrelated to patch export but in the
  same file: *"ForskScope opens two files (or two directories) side by
  side"* implied the `forskscope <left> <right>` CLI form (shown in the
  code block directly above it) accepts directories. It does not —
  `cli.md`'s own "Compare two files" section names only files, and
  `load_path` on a directory errors. Directory comparison is real, but
  only through the separate Explorer flow the very next two lines already
  describe. Reworded to say so, rather than leave a claim I'd already
  confirmed false sitting untouched in a file this handoff was editing —
  handoff 019's precedent for claims outside the primary defect's scope.
- **`patch-export.md:4-5`** — the compatibility claim is now true; left
  unchanged.
- **`patch-export.md`'s identical-files line** — updated from "does
  nothing" to "shows a notice", matching the §5/§7.6 fix.
- **`patch-export.md:22`** — *"a standard POSIX unified diff"* is not
  accurate: unified diff is not itself a POSIX-standardized format (POSIX
  historically standardizes context diff), and the patches this tool
  emits now use git-specific path-quoting conventions that have no POSIX
  definition at all. Reworded to describe what's actually true: unified
  diff format, git-compatible headers, git's own quoting for names that
  need it.
- **`patch-export.md:44`** (context lines follow the setting) — now true;
  left unchanged.
- **`patch-export.md:38-40`** (directory export "planned for a future
  release") — checked against the RFC's own §4, which confirms this line
  **is already correct** and should not be touched. Left as-is.

## RFC-084 Q1 — confirmed, not re-litigated

`patch_from_directories` stays unwired. I did not touch
`core/src/patch/directory.rs`; the only correction was README's claim.

## Scope

Touched exactly what §8 named: `core/src/patch/unified.rs`,
`core/src/patch/build.rs`, `ui/src/ui/view/diff_actions.rs`,
`tests/patch_apply.rs`, `README.md`, `intermediate/patch-export.md` —
plus `core/src/patch/model.rs` (the `PatchLine`/`NewlineMarker` shape
change unified.rs and build.rs both need), `core/src/tests/patch_tests.rs`
(the non-UTF-8 unit test, since it needs `PathBuf` construction the
differential suite can't shell out with), `ui/src/state.rs` (removed two
now-stale `#[allow(dead_code)]` on `notify_info`/`Notice::info`, both
genuinely used now by the §5 fix — not touched otherwise), and
`ui/src/ui/view/diff/toolbar.rs` (the one `export_patch` call site,
one-line change for the `&mut Store` signature). `ui/src/i18n.rs` gained
one new key pair for the no-op notice. Untouched: `xlsx.rs`,
`patch_from_directories` and its wiring, encoding work.

## Gates

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace` (733 core / 5 patch_apply / 120
ui-lib / 120 ui-bin / 200 ui-logic, all green — +1 core test over the
pre-handoff baseline, +3 patch_apply differential cases), `cargo xtask css
--check`, `version-sync`, `i18n` (244 keys, +1), `rfc-sync`,
`audit-deps`, `git diff --check`, `mdbook build docs` — all green. `cargo
audit`: exit 0, the same 14 pre-existing, unrelated warnings as review 093
(no dependency added). Both `git` and `patch` were available in this
environment, so all three new differential cases ran for real against
both tools rather than being skipped.
