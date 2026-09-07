# RFC 086: Stable Hunk Identity, and an Amendment to RFC-015 Rule 4

**Status.** Proposed
**Scheduling.** 0.171.0 candidate. See `ROADMAP.md` § "Remaining proposed RFCs", which must list every file in `proposed/` and `accepted/` and nothing else (F83).
**Tracks.** Register F47. RFC-015 §8 rule 4, recorded **Not met** since F40.
**Touches.** `core/src/diff/model.rs`, `core/src/diff/engine.rs`,
`core/src/merge/session.rs`, `rfcs/done/015-undo-redo-transaction-log.md`.

## Summary

Make hunk identity a function of hunk content and position alone, by removing
the process-global counter from it. Then **amend RFC-015 rule 4**, because as
written it promises something this design cannot deliver safely, and pretending
otherwise is how a rule stays "Not met" for months.

## 1. The mechanism, traced

`hunk_id_for` (`diff/model.rs:167`) hashes five inputs:

```text
hash(diff_id, ordinal, kind, left_range, right_range)
```

Four are derived from the hunk itself. The fifth, `diff_id`, comes from a
**process-global counter** incremented on every call:

```rust
static DIFF_COUNTER: AtomicU64 = AtomicU64::new(1);           // engine.rs:20
let diff_id = DIFF_COUNTER.fetch_add(1, Ordering::Relaxed);   // engine.rs:132
```

So **every recompute changes every hunk's identity**, including a recompute over
byte-identical content producing byte-identical hunks.

The consequence is not cosmetic. `MergeSession::swap_in` locates the hunk to
restore **by id**:

```rust
.find(|h| h.hunk_id == transaction.hunk_id)
.ok_or(CoreError::InternalInvariant {
    message: "transaction references missing hunk".into(),
})?
```

**An internal-invariant error is reachable by ordinary user action** — that is
its own small defect, and worth naming: `InternalInvariant` should mean "this
cannot happen", not "the user changed a setting."

## 2. `diff_id` has no other consumer

Checked rather than assumed:

- Its only use is as an input to `hunk_id_for`.
- `MergeSession::diff_id()` (`session.rs:107`) has **zero callers** outside
  tests — a dead accessor.
- No staleness or generation check reads it. Generation guarding is
  `DigestEpoch`'s job (F78), in a different layer, and is unaffected.

So removing it from the hash removes it entirely, and the accessor with it.

## 3. What removing it fixes, precisely

**A recompute over unchanged content preserves every hunk id.** That is exactly
the case F47 names — *"even one that changes nothing"* — and it is not
hypothetical: `change_diff_options` recomputes with different options, and for
options that do not alter hunk boundaries the hunks are identical. Today that
path discards the undo stack after a confirmation prompt; afterwards it need
not.

**What it does not fix, stated plainly:** after an *edit*, ranges shift from the
applied hunk onward, so those hunks are genuinely different and their ids change
correctly. Stable identity does not make history survive a structural change,
because the hunks the history refers to no longer exist.

## 4. Collision safety

Without `diff_id`, two different documents can produce equal hunk ids. That is
safe **because ids are only ever compared within one `MergeSession`** — `swap_in`
searches `self.hunks`, and the merge log belongs to the same session.

This is an invariant the code currently gets for free from a global counter and
would then depend on. **It must be written down and asserted**, not assumed:
a `debug_assert` that a session's hunk ids are unique within that session, and a
doc comment on `hunk_id_for` stating the scope of uniqueness. Otherwise this
becomes a latent trap for whoever next compares ids across documents.

## 5. The amendment — this is the part that needs a decision

RFC-015 §8 rule 4 says:

> Recomputing diff after an edit must not erase undo history.

**As written, that cannot be met safely**, and §3 explains why: after an edit the
hunks the history references are gone. There are three ways to respond, and only
one is defensible.

- **Rebase transactions onto the new hunks heuristically** — match old to new by
  content or position. **Rejected.** A wrong match applies stored rows to the
  wrong hunk, which is the F73/F85 defect class: writing to a place the user did
  not choose, silently. This project's standard is to fail closed rather than
  guess, and there is no way to guess here that is safe at the edges.
- **Preserve history and refuse to apply it when it no longer maps** — keeps the
  stack visible but makes undo fail at an arbitrary later point. Worse than
  discarding, because the failure arrives detached from its cause.
- **Preserve exactly when every logged hunk still exists; otherwise discard,
  after telling the user.** Implementable, honest, and strictly better than
  today.

**Recommendation: the third**, and amend rule 4 to say what is actually
achievable:

> Recomputing the diff must not erase undo history when the recomputed hunks are
> identical to the ones the history references. When they are not, the history
> is discarded and the user is told before it happens.

`change_diff_options` already asks before discarding, so the confirmation path
exists; what changes is that it stops firing when nothing structural changed.

## 6. Acceptance criteria

- Recomputing over byte-identical content with unchanged options yields
  **identical hunk ids** — the same undo stack still applies.
- `change_diff_options` with an option that does not alter hunk boundaries
  **preserves** the undo stack and does not prompt.
- An option or edit that does alter hunks still prompts, and discards on confirm.
- `swap_in` no longer returns `InternalInvariant` for any user-reachable action.
- Hunk ids are unique within a session, asserted in debug builds.
- RFC-015 §8 rule 4 is amended and its **Not met** marker resolved — either as
  met under the amended wording, or removed as superseded by this RFC.

## 7. Falsification

- Reintroducing `diff_id` into the hash must fail the identical-recompute test.
- Removing the uniqueness assertion must fail a test that constructs a colliding
  session.
- A structural change must still discard, proven by a test that would fail if
  history were silently retained and later misapplied.

## Open questions

None for the owner. §5's amendment is a design decision and is taken here; the
owner authorizes scheduling, and may of course overrule the amendment — but
**rule 4 should not remain recorded as "Not met" against a promise nobody
intends to keep.**
