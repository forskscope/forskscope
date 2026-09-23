# M4-C2 (§2–4) review — corrections, F49, RFC-078 execution model, and F50

**Review date:** 2026-08-13
**Request:** `dev-record/review-requests/054-m4c2-review056-corrections-f49-rfc078-execution-model.md`
**Baseline:** `ea46991`, F50 registered at `6ce1b94`
**Review mode:** Independent verification, including reproducing F50 and testing its fix. No implementation changes made.

## 1. Verdict

**Approved.** §2's corrections, F49's reconciliation and RFC-078's execution-model
amendment all land, and the judgment call on `MACOSX_DEPLOYMENT_TARGET` was
correct.

**F50 is the urgent item and it changes this slice's priority.** `main` is red.
It is a genuine advisory in a linked runtime dependency, and the fix is smaller
than the process around it — see §3, which includes a verified remedy.

B4 remains open; v1/public release stays **No-Go**.

## 2. Verified

| Check | Result |
|---|---|
| `release.yml` macOS job on `macos-latest` | Confirmed — line 129 |
| `MACOSX_DEPLOYMENT_TARGET: "13.0"` on the build step | Confirmed — line 155 |
| RFC-078 macOS collapsed to one row | Confirmed — line 110 |
| RFC-078 `## Execution model` with F45/F46 stated as blind spots | Confirmed — lines 128, 167, 174 |
| `testing.md` stale token replaced | Confirmed; historical note correctly untouched |
| `cargo audit` on `main` | **exit 1** — reproduced |
| `cargo fmt`, `clippy --all-targets`, `test --workspace` | Pass — 1094 on this HEAD |

On the test count: 1094 here, against 1112 at M4-B. That is not a regression —
it is the expected effect of the §5–10 slice removing `archive-layout` and the
dead decoration layer. Noting it because a bare count comparison across these
two slices looks alarming and is not.

## 3. F50 — reproduced, and the fix is a lockfile bump

Confirmed independently:

```text
Crate:     webbrowser
Version:   1.2.1
Title:     Unix `BROWSER` handling allows browser argument injection
ID:        RUSTSEC-2026-0257
Solution:  Upgrade to >=1.2.2
audit exit=1
```

**This is a linked runtime dependency, not a build-time one** — the material
difference from `rand` in M4-C1:

```text
webbrowser v1.2.1 └── dioxus-desktop v0.7.9 └── forskscope-ui
```

`dioxus-desktop` calls it in two places, `app.rs:279` and `webview.rs:386`, both
`webbrowser::open(...)` on external-link activation.

### 3.1 The fix is smaller than the question

`dioxus-desktop` requires `webbrowser = "1.0"`, so the advisory clears with a
**`Cargo.lock`-only change** — no manifest edit, no dependency-graph
restructuring, no `dioxus-desktop` bump:

```text
$ cargo update -p webbrowser
    Updating webbrowser v1.2.1 -> v1.2.4

$ cargo audit          → exit 0, RUSTSEC-2026-0257 gone
$ cargo test --workspace → 1094, unchanged
```

I ran this, verified it, and reverted `Cargo.lock` — the tree I reviewed is
unmodified. The remedy is established, not proposed.

### 3.2 Reachability — likely nil, and that does not change the answer

Worth recording since this project disposes of advisories rather than reacting
to them: **ForskScope renders no external links.** `grep -rn "href"
crates/forskscope-ui/src/` returns nothing, so nothing in this application
appears to reach `webbrowser::open` at all. The exploit also requires control of
the victim's `BROWSER` environment variable, which implies an attacker who can
already run commands as that user.

So the practical risk here is low. **Fix it anyway, promptly**, for reasons that
have nothing to do with severity:

- `main` is red, and a red `main` blocks every subsequent gate. M4 cannot close
  against a failing `cargo audit`.
- The remedy costs one lockfile line and is verified.
- Adding it to `audit.toml`'s ignore list would be the wrong instinct: that list
  is for advisories with no available fix and a reviewed rationale. A fix exists.

### 3.3 Answer to your scheduling question

**A dedicated minimal-diff slice, now, ahead of the rest of M4-C2.** Not folded
into whichever slice next touches `Cargo.lock`.

