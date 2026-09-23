# M4-B gate integrity review — F42, F24, F6, F18, F36, F34

**Review date:** 2026-08-11
**Request:** `dev-record/review-requests/052-m4b-gate-integrity-f42-f24-f6-f18-f36-f34.md`
**Baseline:** `8d82f11`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4b-gate-integrity-handoff.md`
**Review mode:** Independent verification, including re-running the F32 mutation myself. No implementation changes made.

## 1. Verdict

**Approved.** All six items land. This is the strongest slice of the program so
far — the falsifiability standard was met item by item, and F34 and F36 both
delivered more than the handoff asked for.

One finding against the new work (N1, §4) and one pre-existing defect the review
surfaced (F48, §5). Neither blocks.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check` | Pass |
| `cargo fmt --manifest-path xtask/Cargo.toml --check` | Pass — F18's gate holds |
| `cargo test --workspace` | Pass — **1112**, matching your count |
| `cargo test --manifest-path xtask/Cargo.toml` | Pass — **6**, xtask's first tests |
| `cargo clippy --workspace -- -D warnings` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass — F6's new gate |
| `cargo xtask version-sync` (dev) | Pass — still accepts the open empty section |
| `cargo xtask version-sync 0.166.1` (release) | **exit 1** — `CHANGELOG section for 0.166.1 has no content` |
| CI run `31465286879` | `success` on `8d82f11` |
| F42 guards present and correctly worded | Confirmed — `ci.yml:43`, `ci.yml:143` |
| F34 wired into `release.yml`'s linux job before packaging | Confirmed — `release.yml:142` |

F24 is the one I most wanted to see both ways, because getting it backwards
would turn every commit red on the current tree. Dev mode accepts, release mode
rejects with exit 1. Correct.

### F34 — I re-ran your mutation

Not taking this one on report. I reintroduced F32's exact defect myself — moved
the `sr-only` span out of `.cell` at both row sites in `hunk.rs` — rebuilt, and
ran the check against the real binary:

```text
FAIL: F34 rendering check found misalignment:
  - left pane: a row has 3 accessible children, other rows have 2 - a label is
    likely rendering as a sibling of the content cell instead of inside it
    (F32's defect shape)
  … ×4 (both panes, both affected row kinds)
exit=1
```

Restored, re-ran, `OK: 7 left rows + 7 right rows all aligned within their pane`,
exit 0. Working tree confirmed clean afterwards.

**So the check genuinely catches the defect that motivated it.** That is the
first automated check in this project that would have caught a shipped visual
regression, and it closes the gap that let F32 through two releases.

## 3. Answers to the requested review focus

### 3.1 F34's Xvfb/CI-wiring gap — land it now

**Acceptable, and holding it would be the worse call.** Three reasons:

- The part that carries the risk — *does this detect the defect* — is proven,
  twice, independently. What is unproven is whether a package list and
  `xvfb-run`/`dbus-run-session` behave the same on a fresh runner, which is
  configuration, not logic.
- Holding it means it cannot run until a release, and it cannot be proven
  without running at a release. That is circular; something has to go first.
- The project already normalised this at R0: the first real release-workflow run
  is where configuration meets reality, and that is exactly how F17 was found.
  A red F34 step on the next cut is a *successful* outcome for this milestone —
  it fails before packaging, and no tag is published from a failed job.

I could not close this one for you: no `xvfb-run` or `Xvfb` on this host either.
Stated rather than glossed.

One operational note for the next cut: if the step fails for environment
reasons, resist the temptation to make it non-blocking. A gate that can be
skipped when inconvenient is the failure mode this entire slice exists to
remove.

### 3.2 Child-count alongside geometry — keep both, but see N1

Keep both. They are not redundant: child-count catches a *structural* change,
geometry catches a *visual* one, and a future defect could produce either alone.
Asserting the cheaper, more explainable signal first is good design, and its
diagnostic re-derived F32's root cause from the symptom, which is genuinely
impressive.

But see **N1 below** — only one of the two has ever been demonstrated to fire.

### 3.3 F36's scope boundary — not retrofitting was right

Correct call, for a reason worth stating: retrofitting four working seams to a
new harness is churn against code that is not changing, and every conversion is
a chance to weaken a test that currently passes for real reasons. "Use it when
you next touch that code" is the right policy, and it is recorded where the next
patch will find it.

What lifts this above a decision item is that you proved the mechanism by
closing a real gap with it — the two new `change_diff_options` tests assert on
actual `Store` state through the real production function. That converts F36
from an unproven capability claim into a demonstrated one, and it retires an
excuse this project has leaned on five times.

Recording the pure-predicate pattern as the *default* and `with_test_store` as
the fallback is the right ordering. F35 and F40 both show the default works.

### 3.4 F6's fixes — all genuinely mechanical, none suppression-shaped

