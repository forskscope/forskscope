# F55 N1 closeout — path-filtered advisory audit

**Review date:** 2026-08-13
**Baseline:** `d8b1afd`
**Responds to:** review 059 §4 (N1) and §5.1
**Review mode:** Independent verification from the repository. The dev team's message arrived partly garbled in transmission, so nothing here rests on its text.

## 1. Verdict

**N1 is closed. Approved.**

One residual is recorded in §3 with a zero-cost resolution — no further work is
requested now.

## 2. Verified

| Check | Result |
|---|---|
| `audit.yml` triggers: `schedule`, `workflow_dispatch`, `push`, `pull_request` | Confirmed |
| Path filter `['Cargo.lock', '**/Cargo.toml']` on both push and PR | Confirmed |
| `push` additionally scoped to `branches: [main, master]` | Confirmed — sensible; PR runs cover branches |
| Required-status-check caveat documented | Confirmed — `audit.yml:42–46` |
| 60-day scheduled-workflow-disable note | Confirmed — `audit.yml:56` |
| `release.yml` preflight `cargo audit` still present and hard-blocking | Confirmed — unchanged |
| CI `31701029787` on `d8b1afd` | `success` |
| Negative case: audit did **not** fire on `d8b1afd` | Confirmed — that commit touches no manifest, and no audit run exists for it |

`**/Cargo.toml` matching a root-level `Cargo.toml` was worth checking rather
than assuming, since only `Cargo.lock` is listed at root. It does — `**` matches
zero or more directories — so a root workspace-manifest change is covered.

## 3. Residual — the positive trigger has never fired, and that is fine

Every run of `audit.yml` to date is `workflow_dispatch` (the two falsifiability
runs from review 059). The `push`/`pull_request` path filter has proven it stays
*silent* on a non-dependency commit; nothing has yet proven it *fires* on a
dependency change.

Ordinarily this project's standard would ask for that demonstration. Here I am
not, for two reasons: the glob is correct per the documented semantics above,
and `Cargo.lock` is matched literally, so the uncertain half is the part already
covered by an exact match.

**Zero-cost resolution instead of a throwaway branch:** F44's `dioxus-desktop`
bump is coming as soon as the upstream release lands, and it is by definition a
`Cargo.lock` change. That slice's review request should record whether
`audit.yml` fired on it. If it did, the trigger is proven by real work. If it
did not, that is a finding worth having at exactly the moment a dependency moved.

Noted here so the expectation is tracked rather than assumed.

## 4. Notable quality observations

- Scoping `push` to `main`/`master` while leaving `pull_request` unscoped —
  branch pushes get covered once, via the PR, instead of twice.
- Checking whether branch protection actually exists before deciding the
  skipped-required-check caveat was moot, rather than reasoning about it
  abstractly — and documenting the caveat anyway for whoever adds protection
  later.
- Putting both operational caveats in the workflow header, where someone
  changing this file will read them.

## 5. Where this leaves M4

Nothing is outstanding on the dev team. The last item gating Gate C is the
owner's answers to `matrix-plan.md` §4 — the Linux support baseline above all,
which decides whether F44 is a documented limitation or a schedule dependency
for M5.
