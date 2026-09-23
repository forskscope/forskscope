# Review Request 082: F92 — remove two false control claims

**Governing task.** `dev-record/handoffs/013-f92-remove-two-false-control-claims.md`
**Register.** F92 (two control-claim edits only). Governing: RFC-082, Gate A.
**Baseline.** `main` at `b79d9d0` (docs: RFC-082 accepted (Gate A); M7 begins)
**Commit.** `32d2210`

Documentation only — no line in `crates/`, `xtask/`, `ROADMAP.md`, or any RFC touched.

## 1. `docs/src/intermediate/file-types.md`

**Before:**
> The detected encoding label is shown in the status bar (e.g. `UTF-8`, `Shift_JIS`). **Save preserves the original encoding by default.** A non-UTF-8 file saves as that same encoding. If you add characters outside the charset, a save guard warns you before writing.

**After:**
> The detected encoding label is shown in the status bar (e.g. `UTF-8`, `Shift_JIS`). **Save preserves the original encoding by default.** A non-UTF-8 file saves as that same encoding.
>
> **Limitation:** if you add characters that the saved encoding cannot represent, they are currently written as numeric character references instead of being rejected or flagged — for example, saving `😀` into a `Shift_JIS` file writes the literal text `&#128512;`.

The false-guard sentence is gone, not softened. No promise about `EditabilityClass::requires_save_guard()` was added — it still has zero call sites (F88). The new limitation sentence describes `encode_text`'s actual current behavior (`crates/forskscope-core/src/encoding.rs:65`): `Encoding::encode` on an unmappable character inserts a numeric character reference rather than failing.

## 2. `docs/src/maintainers/threat-model.md`

**Before:**
> Very large files: the diff engine has a deadline policy (RFC-012) that may produce approximate results; a warning banner is shown. No crash or panic path exists from oversized input (fuzzing confirmed in test suite).

**After:**
> Very large files: the diff engine has a deadline policy (RFC-012) that may produce approximate results; a warning banner is shown. Whether a crash or panic path exists from oversized input is unverified — no fuzzing or property-based testing exists in this repository.

The fuzzing parenthetical is gone. Checked whether the surrounding "no crash or panic" claim could stand without it: it can't — grepped `crates/forskscope-core/src/diff/` for any test touching `deadline`, and found none (`deadline_ms` appears only in `engine.rs`'s own implementation and `options.rs`'s field/default; zero `#[test]` in `engine.rs`). Nothing in the test suite demonstrates crash/panic freedom under oversized input, so per the handoff I weakened the sentence to say exactly that, rather than leaving an unsupported absolute claim standing on nothing.

## 3. Checks

- `grep -ri "fuzz" docs/src/` returns one line — my own new sentence, which states fuzzing does **not** exist. No remaining text claims fuzzing is performed.
- `mdbook build docs` — passes clean. (`docs/book/` is gitignored, untracked, and unaffected by this diff.)
- `git diff --check` — clean.
- Confirmed no code touched: `git status` shows only the two `docs/src/*.md` files staged.

Noticed but not touched: `rfcs/index.html` shows a one-line uncommitted diff (a regenerated searchindex hash) unrelated to this change and predating it — left alone, out of scope for F92.
