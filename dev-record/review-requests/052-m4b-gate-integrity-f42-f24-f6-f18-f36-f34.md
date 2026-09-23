# Review Request: M4-B Gate Integrity — F42, F24, F6, F18, F36, F34

**Date:** 2026-08-11
**Reviewer stance:** focused implementation review
**Repository baseline:** `8d82f11`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4b-gate-integrity-handoff.md`

## 1. Implementation summary

All six items, sequenced per the handoff's suggestion (small/self-contained
first, F34 last since it needed iteration), each its own commit:

- `13767c5` — F42: actionlint/shellcheck and the F41 umask filter can no
  longer silently degrade.
- `454cb14` — F24: the CHANGELOG empty-section guard moves to preflight.
- `aed5a97` — F18: `xtask` no longer escapes `cargo fmt --check`.
- `534798a` — F6: `clippy --all-targets` run, findings fixed, gate added.
- `1014a86` — F36: decided and adopted a lightweight `Store` test harness.
- `8d82f11` — F34: a rendering check that would have caught F32.

## 2. Addressed items — falsifiability evidence (handoff §3, the review's main focus)

Every item below was demonstrated **failing** on a deliberately broken
input, then reverted, before being accepted as working.

### F42 — two gates that could silently degrade

**Guard 1 (shellcheck absence).** Simulated via an empty-of-shellcheck
`PATH`:
```
::error::shellcheck is not on PATH — actionlint's shellcheck pass would
silently skip every run: block instead of checking them (F42)
exit 1
```
Positive case (real PATH): `shellcheck --version` prints normally, exit 0.

**Guard 2 (F41 filter rename).** Grepped for a name that doesn't exist:
```
::error::the "permissions" filter no longer matches
persist_noclobber_output_is_not_left_with_tempfiles_narrow_default_permissions
— F38's umask regression test is no longer covered by this step (F42)
exit 1
```
Against the real (unrenamed) test name: lists it, then runs the filtered
tests normally, exit 0.

### F24 — CHANGELOG guard moved to preflight

The tree's own `CHANGELOG.md` currently has a genuinely empty `## [0.166.1]`
section (open since the post-`0.166.0` bump), so no synthetic fixture was
needed for the failing case:

```
$ cargo xtask version-sync            # dev mode — no arg
version sync passed for v0.166.1.     # must keep accepting the empty section

$ cargo xtask version-sync 0.166.1    # release mode — tag arg
CHANGELOG section for 0.166.1 has no content — release notes would ship blank
exit 1
```
Positive case: temporarily added real content under `## [0.166.1]`
(diffed against a pre-edit backup to confirm exact revert), release mode
passed; reverted, `git status --short CHANGELOG.md` empty afterward.

Also added 6 unit tests for the extraction logic itself (empty,
whitespace-only, real content, cross-version-boundary leakage, missing
header, trailing text after the bracket) — xtask's first tests.

### F18 — xtask escapes `cargo fmt --check`

Reintroduced one misindented line in `xtask/src/main.rs`:
```
Diff in .../xtask/src/main.rs:20:
-        let args: Vec<String> = std::env::args().skip(1).collect();
+    let args: Vec<String> = std::env::args().skip(1).collect();
exit 1
```
Reverted (diffed against a pre-edit backup), re-verified `exit 0`.

### F6 — `clippy --all-targets` never run or recorded

