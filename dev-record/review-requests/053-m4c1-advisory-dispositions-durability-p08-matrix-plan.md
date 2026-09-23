# Review Request: M4-C1 — Advisory Dispositions and the Platform-Matrix Freeze

**Date:** 2026-08-11
**Reviewer stance:** focused documentation/evidence review
**Repository baseline:** `fb82887`
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c1-dispositions-and-matrix-freeze-handoff.md`, RFC-078

## 1. Implementation summary

One commit (`fb82887`), covering all four items — this slice is documentation
and evidence, not independent code fixes, so it landed as one cohesive act
rather than per-item commits (unlike M4-A/B).

- F7/N5 — `docs/src/maintainers/release-evidence/0.167.0-rc1/advisories.md`.
- F9/N2 — durability wording narrowed everywhere it appeared.
- F37 — RFC-078's P08 amended.
- `matrix-plan.md` — case-to-row mapping decided, owner questions collected.

## 2. Addressed items

### F7/N5 — advisories.md

All 14 `cargo audit` warnings (12 unmaintained, 2 unsound) plus the 2
suppressed advisories dispositioned. See §4 below for the reachability
statements, the review's stated main focus.

### F9/N2 — durability wording

See §5.

### F37 — P08 amendment

`rfcs/proposed/078-platform-runtime-acceptance.md`'s P08 now requires all
three `RecoveryDialogAction` choices (Exit, Continue — either variant, Reset)
on every platform row, explicitly overriding a row's otherwise-narrower
"Required level" — Exit is named as the one that matters most, since it
terminates the process from inside a startup modal, a path that can behave
inconsistently across WebKitGTK/WebView2/WKWebView. Also added a note to the
"Required platform matrix" table so it doesn't silently disagree with the
amendment (a row's narrower stated level no longer implies P08 is narrowed
too).

### `matrix-plan.md`

See §6 and §7.

## 3. Changed and created files

`docs/src/maintainers/release-evidence/0.167.0-rc1/advisories.md` (new),
`docs/src/maintainers/release-evidence/0.167.0-rc1/matrix-plan.md` (new),
`rfcs/proposed/078-platform-runtime-acceptance.md` (F37 amendment),
`crates/forskscope-core/src/save.rs` (F9, doc comments only — no behavior
change), `README.md`, `docs/src/users/features.md`,
`docs/src/users/merging.md`, `docs/src/maintainers/architecture.md`,
`docs/src/maintainers/threat-model.md` (F9 wording), `ROADMAP.md` (F7/F9/F37
entries, M4-B and M4-C1 progress notes — M4-B's own progress note had not
yet been added to the roadmap narrative before this slice; added it here
alongside M4-C1's).

## 4. The two unsoundness reachability statements (the review's main focus)

### `RUSTSEC-2024-0429` — `glib` 0.18.5, `VariantStrIter`

**Not reachable.** `VariantStrIter` has exactly one public constructor:
`glib::Variant::array_iter_str(&self)` (`glib-0.18.5/src/variant.rs:843`) —
confirmed by reading the crate source directly, not inferring from the
advisory text. Established non-reachability by searching every locally
cached version of every crate in the dependency chain (`atk`, `atk-sys`,
`cairo-rs`, `gdk`, `gdk-pixbuf`, `gdk-sys`, `gio`, `glib-macros`, `gtk`,
`gtk-sys`, `gtk3-macros`, `muda`, `tao`, `tray-icon`, `webkit2gtk`,
`webkit2gtk-sys`, `wry` — including versions newer than what's actually
locked, a superset of the real search space) for any call to
`.array_iter_str(`:

```
$ grep -rln "array_iter_str" ~/.cargo/registry/src/*/*/src
glib-0.18.5/src/variant.rs        # the definition
glib-0.18.5/src/variant_iter.rs   # the Iterator impls themselves
```

No other crate's source calls it. Also confirmed ForskScope's own crates
never reference `glib` at all (`grep -rn "glib::\|use glib" crates/` — no
matches).

### `RUSTSEC-2026-0097` — `rand` 0.7.3

**Not reachable, on two independent grounds.** First, `cargo tree -i
rand@0.7.3 --target all` shows the entire path passes through a
`[build-dependencies]` edge:

```
rand v0.7.3
└── phf_generator v0.8.0
    └── phf_codegen v0.8.0
        [build-dependencies]
        └── selectors v0.24.0 → kuchikiki → wry → dioxus-desktop → forskscope-ui
```

`rand` here is a build-time-only dependency of a codegen tool's build
script — it never links into the shipped `forskscope`/`forskscope-ui`
binary. This also answers the handoff's specific question of *why* an old
`rand` 0.7 is still in the graph: it's inert build tooling nobody has had
reason to touch, not a live runtime choice.

Second, even granting build-time-only code could still matter: read the
actual advisory text (`RUSTSEC-2026-0097.md`) rather than trusting the
one-line title. The unsound path requires *all* of: `log`+`thread_rng`
features, a custom `log::Log` implementation installed as the global
logger, that logger calling `RngCore` methods on `ThreadRng`, a reseed
happening during that call, and trace-level (or warn-level-with-failure)
logging. `phf_generator`'s own source defines no custom logger, and nothing
in this workspace's build scripts installs one. Both grounds are stated
independently in `advisories.md` rather than relying on either alone.

## 5. F9's outcome — full list of durability claims found

| Location | Before | After |
|---|---|---|
| `save.rs` module doc | "Writes are atomic (temp file in the same directory, then rename)" | States visibility-atomicity explicitly, explicitly disclaims power-loss durability, names the absence of `fsync`/`sync_all` |
| `save.rs::atomic_replace` doc | "Atomic on POSIX (`rename` within the same volume)" | Same treatment, function-level |
| `README.md` | "Safe saves — atomic write, ..." | "atomic write (never a partial file; not a power-loss guarantee — see [Merging])" |
| `docs/src/users/features.md` | "...automatic `.bak` backup; atomic write." | Same pattern, links to merging.md's full statement |
| `docs/src/users/merging.md` §"Saving the result" | "...atomically — it either appears complete or not at all, never partially written." (already visibility-scoped, not incorrect) | Kept, plus a new paragraph stating explicitly what "atomically" does and does not cover — the authoritative full statement everything else links to |
| `docs/src/maintainers/architecture.md` | "`save_text`, `AtomicSaveStrategy`, ... atomic write (RFC-007)." | Narrowed wording, **and** `AtomicSaveStrategy` (a type that does not exist anywhere in the code) replaced with the real `TargetPrecondition` — see the note below |
| `docs/src/maintainers/threat-model.md` §4 | "...and durably commits a legacy migration immediately if one applies." | "commits... through the same temp-file-then-rename primitive `save.rs` uses — visibility-atomic..., not a power-loss guarantee" |

**The explicit statement of the actual guarantee** lives in
`docs/src/users/merging.md` §"Saving the result": *visibility-atomic — a
concurrent reader never sees a partial file — not power-loss durable, since
neither the temp file nor its parent directory is `fsync`ed.* Every shorter
mention links or points there rather than restating it in full.

**Confirmed no `fsync`/`sync_all`/`sync_data` calls anywhere in
`forskscope-core`** (`grep -rn "fsync\|sync_all\|sync_data" crates/forskscope-core/src` —
zero matches), which is what makes "narrow the wording" (N2's option 1) the
right choice rather than "implement it" (option 2, explicitly out of scope
per the handoff's constraints).

**One latent documentation-truth defect surfaced while sweeping, not
introduced by this patch:** `architecture.md`'s `save` module row named a
type, `AtomicSaveStrategy`, that does not exist anywhere in
`forskscope-core::save` — confirmed by grep. Fixed in the same edit, flagged
as its own line item per this project's established convention (the same
class as `cli.md`'s exit-code table and `persist.rs`'s `VersionedEnvelope`
claim from earlier reviews).

## 6. Matrix plan's case-to-row mapping, with justification for every N/A/Spot-check

Full table and per-case justification prose are in `matrix-plan.md` §2.
Summary of the reasoning shape: **Required** everywhere unless a specific
reason narrows it. Three cases got narrowed on specific rows:

- **P03** (layout) — Spot-check on `linux-x11`/Windows/macOS, full only on
  `linux-wayland`: RFC-078's own text says "mandatory on WebKitGTK; repeat a
  basic layout observation" elsewhere, and X11 shares WebKitGTK's rendering
  engine with Wayland (F32 was a WebKitGTK table-layout bug, not a
  Wayland/X11-specific one).
- **P06** (async identity) — Spot-check on `linux-x11`/`windows-10`: RFC-075's
  deterministic tests already exhaustively cover the state machine itself;
  P06 confirms platform *integration*, and the narrower rows share an engine
  family with a row that already gets it in full.
- **P10** (binary/XLSX policy) — Spot-check on three of five rows: low
  platform-variance risk (a translated message rendering, not filesystem or
  process behavior).
- **P12** (session restart) — Spot-check on `linux-x11`/`windows-10`:
  substantial overlap with P08, which is Required everywhere per F37.

**P08 and P09 are Required on every row with no exceptions** — P08 per F37
directly; P09 (mergetool) because RFC-077 itself states "RFC-078 exercises
this on every primary platform" as its own explicit dependency for
`persist_noclobber`'s cross-platform guarantee.

No case was marked N/A outright in the final table — every case applies to
every row at some level, which itself seemed worth stating plainly rather
than manufacturing an N/A to fill the request's template.

## 7. Questions for the owner (collected, not guessed)

Reproduced from `matrix-plan.md` §4:

1. **Exact OS versions per row** — RFC-078 §118 requires concrete versions,
   not "current"/"oldest claimed." Specific sub-questions for `linux-*`
   (does the claimed baseline include libxdo-4 distros, which F44 currently
   blocks?), `windows-10` (narrow the 1903+ floor, or keep it?), and
   `macos-aarch64` (see Question 4).
2. **Executor owner/role per row** — `ROADMAP.md`'s "host access is
   confirmed available" note names no one; RFC-078's Precondition requires
   a named executor per row.
3. **Host-access status per row** — is there a real host (physical or VM)
   for each of the five rows right now, and does any row's plan rely on a
   VM specifically?
4. **macOS: one row or two?** RFC-078's "Required platform matrix" table
   lists macOS twice (oldest-claimed vs. current, different required
   levels) but the evidence layout and this handoff's row list both name
   only one `macos-aarch64` row/file. Does `macos-aarch64.md` need two dated
   sub-sections, or is the "current macOS, full matrix" target dropped for
   v1?
5. **Version/RC identifier** — `0.167.0-rc1` is a placeholder so this
   evidence has somewhere to live; what should the real directory be named,
   or does it stay a placeholder renamed at the actual cut?

## 8. Difference from the handoff, RFC-074, or RFC-078

None in scope. One judgment call worth naming: the handoff's required
review-request content (§8 item 6) asks for "the matrix plan's case-to-row
mapping with justification for every not-applicable" — the final mapping
has zero outright N/As (every case applies to every row at Required or
Spot-check level), so §6 above documents the Spot-check justifications
instead, which is where the real judgment calls live.

## 9. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass — 1112 (unchanged — no code behavior touched, only doc comments)
cargo xtask i18n                                                 pass — 227 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.166.1
cargo xtask audit-deps                                           pass
cargo audit                                                      pass — same 14 warnings as before, all now dispositioned in advisories.md
mdbook build docs                                                pass, no broken-reference/anchor warnings
git diff --check                                                 pass
```

No dependency added, removed, or version-changed (confirmed: `cargo audit`
reports the identical 14 warnings before and after this slice). No product
behavior changed — `save.rs`'s edits are doc comments only, confirmed by
`cargo build -p forskscope-core` and the unchanged test count.

CI run `31472515564`: success, on `fb82887`.

## 10. Unresolved issues and known limitations

- `matrix-plan.md` is structurally complete but explicitly **not frozen** —
  the owner-dependent fields (§7) are open, and RFC-078's Precondition
  requires them resolved before M5 can actually start, even though the plan
  itself is committed now per the Precondition's own wording ("before M5
  begins," not "before every field is known").
- `0.167.0-rc1` as a directory name is a placeholder (§7 Q5) — nothing in
  either evidence document's content depends on it being correct.
- F44/F45/F46 remain at their existing evidence levels (F44 has direct
  execution evidence; F45/F46 are inspection-only) — this slice only folds
  them into the plan as explicit P01 sub-requirements, per the handoff's
  explicit instruction not to gather platform evidence here.

## 11. Requested review focus

1. The two unsoundness reachability statements (§4) — whether the
   full-source-grep methodology for `glib` and the build-dependency-only
   argument for `rand` meet Gate C's bar, or need something stronger.
2. Whether landing all four items as one commit (rather than per-item, as
   M4-A/B did) was the right call given ROADMAP.md's edits couldn't be
   cleanly split across them without fragile partial-hunk staging.
3. The P03/P06/P10/P12 Spot-check justifications (§6) — whether any should
   be Required instead, particularly P06 given how central async-identity
   correctness is to this whole stabilization program (RFC-075/B1).
4. Whether the owner questions (§7) are the right set, or whether anything
   should have been decided rather than asked.
