# Handoff 040 — F111 and F112: make the ignore patterns actually work

**From:** architect. **Date:** 2026-09-24. **Release:** 0.172.0.
**Register:** F111, F112. **Origin:** user issue #145 (macOS, 0.170.1).

A user could not exclude `.git` and `.jekyll-cache` from a folder comparison.
They tried six syntaxes. **Their first one was correct** — the feature is half
built, and the half they needed does not exist.

**Interaction with handoff 039 (RFC-080 tier 1):** tier 1 folds a verdict over
the same recursive walk this handoff teaches to filter. Without this, any two
git checkouts report *Different* on `.git` alone, which is the noise the
feature exists to cut through. **Whoever lands second rebases.** If you are
doing both, do this one first.

## A. F112 — a settings change never reaches a running Explorer

**Reproduce it first, and say in the reply what you saw.** I established this
by reading, not by running:

- `explorer.rs:241` re-reads the rules every render and builds a fresh
  `FilteringExecutor` (`:264`, `:297`).
- `use_scan_driver` hands it to `use_coroutine` → `use_future` → `use_hook`
  (`dioxus-hooks-0.7.9/src/use_future.rs:61`), whose init closure runs **once
  per mount**. It keeps the executor it captured then; every later one is
  built and dropped unread.
- Navigation does not help: it posts a scan request to that same coroutine.
- The Explorer unmounts only when another tab replaces it
  (`app.rs:196-205`), so leaving the tab and returning, or restarting, is what
  makes a new setting take effect.

**The fix, two parts — both are needed, and the second is the one that is easy
to forget:**

1. **The scan path must read the current rules.** `FilteringExecutor` already
   clones `self.rules` inside `spawn_blocking` (`dir_pane.rs:99`), so give it a
   shared handle whose contents the Explorer updates when the settings change,
   rather than a snapshot taken at mount. Keep it `Send + Sync`; the closure
   crosses a thread boundary. Do not change `use_scan_driver`'s signature —
   it belongs to `dioxus-swdir-tree`.
2. **Already-loaded rows must be re-scanned.** Even with current rules, a tree
   that is already expanded shows what it loaded under the old ones. Rebuild
   the tree the way the root-change effect does (`explorer.rs:270-280`), so the
   change is visible without navigating. Cover both panes.

## B. F111 — the ignore list never reaches directory comparison

`IgnoreRules` has exactly one consumer in the whole product:
`FilteringExecutor` (`dir_pane.rs:92-110`), filtering the Explorer panes.

`rfcs/done/056-ignore-patterns-for-files-and-directories.md` is marked
*Implemented* and promises three surfaces in its § *Where ignore rules apply*:
Explorer listing, digest equality, and **"Recursive compare — ignored entries
are not walked or reported."** Only the first shipped.

1. **Teach the recursive walk to filter.** `list_recursive_for_display_with_cancel`
   and `recursive_diff_with_cancel` (`core/src/dir/recursive.rs:114`, `:143`)
   take no rules.
   - An ignored **directory is not descended into** and does not appear.
   - An ignored **file is not reported**.
   - Both sides filter by the same rules, so an ignored entry present on one
     side only does not become a one-sided difference.
   - **Do not break the existing signatures silently.** Either add parameters
     and update callers, or add new entry points and make the old ones
     delegate with empty rules. Say which you chose and why.
2. **Deep Compare must pass the current rules** (`deep_compare.rs`), read the
   same way the Explorer reads them, so A's fix covers this view too.
3. **Out of scope:** `dir_digest_equal` (unused by any view) and RFC-056's
   "digest equality" row. Say in the reply that it remains unimplemented; I
   will decide separately whether it is worth building or whether the RFC
   should be corrected.

## Verification

Each check shown failing first, against a deliberately broken input, and not
against a helper this change introduces. Real temporary trees, following
`temp_dir(tag)` in `explorer.rs:592-770`.

1. **The user's case, end to end.** Two trees that differ **only** inside
   `.git`, with `.git` ignored: the folder comparison reports no differences
   and lists no `.git` entry. Without the ignore rule, the same pair reports
   the differences. This is the check that would have caught #145.
2. **An ignored directory is not walked**, not merely hidden afterwards.
   Prove the walk did not descend — for example, make the ignored subtree
   unreadable, and show that the scan neither fails nor flags it.
3. **An ignored file is not reported**, on either side, including when it
   exists on one side only.
4. **A settings change takes effect with no restart and no navigation**
   (F112): with the Explorer open and a tree expanded, adding `.git` removes it
   from both panes.
5. **Nothing else changes when the rules are empty** — the default. Existing
   Explorer and Deep Compare tests must pass unchanged.
6. **The Japanese and English settings labels are untouched**; this handoff
   changes behaviour, not wording.

## Documentation

`docs/src/users/settings.md:97-120` currently says these filters "apply to the
Explorer tree". After this change they apply to folder comparison as well.
Update it, and state plainly that opening a specific file always compares it,
ignore list or not — that rule is unchanged.

## Scope

- **In:** `core/src/dir/recursive.rs`, `dir_pane.rs`'s `FilteringExecutor`,
  `explorer.rs`'s wiring, `deep_compare.rs`'s call, `settings.md`, tests.
- **Out:** the ignore syntax itself (it is fine — the user's first attempt was
  valid), the Settings UI's wording and layout, `CHANGELOG.md` and
  `ROADMAP.md`, handoffs 038 and 039's subject matter, RFC-056's own text.

## Gates

The standard set, including the Windows-target clippy, `cargo xtask` with
`css --check`, `i18n`, `ui-logic-connectivity`, `ui-logic-docs`, `rfc-sync`,
`audit-deps`, `version-sync`, `mdbook build docs`, and a green CI run with its
run ID.

Reply in `dev-record/review-requests/112-f111-f112-ignore-patterns.md`.
