# RFC 084: Patch Export Conformance

**Status.** Implemented — `e2dd55d`, review 097 (2026-09-05). Ships in 0.170.0.
**Tracks.** Register F91. Audit 2026-09-01 findings A9, A17, A18, docs #4, #5, #6.
**Touches.** `core/src/patch/unified.rs`, `core/src/patch/build.rs`,
`ui/src/ui/view/diff_actions.rs`, `crates/forskscope-core/tests/patch_apply.rs`,
`README.md`, `intermediate/patch-export.md`.

## Summary

`intermediate/patch-export.md:4` states the exported patch is *"compatible with
`patch -p1` and `git apply`"*. **It is not, for two input classes**, and a third
documented behaviour is not implemented.

## 1. CRLF files produce patches that do not apply

`split_lines` strips the terminator into a `NewlineMarker`; `write_lines`
(`unified.rs`) re-appends a hardcoded `'\n'` — verified:

```rust
out.push(line.origin.marker());
out.push_str(&line.content);
out.push('\n');            // ← unconditional
```

So a CRLF file's patch carries LF terminators against a CRLF target. The audit
ran both tools the document names: `git apply` → *"patch does not apply"*;
`patch -p1` → *"Hunk #1 FAILED at 1 (different line endings)"*.

**Every CRLF file — that is, the Windows platform ForskScope ships for —
produces an unusable patch.**

`crates/forskscope-core/tests/patch_apply.rs` has **zero** CRLF cases;
confirmed. The differential test that would have caught this exists and does not
exercise the input class that breaks it.

**Design: carry `NewlineMarker` into `PatchLine` and emit the line's own
terminator**, falling back to `\n` for `None`, before the `\ No newline` marker.

## 2. Paths with spaces, and paths that are silently truncated

`display_path` emits `--- a/my file.txt` for a path containing a space, which
`patch -p1` cannot resolve (`git apply` copes). And it uses
`filter_map(|c| c.as_os_str().to_str())`, so a **non-UTF-8 path component is
silently dropped** — emitting a path that is not the one compared.

**Design: adopt git's C-style quoting for paths that need it; return an error (or
an explicit lossy marker) rather than dropping a component.** Silently emitting a
different path than the one compared is the worse of the two failures and has no
defensible form.

## 3. Context lines ignore the user's setting

`export_patch` passes `PatchOptions::default()` — a hardcoded `context_lines: 3`
— while `patch-export.md:44` says the value follows the **Context lines**
setting. One-line fix: read `store.settings.read().context_lines`.

## 4. Directory patch export does not exist

`patch_from_directories` has **zero UI call sites**; the only export button is
the file-level one. `README.md:96` claims *"from any file or directory
comparison"*. `intermediate/patch-export.md:38` states the opposite —
*"not directory trees… planned for a future release"* — and **is correct**.

**Decision required (§Open questions): wire it, or correct the README.** Both are
defensible; leaving them contradicting each other is not.

## 5. Silent no-op

Exporting a patch for identical files returns with no feedback. The code comment
says *"Notify but don't error"* and then does not notify.

## Acceptance criteria

- A patch exported from a CRLF file applies with both `git apply` and `patch -p1`.
- Mixed-newline files round-trip.
- A path containing a space applies with both tools.
- A non-UTF-8 path component is never silently dropped.
- Exported context matches the user's setting.
- Exporting with no changes tells the user so.
- `README.md` and `patch-export.md` agree, and both are true.

## Testing

`patch_apply.rs` is a **differential** suite — it shells out to the real tools —
which is the right shape and is why it is trusted. It simply has no CRLF,
mixed-newline or space-in-filename case. Add those three; each must be shown
failing before the fix.

That is the whole lesson of this finding and it is worth stating: the test that
would have caught it existed, was well designed, and did not cover the input
class that breaks. Coverage of a *format* means coverage of its variants.

## Sequencing

**Documentation first and immediately** — the compatibility claim and the
README's directory-export claim are false today and are corrected with an edit.

**Fixes post-v1.** The audit does not consider these v1-blocking, and neither do
I: a patch export that fails loudly on CRLF is a limitation, not a data-loss
path. The proviso is the same as RFC-083's — shipping the limitation is fine only
if the documents stop claiming otherwise.

## Open questions — both closed 2026-09-05

Closed by the architect under the role division the owner stated on 2026-09-05:
scheduling and design are the architect's, authorization the owner's.

- **Q1 — directory patch export: correct `README.md`, do not wire it. Closed.**
  `patch_from_directories` exists and is tested, but wiring it means a new export
  surface, a destination picker and a progress story for a large tree — a
  feature, not a conformance fix, and one nobody has asked for. **This RFC is
  about making exported patches apply**, and a second export path would widen it
  into something else. *Note the irony deliberately:* `patch_from_directories`
  is another built-and-tested-and-unwired layer (F75's count), and the correct
  answer here is still not to wire it — the fix for an unwired layer is a
  decision about whether it should exist, not reflexive wiring.
- **Q2 — CRLF priority: 0.170.0. Closed.** The question presupposed the post-v1
  scheduling the owner has since overruled.
