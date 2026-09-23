# RFC-077 — Save As pre-check fix and `done/` move review

**Review date:** 2026-08-08
**Request:** `dev-record/review-requests/048-rfc077-review050-saveas-precheck-and-done-move.md`
**Baseline:** `e027c1d`, over `7595802` and `765f60c`
**Responds to:** review 050 §3.1 and §3.2
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved.** Both recommended actions are complete and **RFC-077 is closed**.

M3 does not close: **F38 is the only item left**, and §3.3 below settles its
design direction so it can be picked up without another round.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — 1094, unchanged |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo xtask version-sync` | Pass — `v0.165.1` |
| CI run `31228349781` | `success` on `e027c1d` |
| `SaveAsPrecheck` resolves all three cases | Confirmed — `New` / `Overwrite` / `Blocked` from `inspect_save_target` |
| RFC-077 at `rfcs/done/`, Status `Implemented (Milestone M3)` | Confirmed |
| `rfcs/README.md` counts | 51 implemented / 16 proposed |

`precheck_save_as_target` reuses the same `inspect_save_target` classification
`build_request` applies, so the pre-write gate and the safety boundary can no
longer disagree about what a path is. That was the actual defect — not that the
check was unsafe, but that it answered a different question than the one the
write would ask.

## 3. Answers to the requested review focus

### 3.1 The `## Implementation outcome` section

**Satisfies all three requirements, and improves on the brief in one place.**

Folding the review-driven corrections into the patch bullets they correct — N1
into patches 1–2, C1/C2 into patch 5 — reads better than a separate list would
have, because a reader learns what the patch ended up being rather than what it
was before review. That matches RFC-076's shape.

The "partially" paragraph is the part worth singling out. It distinguishes what
is *argued* (the shared `check_precondition` path means the core tests exercise
the real save-time code, not a parallel implementation) from what is *proven to
the same standard as the runtime-evidenced cases* — and says plainly that the
first is a judgment call. An RFC that records a judgment as a judgment is worth
more later than one that records it as a fact.

Windows `persist_noclobber` semantics are explicitly deferred to RFC-078, and the
section states that the folder move is not M3 closing. Both correct.

### 3.2 The two-commit split — leave it

**No history cleanup.** Three reasons:

- Both commits are pushed and CI ran on the pair. Rewriting pushed history to
  correct a message is disproportionate to the harm.
- `e027c1d`'s message states that it completes `765f60c` and what happened, so a
  reader of either lands on the explanation.
- This project already settled the principle during the 2026-08-04 collision:
  force-pushing to tidy is worse than the untidiness. That reasoning was right
  then and applies unchanged.

Disclosing it under its own heading rather than leaving it discoverable only by
reading two diffs is the correct handling, and it is what makes leaving it
defensible.

### 3.3 F38 — assigned, and the direction is settled by construction

**Yes, take it.** The choice review 048 left open resolves itself once you look
at which preconditions reach the code:

```rust
match request.precondition {
    TargetPrecondition::MustBeAbsent => persist_noclobber(target, &bytes)?,
    TargetPrecondition::MustMatch(_) | TargetPrecondition::Force => { …atomic_replace… }
}
```

`persist_noclobber` runs **only** for `MustBeAbsent` — a target that by
definition does not exist. So "preserve the existing target's mode" is not a
weaker option, it is **inapplicable**: there is no existing mode. The direction
is umask-derived.

**State it as a property, not a mechanism:**

> A file created by the no-clobber path has the permissions it would have had if
> the normal save path had created it.

That also fixes the test. The current one asserts the constant `0o644`, which is
only correct under `umask 022` and would fail under any other — assert instead
that the created file's mode equals that of a file `fs::write` creates in the
same directory in the same process. Self-maintaining, and it tests the property
rather than one environment's instance of it.

**A distinction worth recording while you are here.** The *other* half of what
review 048 raised belongs to a different finding: `atomic_replace` writes a temp
and renames, so overwriting an existing file replaces its mode with the temp's.
A user's `0600` file saved normally comes back `0644` under a default umask.
That is mode-loss on overwrite, which is **F9**'s territory (audit N2, "does not
preserve the original file's permissions/extended metadata") — not F38's, and
not in M3's scope.

So the two options you were choosing between turn out to belong to two different
findings: umask-derived for F38's newly created files, preserve-existing-mode for
F9's overwrites. Do only the first.

## 4. Notable quality observations

- Catching the bad-pathspec `git add` immediately via `git status`/`git show
  --stat` after committing, and correcting forward with an explicit message
  rather than a silent amend, is the right instinct on pushed history.
- Choosing a fixture where the Blocked target is the *default* Save As path,
  specifically to work around the AT-SPI `EditableText` limitation rather than
  claiming the input field was exercised, is honest test design.
- Verifying the directory target untouched after the run, not just the toast text.

## 5. Recommended next action

1. **F38**, per §3.3 — umask-derived, property-based test. This is the last item
   before M3 closes.
2. Then M3 closes and M4 begins: Gate C, advisory dispositions, `matrix-plan.md`,
   and the accumulated register (F6–F9, F13, F16, F18, F23–F25b, F31, F34–F37,
   F39).
3. **F23** still gates M2's cut — unchanged and still outstanding.
