# RFC-074 M4-C1 Developer Handoff: Advisory Dispositions and the Platform-Matrix Freeze

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md), with [RFC-078](../../proposed/078-platform-runtime-acceptance.md) for the matrix
**Milestone.** M4-C1 — the Gate C and M5 prerequisites, ahead of M4-C2 (documentation and code truth)
**Register items.** F7 (advisory N5), F9 (advisory N2), F37, plus the `matrix-plan.md` freeze
**Baseline.** `main` at `eae2ae9`

This handoff directs execution of one slice. It does not redefine RFC-074 or
RFC-078. If implementation evidence contradicts a decision below, amend the RFC
first, then update this handoff to match.

## 1. Summary

M4-C is split. **C1 is the part that gates something else:** M4's exit requires
advisory dispositions recorded and `matrix-plan.md` frozen, and M5 cannot begin
without the latter. **C2 is documentation and code truth** (F11, F12, F16,
F25/F25b, F31, F39, F43, F48) and follows separately.

C1 is separated because `matrix-plan.md` needs owner input — host access,
executor identity, and which OS versions the project actually claims — and that
dependency benefits from lead time. It should not sit behind a queue of
one-line RFC status corrections.

**This slice writes almost no code.** It is mostly the project stating plainly
what is true, what is accepted, and who accepted it. That is Gate C's actual
content, not a formality around it.

No audit blocker closes here. B4 remains open; v1/public release stays **No-Go**.

## 2. Scope

In scope: F7/N5, F9/N2, F37, and creating `matrix-plan.md`.

Not in scope:

- **M4-C2** — F11, F12, F16, F25/F25b, F31, F39, F43, F48. F43 additionally
  awaits an owner decision and must not be actioned by you.
- **Advisory N3** (handoff drift) — M6.
- **Running the matrix.** C1 *plans* it and freezes the plan. Executing cases is
  M5. Do not gather platform evidence here.
- **F44** — waiting on a `dioxus-desktop` release. **F45, F46** — M5.

## 3. F7 / advisory N5 — record a disposition for every advisory

`cargo audit` currently exits 0 while reporting **14 warnings**: 12
`unmaintained` and 2 `unsound`. Two further advisories are suppressed outright
in `.cargo/audit.toml`.

Gate C is explicit that this is not sufficient:

> `cargo audit` exit success is not sufficient by itself. Every unsoundness
> advisory must have a reachability statement, owner, review date, and upgrade
> trigger in the release evidence.

And N5 adds: *"keep policy-pass distinct from a clean advisory set."*

Create `docs/src/maintainers/release-evidence/<version>-rcN/advisories.md`
(RFC-078's "Durable evidence layout" names this file) containing:

### 3.1 The two unsoundness advisories — full disposition each

| ID | Crate | Title |
|---|---|---|
| `RUSTSEC-2024-0429` | `glib` 0.18.5 | Unsoundness in `Iterator`/`DoubleEndedIterator` for `VariantStrIter` |
| `RUSTSEC-2026-0097` | `rand` 0.7.3 | Rand is unsound with a custom logger using `rand::rng()` |

Each needs all four fields, and the reachability statement is the one that takes
real work:

- **Reachability** — is the unsound API reached from any ForskScope code path,
  including transitively? Name the path or state that no path reaches it, and
  say how you established that (`cargo tree -i`, call-site search, both).
  "Probably not reachable" is not a disposition.
- **Owner** — a person, not "the project."
- **Review date** — when this disposition was made.
- **Upgrade trigger** — the specific condition that forces revisiting: an
  upstream release, a dependency bump, a date.

`rand 0.7.3` is worth particular attention: it is an old major version, and
establishing *why* a 0.7 is still in the graph at all is part of the
disposition.

### 3.2 The twelve unmaintained advisories — a policy statement, not twelve essays

These do not each need the four-field treatment. What they need is one recorded
policy: what "unmaintained" means for this project's risk posture, why a pass
under that policy is not a clean advisory set, and under what condition an
unmaintained crate becomes a blocker. List the twelve with their crates so the
set is enumerable later.

### 3.3 The two suppressed advisories

`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` are in `audit.toml`'s `ignore` list
with a rationale in a code comment. That rationale is good, and a comment in a
config file is not release evidence. Restate it in `advisories.md` in the same
four-field form, and cross-reference `cargo xtask audit-deps`, which is what
actually enforces the reviewed path.

## 4. F9 / advisory N2 — narrow the durability claim, or earn it

**Establish the distinction first, because the current wording conflates two
different guarantees.**

What the implementation does: write a temp file in the target directory, then
`rename`. There is **no `fsync`/`sync_all` anywhere in `forskscope-core`** —
confirmed by search.

That yields:

- **Atomic visibility — true.** A concurrent reader sees the old file or the new
  file, never a partial one. `rename` within a volume is atomic on POSIX.
- **Power-loss durability — not established.** Without syncing the temp file
  before the rename and the parent directory after it, a crash can leave the
  target missing or, on some filesystems and mount options, present with
  unexpected content.

Current claims to audit and reconcile, at minimum:

- `crates/forskscope-core/src/save.rs:7` — "Writes are atomic (temp file in the
  same directory, then rename)"
