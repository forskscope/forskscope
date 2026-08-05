# Upstream request to `sheets-diff` — draft, 2026-08-04

**Status:** Draft for the project owner to send. Not sent.
**Supersedes for sending purposes:** `sheets-diff-v2-questions.md` (pre-v2-migration)
and RFC-058 §"Questions and Feature Requests" — see the selection note below.

## Selection note (for us, not for sending)

The two existing lists total nine questions and requests. Sending all of them
now would be counterproductive: most concern the shape of a code path ForskScope
currently has **switched off**, and a maintainer who receives nine items is less
likely to answer the one that matters.

Included below, because each answer changes what we do:

- the `quick-xml` chain — the actual blocker;
- Q5, non-UTF-8 paths — a precondition for re-enabling safely;
- Q4 and FR3, stability of `change_kind()` and `code()` — contract questions our
  adapter depends on, cheap to answer;
- FR2, cancellation granularity — bears on our large-workbook handling.

Parked until the path is re-enabled and we know what we actually need: Q1 (row
cardinality), Q2 (formula text in `CellChangeRow`), Q3 (owned row type), FR1
(cancellation doc example), FR4 (documented drop-the-bulk note). Q2 remains the
one we expect to want for a future aligned-cell view; it is parked, not dropped.

Facts below are from our own records — the dependency tree and advisory IDs were
captured in an internal review on 2026-07-09, so upstream state may have moved
since. The message asks rather than asserts for that reason.

---

## Draft message

**Subject:** ForskScope: XLSX support currently disabled over the `quick-xml`
chain — is a `calamine` bump feasible, and can we help?

Hello,

I maintain ForskScope, a local-first cross-platform diff/merge desktop tool. We
integrated `sheets-diff` v2 through a single adapter module and the v2 rewrite
was a genuine improvement for us — typed values, `Result`-based errors,
cancellation, and the framework-neutral `output::view` all made our integration
cleaner than v1 did.

I'm writing about a dependency issue that has forced us to disable spreadsheet
comparison, and about four small contract questions.

### 1. The blocker: `quick-xml` advisories reachable from workbook parsing

As of our audit on 2026-07-09 we observed this chain:

```
quick-xml v0.39.4
└── calamine v0.35.0
    └── sheets-diff v2.2.3
        └── forskscope-core
```

with two advisories against `quick-xml 0.39.4`, both fixed in `>= 0.41.0`:

- **RUSTSEC-2026-0194** — duplicate-attribute quadratic runtime
- **RUSTSEC-2026-0195** — namespace allocation memory exhaustion

Both are denial-of-service on XML input. Our users open files they did not
author — that is the product's whole purpose — so for us this is a reachable
path from untrusted input, not a theoretical one.

We took the conservative route: we removed the runtime XLSX parser dependency
entirely and `.xlsx` comparison now fails closed with a user-visible message. So
we are currently *not* a consumer, which I want to be upfront about. We would
like to come back.

Three questions, in order of usefulness to us:

1. **Is a `calamine` bump planned or blocked?** We could not tell from the
   outside whether the constraint is upstream in `calamine` or a compatibility
   issue on your side. If upstream state has moved since our July audit, please
   just correct me.
2. **Would a PR be welcome?** If the bump is mechanical and only needs someone
   to do it and verify, we are willing.
3. **Is there an interim boundary we could rely on?** For example, a feature
   flag, or documentation of which APIs never touch untrusted workbook XML. That
   might let a consumer re-enable a reduced surface before the whole chain moves.

**What we can offer.** We have a reproducible consumer, a fail-closed posture
today, and a CI gate that asserts which dependency paths are permitted, so we
can verify a candidate quickly and report back precisely. We did exactly this
recently with another crate — a Windows build failure reported upstream, fixed
the same day, re-verified against our gates before we shipped. Happy to be that
feedback loop again.

### 2. Four small contract questions

These are independent of the above and each is a yes/no or one sentence.

**a. Non-UTF-8 paths.** `compare_paths` takes `impl AsRef<Path>`. We run on
Linux, Windows, and macOS and must handle paths that are not valid UTF-8. Can we
rely on arbitrary `Path` values flowing through to `calamine` unchanged — that
is, no internal `path.to_str().unwrap()` that would panic or reject them? Our
no-panic contract depends on this.

**b. `CellDiff::change_kind()` stability.** Is its derivation (Added = all
sub-changes have empty `old`; Removed = all have empty `new`; else Modified)
stable API we can depend on, rather than re-deriving it ourselves?

**c. `DiagnosticKind::code()` strings.** The docs say these are never renamed
within a major version. We would like to match on them (e.g.
`"unsupported_workbook_feature"`) to drive our own messaging rather than matching
`#[non_exhaustive]` enum variants. Can you confirm `code()` is the intended
stable programmatic surface? A single doc table listing the full set would help.

**d. Cancellation granularity.** We surface a Cancel button for long
comparisons. How often is `is_cancelled()` polled — per sheet, per row, per N
cells? One sentence in the docs would let us set honest user expectations on a
very large single sheet.

### 3. Not asking for

To be clear about scope, since it may save you reading time: we are not asking
for merge or write capability (we own the merge model — you are read-only diff,
which is correct), a GUI binding (`output::view` is the right boundary), formula
evaluation, or style diffs beyond what `calamine` can expose.

Thanks for the crate — the v2 API design made our adapter genuinely small, and
we would like to re-enable the feature as soon as the dependency path allows.

---

## After sending

Record the reply here and update:

- RFC-058's status note (F11 — the security-suspension annotation is still
  outstanding for M4);
- `docs/src/maintainers/threat-model.md`'s XLSX section if the chain changes;
- `.cargo/audit.toml` only if the reviewed exception actually narrows.

Re-enabling `.xlsx` is a dependency-policy change and needs `cargo xtask
audit-deps` plus `cargo audit` evidence, not just a version bump.
