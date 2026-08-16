# Draft reply to the sheets-diff team — 2.3.0 gate results

**Status:** Draft for the project owner to send. Not sent.
**Responds to:** their message of 2026-08-16 (`sheets-diff` 2.3.0)
**Prepared:** 2026-08-16, after running the checks described below
**Register:** F65

## Note for us, not for sending

Their ask was "run your gate against 2.3.0 and tell us what it finds." Answering
that honestly required saying something they will not expect: **our gate is
green because `sheets-diff` is absent from our tree entirely**, not because
2.3.0 passes it. Reporting "our gate is green" would have been true and
misleading — the same credited-with-more-than-it-measures error this project
keeps finding in its own checks, pointed outward at someone relying on it.

So the gate was actually run, on a throwaway branch that re-added 2.3.0. It
found something worth reporting: `cargo xtask audit-deps` **rejects
`sheets-diff` by name**. That is our own fail-closed policy, not a verdict on
their release, and the distinction is the substance of the reply.

The tree was restored afterwards; nothing was committed.

---

## Draft message

**Subject:** 2.3.0 gate results — and what our green currently means

Hello,

Thank you for this. It is the most useful upstream reply we have had: you
answered the question we asked, disclosed things we did not ask about, and named
what is still wrong. The rest of this is what our checks found, including one
thing you should know about how to read them.

### 1. Your dependency claim verified, independently

Verified from a scratch project rather than your lockfile or ours:

```text
sheets-diff 2.3.0
└── calamine 0.36.1
    ├── quick-xml 0.41.0
    └── zip 8.6.0

cargo audit → exit 0
```

No `quick-xml` 0.39.x anywhere in that graph, and no advisories. Your §1 holds.

We also re-added 2.3.0 to our own tree on a throwaway branch and confirmed the
same there: `quick-xml` 0.39.4 is still present, but **only** through
`wayland-scanner`, a proc-macro/codegen dependency in the GTK/Wayland stack, and
never through your chain. That path carries no user-supplied workbook XML, which
is why our advisory policy already accepts it. Your release does not reintroduce
0.39.x by any route.

### 2. What our gate actually says — please read this before quoting it

Our gate is green, and **that green does not mean what you need it to mean.**

When we disabled `.xlsx` comparison in July we did not pin `sheets-diff` to a
safe version — we removed the dependency outright, and added it to the
deny-by-name list that `cargo xtask audit-deps` enforces. So on our tree today:

```text
cargo audit               → exit 0
cargo xtask audit-deps    → passed
```

Both are statements about **absence**. Neither observes 2.3.0 at all. If we had
simply answered "our gate passes," you would have recorded a demonstration that
had not happened — and half your milestone's exit criterion would have been
closed by a measurement of nothing.

With 2.3.0 actually re-added, the honest result is:

```text
cargo audit               → exit 0, unchanged advisory set
cargo xtask audit-deps    → exit 1: unexpected dependency present: sheets-diff
```

**That failure is our policy, not your defect.** It is the fail-closed decision
working exactly as intended: `sheets-diff` cannot return to our tree by accident,
only by someone deliberately removing it from the deny list. Your advisory
situation is clean; our gate is refusing the *category*, not the version.

So the accurate report is: **2.3.0 clears every advisory-based check we have,
and re-adoption is blocked only by a policy switch we control.**

### 3. MSRV — not a cost for us

Our workspace declares `rust-version = "1.91"`, so your move to 1.88 is below our
floor and costs us nothing. Worth saying because you flagged it as the larger of
the two costs; for us it is not a cost at all.

### 4. The comparison-output changes

You are right that in a merge tool a false negative is a data-loss path, and
right to ship those fixes in the security release rather than later. Two notes:

- We do not cache or store diffs between runs, so "stored diffs from 2.2.x may
  not reproduce" does not affect us. Every comparison is computed fresh.
- Our adapter matched on `code()`, and you have kept every pre-existing `code()`
  string, so the surface we depend on is unchanged.

The formula-attachment defect is the one we would have been most exposed to,
since it is silent and content-dependent rather than something a user would
notice as an error.

### 5. Your §4 disclosures are worth more to us than the dependency fix

The unbounded `m × n` row-alignment allocation is the item we care about most.
Our threat model is exactly the one you quoted back — users open files they did
not author — and an allocation failure that aborts the process is a
denial-of-service we would have inherited without ever seeing it in an advisory
feed. Bounding it *and* degrading to positional comparison with a diagnostic,
rather than erroring, is the right shape: a bound that fails the whole
comparison would have been a second denial-of-service wearing the costume of a
fix.

`Limits::default()` protecting nobody is the kind of thing that is only found by
someone deciding to look. We would not have found it from the outside.

### 6. Something in your message we are taking internally

> a golden detects *change* and cannot detect having been born wrong

We have made the same mistake in a different place — a golden fixture specified
to cover a property it structurally could not, which we only caught later. We
are treating your sentence as the general statement of it. Thank you for writing
it down rather than quietly fixing the file.

### 7. What we are doing next, and the honest timing

We are not adopting 2.3.0 immediately, and the reason is scheduling on our side
rather than any doubt about the release.

We are mid-way through a platform-acceptance matrix that gathers runtime
evidence against a specific set of published artifacts, identified by SHA-256.
Adding a runtime dependency changes every one of those artifacts and invalidates
the evidence already collected — our own rules say evidence from an older hash
cannot approve a newer artifact. So re-enabling `.xlsx` is queued behind that
gate rather than slipped in beside it.

When we do take it up, the open questions on our side are the residual risks you
named — particularly `compare_bytes` doubling peak memory, since our adapter
does pass bytes. We will tell you what we find.

### 8. On your ask

You asked us to close the half of your milestone that reads "their dependency
gate passes." We would rather you record what is actually true than something
tidier:

- **Advisory-based checks: pass.** `cargo audit` is clean against 2.3.0, both in
  isolation and inside our tree.
- **Dependency-path gate: rejects by name**, by our policy, pending our own
  decision to re-enable `.xlsx`.
- **Runtime verification: not performed.** We have not run a workbook through
  2.3.0, because the code path is still disabled on our side.

If the third line matters to your milestone, say so and we will find a way to
exercise it independently of our release schedule.

Thanks again — for the fixes, and for the parts of the message that were about
what is still wrong.
