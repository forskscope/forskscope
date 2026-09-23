# F38 — `persist_noclobber` umask-derived permissions review

**Review date:** 2026-08-08
**Request:** `dev-record/review-requests/049-f38-persist-noclobber-umask-derived-permissions.md`
**Baseline:** `c54f2f4`, over `aa249a9`
**Responds to:** review 051 §3.3
**Review mode:** Independent verification, including mutation. No implementation changes made.

## 1. Verdict

**Approved.** F38 is resolved, and the mechanism is better than the one review
051 §3.3 implied.

**M3 is closed** — see §3.3. One new finding, non-blocking and not against M3
(§4).

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — 1094, unchanged |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo xtask version-sync` | Pass — `v0.165.1` |
| CI run `31229639704` | `success` on `c54f2f4` |
| `persist_noclobber` tests under `umask` 022 / 077 / 002 / **007** | 5/5 pass under each |
| `tempfile` 3.27.0, `Builder::permissions` `cfg(unix)`-gated | Confirmed |

`umask 007` is mine — a group-writable mask you did not test. It passes, which
matters because it is the one common mask where the correct answer (`0o660`) is
neither the old constant nor the restrictive case you did check.

### Mutation

Reverted the implementation to the pre-fix behaviour (`NamedTempFile::new_in`
plus `set_permissions(0o644)`) and re-ran your new test:

```text
umask 022 → ok           (5 passed)
umask 077 → FAILED
  expected 600 (a plain fs::write's mode in the same directory), got 644
```

So the test is genuinely load-bearing rather than vacuously passing, and it
fails with the exact message you wrote. That is the confirmation your §"The
test" section claims, obtained independently.

It also demonstrates the finding in §4.

## 3. Answers to the requested review focus

### 3.1 `Builder::permissions(0o666)` — right mechanism, and for a better reason than portability

Yes, and the deciding argument is stronger than the doc-comment one you cite.

`libc::umask` has no read-only form: querying it means `umask(0)` then
`umask(old)`, a **process-global** mutation with a window during which every
other thread's file creation sees mask 0. Rust test binaries run tests as
threads in one process, so a umask query inside this code would race the rest
of the suite — and would race any future concurrent save in the app itself.
`Builder::permissions` asks the kernel to apply the mask at `open(2)` time,
which is the same thing `fs::write` does and involves no shared state at all.
So the `unsafe`/global concern is not merely aesthetic; the alternative is
actually incorrect under concurrency, and this avoids it entirely.

On portability: the `cfg(unix)` block is the right shape, and Windows needs no
branch rather than needing a different one. Neither `persist_noclobber` nor
`atomic_replace` sets any permission there — both fall through to the platform
default, where a newly created file inherits the parent directory's inheritable
ACEs. There is no divergence between the two paths to test, which is why F38
was a Unix-only defect in the first place. Worth noting that this is reasoning
from documented Win32 default-ACL behaviour, not something observed; it needs
no RFC-078 platform case precisely because no code differs between the paths.

Your comment in `save.rs` explains all of this at the call site. Keep it.

### 3.2 `ROADMAP.md` F38 marked `**Resolved.**` — correct, keep it

The F26/F17 convention exists, your entry matches it, and the content is
accurate — I have now re-verified every claim in it independently, so it stands
as written rather than provisionally.

The general rule is worth stating since you asked: mark it resolved when the
fix has landed and its evidence is in the request, not when review confirms it.
A register that lags review by a round is a register that misreports the state
of the tree to whoever reads it next. If review disagrees, the entry gets
corrected — that is cheaper than it being wrong in the meantime.

### 3.3 M3 — closed

Your caution was right to exercise, and the answer is yes.

- M3's exit gate is "RFC-077 acceptance complete." RFC-077 is in `rfcs/done/`
  with its `## Implementation outcome` section, every acceptance criterion met
  or explicitly recorded as a judgment, and **B3 closed**.
- I swept the register: **F38 was the only entry tagged against M3**, and it is
  resolved. Nothing else is outstanding.

Two things that closing M3 does *not* mean, both worth recording:

**It closes out of table order,** before M2. That is permitted — M3's only hard
dependency is M1, and the "sequenced after M2" note is explicitly a
single-developer resource constraint, not a gate. But the plan and the tree have
now diverged, so the roadmap says so rather than implying the sequence held.

**M4 still cannot start.** It depends on M2 *and* M3.

M2's remaining work is **F23 and then the cut itself** — M2-A's content was
approved back at review 034 and F19–F22 are done; the register simply never
marked them, which I have now corrected. M2's exit gate additionally requires
the release mechanics to be *verified at a real release cut*, and none has
happened since `0.165.0`. So the path is F23 → cut → M2 closes → M4.

F40 (registered while you were on this) is against M4, not M3 — it concerns the
two-way merge session and the diff-option toggles, nothing in RFC-077's scope.

## 4. New finding — F41: the fix is unprotected on CI

The mutation in §2 passed under `umask 022`. CI runners use `umask 022`.

So a future regression to a hardcoded mode — the exact defect F38 just fixed —
would go green on every CI run forever. The new test is correct and detects the
defect; it just never runs anywhere that the defect is observable.

This is the fifth instance of the pattern this project has now named: a green
gate credited with more than it measures. `version-sync` blind to published
tags, `css_coverage` blind to layout, the release workflow that had never
fired, `i18n` blind to strings bypassing `t()` — and now a permission test that
can only fail under a mask CI does not use.

The fix is one CI step re-running these tests in a subshell under a
non-default umask, because a separate process is the only safe way to vary it
(§3.1's threading argument applies to the test binary too — an in-test
`libc::umask` would race the other 1093 tests). Registered as **F41** at M4
with the other gate-integrity work.

**Not your omission.** Review 051 §3.3 specified the property and the test, and
said nothing about where the test would be able to fail. That is the same class
of miss as F26's — I specified a mechanism's outcome without asking under what
conditions the check could actually fire.

## 5. Nit

`persist_noclobber_output_is_not_left_with_tempfiles_narrow_default_permissions`
removes `path` before the run but not `reference_path`. `temp_dir` keys on
`std::process::id()` and never cleans, and `fs::write` preserves an existing
file's mode — so a reused PID with a surviving temp directory from a run under a
different umask would compare against a stale reference. Improbable, and one
`let _ = fs::remove_file(&reference_path);` closes it, consistent with what the
rest of the file already does.

## 6. Notable quality observations

- Reaching for `Builder::permissions` over a umask query, and stating the
  no-process-global-state reason, arrived at the right answer for the right
  reason — the direction I gave named neither mechanism.
- Verifying under `umask 077` **and** reasoning about why the old
  implementation would have failed the new assertion, rather than only
  confirming the new one passes. That is the difference between testing a fix
  and testing a test.
- Declining to declare M3 closed from inside one patch (§"Not addressed here").
  Correct — the sweep is not visible from where you were standing.
- Flagging `f48bc00` as an unexplained commit rather than silently building on
  it. It was mine, recording review 051 §3.3 before you started; nothing was
  lost, and noticing was right.

## 7. Recommended next action

1. The §5 nit, if you touch the file again — not worth its own commit.
2. **F23** (`actionlint`), the last item before M2's release cut. Handoff:
   `rfcs/handoffs/074-v1-release-stabilization-program/f23-workflow-linting-handoff.md`.
   F41 folds into it, since both are CI-gate additions to the same file and
   splitting them would mean two passes over `ci.yml` for no benefit.
3. Then **M2's cut**, which its exit gate requires as verification.
4. **F40** rides with M4.
