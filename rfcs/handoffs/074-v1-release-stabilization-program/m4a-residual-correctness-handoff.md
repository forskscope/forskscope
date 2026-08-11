# RFC-074 M4-A Developer Handoff: Residual Correctness

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M4-A — residual correctness, ahead of M4-B (gate integrity) and M4-C (truth reconciliation)
**Register items.** F40, F8, F35, F10
**Baseline.** `main` at `0.166.1`

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend RFC-074 first, then
update this handoff to match.

## 1. Summary

M4 opens with 20 open register items. This slice takes the four that are
**defects in shipped behaviour**, separated from the gate and documentation work
so they are reviewed on their own merits rather than buried in a large batch.

F40 leads because it is the only open item that costs a user their work.

This slice closes no audit blocker. B4 remains open; the v1/public release
decision remains **No-Go**.

## 2. Scope

In scope: F40, F8, F35, F10, and their tests.

Not in scope:

- **F6, F18, F24, F34, F36, F42** — M4-B. Notably, F34 (nothing looks at the
  rendered application) is the gate that would have caught F35's shape. Do not
  start building a rendering check here; it belongs with the other gate work.
- **F7, F9, F11, F12, F16, F25/F25b, F31, F37, F39, F43** — M4-C.
- **F44** — fixed upstream (`DioxusLabs/dioxus#5749`, merged), waiting on a
  `dioxus-desktop` release. Nothing to do until then.
- **F45, F46** — M5, and they need real Windows/macOS hosts.
- **F33** — architect-owned, running in parallel.

## 3. F40 — diff-option toggles silently destroy merged work

**Severity: data loss. Take this first.**

### The defect

`recompute_diff` (`crates/forskscope-ui/src/state/tab.rs:81`) rebuilds the merge
session from the two documents:

```rust
tab.merge = MergeSession::from_diff(&diff);
```

Applied hunks live in the `MergeSession`, not in `right_doc`, so this discards
every applied merge **and** the entire undo/redo stack. Four call sites reach it:

| Call site | Guarded? |
|---|---|
| `swap_sides` (`tab.rs:101`) | Yes — `ConfirmSwap` when dirty |
| Toolbar **Ignore WS** (`toolbar.rs:134`) | **No** |
| Toolbar **Ignore case** (`toolbar.rs:146`) | **No** |
| Toolbar **algorithm** select (`toolbar.rs:163`) | **No** |

The second failure is worse than the first: the fresh session reports
`is_dirty() == false` (empty stack, baseline 0), so the unsaved-work warning is
destroyed along with the work. Ctrl+W then closes the tab without prompting.

It also contradicts RFC-015 §8 rule 4 verbatim — *"Recomputing diff after an
edit must not erase undo history"* — in an RFC marked Implemented.

### Required property

> Applied merges and undo history survive a diff-option change, or the user is
> asked first and the answer is recorded.

### Two acceptable designs — your call, justify it

**(a) Preserve across recompute.** What RFC-015 actually asks for. Re-derive the
diff, then reapply the transaction log against the new hunk set. This is the
right long-term answer and the harder one: hunk identity changes when the diff
options change, so reapplication needs a rule for transactions whose hunk no
longer exists. RFC-015 §13 already requires that undo apply stored inverse
patches rather than locating hunks by index, which is the same problem.

**(b) Confirm, like `swap_sides` already does.** Reuse the existing dirty-check
pattern. Small, closes the data loss, does not deliver RFC-015's requirement.

**If you choose (b), say so explicitly in the RFC-015 record** — leaving §8 rule
4 stated but unmet is the documentation-truth defect this project keeps finding.
Either amend RFC-015 to describe what is actually guaranteed, or register the
remainder as a follow-up with the gap named. Do not leave the rule asserting
something the code does not do.

Whichever you choose, `is_dirty()` must never silently become `false` while
unsaved work is being discarded.

### Tests

Core-level where possible. The toolbar handler is `Store`-dependent (F36), but
the property — a session that has applied transactions, subjected to a
diff-option change — is expressible in `forskscope-ui-logic` or against
`MergeSession` directly. If you can only reach it at runtime, say so and give
AT-SPI evidence rather than claiming coverage.

## 4. F8 — the captured save fingerprint is unused

`save_text` captures a fingerprint after writing (`save.rs:87`) and returns it in
`new_fingerprint` (`save.rs:45`). Establish what consumes it.

**This is an investigation before it is a fix.** Three outcomes, all acceptable:

1. A caller should be storing it and is not — that is a real defect; fix it and
   say what breaks without it.
2. Nothing needs it, because RFC-077's `SaveTargetSnapshot` now owns
   post-save identity — then it is dead weight; remove it and update the
   module doc.
3. It is genuinely used and the register entry is wrong — say so and close F40's
   sibling entry as a misreading.

Report which, with the call sites. Do not "fix" it by wiring up a consumer that
nothing asked for.

## 5. F35 — screen readers announce "Changed" on blank rows

In a multi-line Replace hunk, empty counterpart rows carry the `Changed:`
screen-reader label with no gutter number and no content, so assistive
technology announces "Changed" once per blank row — four times for one logical
change in the review fixture. Pre-existing; surfaced by F32's AT-SPI pass.

**This is a decision before it is a patch.** A blank counterpart row is
structurally part of the change, so it is not obviously wrong to label it — but
announcing it four times is. Options: label only the first row of a run, label
the run once at the hunk level, or leave blank rows unlabelled. Pick one, state
what a screen-reader user hears afterwards, and verify with AT-SPI rather than
by reading the DOM.

RFC-061 is the accessibility track; record the decision there.

## 6. F10 — VCS discovery tests assume the temp dir is outside a repository

`crates/forskscope-core/src/tests/vcs_tests.rs:15` builds fixtures under
`std::env::temp_dir()`. If that path happens to sit inside a Git or JJ
repository — a developer with `/tmp` on a versioned filesystem, some container
layouts, a `TMPDIR` pointed into a checkout — discovery walks up and finds the
enclosing repo, and the tests assert against the wrong thing.

The property: **VCS discovery tests must not depend on what encloses the OS temp
directory.** Either establish the condition explicitly (create a barrier the
walk cannot cross) or detect and skip with a loud reason, following the
precedent already in `save_target_tests.rs`, which checks whether a `0o000`
restriction actually took effect before trusting its assertion rather than
assuming non-root.

Do not simply move the fixtures somewhere else that happens to work here.

## 7. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed. `dioxus-desktop` stays
  where it is until F44's upstream fix is released.
- `cargo audit` and `cargo xtask audit-deps` must still pass with the reviewed
  `.cargo/audit.toml` exceptions intact.
- No product behaviour changes beyond the four items above. If you find a fifth
  defect, register it — do not fold it in.
- No real user paths, host names, or secrets in evidence.

## 8. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F40, F8, F35, F10);
3. changed files;
4. **F40's design choice (a or b) with its justification, and what you did about
   RFC-015 §8 rule 4** — this is the review's main focus;
5. **F8's outcome** — which of the three, with call sites;
6. **F35's decision and what a screen-reader user hears afterwards**, with
   AT-SPI evidence;
7. any difference from this handoff or from RFC-074;
8. executed gates with observed output;
9. runtime evidence for anything not covered by a test, with the limitation
   stated rather than implied;
10. unresolved issues and known limitations;
11. requested review focus.
