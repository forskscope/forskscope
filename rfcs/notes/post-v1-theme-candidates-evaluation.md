# Post-v1 theme candidates — architect evaluation

**Date:** 2026-08-04
**Status:** Evaluation only. Not a roadmap, not an RFC, not a commitment.
**Source:** Four extension ideas proposed by the project owner, 2026-08-04.

Post-v1 theme selection is a joint owner/architect decision taken after Gate E
(RFC-074 §"Planning renewal"). This note evaluates the candidates so that
discussion starts from analysis rather than from scratch. Nothing here is
scheduled, and none of it competes with M3–M6.

## Summary of findings

| Candidate | Verdict | Principal risk |
|---|---|---|
| Sort before compare | Split it — one half is cheap, one is not | Sorted view breaks the merge coordinate system |
| Robustness at scale | Real, partly built already | The name "memory safety" misdescribes it |
| Spreadsheet comparison | Blocked upstream, not by design | Non-goal boundary: "universal document comparator" |
| History | Coherent, but it is a **new persistent data category** | Privacy — it records what a user opened |

## 1. Structured data: sort before compare

### The stated rationale is not the real benefit

The proposal gives "performance stability" as the reason. Sorting is additional
`O(n log n)` work on every compare — it costs performance, it does not stabilise
it.

The genuine benefit is **diff quality**: when content is reordered but unchanged
— config keys, export lists, dependency manifests, translation catalogues — an
unsorted diff reports nearly every line as changed. Sorting collapses that to the
real delta. That is a strong reason on its own and worth stating as the actual
motivation, because it determines the design: this is a *comparison semantics*
feature, not a performance feature.

### This is two features, not one

**(a) Sort as a compare option — tractable.** It belongs beside the existing
`WhitespaceMode`, `NewlineCompareMode`, `CaseSensitivity`, `InlineMode`, and
`DiffAlgorithm` in `CompareProfile`. That is the established shape for "how do we
decide two lines are the same."

One consequence to plan for rather than discover: `PersistedDiffProfile` is part
of the schema-v2 on-disk contract, pinned by F26's per-variant wire-format tests.
Adding a `SortMode` is a schema change — either a defaulted field or a version
bump. Cheap if anticipated, disruptive if not.

**(b) Sort applied to the view while merging into the original — expensive, and
the proposal understates it.**

`MergeSession::apply_left_to_right(hunk_id)` operates on the diff's own line
indices, and `LineMap` assumes view rows correspond to file lines. If the view is
sorted and the file is not, every merge action needs a bidirectional mapping from
sorted-view position back to original-file position. That is a new coordinate
system threaded through the diff model, the merge model, the decoration engine,
and scroll sync.

**Recommendation:** treat sorted comparison as **read-only** in its first form —
merge actions disabled while a sort is active, exactly as binary comparison is
gated today. That gives the diff-quality benefit at a fraction of the cost, and
it is honest about what the view represents. Sorted-and-mergeable can be
revisited from evidence that users want it.

### "Apply to the original file" is a different operation entirely

This is not a merge result — it is a *rewrite of a file the user did not author
in that form*. It should not be a save mode. It is a deliberate transform
("Sort and save"), and if it happens at all it needs the full RFC-077 target
model: explicit target, precondition, backup, confirmation. Folding it into the
existing save path would put a destructive whole-file rewrite behind the same
button as a merge commit.

Lowest priority of the three parts, and the one most likely to be better served
by telling the user to use `sort`.

## 2. "Memory safety" — robustness at scale

### The name should change

Rust gives memory safety by construction, and this project's threat model uses
"safety" with precision. What is actually proposed is **robustness under scale**:
bounded resource use, responsiveness, and honest failure when input exceeds what
the tool can handle. Calling it memory safety would mislead a Rust audience and
blur a term the security documentation depends on.

### Compare view — mostly built, with a real gap

`LoadGuard`/`guard_for_sizes` already returns Proceed / WarnBanner /
ConfirmPrompt from `FileSizeClass` thresholds, and `PerformanceLimits` carries
`max_eager_lines`, `max_inline_diff_chars_per_hunk`, and
`max_directory_entries_eager`. RFC-012's diff deadline emits a `DiffWarning`
rather than an error.

The gap is that these gate on **input size**, not on **result size**. A modest
pair of files can produce a pathological diff. If this theme is pursued, the
worthwhile part is a guard on the computed result, not another input threshold.

