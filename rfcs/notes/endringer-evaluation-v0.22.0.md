# endringer v0.22.0 — Evaluation Note for RFC-038 Upgrade

**Date evaluated:** 2026-06-10
**Version reviewed:** 0.22.0 (source archive supplied by maintainer)
**Decision:** Accepted as preferred path for a future RFC-038 backend upgrade.
  Not adopted in v0.55.0. No implementation action required now.

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

## Mapping to ForskScope's VcsProvider trait

| ForskScope `VcsProvider` method | endringer equivalent |
|---|---|
| `status() → Vec<VcsFileStatus>` | `repo.worktree_status() → WorktreeStatus` (staged + unstaged + untracked, gitignore applied) |
| `read_revision_file(rev, path) → Vec<u8>` | `repo.file_at_commit(path, commit_id) → Vec<u8>` (direct object-DB lookup) |
| `merge_base(left, right) → Option<VcsRevision>` | `repo.merge_base(a, b) → Option<CommitId>` |
| `root() → &Path` | `repo.repository_info().workdir` |
| `system_name() → &str` | `repo.backend_kind()` → `BackendKind::Git` or `BackendKind::Jj` |

`worktree_status` is richer than the current `git status --porcelain` parse:
it distinguishes staged from unstaged changes, which the VCS Changes Panel
(RFC-038 §"VCS Changes Panel") needs.

---

## Key advantages over current git-CLI approach

1. **No subprocess.** All operations are in-process `gix` calls. No fork/exec
   overhead on every status check or tab-switch.
2. **JJ for free.** `JjBackend` delegates to `GitBackend` on jj's git store.
   ForskScope gains jj support without a second provider implementation.
3. **`ThreadSafeRepository` is `Send + Sync`.** Fits naturally into Dioxus 0.7
   async task spawning for background VCS status refresh.
4. **`file_at_commit` is the right primitive.** Direct object-DB tree walk —
   no round-trip through stdout parsing.

---

## Risks

| Risk | Mitigation |
|---|---|
| Pre-1.0 `VcsBackend` trait: new required methods may be added | ForskScope uses its own `VcsProvider` trait as adapter; only the adapter sees endringer directly |
| `gix 0.83` is a new transitive dep (~1 MiB compiled) | Evaluate lock file for conflicts before adopting |
| Rapid versioning (v0.22.0 suggests early phase) | Pin `endringer = "=0.22.0"` until 1.0; review on each ForskScope release |
| Two `unwrap()` calls in `branch.rs` / `status.rs` | Both are guarded by explicit checks above; not a correctness risk |
| No `detect(path)` that walks upward | ForskScope's existing `find_git_root` (five lines) stays in `vcs.rs` |

---

## Recommended migration path (when ready)

1. Add `endringer = "=0.22.0"` to `forskscope-core/Cargo.toml`.
2. Replace `GitProvider::git(args)` subprocess calls with endringer
   `Repository` method calls inside `crates/forskscope-core/src/vcs.rs`.
3. Keep `VcsProvider` trait unchanged — it is the stable internal interface.
4. Update `vcs_tests.rs`: replace temp-dir `git init` + subprocess setup
   with `endringer::repository::repository(path)` calls.
5. Add `jj_repository` branch to `detect()` for jj repo detection.

The rest of the codebase (UI, merge, diff) is unaffected because it talks to
`Box<dyn VcsProvider>`, not to endringer.

---

## Not in scope for now

- File manager launch (RFC-029) — separate from VCS.
- Commit history browser — RFC non-goal (NG-001: not a Git GUI).
- Write operations — VcsBackend has tag write methods; ForskScope's
  `VcsProvider` trait intentionally exposes no write operations.