All 12 original findings were mechanical (§4 below has the list). After
fixing them, reintroduced one (`diff_corpus.rs`'s `manual_contains`) and
ran both gates side by side:

```
$ cargo clippy --workspace -- -D warnings              # mandatory gate
Finished ... — exit 0        (does NOT see it — this is the whole point of F6)

$ cargo clippy --workspace --all-targets -- -D warnings
error: using `contains()` instead of `iter().any()` is more efficient
  --> crates/forskscope-core/tests/diff_corpus.rs:76:9
error: could not compile `forskscope-core` (test "diff_corpus")
exit 1
```
Reverted, re-verified `--all-targets` clean.

### F36 — decided a `Store` test harness is worth it, proved it against a real gap

Not just "argued a harness would work" — built `state::with_test_store`
(headless `dioxus_core::VirtualDom`, no renderer/WebView/GTK) and used it to
write two **new, permanent** tests exercising F40's `change_diff_options`
guard directly:

```
test state::tab::tests::change_diff_options_defers_to_confirmation_when_the_tab_is_dirty ... ok
test state::tab::tests::change_diff_options_applies_immediately_when_the_tab_is_clean ... ok
```
Both assert on real `Store` state (`store.modal`, `store.tabs`) after
calling the real production function — no mocking, no AT-SPI. This closes
one of the five historically-named gaps as a direct byproduct of proving
the decision, not left as an unproven capability claim.

### F34 — rendering check, demonstrated against the actual historical bug

Reintroduced F32's exact defect (moved `hunk.rs`'s `sr-only` span from
inside `.cell` back to being its sibling — the literal historical bug,
confirmed against `cb6a852`'s fix), rebuilt, ran the new check against the
real binary:

```
$ python3 packaging/render_check.py ./target/debug/forskscope
FAIL: F34 rendering check found misalignment:
  - left pane: a row has 3 accessible children, other rows have 2 - a label
    is likely rendering as a sibling of the content cell instead of inside
    it (F32's defect shape)
exit 1
```
The check's own diagnostic text independently re-derived F32's actual root
cause from the geometry alone — it wasn't told what to look for beyond
"rows should be uniform." Reverted, re-verified:
```
$ python3 packaging/render_check.py ./target/debug/forskscope
OK: 7 left rows + 7 right rows all aligned within their pane.
```

## 3. F34's design decisions and reasons (handoff §4.5)

**What is asserted:** AT-SPI geometry (`Component.get_extents`) plus
accessible child-count, not a DOM-structure assertion or an image
comparison. Reasoning: F32's actual cause (a `sr-only` span as `.cell`'s
sibling) is one possible source of a column shift, but the *visual
symptom* — content starting at the wrong x, or a row gaining an extra
accessible child — is what a human actually sees, and would also be
produced by an unrelated future markup mistake with a different DOM
shape. A DOM-string assertion would only catch a re-occurrence of this
exact pattern; geometry catches the outcome. An image comparison was
rejected as the handoff itself suggested — brittle across renderer/font
versions, and this bug doesn't need pixel-perfect comparison to detect,
just alignment.

