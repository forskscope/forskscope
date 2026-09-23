# M4-C1 review — advisory dispositions, durability wording, P08, matrix plan

**Review date:** 2026-08-11
**Request:** `dev-record/review-requests/053-m4c1-advisory-dispositions-durability-p08-matrix-plan.md`
**Baseline:** `fb82887`
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c1-dispositions-and-matrix-freeze-handoff.md`, RFC-078
**Review mode:** Independent verification, including re-deriving both reachability statements from source. No implementation changes made.

## 1. Verdict

**Approved with one minor correction (N1, §4).**

Both unsoundness reachability statements hold — I re-derived each independently
and one is stronger than you claimed. The durability sweep, P08 amendment and
matrix plan all land.

B4 remains open; v1/public release stays **No-Go**. `matrix-plan.md` is
committed but correctly **not frozen** — the owner-dependent fields are open,
which is the right state to hand back.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check`, `clippy --workspace --all-targets -D warnings` | Pass |
| `cargo test --workspace` | Pass — 1112, unchanged (doc comments only) |
| `cargo xtask version-sync` | Pass — `v0.166.1` |
| `mdbook build docs` | Pass |
| CI run `31472515564` | `success` on `fb82887` |
| `RUSTSEC-2026-0097` — build-dependency-only | **Confirmed** — see §3.1 |
| `RUSTSEC-2024-0429` — not reachable | **Confirmed, and stronger** — see §3.2 |
| `AtomicSaveStrategy` absent from `forskscope-core` | Confirmed — the correction was right |

## 3. The two reachability statements — both hold

This was the review's main focus, so I re-derived both rather than checking your
working.

### 3.1 `RUSTSEC-2026-0097` — `rand` 0.7.3

```text
rand v0.7.3
└── phf_generator v0.8.0
    └── phf_codegen v0.8.0
        [build-dependencies]
        └── selectors v0.24.0 → kuchikiki → wry → dioxus-desktop → forskscope-ui
```

Reproduced exactly. The `[build-dependencies]` edge is the whole argument and it
is correct: `rand 0.7.3` is build tooling for a codegen crate and never links
into the shipped binary.

Note for anyone re-checking this later: `cargo tree -i rand@0.7.3 -p
forskscope-ui` alone prints *"nothing to print"* — `--target all` is required to
see it. That is worth recording in `advisories.md`, because a future reviewer
running the obvious command will conclude the advisory no longer applies rather
than that they used the wrong flags.

Giving the second, independent ground — that the unsound path needs a custom
`log::Log` global logger calling `RngCore` on `ThreadRng` during a reseed — was
the right instinct. Reading `RUSTSEC-2026-0097.md` instead of trusting the
one-line title is exactly the standard this program has been pushing for.

### 3.2 `RUSTSEC-2024-0429` — `glib` 0.18.5

Your conclusion is right and **the argument is stronger than you made it.** You
said `VariantStrIter` has "exactly one public constructor." In fact it has
**none**:

```rust
impl<'a> VariantStrIter<'a> {
    pub(crate) fn new(variant: &'a Variant) -> Self { … }
```

`new` is `pub(crate)`, so the type cannot be constructed from outside `glib` at
all. The only route in is `Variant::array_iter_str`, and the registry-wide grep
confirms nothing outside glib's own two files calls it. This matters more than
the `rand` case, because unlike `rand`, `glib` genuinely links into the shipped
binary — so it is the one where reachability had to be established rather than
sidestepped.

Worth putting the `pub(crate)` fact into `advisories.md`: "no external
construction path exists" survives a dependency bump better than "nothing
currently calls the one constructor," and it is cheaper to re-verify.

## 4. Minor correction — N1: the `AtomicSaveStrategy` sweep is incomplete

You found `architecture.md` naming a type that does not exist, fixed it, and
flagged it as its own line item. Correct on all three counts — and the sweep
stopped one file short:

```text
docs/src/maintainers/testing.md:93
| `save_tests` | `save_text` with fingerprint match, `AtomicSaveStrategy`, `BackupPolicy`. | RFC-007 |
```

`testing.md` is current documentation — it is in `SUMMARY.md` as "Testing
strategy," not an archived note. `AtomicSaveStrategy` is absent from
`forskscope-core`; `BackupPolicy` does exist, so only the one token is wrong.

**Leave `rfcs/notes/core-completion-summary-v0.72.md` alone.** It carries the
same token, but it is a dated historical note, and RFC-074's advisory N3 is
explicit that the historical archive is not revised. That distinction is worth
stating in the fix so the next sweep does not "correct" it.

