# RFC-077 patch 5 — presentation, Save As confirmation, docs review

**Review date:** 2026-08-07
**Request:** `dev-record/review-requests/047-rfc077-patch5-presentation-saveas-docs.md`
**Baseline:** `c789636`
**Governing documents:** RFC-077; `rfcs/handoffs/077-mergetool-save-target-model/implementation-handoff.md`; review 049
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved,** with one non-blocking finding (§4) and a decision granted on the
`done/` move (§3.1).

The Save As default-path bug you found is the most important thing in this patch,
and I should have caught it in review 048 — see §2.

M3 does **not** close on this patch; F38 remains registered against it. B4 remains
open; v1/public release stays **No-Go**.

## 2. The default-path bug — and my miss

The toolbar defaulted Save As to `tab.right_path`. That was correct until patch
4a redefined `right_path` to always mean the compared input, at which point Save
As on a mergetool tab silently pointed at `<remote>` instead of the merge target.
`git log -S` confirms the line predates RFC-077, so patch 4a is what made it
wrong.

**Review 048 should have found this.** I traced `build_request`, `handle_result`,
and `save_as`, and concluded the wrong-target class was closed. I did not sweep
for *other* readers of `right_path` whose meaning patch 4a had changed underneath
them — which is exactly the sweep a redefinition warrants. The two corrections I
did raise were both in the same file I was already reading.

Worth generalising, because RFC-077's whole thesis invites it: when a field's
*meaning* changes rather than its type, the compiler cannot help, so every reader
has to be re-read. There are now three wrong-target defects in this workstream —
the original B3, review 048's C1, and this one — and this is the only one found
by someone actually using the feature rather than reading the diff.

## 3. Answers to the requested review focus

### 3.1 Move RFC-077 to `rfcs/done/` — yes, now

Two things settle it.

**RFC-000 §"Granularity of transitions"** is explicit: partial implementation
qualifies if the partial work captures the RFC's main design decision, and
*"don't keep an RFC in `proposed/` indefinitely just because one open question
remains. Move it to `done/` when the design has shipped, and record what didn't
make it."*

**Precedent in this project is unambiguous.** RFC-075 moved to `done/` with
platform verification outstanding. RFC-076 moved to `done/` at patch 6 with the
same caveat. RFC-077's "accepted under RFC-078" line says nothing different from
what was true for both of those — RFC-078 is where *every* workstream's
cross-platform behaviour is accepted. Holding RFC-077 alone in `proposed/` would
make the folder mean something different for it than for its siblings.

Record in an `## Implementation outcome` section, following RFC-075's shape:

- the six patch commits;
- each acceptance criterion and its status, including the "partially" on target
  transitions **in your own words** — the core-level coverage through the shared
  `check_precondition` path, and the fact that each transition is not
  independently re-verified through a live process with a concurrent external
  actor;
- Windows replacement semantics for `persist_noclobber` as explicitly deferred
  to RFC-078.

Status line: `Implemented (Milestone M3)`, version left for the cut, as RFC-076 did.

**Do not read the folder move as M3 closing.** They are different things: the
folder records that the design shipped; M3's gate additionally requires F38
(permission/umask), which is registered against it. Asking rather than assuming
was right — the two had genuinely diverged in this case.

### 3.2 `Path::exists()` for the Save As pre-check — change it

**Your safety reasoning is correct and the conclusion still does not follow.**
The real boundary is downstream, a race still surfaces as `ConfirmOverwrite`, and
nothing unsafe can happen. That is all true.

The problem is not safety, it is that `Path::exists()` cannot distinguish
*overwritable* from *never writable*, so the dialog asks the wrong question:

```text
user types a directory path
  exists() → true
  → "Overwrite existing file?" / "A file already exists at this path."
       ← wrong on both counts: not a file, and it cannot be overwritten
  user confirms
  → save_as → inspect_save_target → Blocked
  → "Cannot save here: not a regular file."
```

The user is asked to confirm something impossible, then refused. Using
`inspect_save_target` in the pre-check resolves all three cases in one place —
`Blocked` reports immediately with the reason you already compute, `Writable` +
exists shows the confirmation, `Writable` + absent proceeds. One extra
classification on a user-initiated action is cheap.

Non-blocking: no data is at risk and the safety boundary holds. But it is a
two-step dead end with misleading copy in step one, and the fix is smaller than
the explanation.

### 3.3 The two corrected doc inaccuracies

**Correct to fix, and correctly separated.** Both are latent inaccuracies the
RFC-077 pass surfaced rather than caused, and calling them out under their own
heading is what makes that visible.

The `cli.md` exit-code one is the more valuable catch: it documented non-zero
exit for "path not found" when `LoadOptions::allow_missing` means a missing side
has always loaded as empty content. That is a documented behaviour that never
existed — the same class as `persist.rs` claiming every file used a
`VersionedEnvelope`, and worth the same treatment.

## 4. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo test --workspace` | Pass — 1094, unchanged |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo xtask i18n` | Pass — 223 keys (+3) |
| `cargo xtask css --check` | Pass — `main.css` current |
| CI run `31226885116` | `success` on `c789636` |
| Result line is plain text, no control | Confirmed — `div` + `span`, `title` tooltip only |
| `ConfirmSaveAsOverwriteModal` → `save_as` | Confirmed — never constructs `Force` |
| Save As default now `save_target`, falling back to `right_path` | Confirmed |

The Result line reading `launch_mode` rather than `save_target` is right for the
reason given: `save_target` is `None` while `Loading`, and the merged path is
known synchronously. Deriving presentation from the earliest source that has the
answer avoids a needless dependency on load completion.

### On "no new tests"

Accepted for the two new UI paths — `Path::exists()` and a `match` on
`CompareLaunchMode` inside a component genuinely have no separable logic.

Worth noting for F36's case rather than as a criticism here: the default-path
fix is a wrong-target correction whose only evidence is a runtime observation,
because the expression lives inside a Dioxus component. That is the third
`Store`-dependent seam in this workstream where correctness rests on someone
having run the binary. F36 asks whether a harness is worth introducing; this is
another data point for that decision.

## 5. Notable quality observations

- Finding the default-path bug *while wiring something else*, recognising it as a
  patch-4a regression rather than a new mistake, and tracing why it used to be
  correct — that is the reading that catches meaning-changes.
- Verifying the corrected default via `Atspi.Text.get_text` because the field
  visually truncates is the right instinct: the screenshot could not have proven it.
- Asking about the `done/` move rather than assuming, with the specific reason
  for doubt cited from the RFC's own Dependencies section.
- Marking the target-transition row "partially" and explaining precisely what is
  and is not proven, rather than claiming the criterion met because the shared
  code path is tested.

## 6. Recommended next action

1. Move RFC-077 to `rfcs/done/` with the `## Implementation outcome` section
   described in §3.1, and update `rfcs/README.md`'s counts in the same commit.
2. Apply §3.2 — `inspect_save_target` for the Save As pre-check.
3. **F38** (permission/umask) — the last item before M3 can close.
4. F23 still gates M2's cut. F39 at M4.