The handoffs' no-dependency-changes constraint exists so that a graph change is
never smuggled in beside unrelated work and reviewed under someone else's
attention. A one-line lockfile bump submitted on its own, with `cargo audit`
before and after, honours that rule rather than breaking it — the constraint is
about *unreviewed* changes, not about never changing anything.

Scope it to exactly: `cargo update -p webbrowser`, the resulting `Cargo.lock`
diff, and F50 marked resolved. Nothing else in the commit. Evidence needed is
just `cargo audit` exit 0 and an unchanged test count.

**You were right to register and ask rather than fix it in-slice.** Discovering
it via two CI runs minutes apart on unrelated commits, and correctly identifying
that no dependency change of yours caused it, is exactly the diagnosis that
stops a red build from being blamed on the last person to touch the tree.

## 4. Answers to the requested review focus

### 4.1 `MACOSX_DEPLOYMENT_TARGET: "13.0"` — right call, applied correctly

You read the constraint correctly. "Ask; do not choose a floor" was about
*selecting which macOS the project supports*. You did not select one — you
pinned the value already enforced by `Info.plist`'s `LSMinimumSystemVersion`,
which macOS applies at launch today.

The alternative was worse in a way worth naming: switching to `macos-latest`
without pinning would have left `minos` as whatever SDK the runner happens to
ship, drifting silently on every runner-image update. That is the F49 defect
recreated the moment we fixed it. Pinning it to a value already true is the only
change that leaves the observable floor unchanged.

Flagging it rather than burying it was right, and the answer is that this was
within your scope.

### 4.2 The Windows floor — owner question, and your framing sharpened it

Correctly left open, and you found the part that makes it non-obvious: the
manifest may back a live Store submission this repository cannot see the state
of. Aligning `installation.md`'s prose to the manifest's *current* value
(1809) without touching the manifest was exactly right — it removes the
contradiction without pre-empting the decision.

The question for the owner, restated precisely: **`MaxVersionTested` is
`10.0.19041.0` (Windows 10 2004), which predates Windows 11 entirely**, while
RFC-078 requires a full Windows 11 row. So the manifest currently claims the
newest tested Windows is five years older than the primary target platform.
Whatever the floor becomes, that ceiling is stale independently, and updating it
plausibly does require a Store resubmission.

### 4.3 The execution model — complete, with one omission worth adding

The section captures the resourcing accurately, and going beyond the handoff to
state what each runner *approximates* — `ubuntu-latest`+Xvfb ≈ X11-family rather
than real Wayland, `windows-latest` ≈ a Server image rather than retail Windows
— is more useful than the handoff asked for. That is the kind of precision that
stops a green matrix from being over-read.

**One blind spot is missing:** `macos-latest` is a *rolling* label. When GitHub
advances it, the macOS row's runtime changes without any commit in this
repository — so evidence recorded under "macos-latest" is not reproducible from
the plan alone, which is precisely what `matrix-plan.md` exists to prevent
(RFC-078 §118: concrete versions, not "current"). The same applies to
`windows-latest` and `ubuntu-latest`, but macOS matters most because it is the
only row with no manual pass behind it.

Not a blocker. Record the resolved image version in each evidence file at
execution time, so a row states which macOS actually ran even though the plan
names a moving label.

## 5. Notable quality observations

- Diagnosing F50 across two CI runs on unrelated commits, and establishing the
  advisory database changed rather than the tree — that is the right conclusion
  and the harder one to reach when your own commit is the one that went red.
- Restating the `glib` disposition as a `pub(crate)` fact and noting that a
  full-source grep can miss macro-expanded call sites, which is a better reason
  than the one I gave.
- Aligning documentation prose to a value without changing the value, when the
  value was not yours to change.
- Naming what each CI runner approximates rather than treating "we run on macOS"
  as equivalent to "we tested macOS."

## 6. Recommended next action

1. **F50's dedicated slice, first** (§3.3) — `cargo update -p webbrowser` alone,
   to get `main` green. Everything else queues behind a passing `cargo audit`.
2. §4.3's rolling-label note in `matrix-plan.md`.
3. Owner: the Windows floor **and** the stale `MaxVersionTested` (§4.2).
4. Review of request 055 (M4-C2 §5–10) follows separately.
