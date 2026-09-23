# Review Request 104 — 0.171.0 follow-ups: stderr pipe, comment-blind gate, doc-table check

Handoff: `dev-record/handoffs/034-0171-review-follow-ups.md`
Four commits, each landing exactly one part, all pushed to `main`, all CI
green:

| Part | Commit | CI run |
|---|---|---|
| A — F102 stderr-to-file | `5fc777f` | `34937135323` |
| B — F54 comment/string stripping | `f6f5089` | `34938115594` |
| C + D — F93 doc-table check + edits | `dd07037` | `34938546382` |

## A. F102: app stderr goes to a file (review 105 §2)

`launch_until_registered` now opens a per-attempt file
(`scratch/forskscope-stderr-{attempt}.log`) and passes that handle as
`stderr=`, closing it immediately after `Popen` returns — the child keeps
writing through its own duplicated descriptor. `tail_lines()` reads the
last 20 lines on demand, from every failure path that follows a
successful launch now, not only the crash-retry path.

### A1 — a crash that leaves a child holding stderr

Stub: `sleep 120 &` (a lingering child that inherits the stderr fd), one
line to stderr, `exit 7`.

**Before the fix** (`9540254`'s code, run from `packaging/`, bounded to
50s): zero output, `timeout` killed it.
```
timeout 50 env NO_AT_BRIDGE=0 python3 packaging/render_check_old_TEMP.py   0.07s user 0.01s system 0% cpu 50.002 total
EXIT: 124
```
`find_app`'s own 30s budget had already elapsed; the process had already
exited; the script was still silent — `proc.stderr.read()` blocked on the
lingering `sleep 120` child, exactly as review 105 §2b described.

**After the fix** (bounded to 40s): the attempt line and its stderr tail
appeared well inside the window, right after `find_app`'s ~30s:
```
attempt 1/3: forskscope exited during start-up (exit code 7) before registering on the accessibility bus
  stderr tail:
stub: crash with lingering child
timeout 40 env NO_AT_BRIDGE=0 python3 packaging/render_check.py   0.07s user 0.01s system 0% cpu 40.002 total
EXIT: 124
```
(The second `timeout` kill at 40s is expected — the stub always exits 7,
so attempt 2 was mid-`find_app` when the bound hit. Attempt 1's own timing
is the evidence.)

### A2 — the success path under heavy stderr

Ran the real binary directly with `G_MESSAGES_DEBUG=all` for 20s (the
same fixture args `render_check.py` uses), stderr to a file, measured
twice independently:
```
6951 bytes
6953 bytes
```
**Does not exceed 64 KiB** — under 7 KiB both times. Reported honestly, as
asked; no before/after comparison was manufactured for this one, since
there's nothing to demonstrate a difference on.

### A3 — real CI

Plain dispatch, run `34937579216`: `OK: 7 left rows + 7 right rows all
aligned within their pane.` — passed first attempt, no `attempt` lines.

Geometry-defect dispatch, run `34937588375`: failed, no `attempt` lines
(single launch, correctly not retried), and now carries the app's stderr
tail beneath the `FAIL`:
```
FAIL: F34 rendering check found misalignment:
  - left pane: a row's content starts at x=46, other rows start at x=26 - a column shift, the visual symptom of F32
  - right pane: a row's content starts at x=661, other rows start at x=641 - a column shift, the visual symptom of F32
  stderr tail:
libEGL warning: DRI3 error: Could not get DRI3 device
libEGL warning: Ensure your X server supports DRI3 to get accelerated rendering
```

`find_app` and `STARTUP_MAX_ATTEMPTS` untouched, as instructed.

## B. F54: the gate no longer counts comments or strings (review 106 §3)

`strip_comments_and_strings`, hand-written, runs over `lib.rs` and every
`forskscope-ui` source file before `extract_root_exports`,
`contains_glob_import`, and `contains_word` ever see them. Removes line
comments (`//`, `///`, `//!`), nested block comments, and string literals
(normal and raw, any hash count), each replaced with a single space. A
char literal is told apart from a lifetime by whether a `'` follows
within a few characters — a lifetime never has one in valid Rust.

**Your exact two-step probe, reproduced against the fix:**
```
$ cargo xtask ui-logic-connectivity   # with f54_probe_symbol planted, comment added
ui-logic-connectivity check failed: these forskscope-ui-logic crate-root exports have no consumer in forskscope-ui (F54/F75):
  - f54_probe_symbol
```
Where `b55acc6` passed. Both files (`lib.rs`, `app.rs`) restored
immediately after — confirmed via `git diff` showing no residual change.
The real tree still passes with no allowlist:
```
ui-logic connectivity check passed: 33 crate-root exports all have a consumer in forskscope-ui.
```