- `save.rs:97` — "Atomic on POSIX (`rename` within the same volume)"
- `docs/src/users/merging.md:73` — "atomically — it either appears complete or
  not at all"
- `README.md:98`, `docs/src/users/features.md:20` — "atomic write"
- `docs/src/maintainers/threat-model.md` — check for durability language

Sweep for others; that list is a starting point, not a closed set.

**Two acceptable outcomes, and N2 states the bar:** *"Narrow the durability
claim unless file and parent synchronization plus metadata behavior are
implemented and evidenced per platform."*

1. **Narrow the wording** (expected). Keep "atomic" where it means visibility —
   that claim is true and worth keeping — and remove or qualify anything a
   reader would take as crash safety. State the actual guarantee explicitly
   somewhere durable rather than only deleting the overclaim.
2. **Implement it.** `sync_all` on the temp file before rename, plus a parent
   directory sync after. If you take this path it is a product change with
   per-platform behaviour to evidence, and it is bigger than this slice —
   propose it rather than doing it here.

Whichever you choose, the point is that a user reading "atomic write" should not
end up with a belief the code does not support.

## 5. F37 — fold the recovery dialogs into P08

RFC-078's **P08 — Persistence migration** predates RFC-076's recovery dialogs,
so the `Exit` action's behaviour on Windows and macOS is in no platform case.
It has been verified on Linux/WebKitGTK only.

Extend P08 so the blocking recovery dialog's three actions — **Exit**,
**Continue**, **Reset** — are each covered on every platform in the matrix. Exit
is the one that matters most: it terminates the process from inside a modal
during startup, which is exactly the kind of path that differs across window
toolkits.

This is a change to RFC-078's case definition, so amend the RFC, then make sure
the matrix plan's case IDs reflect it.

## 6. The `matrix-plan.md` freeze

**This file does not exist yet.** M4's exit gate requires it frozen and RFC-078
§"Preconditions" requires it committed before M5 begins, so creating it is part
of this slice.

Location, per RFC-078's durable evidence layout:

```text
docs/src/maintainers/release-evidence/<version>-rcN/matrix-plan.md
```

RFC-078 requires each row to freeze:

- exact OS/distribution version
- architecture
- executor owner/role
- host-access status
- applicable case IDs

Cases are **P01–P12** as defined in RFC-078 §"Platform cases", with P08 as
amended by §5 above.

Rows to cover, from RFC-078's evidence layout: `linux-wayland`, `linux-x11`,
`windows-11`, `windows-10`, `macos-aarch64`.

### 6.1 What you decide, and what you must ask the owner

**Yours:** the case-to-row mapping — which of P01–P12 apply to which platform,
and where a case is platform-specific or meaningless. Justify any case you mark
not-applicable; "N/A" without a reason is how coverage silently shrinks.

**The owner's, and you must ask rather than assume:**

- exact OS versions the project claims to support (RFC-078 §118 is explicit that
  `matrix-plan.md` replaces vague "current" and "oldest claimed" language with
  concrete versions)
- executor owner/role per row
- host-access status per row — the roadmap records access is confirmed available
  for all three platforms, but that is not the same as a named executor with a
  specific machine

Collect these as questions in your review request rather than inventing
plausible values. A frozen plan built on guesses is worse than an unfrozen one.

### 6.2 Known facts to fold in

Three findings already constrain the matrix, and the plan should reference them
rather than rediscover them at M5:

- **F44** — the current Linux artifact does not start on libxdo-4 distributions.
  Fixed upstream but unreleased. The Linux rows must state which artifact they
  test and that this is expected until the fix ships.
- **F45** — the Windows artifact's undeclared `VCRUNTIME140.dll`/WebView2
  runtime dependencies. P01 (install and cold launch) should explicitly cover a
  machine *without* them.
- **F46** — the macOS artifact is unsigned and unnotarized, so Gatekeeper is
  expected to refuse a quarantined download. P01 must cover the real download
  path, not a locally-built bundle, or it will not observe this at all.

All three are inspection-level findings except F44. The matrix is where they
become verified or refuted.

## 7. Constraints

- `0.165.0` and `0.166.0` are published and immutable.
- No dependency is added, removed, or version-changed. In particular, **do not
  attempt to resolve an advisory by bumping a dependency in this slice** — a
  disposition records the decision; changing the graph is separate work with its
  own review.
- No product behaviour changes. §4's option 2 is explicitly out of scope.
- Existing gates must keep passing.
- No real user paths, host names, or secrets in evidence files. The matrix plan
  names roles and machine classes, not personal identifiers beyond what the
  owner supplies.

## 8. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F7/N5, F9/N2, F37, `matrix-plan.md`);
3. changed and created files;
4. **the two unsoundness reachability statements, and how you established
   each** (§3.1) — this is the review's main focus;
5. **F9's outcome**: the full list of durability claims you found, what each
   became, and the explicit statement of the actual guarantee (§4);
6. **the matrix plan's case-to-row mapping with justification for every
   not-applicable** (§6.1);
7. **the questions for the owner** (§6.1), collected and unanswered rather than
   guessed;
8. any difference from this handoff, RFC-074, or RFC-078;
9. executed gates with observed output;
10. unresolved issues and known limitations;
11. requested review focus.