One line. Fold it into whatever lands next; it does not need its own round.

## 5. Answers to the requested review focus

### 5.1 Methodology — meets Gate C's bar

Gate C asks for a reachability statement, owner, review date, and upgrade
trigger. What you produced goes past the letter of it in the way that matters:
both statements are *falsifiable by a third party in one command*, which is what
makes a disposition worth more than an assertion. I re-ran both in a few
minutes.

The one improvement is §3.1's flag note and §3.2's `pub(crate)` framing — both
make re-verification cheaper for whoever revisits at the upgrade trigger.

### 5.2 One commit — right for this slice, and the reason generalises

Yes. M4-A and M4-B were multi-commit because their items were independent code
changes with independent failure modes. This slice is one act of recording what
is true, and the `ROADMAP.md` edits genuinely do not split cleanly.

The rule worth carrying: **split by what a reviewer needs to be able to revert
independently.** Nobody would revert F9's wording while keeping the advisory
dispositions. Splitting there would have produced fragile partial-hunk staging
for no reviewer benefit — and this project has already been burned once by
creative staging.

### 5.3 P03/P06/P10/P12 spot-checks — accepted, with one refinement to P06

The general shape is sound: **every engine family gets each case in full at
least once**, and the narrowed rows are the second member of a family. I checked
that holds — P06 is Required on `linux-wayland` (WebKitGTK), `windows-11`
(WebView2) and `macos-aarch64` (WKWebView), with `linux-x11` and `windows-10`
narrowed. Each family is covered in full. That is a real rationale, not a
convenience.

**The refinement:** for P06 specifically, the engine family may not be the right
axis. Async identity is about *timing* — a background compare completing against
live tab state — and the completion path runs through the windowing event loop,
which is `tao`'s Wayland versus X11 backend, not the web engine. Wayland and X11
share WebKitGTK but not their event-loop integration, so `linux-x11` is not
obviously covered by `linux-wayland` for this case the way it is for P03.

Not a blocker, and not something to change unilaterally now: it is a judgement
about where a race is likely to differ, and yours is defensible. **Record the
axis question in the plan** so M5 does not have to reconstruct the reasoning,
and adopt one rule regardless: **if any P06 defect appears on any row, every
P06 spot-check on that platform is upgraded to Required before the matrix
closes.** That converts a cheap assumption into one that fails loudly.

P03, P10 and P12 I accept as written. P10 in particular is a message rendering,
and RFC-078's own text narrows P03.

### 5.4 The owner questions — the right set, and Q4 is the valuable one

All five are genuinely owner-owned. Two observations:

**Q4 (macOS one row or two) is the most valuable thing in §7**, because it is
not a preference — it is an inconsistency *inside RFC-078*. The required-matrix
table lists macOS twice with different levels while the evidence layout names a
single `macos-aarch64.md`. That is a spec defect you surfaced by trying to
execute the spec, which is the only way these get found. Whatever the owner
decides, RFC-078 needs amending so it stops disagreeing with itself.

**Q1 is load-bearing for F44.** Whether the claimed Linux baseline includes
libxdo-4 distributions decides whether F44 is a release blocker or a documented
limitation. Right to ask rather than assume.

Nothing in the five should have been decided by you. Asking was correct.

## 6. Notable quality observations

- Reading the advisory text rather than the title, and giving two independent
  grounds for `rand` rather than resting on the build-dependency edge alone.
- Searching a deliberate superset of the dependency chain, including versions
  newer than those locked, so the negative result does not depend on the lock
  file staying still.
- Catching `AtomicSaveStrategy` while sweeping for something else, and treating
  it as its own line item rather than folding it silently into an unrelated fix.
- Keeping `merging.md`'s existing sentence — which was already visibility-scoped
  and therefore not wrong — and adding the explicit statement beneath it, rather
  than rewriting text that was correct.
- Marking the matrix plan committed-but-not-frozen instead of freezing it around
  placeholder values.

## 7. Recommended next action

1. **N1** (§4) — one line in `testing.md`, plus §3's two `advisories.md` notes
   and §5.3's P06 rule. All small; fold into M4-C2's first commit.
2. **Owner** — the five questions in `matrix-plan.md` §4, and the RFC-078
   self-inconsistency behind Q4.
3. **M4-C2 — documentation and code truth**: F11, F12, F16, F25/F25b, F31, F39,
   F48, with F43 held for the owner. Handoff to follow.
4. After M4-C2, Gate C is assessable and M4 closes.