New unit tests cover each comment form, normal and raw strings (including
an escaped quote inside a normal string, and a `"#` sequence inside a
3-hash raw string that must not be mistaken for the closer), a lifetime
next to an identifier (left completely untouched), a char literal next to
a following lifetime (only the literal stripped), and the exact
comment-only-mention scenario from your falsification.

## C + D. F93: doc tables tied to disk, and rebuilt (review 106 §4)

`cargo xtask ui-logic-docs` walks `crates/forskscope-ui-logic/src` for
leaf modules (every `.rs` file except `lib.rs`, parent module files, and
`tests.rs`), and checks both `architecture.md`'s `` `ui-logic` modules
(N) `` table and `testing.md`'s `` `forskscope-ui-logic` test modules ``
table against that set — module names in both directions, plus
`architecture.md`'s heading count. No row content is parsed, per your
explicit "the check concerns which modules are listed" — only D's edits
touch wording.

**A real bug, found before this ever ran against the tree**: the first
"parent module file" heuristic (a same-named sibling directory exists)
misclassified `compare/load_identity.rs` — it has a sibling
`compare/load_identity/` directory, but that directory holds only an
external `tests.rs`, not a submodule tree. The check reported 10 modules
and claimed `load_identity` "does not exist on disk," which is false.
Fixed by requiring the sibling directory to hold a *non-test* `.rs` file
before treating the outer file as a parent — confirmed against the real
tree afterward: 11 modules, `load_identity` correctly present.

**Falsifications, each run separately against the corrected docs, each
reverted, verified byte-identical to a pre-falsification backup afterward:**

Deleted a row (`architecture.md`'s `compare::load_guard`):
```
ui-logic-docs check failed:
  - architecture.md is missing module `compare/load_guard`
```
Added a fictitious row (`testing.md`):
```
ui-logic-docs check failed:
  - testing.md lists `compare/nonexistent_module`, which does not exist on disk
```
Changed the heading count (11 → 12):
```
ui-logic-docs check failed:
  - architecture.md's `## `ui-logic` modules (12)` heading says 12, but there are 11 ui-logic leaf modules on disk
```

**The document edits (D):** `architecture.md`'s heading corrected to
`(11)`; `compare::conflict_nav_view`/`compare::palette_view` rows removed;
`explore::status`'s row no longer names the deleted `StatusRow`;
`settings::settings_view`'s row now names only `theme_choices` and
`clamp_font_size`. `testing.md`'s table rebuilt from the disk set:
`command_bar`/`hunk_decorations`/`scroll_sync`/`summary`/`tab_state`
(deleted long before this handoff, never removed) and
`conflict_nav_view`/`palette_view` (deleted by handoff 033) are gone;
`compare/load_identity`, `compare/startup`,
`session/persistence_recovery`, `settings/persistence_recovery` are added
with real *Covers* text describing their actual tests (read from each
file directly, not guessed).

Two comments corrected: `forskscope-core/src/diff/options.rs`'s
`all_presets` doc now says it has no non-test caller (its only bridge,
`ui-logic::settings_view::profile_presets`, was deleted by handoff 033) —
`all_presets` itself is untouched, per the explicit instruction not to
delete it, F25's decision standing regardless of current callers.
`forskscope-ui-logic/src/lib.rs`'s parenthetical no longer attributes
RFC-028's toolbar profile picker to `conflict_nav_view`/`palette_view`
(RFC-034 and RFC-019 respectively) — the profile picker was
`settings_view`'s `ProfileChoice`/`profile_presets`.

## Scope

Held to exactly what handoff 034 named. `find_app`, `release.yml`,
`ROADMAP.md`, `CHANGELOG.md`, RFC-080, `all_presets` itself, and
`forskscope-ui` behavior were not touched.

## Gates

Run separately for each of the three commits, all green: `cargo fmt
--check` (workspace and `xtask/Cargo.toml`), `cargo clippy --workspace
--all-targets -- -D warnings` (workspace and `xtask`), `cargo test
--workspace` (736/29/16/5/133/133/152/5/6 — unchanged by these commits
except B and C's own new xtask tests), `cargo test --manifest-path
xtask/Cargo.toml` (40 tests after both new checks landed), `cargo xtask
css/version-sync/i18n/rfc-sync/audit-deps/ui-logic-connectivity/ui-logic-docs`,
`git diff --check`, `mdbook build docs`, `python3 -m py_compile
packaging/render_check.py`, `actionlint`. CI runs `34937135323` (A),
`34938115594` (B), `34938546382` (C+D) all confirmed green, including
both new `ui-logic connectivity check` and `ui-logic docs check` steps
running for real in the C+D run.
