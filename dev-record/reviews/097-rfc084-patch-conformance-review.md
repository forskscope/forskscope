# Review 097 — Request 094: RFC-084 patch export conformance

**Reviewer:** architect. **Date:** 2026-09-05. **Reviewed:** `e2dd55d`.
**Verdict:** **Approved. F91 closed, RFC-084 → `done/`.**
**F92 stays open** — two doc-vs-doc contradictions were outside this handoff and
I have closed them myself (§5). No follow-up for you.

## 1. Falsification, reproduced here

I restored the hardcoded `'\n'` in `write_lines` and ran the differential suite:

```
test crlf_patch_applies_with_both_tools ... FAILED
test mixed_newline_file_round_trips ...... FAILED
test space_in_path_applies_with_both_tools ... ok
test generated_patch_transforms_left_into_right ... ok
  Hunk #1 FAILED at 1 (different line endings).
```

**Exactly the two CRLF cases**, with the space case and the base cases still
passing — so these are targeted, not a blanket assertion. And the failure text is
**real GNU `patch` output**, not a synthetic message: the suite shells out to the
actual tools the documentation names. That is the strongest form this project
has. Full suite: **1233 passed, 0 failed.**

## 2. The method on quoting is the best part of this change

You built a scratch git repo and drove real `git diff` / `git apply` /
`patch -p1` against a space, a double quote, a literal tab, a backslash, a
control byte and DEL **before writing any code** — then implemented what the
failure modes actually require.

And you drew the line deliberately: **not** replicating git's default
octal-escaping of plain non-ASCII UTF-8, because both tools accept it unquoted
and the audit never observed it failing. That is conformance reasoning rather
than imitation, and you named the precedent correctly — it is F88's
`had_decode_errors` narrowing in a different file.

## 3. Lossy over refusal — right, and for the right reason

RFC-084 left this open and asked you to say which. Rendering a non-UTF-8
component with U+FFFD keeps the component's **presence and position**, which is
what "must not be silent" was protecting; refusing would thread a `Result`
through `to_unified`/`display_path` and every caller for a case never observed
in the wild, while the silent drop **was** observed and reproduced. Correct
trade.

## 4. The README line you found on your own

`README.md`'s opening sentence said ForskScope *"opens two files (or two
directories) side by side"* directly above a `forskscope <left> <right>` code
block — implying the CLI form takes directories. **I verified it does not:**
`classify` returns `Unsupported { "not a regular file" }` for a directory, so
`load_path` errors.

Correcting a claim you had confirmed false, in a file you were already editing,
is exactly handoff 019's precedent. Good.

`patch-export.md:22` — your rewording sidesteps the standards question entirely
by describing what is true (unified-diff format, git-compatible headers, git's
quoting) rather than asserting what is not. That is better than the argument you
gave for it in the request, which rests on a POSIX claim I would not have
accepted without checking; the change stands on its own without it.

## 5. F92 — two contradictions remained, and I have closed them

The audit named **three** doc-vs-doc contradictions. Your work resolved **#4**
(README vs `patch-export.md:38` on directory export). Two were outside this
handoff, so I fixed them:

- **#7** — `diff-options.md:65` said `context_lines = 0` *"collapses all equal
  hunks"*. **False:** `hunk.rs:60` requires `context_lines > 0` to collapse, so
  `0` means never collapse. `settings.md:88` was the correct side. Corrected to
  match.
- **#8** — `known-limitations.md` said *"files over 64 MiB trigger a time-bounded
  diff with a shortened deadline."* **False twice:** `deadline_ms` is a constant
  `Some(5_000)` for every diff regardless of size, and the size behaviour is
  4 MiB → warn banner, 64 MiB → confirmation prompt.

**Worth recording:** I nearly wrote those thresholds as 10 KB / 100 KB, having
read them out of `load_guard.rs` — they are **test fixtures** inside a
`#[cfg(test)]` module. Production is `job/limits.rs`. Caught before writing, but
it is the same instrument error as counting `grep` matches, and it would have
put false numbers into a document *while correcting a false document*.

## 6. Disclosures

`&mut Store` on `export_patch` — required for any `notify_*`, brings it in line
with every sibling in the file, one call site updated. Fine, and right to
disclose.

Extracting `export_patch_options` so the context-lines wiring is testable without
a live dialog is the same shape as `to_precondition`, and it is the honest answer
to a genuinely untestable surface — better than skipping the falsification or
faking an end-to-end dialog test.

Removing two stale `#[allow(dead_code)]` because `notify_info`/`Notice::info`
are now genuinely used is a small decrement of F75's count, unprompted.

**F91 closed. RFC-084 → `done/`. F92 remains open** for the audit's separate
*missing limitations* items, which are not this handoff's.
