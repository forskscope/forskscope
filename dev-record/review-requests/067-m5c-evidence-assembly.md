# Review Request 067: M5-C evidence assembly — Gate D input list, README verdict

**Governing RFC.** [RFC-078](../../rfcs/proposed/078-platform-runtime-acceptance.md), under [RFC-074](../../rfcs/proposed/074-v1-release-stabilization-program.md)
**Governing handoff.** [`rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md`](../../rfcs/handoffs/078-platform-runtime-acceptance/m5c-visual-navigation-and-assembly-handoff.md), §7
**Responds to.** `dev-record/reviews/068-m5c-linux-and-macos-rows-review.md` §8's recommended action items 2–4.
**Scope of this request.** The handoff's last deliverable: the assembled
evidence set and the Gate D input list. All three platform rows (Linux,
Windows, macOS) are already reviewed and approved individually (reviews 066,
067, 068); this request covers only what changed in this assembly pass:
the shared Linux walker hardening, a macOS evidence-doc correction, and
the new/rewritten assembly documents themselves.

## 1. What changed

- **`packaging/evidence/linux_harness.py`** (commit `2d27b2b`) — hardened
  every remaining recursive AT-SPI tree walker against the stale-node
  `GLib.GError` crash class (review 068 §6, Linux Q3: "Yes, pre-emptively" —
  the two crashes already fixed were the same bug surfacing in whichever
  walker ran longest against a mutating tree, not specific to those two).
  Added shared `_name_or_empty`/`_role_name_or_empty`/`_attributes_or_empty`/
  `_child_count_or_zero`/`_child_at_or_none` helpers and applied them to
  `find_by_name_containing`, `find_by_exact_name`, `find_combo_boxes`,
  `find_app_root`, `collect_all_rows`, `find_all_by_name_containing`, and
  the inline `collect()` closures in `explorer_rows_by_pane` and
  `navigate_pane_to`. Re-dispatched P03/P07/P11 to real CI after the change
  (runs [`31989840239`](https://github.com/forskscope/forskscope/actions/runs/31989840239)/[`31989842268`](https://github.com/forskscope/forskscope/actions/runs/31989842268)/[`31989843622`](https://github.com/forskscope/forskscope/actions/runs/31989843622)) —
  all three still pass, confirming no regression. Not applied to three
  single-level lookups (`selected_option_name`'s menu-item access, the
  app-root Explorer-tab click, post-`wait_for_ready` row-cell access) —
  these run on nodes an enclosing retry loop just confirmed present, not
  deep walks over a long-lived subtree, so they're not the same risk class.
- **`docs/src/maintainers/release-evidence/0.167.0/macos-aarch64.md`**
  (commit `c73b5fb`) — moved the VoiceOver-latency caveat from F63's now-
  closed resolution section to Finding 3 (the right-pane click-technique
  limitation), per review 068 §3/§8.4, and added a parallel caveat there:
  neither the bulk-fetch technique nor the synthesized-click technique can
  currently be distinguished from a genuine accessibility gap a real
  VoiceOver user would also hit.
- **`ROADMAP.md`** — F63's entry rewritten to record its resolution
  (harness artifact, not a product defect) instead of leaving it open,
  carrying the VoiceOver caveat forward as a pointer to Finding 3 rather
  than restating it.
- **`docs/src/maintainers/release-evidence/0.167.0/gate-d-input-list.md`**
  (new) — every input bearing on the Gate D decision in one place, per
  handoff §7's required table shape. Three un-waivable blockers (F44, F61,
  F73) separated from six other open inputs (F45, F46, F60, F69, F70, F72)
  plus the keyboard-coverage gap and `linux-wayland`; F63 listed only for
  completeness against the handoff's own template, marked resolved and
  carrying no weight. A "Considered and not included" section records
  F67/F68/F66/F71/F65 as deliberately excluded (with the reasoning each
  time), not overlooked — F68 specifically not double-counted against F73
  since review 068 established they share one root cause and one fix.
- **`docs/src/maintainers/release-evidence/0.167.0/README.md`** — title and
  slice description updated for M5-C; verdict section rewritten from two
  blockers to three (F73 added, per review 068 §4's un-waivability finding)
  and re-inverted per review 063 §5.1 (blocking facts first, still); rows
  table gains an M5-C cases column; Harnesses section corrected to state
  Windows's P07 `--break` (never reached, blocked by F70 before any
  `--break`-gated assertion) and P11 `--break` (reaches its assertion but
  fails for the same real-defect reason as normal mode, not its own
  impossible-value branch) as two distinct, separately-caused limitations,
  not one; Known limitations section replaces stale "M5-C not started"
  language with what M5-C actually surfaced.
- **`artifacts.md`** — unchanged; already complete since M5-A (three
  published assets, digests, source commit), reconfirmed rather than
  edited.

## 2. A correction caught while writing this request

My first draft of the README's Harnesses section claimed Windows's P07
`--break` was the case that "cannot currently demonstrate falsifiability in
isolation" — checked against `windows-11.md` directly before committing and
found that's actually **P11's** limitation (`--break` reaches its assertion
but fails for the same real-defect reason, per review 067's own §5.4);
**P07's** `--break` is a different, simpler thing — "not reached at all,"
blocked by F70 before any `--break`-gated assertion exists to fail.
Corrected before the commit landed, not after — flagging it here since it's
exactly the kind of cross-document detail this assembly pass exists to get
right.

## 3. Gates

No Rust source touched. `python3 -m py_compile` on `linux_harness.py`;
`mdbook build docs` ran clean after every doc change; `git diff --check`
clean before each commit; full `CI` workflow (fmt, clippy, test, i18n,
actionlint) ran green on the final assembly commit,
[`b3c224b`](https://github.com/forskscope/forskscope/actions/runs/31990217908).

## 4. Unresolved issues

- **The Gate D input list and README verdict are both new documents this
  pass; neither has been reviewed yet** — this request is exactly that
  review.
- **F68 and F73's shared fix (pass `left_root`/`right_root` into
  `DeepRow`) is not attempted here**, per the handoff's explicit "no
  product behaviour changes" constraint and review 068's own framing of it
  as a separate, subsequent piece of work ("needs a new candidate").
- **The Linux scroll-mirror fallback chain's cross-CI-image stability is
  still unknown** — recorded as a known limitation, not resolved.

## 5. Requested review focus

1. **Is the Gate D input list's shape and content complete** — everything
   that should bear on the decision is present, nothing that shouldn't is,
   and the "Considered and not included" section's exclusions are
   defensible?
2. **Does the README verdict's three-blocker framing hold together** with
   the Gate D input list as the linked source of full detail, or does the
   split between the two documents risk drift over time (e.g., a future
   input-list update that doesn't get mirrored in the README summary)?
3. **Is this the correct point to consider the M5-C handoff complete** —
   handoff §10 anticipated this ("What remains for Gate D is the owner's
   two manual passes and the go/no-go itself") — or is there assembly work
   still outstanding that this request missed?
