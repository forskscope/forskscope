# RFC 083: Text Encoding Breadth — UTF-16, BOM, and Override

**Status.** Implemented — `bc3fd7f`, review 096 (2026-09-05). Ships in 0.169.0.
**Tracks.** Register F90. Audit 2026-09-01 findings A8, A11, A12, docs #15, #18.
**Touches.** `core/src/file_kind.rs`, `core/src/encoding.rs`, the diff toolbar,
`intermediate/file-types.md`, `users/known-limitations.md`.
**Depends on.** Nothing. Independent of RFC-082.

## Summary

Three encoding gaps, all sharing one shape: **the capability exists in core and
the product does not reach it.**

## 1. UTF-16 files cannot be compared at all

`classify` sniffs 8 KiB for a NUL byte before anything else. UTF-16-encoded ASCII
is half NUL bytes, so **every UTF-16 file is `FileKind::Binary`** — and with
`enable_binary_comparison` off (the default) the tab simply errors.

The decoder is fine. Verified by the architect:

```
classify(utf16le_file)  = Ok(Binary)
decode_bytes(same)      = label "UTF-16LE", "hello\nworld\n", lossy = false
```

`BomPresence::Utf16Le`/`Utf16Be` already exist and are unused. **The gate in
front of a working decoder is the whole defect.**

This limitation appears in **no** document. WinMerge, Meld and KDiff3 have all
handled UTF-16 for years, and ForskScope ships for Windows, where UTF-16 is
ordinary.

**Design: run `detect_bom` before the NUL sniff.** A UTF-16 BOM forces
`FileKind::Text`. A BOM-less UTF-16 heuristic (NUL density at alternating
offsets) is optional and explicitly *not* required by this RFC — the BOM case is
the common one and the cheap one.

## 2. BOM handling is designed, tested, and unwired

`detect_bom`, `BomPresence` and `BomPolicy::resolve_bytes` have zero production
call sites. A UTF-8 BOM therefore survives as a literal `U+FEFF` **inside line 1's
content**.

Consequences, in order of how badly they read:

- a BOM'd file diffed against a non-BOM'd one reports **line 1 as changed with no
  visible difference** — the diff lies about what changed;
- applying that hunk silently adds or removes a BOM;
- save round-trips it, so nothing is corrupted — which is why this survived.

`file-types.md:47` says "UTF-8 BOM is preserved". That is **true by accident**:
it is preserved because nobody strips it, not because `BomPolicy` runs.

**Design: strip at load, record `BomPresence` on the document, re-apply per
`BomPolicy` at save.** All three pieces are written and tested already.

## 3. No encoding override

Detection is `chardetng` + `encoding_rs` with no user control; the status bar
shows the label read-only. The audit reproduced a realistic misdetection — a
short Shift_JIS document detected as `windows-1252`, rendering mojibake with no
in-app recovery.

For a tool that markets legacy-encoding preservation, and against three
comparators that all expose a picker, this is the most visible gap of the three.

**Design: an encoding selector in the diff toolbar that re-decodes the bytes
already loaded** — no re-read — and updates the save label. Note the interaction
with RFC-082 §D4: changing the label changes what a save can represent, so the
lossy-encode guard must run against the *chosen* label.

## Acceptance criteria

- A UTF-16LE/BE file with a BOM opens as text and diffs correctly.
- A BOM'd file compared with an otherwise identical non-BOM'd file reports **no
  difference on line 1**.
- A file loaded with a BOM is saved with a BOM; one loaded without is saved
  without.
- Choosing an encoding re-decodes without re-reading, and the save label follows.
- `known-limitations.md` documents whatever is *not* supported after this lands.

## Sequencing

**The documentation corrections are immediate and unconditional.** UTF-16 being
unsupported is currently documented nowhere, and `file-types.md`'s BOM sentence
describes a mechanism that does not run. Both are wrong today and can be
corrected today, regardless of whether the features are ever built.

**The features are post-v1.** The audit's own judgement, which this RFC adopts:

> Items 6 and 7 from the Top 10 (CRLF patches, UTF-16) I would **not** call
> v1-blocking: both are honest feature limitations that can ship documented,
> provided the README and `patch-export.md` stop claiming otherwise.

That proviso is the binding half. Shipping the limitation is fine; shipping it
while the docs are silent is not.

## Open questions — both closed 2026-09-05

Closed by the architect under the role division the owner stated on 2026-09-05:
**scheduling and design are the architect's; authorization is the owner's.** Both
had been parked as owner decisions that did not need to be.

- **Q1 — BOM-less UTF-16: BOM-only. Closed, no heuristic.** A NUL-density
  heuristic misfires on genuine binaries, and mistaking a binary for text is the
  more damaging direction in a tool that opens files for editing. The BOM case
  covers what Windows tools actually emit. `known-limitations.md` must then say
  plainly that BOM-less UTF-16 is unsupported — an honest limitation, not a
  silent gap. **Override this if you want the heuristic; it is a behaviour
  change, not a correction.**
- **Q2 — order: not separated. Closed.** The question only existed under the
  original "post-v1" scheduling. The whole RFC ships in 0.169.0, so there is
  nothing to sequence apart.
