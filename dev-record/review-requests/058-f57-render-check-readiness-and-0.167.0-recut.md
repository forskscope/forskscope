# Review Request: F57 — Render-Check Readiness and the 0.167.0 Re-cut

**Date:** 2026-08-14
**Reviewer stance:** the readiness condition and the three falsifiability demonstrations are the main focus, per the handoff
**Repository baseline:** `cb6f5b6` (re-cut tag `0.167.0` points here)
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/f57-render-check-readiness-and-recut-handoff.md`

## 1. Implementation summary

Four commits, sequenced deliberately so the falsifiability demonstrations
would use genuinely pre-fix code on a real runner rather than a
reconstruction:

1. `26c4527` — the on-demand entry point (§4) and the defect-reintroduction
   script, committed *before* the readiness fix, so the original failure
   could be reproduced through the new infrastructure using the actual
   code that shipped in the failed run.
2. `c6f2459` — the readiness fix itself (§3) and a new corpus test pinning
   the fixture's row count.
3. `cb6f5b6` — the `CHANGELOG.md` re-cut note (§6.2).
4. The re-cut (§6.1, §6.3): delete + re-tag `0.167.0`, push, release
   workflow runs to a draft.

## 2. The readiness condition, and why (§3)

**Condition: wait until the accessible tree reaches the fixture's known,
pinned shape — the landmark exists AND each pane has exactly 7 table
rows — not until the landmark merely exists.**

Diagnosis confirmed the mechanism, not just the symptom: `find_by_role`
does a single traversal with no retry; `find_app` polls for AT-SPI
registration (fast — a window exists as soon as it's created) but then
hands off to one landmark-finding attempt with no polling of its own. The
failed run's timestamps (launch `13:55:25`, failure `13:55:28`) confirmed
this exactly — 3 of a 30-second budget consumed, 27 unspent, because
nothing ever retried.

"Landmark exists" was deliberately rejected as the condition (per the
handoff's own steer): `collect_rows` is also a single traversal, so a tree
caught between landmark-appearing and full-content-painting could yield a
partial row set — failing confusingly ("found only N rows") or, worse,
comparing a subset and passing. Waiting for the *known* shape is what
makes the check slow-tolerant rather than merely lucky.

**Where the row count (7) comes from, and how it's pinned:** the fixture
pair (`left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`) produces 7
visual rows — alpha (Equal), old-line/new-line (Replace), gamma (Equal),
delete-line/*empty* (Delete), epsilon (Equal), zeta (Equal),
*empty*/insert-line (Insert). The existing corpus test only pinned the
*hunk-kind sequence*, not the literal row count, so I added a new one:
`all_hunk_kinds_fixture_produces_exactly_seven_visual_rows`
(`crates/forskscope-core/tests/diff_corpus.rs`), asserting
`doc.hunks.iter().map(|h| h.rows.len()).sum() == 7` directly against
`compute_diff`'s output — the same `DiffRow` model `hunk.rs`'s
`RowLeft`/`RowRight` iterate to render. This makes the handoff's framing
("pinned by a corpus test") literally true rather than aspirational; the
existing kind-sequence test only pinned the coarser shape.

**Independently cross-checked against a real running instance**, not just
derived on paper: built the release binary locally
(`cargo build --release --locked -p forskscope-ui`) and ran the (then
pre-fix) `render_check.py` directly against it with a real AT-SPI bus
(`NO_AT_BRIDGE=0`, no `dbus-run-session` needed — a real desktop session
already has one), confirming `7 left rows + 7 right rows` before writing
the pinning test, and again after each change in this slice.

## 3. The on-demand entry point (§4), and how to use it

`.github/workflows/render-check.yml` — `workflow_dispatch` only, one
boolean input (`inject_f32_defect`, default `false`). Builds the Linux
release binary and runs `render_check.py` under the same
`xvfb-run --auto-servernum dbus-run-session -- env NO_AT_BRIDGE=0`
invocation `release.yml`'s `linux` job uses, without creating a release or
moving a tag.

```sh
gh workflow run "Render Check (on demand)" --ref <branch-or-sha>
gh workflow run "Render Check (on demand)" --ref <branch-or-sha> -f inject_f32_defect=true
```

`inject_f32_defect=true` runs `packaging/reintroduce_f32_defect.py` first,
which moves `hunk.rs`'s `sr_label` span back out of `.cell` (F32's exact
defect shape) in that run's checkout only — never committed, asserts
loudly (exit 1) rather than silently no-op'ing if `hunk.rs`'s shape has
changed since the script was written.

## 4. The three demonstrations, with observed output

### (1) Reproduce the failure, on a runner, via the new entry point

Commit `26c4527` added the entry point *without* the readiness fix.
Dispatched against `main` at that commit:

Run [`31847082187`](https://github.com/forskscope/forskscope/actions/runs/31847082187) — **failure**, exact original message:

```text
FAIL: could not find the 'File comparison' landmark
```

(Timestamps confirm the same 3-of-30-second pattern as the original
failed release run `31706778085`.)

### (2) Show the fix passing, on the same path

Commit `c6f2459` (the readiness fix) pushed. Dispatched again, same
workflow, `inject_f32_defect=false`:

Run [`31847446602`](https://github.com/forskscope/forskscope/actions/runs/31847446602) — **success**:

```text
OK: 7 left rows + 7 right rows all aligned within their pane.
```

### (3) Show it still detects the defect it exists for — the one that matters most

Same commit, `inject_f32_defect=true`:

Run [`31847749205`](https://github.com/forskscope/forskscope/actions/runs/31847749205) — **failure**, the correct one:

```text
FAIL: F34 rendering check found misalignment:
  - left pane: a row has 3 accessible children, other rows have 2 - a label is likely rendering as a sibling of the content cell instead of inside it (F32's defect shape)
  - left pane: a row has 3 accessible children, other rows have 2 - a label is likely rendering as a sibling of the content cell instead of inside it (F32's defect shape)
  - right pane: a row has 3 accessible children, other rows have 2 - a label is likely rendering as a sibling of the content cell instead of inside it (F32's defect shape)
  - right pane: a row has 3 accessible children, other rows have 2 - a label is likely rendering as a sibling of the content cell instead of inside it (F32's defect shape)
```

Confirms the readiness fix widened *when* the check runs, not *what* it
accepts — a check that waits longer but has quietly gone permissive would
pass (1)/(2) and fail here; it doesn't.

All three demonstrations were also run locally first (real desktop AT-SPI
bus, not CI) before touching CI at all, to catch a broken patch script or
a wrong row count cheaply — same three outcomes, same messages.

## 5. Changed files

`packaging/render_check.py` (readiness fix — `wait_for_ready`, module doc,
constants), `crates/forskscope-core/tests/diff_corpus.rs` (new row-count
pinning test), `.github/workflows/render-check.yml` (new, the on-demand
entry point), `packaging/reintroduce_f32_defect.py` (new, demonstration
helper), `CHANGELOG.md` (re-cut note).

## 6. The re-cut

- **Old tag:** `0.167.0` → `2948008` ("release: 0.167.0 - integrated
  stabilization, the M5 candidate") — the commit whose Linux build failed
  in run `31706778085`.
- **New tag:** `0.167.0` → `cb6f5b6` ("docs: F57 - record the 0.167.0
  re-cut in CHANGELOG") — the corrected commit, three commits ahead.
- **CHANGELOG.md** — added a re-cut note to the `[0.167.0]` entry (quoted
  in full):

  > **Re-cut (F57).** The first `0.167.0` tag failed at the rendering
  > check on its Linux build before any artifact was published — the
  > check itself was at fault (it gave up after one tree walk instead of
  > retrying until the WebView had actually painted, which a
  > software-rendered CI runner reaches much slower than a developer
  > machine), not the product; macOS and Windows had already passed. No
  > release was created, not even a draft, so the tag was deleted and
  > re-cut against the corrected commit per `release.md`'s immutability
  > policy — nothing published needed to change.

- **Executed exactly:** `git push origin --delete 0.167.0` (not
  `gh release delete --cleanup-tag`, since there was no release to delete
  — `release.md`'s own caution about that flag's scope), then
  `git tag -d 0.167.0` / `git tag -a 0.167.0 -m "Release 0.167.0" cb6f5b6`
  / `git push origin 0.167.0`.
- **Not published.** `gh release view 0.167.0 --json isDraft` reports
  `"isDraft": true`.

## 7. Executed gates, with observed output

```text
cargo fmt --check                                              pass
cargo clippy --workspace --all-targets -- -D warnings           pass
cargo test --workspace                                          pass — 1095 (1094 + the new row-count pinning test)
cargo xtask i18n                                                 pass — 227 keys, unchanged
mdbook build docs                                                pass
git diff --check                                                 pass
actionlint .github/workflows/*.yml                               pass (v1.7.12, matching ci.yml's pin)
```

No dependency added, removed, or version-changed — confirmed by the
unchanged `Cargo.lock` throughout (`git diff --stat Cargo.lock` empty at
every commit in this slice). `dioxus-desktop` was not touched; this
candidate knowingly carries F44 per the handoff's explicit constraint.

## 8. The release run's result, job by job, and the draft's artifact list

Run [`31847956550`](https://github.com/forskscope/forskscope/actions/runs/31847956550) on tag `0.167.0` (→ `cb6f5b6`):

| Job | Result |
|---|---|
| Release gates | success |
| macOS aarch64 | success (one benign annotation — an unrelated Homebrew tap-trust notice, not a failure) |
| Linux x86_64 | **success** — the job that failed before |
| Windows x86_64 | success |
| Create GitHub Release | success |

Draft artifacts (`gh release view 0.167.0 --json isDraft,assets`):

| File | SHA-256 |
|---|---|
| `forskscope-v0.167.0-linux-x86_64.tar.gz` | `e17baa26abbb91e5e8e046d3812b08203f0d1ddfd6f8dc9fb9182326ed04bf09` |
| `forskscope-v0.167.0-macos-aarch64.dmg` | `2d66f125f0325adfef36cdf9bbb643a8deed50112db77707f7e6c9970ca25099` |
| `forskscope-v0.167.0-windows-x64.zip` | `bd7c1d9107754f7866639de7d09668fcd0c70ca5669f5cbee15ccdfeca293c1d` |

`isDraft: true`. Not published — publication is the owner's action, per
the handoff.

## 9. Unresolved issues and known limitations

- **`cargo xtask version-sync`'s dev-mode check now fails on every
  ordinary `main` push**, and will keep doing so until `0.167.0` is
  either published (and the post-release commit bumps the workspace
  version) or the candidate is abandoned. This is expected, not a
  regression this slice introduced: the check's dev-mode rule is "the
  workspace version must never equal an already-*tagged* version"
  (`release.md`'s own documented tagged-vs-published distinction), and
  `0.167.0` has been tagged (first at `2948008`, now at `cb6f5b6`)
  throughout. Confirmed via `gh run view` on `26c4527`'s ordinary CI run —
  every other step passed; only this one failed, with the message
  `version 0.167.0 is already tagged; bump it`. Not fixed here — out of
  scope for F57, and "fixing" a version-collision check during an active
  release-candidate window is exactly the kind of softening the handoff's
  constraints warn against.
- F44 is knowingly carried, per the handoff's explicit constraint against
  bumping `dioxus-desktop`.
- `READY_TIMEOUT_S = 45` is a judgment call, not a measured worst case —
  the observed CI runs in this slice completed the readiness wait in
  under 4 seconds every time, so 45s is headroom, not a tuned minimum. If
  a future CI runner generation is meaningfully slower, this may need
  revisiting; nothing here claims 45s is exactly right, only that it's
  generous relative to every observed run.

## 10. Requested review focus

1. Whether the readiness condition (§2) — exact row-count match, not just
   landmark presence — is the right level of strictness, or whether a
   looser condition (e.g. "landmark exists AND at least N rows") would
   have been sufficient and more robust to a future fixture change.
2. The three demonstrations (§4) — whether reintroducing F32's literal
   historical defect is convincing evidence of continued detection, or
   whether a different defect shape should also have been exercised.
3. §9's `version-sync` observation — whether that's genuinely expected
   behavior to leave alone until publish/abandon, or whether it should be
   escalated as its own finding.