Reviewed all twelve. `manual_contains` → `.contains()`, `cmp_owned` → borrow
instead of allocating, `type_complexity` → named type aliases: all
straightforward improvements to the test code, no behaviour change.

`assertions_on_constants` → `const { assert!(..) }` is the only one worth a
second look, because it moves the check from run time to compile time. That is
strictly stronger — the assertion now cannot fail at run time because it cannot
compile if false — and keeping them as named `#[test]`s preserves discoverability.
Not a suppression.

No `#[allow]` was added anywhere in this slice. That is the right outcome, and
it means F6's stronger gate starts clean rather than starting with debt.

## 4. Finding N1 — half of F34's check has never been demonstrated to fire

Non-blocking, but it is this slice's own theme applied one level down.

`check_pane` has two assertions, and a `continue` after the first:

```python
if n != baseline_count:
    failures.append(…)   # child-count
    continue             # ← geometry never evaluated for this row
cell = row.get_child_at_index(n - 1)
x = extents(cell).x      # geometry
```

In the F32 mutation — yours and mine — **only the child-count branch fired.**
The geometry comparison, which the module docstring presents as the design's
whole point ("deliberately a *geometry* check, not a DOM-structure check"), has
never been observed failing.

I tried twice to exercise it with CSS mutations that shift x without changing
DOM shape, and **both attempts failed to produce a shift** — so I cannot tell
you the branch is broken, only that it is unproven. Reporting the attempts
because they are informative:

- `.diff-row.fs-line-deleted .cell { padding-left: 24px }` — no effect, because
  those classes are never emitted (that is F48, §5).
- `.diff-row.match .cell { padding-left: 24px }` — reached `main.css`, rebuilt,
  and still produced no visible shift. Screenshot confirms rows unmoved.

That second result is itself worth knowing: producing a pure geometry shift in
this layout is harder than it looks, which raises a fair question about how much
the geometry branch adds over child-count in practice.

**Asked for, not required:** either demonstrate the geometry branch failing, or
state in the docstring that it is defence-in-depth which no known defect
triggers. Both are fine. What is not fine by this slice's own standard is a new
gate with an undemonstrated half — that is precisely "credited with more than it
measures," and this slice is where the project stopped accepting it.

## 5. New finding — F48: a core→CSS class contract the DOM never receives

Surfaced while probing N1, pre-existing and unrelated to your work.

`crates/forskscope-ui/assets/css/30-contract-diff-decorations.css` styles
`.diff-row.fs-line-added` / `-deleted` / `-modified` / `-empty-counterpart` /
`-conflict` / `-merge-applied`, and its header asserts:

> These classes are produced by `LineDecorationKind::css_class()` … **Current
> DOM (v0.162.0+):** `.diff-row[.fs-line-*]`

That DOM does not exist. `hunk.rs` emits `"diff-row"` or `"diff-row match"` and
nothing else; `grep -rn "fs-line-" crates/forskscope-ui/src/` returns nothing.
The chain stops one layer short: core's `css_class()` feeds
`forskscope-ui-logic::compare::hunk_decorations`, and **no UI code consumes
`hunk_decorations` at all.**

So there is a stylesheet, a view-model, and an RFC-024 contract, none of which
reach a rendered element — and `cargo xtask css --check` passes throughout,
because it verifies `main.css` is a current concatenation, not that its
selectors are reachable. Same shape as `css_coverage` being blind to layout,
which is what let F32 ship.

Registered as **F48** at M4-C, where the other truth-reconciliation work sits.
It needs a decision, not a patch: wire the decorations through, or delete the
stylesheet and view-model and correct RFC-024's status. **Not yours to fix in
this slice** — flagging so it is not discovered a third time.

## 6. Notable quality observations

- Meeting the falsifiability standard on all six items without being chased for
  it, including using the tree's own open empty CHANGELOG section as F24's
  failing case rather than building a synthetic fixture.
- Building F36's harness and then *using it* to close a real gap, rather than
  landing a capability and declaring the decision made.
- Choosing the full geometry check over the launch-smoke-test the handoff
  permitted, with the correct reason: a bare launch check would not have caught
  F32, because the window opened and rendered "successfully" — it just looked
  wrong.
- Pinning the F34 fixture's hunk-kind sequence with a corpus test, so the check
  cannot quietly stop covering Insert or pure Delete.
- Naming the CI-wiring gap precisely — what is proven, what is not, and which
  class of risk it belongs to — instead of letting a green CI run imply more.

## 7. Recommended next action

1. **N1** (§4) — demonstrate the geometry branch or document it as
   defence-in-depth. One line either way; no rush, and it can ride with M4-C.
2. **M4-C — truth reconciliation, advisory dispositions, `matrix-plan.md`
   freeze**: F7, F9, F11, F12, F16, F25/F25b, F31, F37, F39, F43, **F48**, and
   RFC-074's advisories N1–N6. Handoff to follow.
3. After M4-C, Gate C is assessable and M4 closes.