### Explorer — the more substantial half

Directory summary status over very large trees is genuinely weak, and it is
already a known deferred item: RFC-037's persistent digest cache was deferred
post-v1, so digests are recomputed on each navigation.

Two cautions on "Rust's concurrency may help":

- The infrastructure already exists — `JobRegistry`, `CancellationToken`,
  `tokio::spawn_blocking`. Parallelism is available without new dependencies;
  adding `rayon` should be a deliberate decision, not a reflex.
- Parallelism reduces *latency*, not *memory*. A tree with a million entries
  consumes the same memory whether walked on one thread or eight. If the concern
  is failure rather than slowness, the answer is bounded/streaming traversal and
  a persistent cache (RFC-037), not threads.

The threat model already records an unbounded-growth item in this family: the
Explorer's `binary_cache` `HashMap<PathBuf, bool>` has no eviction beyond the
directory-change clear. A scale theme should absorb it.

## 3. Spreadsheet comparison

### Not blocked by design — blocked by a dependency advisory

The adapter boundary is built and RFC-058 shipped it. It was suspended because
`sheets-diff → calamine → quick-xml 0.39` carries unresolved denial-of-service
advisories on XML input, and `sheets-diff` cannot yet move to a fixed path. The
runtime parser was removed and `.xlsx` comparison fails closed.

So this is not a feature to design. It is a **dependency remediation** with four
outcomes:

1. `calamine` moves to `quick-xml ≥ 0.41` and `sheets-diff` follows — wait;
2. contribute the upgrade upstream — the F17 precedent shows this project can get
   an upstream fix landed same-day;
3. replace the parser behind the existing adapter boundary — RFC-058 deliberately
   kept `sheets-diff` types out of the app model, so this is possible;
4. keep failing closed.

Option 2 is the highest-value move and the proposal's "ask the sheets-diff team"
is the right instinct. Worth noting that RFC-058's status still needs its
security-suspension annotation (F11, M4) regardless of which path is taken.

### One boundary to watch

"A universal document comparator" is an explicit non-goal. XLSX is already
accepted scope, so restoring it is consistent. Extending to further binary
document formats would cross the line, and that boundary should be restated if
this theme is opened.

## 4. History

### Coherent, and the smallest item is separable

"Open parent folder" from a comparison tab into the Explorer is a navigation
affordance, not history. It needs no persistence, it is small, and it could ship
independently of everything else here.

### The model was just deleted

`RecentSessionEntry` and `RecentKind` existed in `forskscope-core::session` and
were removed with that module in RFC-076 patch 5, because nothing consumed them.
Rebuilding history means **new persisted state**, days after persistence was
converged onto schema v2. `PersistedSession` currently holds `tabs`,
`active_tab`, and `explorer_roots`.

Design question to settle early: is history a third document, or an extension of
the session payload? A separate document is cleaner for a feature the user may
want to clear or disable independently, and it avoids version-bumping the session
schema for a non-essential feature.

### Privacy is the substantive issue, and the proposal already senses it

This product's positioning is that users compare private source code,
credentials, and production logs, and that it is trustworthy by default rather
than by opt-out. A history of opened paths is not secret material, but it *is* a
durable behavioural record — a new category the threat model does not currently
cover. S-007 says no persistent sensitive data; a path list sits right at that
edge.

The proposal includes "private mode not to store at current session", which is
the right instinct. Two things follow:

- **Default matters more than the feature.** Given the trust positioning, the
  defensible defaults are history off by default, or on with a visible and
  genuinely complete clear. "On by default, private mode available" inverts the
  product's own stance.
- **"Clear history" must actually erase.** If entries are removed from a JSON
  file by rewriting it, the previous content may survive in the `.bak` or
  `.pre-v2.bak` sidecars the save path creates. A clear that leaves a backup
  containing the cleared data is not a clear. This interacts directly with the
  backup behaviour RFC-076 and RFC-077 just established, and it is the kind of
  detail that is cheap to design in and expensive to retrofit.

Any history work needs a threat-model data-flow section of its own.

## Sequencing observation

If more than one of these is eventually pursued, **sort-before-compare and
history both touch the persisted schema**, and doing them in one schema revision
is much cheaper than two. Robustness-at-scale and spreadsheet remediation touch
neither and can proceed independently.

That is a planning input for the Gate E discussion, not a recommendation to
start any of it now.
