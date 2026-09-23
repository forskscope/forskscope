# Handoff 023 — RFC-083: UTF-16, BOM wiring, and encoding override

**From:** architect. **RFC:** `rfcs/accepted/083-text-encoding-breadth.md`
(accepted 2026-09-05 — read it; this handoff assumes its reasoning).
**Release:** 0.169.0, lead item. **Register:** F90, plus F93 in §5.

## 1. Why this is first

Not because a gate demands it. **A whole encoding family cannot be opened at
all**, and unlike most open findings it is invisible until a user hits it —
nothing in the product or the documentation says UTF-16 is unsupported.

Three defects, one RFC. The third is the one users notice without understanding.

## 2. §1 — UTF-16 is refused by a gate in front of a working decoder

`classify` sniffs 8 KiB for a NUL byte before anything else, so **every** UTF-16
file is `Binary`; with binary comparison off by default the tab errors.

**The decoder is fine.** Verified in the register: `decode_bytes` on a UTF-16LE
file returns label `UTF-16LE` and correct text, not lossy. `BomPresence::Utf16Le`
and `Utf16Be` already exist, unused.

**So this is a classification fix, not a decoding one.** Do not touch
`decode_bytes` unless something forces you to; if it does, say so.

**Decided, not open (RFC-083 Q1): BOM-only. No NUL-density heuristic.** A
heuristic misfires on genuine binaries, and mistaking a binary for text is the
more damaging direction in a tool that opens files for editing. BOM-less UTF-16
stays unsupported **and must be documented as such** — see §5.

## 3. §2 — the BOM layer is unwired, and the diff currently lies

`detect_bom` and `BomPolicy` have **zero production call sites**. A UTF-8 BOM
therefore survives as a literal U+FEFF inside line 1, so a BOM'd file compared
against an otherwise identical non-BOM'd file reports **line 1 changed with
nothing visibly different**.

That is the defect a user meets and cannot explain: two files that look
identical, one reported difference, no visible cause.

**This is the fifth instance of the pattern** — built, tested, documented, never
connected (F52, F75, F84, F88a, and RFC-085's own wiring on 2026-09-04). Treat
"the module exists and has tests" as evidence of nothing until a call site
exists.

`file-types.md`'s *"BOM is preserved"* is currently true **by accident**. After
this it must be true by mechanism: loaded with a BOM → saved with a BOM; loaded
without → saved without.

## 4. §3 — no encoding override

A user who knows better than the detector has no way to say so. Choosing an
encoding must **re-decode without re-reading**, and the save label must follow
the choice — the same relationship F87 established for Save-as-UTF-8, where the
tracked encoding sticks for later saves.

## 5. Documentation, in the same change

- **`known-limitations.md`** — state that **BOM-less UTF-16 is unsupported**.
  §2's decision creates that limitation deliberately; leaving it undocumented
  would make it a silent gap rather than an honest one.
- **`file-types.md`** — UTF-16 support, and BOM behaviour stated as mechanism.
- **F93** — `architecture.md` documents four `ui-logic` modules **this project
  deleted** (`d69c83b`, F75(b) part 1). Correct it in this pass; it is minutes,
  and it is the same defect class as everything else in this handoff.

## 6. Falsification

Each must be demonstrated **failing** against the shipped defect:

1. A UTF-16LE file with a BOM opens as text and diffs correctly — falsify by
   restoring the NUL sniff ahead of BOM detection.
2. A BOM'd file against an otherwise identical non-BOM'd file reports **no
   difference on line 1** — falsify by unwiring `detect_bom` again. This is the
   one that proves §3 rather than just exercising it.
3. Round-trip: BOM in → BOM out; no BOM in → no BOM out.
4. Choosing an encoding re-decodes and the save label follows.

## 7. Scope

**In:** `core/src/file_kind.rs`, `core/src/encoding.rs`, the diff toolbar's
encoding control, the three documents above.

**Out:** merge/save semantics beyond the label; `save_capability`'s block sites;
anything in `xlsx.rs`; RFC-084's patch work (that is 0.170.0).

**If the override control needs UI beyond the toolbar, stop and tell me** rather
than deciding scope yourself — but note that handoff 022's §6 wrongly excluded
"any UI work" and would have shipped dead code, so **wiring is in scope even
when it lands in `forskscope-ui`.** That was my error and I am not repeating it.

## 8. Gates

The usual set. `cargo audit` now measures a real dependency chain; if it reports
anything, stop and tell me.
