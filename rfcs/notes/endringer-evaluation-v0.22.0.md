# endringer — Evaluation Note for RFC-038 Upgrade

**Versions reviewed:** v0.22.0 (2026-06-10), v0.33.0 (2026-06-10, "in-fact-stable")
**Decision:** Accepted as preferred path for a future RFC-038 backend upgrade.
  No code change now. The v0.33.0 note updates the risk picture.

---

## What it is

A workspace of five Rust crates by nabbisen (same author as ForskScope),
built on `gix 0.83`. Provides a `VcsBackend` trait with two implementations:

- `GitBackend` — wraps `gix::ThreadSafeRepository`; no `git` subprocess.
- `JjBackend` — opens jj's underlying git object store with `GitBackend`;
  the `jj` binary is **not** required at runtime.

Facade crate `endringer` exposes a single `Repository` type that is
backend-agnostic. License: Apache-2.0 (matches ForskScope).

---

## v0.33.0 changes vs v0.22.0

The trait grew substantially. New methods added since v0.22.0, all with
default implementations (so existing consumers are unaffected):

| New method | Relevance to ForskScope |
|---|---|
| `operation_state()` | Shows in-progress merge/rebase/cherry-pick — useful for the VCS Changes Panel |
| `unmerged_paths()` | Lists conflict paths — directly useful for RFC-034 three-way merge workflow |
| `conflict_summary()` | Per-stage object IDs for conflicted paths — strong complement to RFC-033 |
| `rich_worktree_status()` | Staged/unstaged/untracked with `Renamed`, `Copied`, `TypeChanged` — better than `worktree_status()` for the VCS panel |
| `diff_entries()` | Rename/copy-aware diff between commits |
| `snapshot()` | Batch read for status-widget efficiency — fits Dioxus reactive refresh |
| `query_commits()` | Bounded paginated history — relevant if history view added |
| `tree_at_commit()` / `tree_at_path()` | Directory listing at a commit — useful for "compare with revision" |
| `blame_at()` | Blame at arbitrary commit |
| `remotes()` / `references()` | Reference inventory |
| `worktree_details()` / `stash_detail()` | Rich worktree/stash metadata |

**Stability improvement:** v0.33.0's stability note explicitly states that
new required methods will always have a default implementation, so adding
them cannot break `VcsBackend` *consumers* using `Repository`. This is a
meaningful improvement over v0.22.0's weaker language.

**Panic count:** 4 in the git implementation (down from 2 in v0.22.0 — same
order of magnitude, all guarded). Not a correctness risk.

**`UnsupportedBackendFeature` error:** New methods that a backend doesn't
implement return this structured error rather than `panic!`. ForskScope's
`VcsProvider` adapter can map it to `None` or a graceful degradation.

---

## Mapping to ForskScope's VcsProvider trait (unchanged)

| ForskScope `VcsProvider` method | endringer equivalent |
|---|---|
| `status()` | `repo.worktree_status()` or `repo.rich_worktree_status()` |
| `read_revision_file(rev, path)` | `repo.file_at_commit(path, commit_id)` |
| `merge_base(left, right)` | `repo.merge_base(a, b)` |
| `root()` | `repo.repository_info().workdir` |
| `system_name()` | `repo.repository_info().backend` → `BackendKind` |

---

## Key advantages over current git-CLI approach (unchanged from v0.22.0)

1. **No subprocess.** All operations are in-process `gix` calls.
2. **JJ for free.** `JjBackend` covers jj without a second provider.
3. **`ThreadSafeRepository` is `Send + Sync`.** Fits Dioxus async tasks.
4. **`file_at_commit` is the right primitive** for "Compare with HEAD".
5. **`conflict_summary` + `unmerged_paths`** directly complement RFC-034
   (conflict workspace) — listing conflicted index paths is now one call.
6. **`operation_state`** tells the UI when a merge/rebase/cherry-pick is in
   progress, enabling the VCS Changes Panel to show meaningful context.

---

## Risks (updated for v0.33.0)

| Risk | Mitigation |
|---|---|
| Pre-1.0 `VcsBackend` trait (new required methods possible) | v0.33.0 guarantees defaults for new required methods; `Repository` consumers are fully protected |
| `gix 0.83` transitive dep | Evaluate lock file for conflicts before adopting; `gix` is widely used and stable in practice |
| 4 `unwrap()` calls in git impl | All guarded; no correctness risk |
| No `detect(path)` walk-upward | ForskScope's `find_git_root` (five lines) stays in `vcs.rs` |

---

## Recommended migration path (unchanged)

1. Add `endringer = "=0.33.0"` to `forskscope-core/Cargo.toml`.
2. Replace `GitProvider::git(args)` subprocess calls with endringer
   `Repository` method calls in `crates/forskscope-core/src/vcs.rs`.
3. Keep `VcsProvider` trait unchanged — stable internal interface.
4. Update `vcs_tests.rs`: replace temp-dir git subprocess setup with
   `endringer::repository::repository(path)`.
5. Add `jj_repository` branch to `detect()` for jj repo detection.

The rest of the codebase (UI, merge, diff) is unaffected.

---

## Additional opportunities in v0.33.0 (beyond RFC-038 baseline)

- **RFC-034 complement:** `unmerged_paths()` + `conflict_summary()` give
  the three-way merge workspace real index conflict data. When a user runs
  `git mergetool`-style, the VCS panel can show which files have index
  conflicts and let the user open them in ForskScope's three-way workspace.
- **`operation_state()`** for UX: show a subtle banner "Merge in progress
  — 3 conflicts remain" in the status bar when HEAD is mid-merge.
- **`snapshot()`** for efficient status refresh: one call returns
  `status_digest + operation_state` in a single roundtrip, ideal for
  polling from a Dioxus async task on tab focus.

These are additive; the RFC-038 VcsProvider migration path itself is
unchanged from the v0.22.0 evaluation.
