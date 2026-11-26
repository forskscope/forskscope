# Handoff 080 — 0.192.0, part 2: replaced lines stay distinct

**From:** architect. **Date:** 2026-10-15. **Release:** `0.192.0`.
**Implements:** RFC-090
(`rfcs/accepted/090-replaced-lines-in-the-red-green-safe-palette.md`), accepted
by the owner on 2026-10-15. **Register:** F199.

**The RFC is the authority.** Where this document and the RFC differ, the RFC
wins. If implementation shows the RFC is wrong anywhere, including a measured
figure, **stop and tell me before changing anything.** Do not change the design
yourselves, and do not edit any RFC.

This ships in the same release as handoff 079, and both touch `xtask`. Do them in
either order, but keep the gates separate.

---

## 1. The token

In `01-tokens.css`, the red–green-safe overrides gain **one** token,
`--rep-bg`, with exactly RFC-090 §1's values:

| Theme | `--rep-bg` |
|---|---|
| Dark | `#35504c` |
| Light | `#abbab7` |
| Night | `#2c4042` |

- The Standard palette is unchanged.
- `cargo xtask css` regenerates `main.css`.

## 2. The gate

Extend `cargo xtask colour-vision` exactly as RFC-090 §3 says. Per theme:

| Palette | Pairs | Simulations | Each must reach |
|---|---|---|---|
| Red–green safe | removed–added, replaced–removed, replaced–added | normal, deuteranopia, protanopia | ΔE ≥ 20 |
| Standard | removed–added | normal | ΔE ≥ 20, unchanged |
| Standard | replaced–removed, replaced–added | normal | ΔE ≥ 15 |

- **Cross-check:** the gate's figures for the red–green-safe replaced pairs must
  match RFC-090 §2's tables to one decimal. All 24 figures in the two tables,
  including tritanopia if the gate prints it. If any differs, stop and report
  it, as you rightly did for RFC-088.
- **Falsify:** set the Light red–green-safe `--rep-bg` back to the Standard
  amber. The gate must fail, naming Light, the pair and the simulation.

## 3. Verification

**RFC-090's acceptance criteria 1 to 4, one by one:**

1. **The tokens:** the three values, and Standard unchanged. Show the diff of
   `01-tokens.css`.
2. **The gate:**
   - three pairs measured;
   - figures matching §2;
   - in CI;
   - falsified as above.
3. **Screenshots:**
   - **Fixture:** the one from request 153b, with a deleted line, an inserted
     line, and a replaced line with *Inline diff* on.
   - **Images:** the three themes in red–green safe, each beside its
     deuteranopia simulation, using the same Machado matrix and command as
     153b.
   - **Look at each image, then describe it.** The point to check: the replaced
     row is distinct from the deleted row and from the inserted row in every
     simulated image.
4. **`CHANGELOG.md` is mine.** Just tell me the known limitation is resolved, and
   I will write the entry.

## Scope

- **In:** §1 and §2; `docs/src/users/settings.md`, if its *Colour vision* text
  says what changes and now needs to mention replaced lines.
- **Out:**
  - Any other token.
  - The Standard palette.
  - A non-colour mark for replaced lines (RFC-090's non-goal).
  - RFCs.
  - `CHANGELOG.md`, `ROADMAP.md`, the version.

## Gates

The standard set, including `colour-vision` (extended), `css --check`, the
Windows-target clippy with `--all-targets`, `cargo fmt --manifest-path
xtask/Cargo.toml --check`, and a green CI run with its ID.

Reply in `.git-exclude/review-request/155-replaced-lines-stay-distinct.md`.
