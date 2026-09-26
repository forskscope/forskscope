# Handoff 056 — RFC-080 §5: the file-row size cap

**From:** architect. **Date:** 2026-09-27. **Release:** 0.173.0.
**RFC:** `rfcs/accepted/080-explorer-tiered-directory-comparison.md` §5 and §2a.
**Register:** F77's remainder. This is the last item in 0.173.0.

## Why this waited, and why it can be done now

RFC-080 §5 deferred the file-row size cap deliberately: *"a file exceeding it
needs somewhere to rest, and the only honest resting state is the tier-1 state
§4 defines."* Tier 1 shipped in `04e690b`, so that state exists —
`EqualityEvidence::MetadataMatch`, rendered `≈`, labelled for a **file** row as
*Size matches; contents not compared*.

F77 recorded the defect this closes: browsing a directory starts an uncapped
content read of every common file, so two identical large files are read in
full because the user opened a folder. The Explorer's digest path applies no
cap at all today.

## 1. The cap

- It governs **automatic** comparison only — what browsing starts. **Deep
  Compare must remain uncapped**: RFC-080's non-goals keep it "the place for a
  full per-file report", and a user who opened it asked for the work.
- Over the cap, a file row rests at **`MetadataMatch`** when the sizes match,
  and at the existing `SizeDifferent` when they do not — a size mismatch is
  free and certain, and the cap must not hide it.
- **Choose the value against measurement**, as F117 and F120 did, and put the
  basis in the constant's doc comment. What matters is the read cost per pair,
  not a round number: `file_digest_equal_with_cancel` streams both sides to the
  first difference, so the worst case is two **identical** files of the cap's
  size read end to end. Say what that costs on this machine and what you are
  admitting.
- `FileSizeClass` and `PerformanceLimits` already exist (`core/src/job/limits.rs`)
  and are settings-backed. **Read F126 first:** that whole persisted block is
  written and never read back, so putting the cap there without wiring it would
  add a seventh inert value. Either wire it properly or keep the cap as a
  constant and say why.

## 2. What the user sees

A capped row is not silently blank: it shows `≈` with the file label, exactly
as a same-size-different-content pair does after tier 1.

**That is a deliberate collapse of two different situations** — "we compared
the sizes and stopped" and "we read both files and they matched byte for byte"
are not the same, and RFC-080 §4 chose one glyph for both on purpose. Check the
rendered result and tell me if it reads as misleading in practice; the RFC's
decision stands unless the product says otherwise.

## 3. What this is not

- Not the tier-2 verify control — that is 0.174.0.
- Not a change to Deep Compare, the directory verdict, or `DigestEpoch`.
- Not a cap on the tier-1 walk, which reads no contents at all.

## Verification

1. **Falsify the cap:** a pair of identical files over it must not be read.
   Prove the read did not happen rather than asserting it — the `chmod 000`
   technique from tier 1's criterion 1 is the precedent, or measure.
2. **A size mismatch over the cap is still `Different`**, and free.
3. **Under the cap nothing changes:** the existing Explorer digest tests pass
   unchanged.
4. **The capped row renders `≈` with the file label**, in the real app.
5. Report what the cap admits, measured, in the terms of §1.

## Scope

- **In:** the Explorer's digest path, the cap's home (constant or
  `PerformanceLimits`, per §1), tests, and the user documentation where it
  describes what browsing compares.
- **Out:** Deep Compare, tier 2, F126's wider inert-settings question,
  `CHANGELOG.md` and `ROADMAP.md`.

## Gates

The standard set, including the Windows-target clippy and `cargo fmt
--manifest-path xtask/Cargo.toml --check`, `cargo xtask` with `css --check`,
`i18n`, `rfc-sync`, `audit-deps`, `version-sync`, `ui-logic-connectivity`,
`ui-logic-docs`, `mdbook build docs`, and a green CI run with its ID.

Reply in `.git-exclude/review-request/128-file-row-size-cap.md`.