**Where it runs:** `release.yml`'s `linux` job, immediately after "Build
release binary," before packaging — not `ci.yml`. Matches the handoff's own
framing ("CI needs a display; the release preflight may be the more honest
home given the cost") and keeps the fast push/PR feedback loop free of a
new Xvfb/AT-SPI dependency chain. It runs against the actual release
artifact, not a debug rebuild.

**The fixture:** one pair, `tests/fixtures/text/{left,right}_all_hunk_kinds.txt`,
producing exactly one Replace, one Delete, one Insert hunk (verified by a
new `diff_corpus.rs` test asserting the exact kind sequence) — per review
044, instead of promoting either of the two ad-hoc demo fixtures already
scattered across this project's `.git-exclude/tmp/` history.

**Which level was built:** the full geometry check, not the fallback
launch-smoke-test the handoff explicitly permitted as an acceptable lesser
deliverable. Chose this because a bare "does it launch" check would not
have caught F32 itself — the window opened and rendered "successfully" in
F32's case, it just looked wrong — and F34 is named "the most valuable item
in this slice."

## 4. F6's twelve findings, all mechanical (no suppressions)

| File | Lint | Fix |
|---|---|---|
| `diff_corpus.rs` (×2) | `manual_contains` | `.iter().any(\|s\| *s == v)` → `.contains(&v)` |
| `diff_tests.rs` | `manual_contains` | same rewrite |
| `dir_index_tests.rs` | `cmp_owned` | `PathBuf::from(rel)` (owned, just for comparison) → `*rel` |
| `search_index.rs` | `type_complexity` | extracted `Row<'a>`/`Hunk<'a>` type aliases for a test fixture builder's signature |
| `job_tests.rs` (×4) | `assertions_on_constants` | moved into `const { assert!(..) }` blocks — checked at compile time now, still named `#[test]`s |

## 5. F36's decision and where it's recorded (handoff §4.6)

**Decision: yes, worth it — but scoped narrowly.** Not a full "UI test
harness with event dispatch"; specifically a way to get a real, usable
`Store` in a test, since that's what the five named occurrences actually
needed (calling an action function and asserting on resulting state), not
simulated clicks or rendered-DOM assertions.

**Mechanism:** `state::with_test_store` (`#[cfg(test)]`, `pub(crate)`) spins
up a headless `dioxus_core::VirtualDom` with a trivial root component,
captures the `Store` it constructs via a `thread_local!`, and hands it to
the caller's closure — usable both inside and after the triggering
`rebuild_in_place()`, no renderer/WebView/GTK involved. Discovered by
direct experimentation (tried it before committing to the design) rather
than assumed feasible.

**Recorded:** `docs/src/maintainers/local-dev.md` §"Adding tests" →
"Testing `Store`-dependent UI logic (F36)" — states the pure-predicate
pattern (F35/F40) as the *default*, `with_test_store` as the fallback for
when logic genuinely needs `Store` state. `ROADMAP.md`'s F36 entry marked
Resolved with the same summary.

**Explicitly not done:** the other four historically-named occurrences
(RFC-076 patch 4 startup wiring, its C1 CLI-mode fix, patch 6's recovery
queue, RFC-077's Save As default-path regression) were **not**
retroactively converted to use this harness. The handoff named this a
decision item, not a retrofit; each stays on AT-SPI evidence unless a
future patch touches that code anyway.

## 6. Changed files

`.github/workflows/ci.yml` (F42, F18, F6), `.github/workflows/release.yml`
(F34), `xtask/src/main.rs` (F24, +6 tests), `.gitignore` (F34, `__pycache__`),
`crates/forskscope-core/tests/diff_corpus.rs` (F6 fix, F34 fixture test),
`crates/forskscope-core/src/tests/{diff_tests,dir_index_tests,job_tests}.rs`
(F6), `crates/forskscope-ui-logic/src/compare/search_index.rs` (F6),
`crates/forskscope-ui/src/state.rs` + `state/tab/tests.rs` (F36),
`docs/src/maintainers/local-dev.md` (F36), `packaging/render_check.py` (new,
F34), `tests/fixtures/text/{left,right}_all_hunk_kinds.txt` (new, F34),
`tests/fixtures/README.md` (F34), `docs/src/maintainers/gtk-smoke-test.md`
(F34), `ROADMAP.md` (all six).

## 7. Difference from the handoff

- **F34**: the handoff's "what is asserted" options didn't name accessible
  child-count as a signal alongside geometry; added it because the
  reintroduced bug produced *both* symptoms together and child-count is a
  cheaper, order-of-magnitude simpler check to explain, so it's asserted
  first as a fast-fail before the geometry comparison.
- No other differences in scope or design.

## 8. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo fmt --manifest-path xtask/Cargo.toml --check              pass
cargo test --workspace                                          pass — 1112 (1105 at M4-B's start, after M4-A/review 054's C1 fix: F42/F24/F18/F6 add 0 net workspace tests — F24's 6 new tests are in xtask, not counted here*; F36 +6 — 3 new with_test_store-based tests, doubled lib+bin; F34 +1 — 1 new diff_corpus.rs test, not doubled)
cargo clippy --workspace -- -D warnings                          pass
cargo clippy --workspace --all-targets -- -D warnings            pass (new gate)
cargo xtask i18n                                                 pass — 227 keys (unchanged)
cargo xtask css --check                                          pass
cargo xtask version-sync                                         pass — v0.166.1
cargo xtask audit-deps                                           pass
mdbook build docs                                                pass, no broken-reference warnings
git diff --check                                                 pass
python3 -m py_compile packaging/render_check.py                  pass
shellcheck (manual, all new run: blocks in ci.yml/release.yml)   pass — no findings
```

\* `cargo test --workspace` doesn't include `xtask` (not a workspace
member, DEC-005/F18) — its 6 new tests are separately verified via
`cargo test --manifest-path xtask/Cargo.toml`, pass.

CI run `31465286879`: success, on `8d82f11` — this confirms `ci.yml`'s
gates including the new `actionlint`/`shellcheck` step (which validated
`release.yml`'s new F34 step as syntactically clean YAML/shell) and the new
`--all-targets` clippy step. It does **not** exercise `release.yml`'s
`linux` job itself, since that only triggers on tag push — see §9.

## 9. Unresolved issues and known limitations

- **F34's CI wiring is unverified in real GitHub Actions.** This sandbox
  has no Xvfb, so the exact sequence (`apt-get install xvfb dbus-x11
  at-spi2-core python3-gi gir1.2-atspi-2.0`, then `xvfb-run --auto-servernum
  dbus-run-session -- env NO_AT_BRIDGE=0 python3 packaging/render_check.py
  ...`) could not be dry-run outside a real tag-triggered release. What's
  proven: the detection logic itself, end to end, against a real running
  window (this session's dev sandbox has a working Wayland compositor and
  AT-SPI bus). What's not proven: that this exact apt-package/xvfb-run/
  dbus-run-session combination behaves the same way on a fresh
  `ubuntu-latest` GitHub Actions runner. `actionlint` confirms the YAML/
  shell is syntactically valid; it cannot confirm the packages resolve or
  the AT-SPI bus actually comes up under Xvfb. If the first real release
  run surfaces friction here, it's the same class of "first real run finds
  something" the project already normalized with R0/F17.
- F42/F24/F18/F6/F36 have no comparable gap — all fully demonstrated in
  this exact environment.

## 10. Requested review focus

1. Whether F34's Xvfb/CI-wiring gap (§9) is acceptable to land now with the
   detection logic proven separately, or whether it should be held until
   verified against a real (or manually-triggered) `release.yml` run first.
2. F34's choice of accessible child-count as a first-class signal alongside
   geometry (§7) — worth keeping both, or is one sufficient/redundant?
3. F36's scope boundary — confirm not retrofitting the other four
   occurrences was the right call, or should any of them be converted now
   that the mechanism exists and is cheap to use?
4. Anything in the F6 fix list (§4) that reads as suppression-shaped rather
   than genuinely mechanical.
