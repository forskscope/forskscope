# M4-C4 review — evidence layout, matrix freeze — and the Gate C assessment

**Review date:** 2026-08-13
**Request:** `dev-record/review-requests/057-m4c4-evidence-layout-matrix-freeze.md`
**Baseline:** `b7960bc`
**Governing documents:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c4-matrix-freeze-handoff.md`, RFC-074, RFC-078
**Review mode:** Independent verification, followed by the Gate C assessment RFC-074 assigns to the architect.

---

# Part I — Review of M4-C4

## 1. Verdict

**Approved.** F56's restructure, the owner's answers, F44's representation and
the freeze all land.

One refinement to F44's framing (§3.1) that makes the plan *more* decisive, not
less. Nothing blocking.

## 2. Verified

| Check | Result |
|---|---|
| Both documents moved as git renames, not delete+recreate | Confirmed — `{0.167.0-rc1 => }/…` in the commit |
| `0.167.0-rc1/` removed | Confirmed |
| RFC-078 layout amended, with reasoning inline | Confirmed |
| Freeze declaration states what frozen *means* | Confirmed — `matrix-plan.md:8–14` |
| `0.167.0-rc1` sweep | Two files remain, both correctly untouched: `ROADMAP.md`'s dated entries and the handoff itself |
| `advisories.md` tally still accurate | Confirmed — claims 12 unmaintained + 2 unsound; `cargo audit` reports exactly that |
| `cargo fmt`, `clippy --all-targets`, `test --workspace` (1094), `i18n` (227), `css --check`, `version-sync`, `audit-deps`, `cargo audit` (exit 0), `mdbook` | All pass |
| CI `31703130411` on `b7960bc` | `success` |

## 3. Answers to the requested review focus

### 3.1 F44's representation — right shape, and RFC-078 has already decided more than you claimed

Your three-way framing is correct, and the reasoning for rejecting both
alternatives is exactly right: pre-declaring a No-Go oversteps a planning slice,
and narrowing the test host to avoid the failure is the "green matrix that
doesn't mean what it appears to" mode RFC-078 exists to prevent. Running it and
recording Fail is the only option that keeps the evidence honest.

**But §3.3 understates the position.** You wrote that the plan "deliberately does
not decide in advance that this failure blocks the release — that's Gate D's
call." I checked RFC-078's Waiver policy, which you cited for the narrower point
about waivers:

> **No waiver may turn these into a release pass:**
> … **inability to launch on a claimed supported platform** …

That is not a factor Gate D weighs. It is a pre-committed decision the RFC
already made. So if F44 is still open when M5 runs, Linux P01 fails on a
libxdo-4 host, and **no waiver can convert that into a pass.** Gate D's
discretion here is not "how bad is this" — it is only "has the upstream fix
landed or not."

That makes F44 a **schedule dependency with a binary outcome**, not a risk to
be weighed later:

- Upstream `dioxus-desktop` release lands before M5 → bump, rebuild, P01 runs
  clean, no special handling.
- It does not → Linux P01 fails un-waivably → the candidate cannot pass Gate D
  until it does.

**Say that in the plan.** It answers your own question 1 ("should the plan name
in advance what would make Gate D lean Go versus No-Go?") with: there is nothing
to lean, and stating so is more useful than leaving a reader to discover the
waiver policy themselves. It also converts F44 from something to watch into
something to *drive* — the upstream release is now on the project's critical
path, and that is worth knowing now rather than at M5.

### 3.2 The host-access inference — correct to infer, correct to flag

Inferring owner-executed implies owner's-own-access is sound, and it is the
answer I would have given. Recording the inference in §4 Q3 rather than silently
closing the field is what makes it reviewable. No change.

### 3.3 The freeze declaration — precise enough on meaning, thin on procedure

"Changing any of them after M5 begins invalidates evidence already gathered
under this plan — a row re-planned mid-matrix is not the row that produced the
evidence collected so far" is a good statement of *why*, and better than most
freeze notices.

What it does not say is **who may unfreeze it and what that costs.** A future
slice under time pressure will read "invalidates evidence" and may still amend a
row, because nothing names a gate. Add one sentence: amending a frozen row after
M5 begins requires owner and architect agreement, and every affected row's
evidence is re-gathered — not reinterpreted.

Small, and it is the difference between a warning and a control.

## 4. Notable quality observations

- Using `git mv` so the moves record as renames, keeping both documents' history
  traversable — easy to lose and irreversible once lost.
- Putting F56's three reasons *inside* RFC-078's layout section rather than only
  in the register, so a future reader who wants an `-rcN` scheme meets the
  argument at the point of temptation.
- Rewriting both documents' placeholder language rather than leaving "TBD"
  fields beside a FROZEN banner — a frozen plan with open placeholders would
  have been worse than an unfrozen one.
- Keeping F44 **open** in the register while recording that its standing
  changed. Status and standing are different properties and conflating them is a
  common way for a defect to look handled.

---

# Part II — Gate C assessment

RFC-074 assigns this to the architect. Assessed against its stated criteria on
`b7960bc`.

## 5. The integrated gate

All eight commands run independently on this baseline:

```text
cargo fmt --check                              pass
cargo xtask css --check                        pass
cargo xtask version-sync                       pass — v0.166.1
cargo xtask i18n                               pass — 227 keys
cargo xtask audit-deps                         pass
cargo audit                                    pass — exit 0
cargo test --workspace                         pass — 1094
cargo clippy --workspace -- -D warnings        pass
```

Plus the gates M4-B added, which did not exist when Gate C was written:
`clippy --all-targets`, `xtask` under `fmt --check`, `actionlint` over every
workflow, the umask-scoped permission tests, and the F34 rendering check.

Gate C's additional condition — *"`cargo audit` exit success is not sufficient
by itself. Every unsoundness advisory must have a reachability statement, owner,
review date, and upgrade trigger in the release evidence"* — is **met**.
`release-evidence/advisories.md` disposes of both unsoundness advisories with
all four fields, both reachability statements independently reproducible in one
command, twelve unmaintained advisories under a stated policy, and the two
suppressed advisories restated as formal dispositions rather than a config
comment.

## 6. M4's exit criteria

| Criterion | Status |
|---|---|
| Full documented gates | **Met** — §5, and strengthened beyond the original list |
| Docs/RFC status synchronized | **Met** — M4-C2: RFC-058, RFC-062 moved to `done/`, RFC-024/028 corrected, feature claims audited against the UI crate |
| Advisory dispositions recorded | **Met** — §5 |
| `matrix-plan.md` frozen | **Met** — as of 2026-08-13, hosts/cases/executors fixed |

## 7. Verdict: Gate C passes

**The release core is approved as a candidate for runtime QA.** M4 closes.

What that does and does not mean, stated precisely because this is exactly where
a gate gets over-read:

- It means the code, gates, documentation and advisory posture are in a state
  where platform evidence is worth gathering — the candidate will not waste M5's
  time failing on things that could have been caught earlier.
- **It does not mean the release is good.** B4 is open. No platform runtime
  evidence exists for any target. **v1/public release remains No-Go**, and
  nothing in this assessment changes that.

Three of four audit blockers are closed: B1 (RFC-075, released `0.165.0`), B2
and B3 (RFC-076/077, released `0.166.0`). B4 is the remaining one and M5 exists
to close it.

## 8. Entering M5 — what carries in

Recorded so M5 does not rediscover them:

- **F44 — a schedule dependency, per §3.1.** Un-waivable if unfixed. The
  upstream `dioxus-desktop` release is now on the critical path.
- **F46 — unverifiable under current resourcing.** No macOS manual host, and CI
  cannot observe Gatekeeper. One person opening the DMG on any Mac once closes
  it; nothing else will.
- **F45 — manual-only by construction.** CI runners carry the prerequisites, so
  only a clean Windows machine can exercise it.
- **F49b** — `MaxVersionTested` is bumped as an *output* of M5's Windows
  evidence, not before it.
- **Review 060's residual** — `audit.yml`'s push trigger has never fired; F44's
  eventual bump is the natural first exercise.

## 9. Recommended next action

1. §3.1's F44 sentence and §3.3's unfreeze procedure — two sentences, no rush.
2. **Owner:** M5 needs a candidate. That means cutting a tag so the release
   workflow produces artifacts and a draft to test against — the draft *is* the
   candidate, per F56. The level is decided at the cut per `release.md`.
3. Then M5: RFC-078's matrix, against those exact artifact digests.
