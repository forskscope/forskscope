# M4-C2 (§5–10) review — source archive, dead layers, i18n scope, versioning, presets, RFC claims

**Review date:** 2026-08-13
**Request:** `dev-record/review-requests/055-m4c2-source-archive-dead-layers-i18n-gate-batch-versioning-preset-divergence-rfc-claims.md`
**Baseline:** `7fd6b66`
**Governing document:** `rfcs/handoffs/074-v1-release-stabilization-program/m4c2-documentation-and-code-truth-handoff.md` §5–10
**Review mode:** Independent verification. No implementation changes made.

## 1. Verdict

**Approved**, with two minor findings (§3) and one architectural observation
that is bigger than any single item in this slice (§4).

Every delegated decision is correct and, more importantly, *traced before being
made* rather than defaulted to. F31 and F25 in particular were decided by
following the actual read path and the actual call graph, which is the
difference between a decision and a preference.

B4 remains open; v1/public release stays **No-Go**. `main` is still red on F50 —
that slice comes first regardless of this approval.

## 2. Verified

| Check | Result |
|---|---|
| `cargo fmt --check`, `clippy --workspace --all-targets -D warnings` | Pass |
| `cargo test --workspace` | Pass — 1094 |
| `cargo xtask css --check`, `mdbook build docs` | Pass |
| Release job graph after removing `source` | Clean — `linux`/`macos`/`windows` on `needs: preflight`, `release` on `needs: [linux, macos, windows]` |
| No dangling `needs.source` / `source.outputs` | Confirmed |
| `PKGBUILD` source URL and extraction dir | Correct — GitHub tag tarball, `cd "$pkgname-$pkgver"` matches its `forskscope-<ver>/` layout |
| **F51** — `conflict_nav` unreferenced in `forskscope-ui` | Confirmed — zero references |
| **F52** — `AppError`/`AppErrorKind`/`SaveErrorView` unreferenced in `forskscope-ui` | Confirmed — zero references |
| F16 spot-check — compare profiles reachable | Confirmed — `ui/view/settings/modal.rs:210` |

The test-count drop to 1094 is fully explained by F48's removals and is not a
regression. Worth stating because it spans two review requests and a bare
comparison against M4-B's 1112 looks alarming.

## 3. Findings

### N1 — the release asset list still carries the source-archive glob

`.github/workflows/release.yml`'s `Create release` step:

```yaml
files: |
  forskscope-v*.tar.gz              # ← the removed source archive
  forskscope-v*-linux-x86_64.tar.gz
  forskscope-v*-macos-aarch64.dmg
  forskscope-v*-windows-x64.zip
```

Not broken — `forskscope-v*.tar.gz` also matches the Linux tarball, so the glob
resolves and the release still assembles. But it is now a **redundant superset**
of the line below it, left over from the job you removed, in the file you
restructured. It will silently attach anything matching that pattern in future.

Delete the first line. Your sweep for stray references was genuinely thorough —
you found three in docs I would have missed — which makes it worth noting the
one that survived was inside the workflow itself.

### N2 — `sha256sums=('SKIP')` costs more now than it did before, and that is partly my error

`PKGBUILD` previously took a **local file** the maintainer placed by hand;
`SKIP` was defensible because there was no network and no remote to distrust.
It now fetches over the network:

```sh
source=("$pkgname-$pkgver.tar.gz::https://github.com/.../archive/refs/tags/$pkgver.tar.gz")
sha256sums=('SKIP')
```

That is the case where GitHub's tarball instability actually bites — the very
argument I used to justify *keeping* a custom archive, and then dropped when
recommending removal. The recommendation was still right; the checksum
consequence was mine to flag and I did not.

Not a blocker and not this slice's fault. Record it: either a real `sha256sums`
value refreshed per release (`updpkgsums`), or an explicit note in `PKGBUILD`
saying why `SKIP` is accepted here. The second is legitimate for a PKGBUILD
users build themselves; leaving it unstated is not.

## 4. The observation that matters more than any item here

This slice found **F51, F52 and F53**, and it already contained F48 and F25.
That is five instances of one shape:

| Layer | Built and tested | Reached by `forskscope-ui` |
|---|---|---|
| RFC-024 decoration contract (F48) | yes | no |
| RFC-034 ConflictNavigator (F51) | yes | no |
| RFC-017 `AppError`/`SaveErrorView` (F52) | yes | no |
| `settings_view` font helpers (F53) | yes | no |
| `CompareProfile::all_presets()` (F25) | yes | no |

These were found one at a time, each while investigating something else. They
are not five coincidences — they are a structural fact: **this project has a
habit of completing a core or ui-logic layer, testing it thoroughly, marking the
RFC "core complete," and never wiring it to the renderer.** Each RFC then reads
as shipped when the user-visible half does not exist.

That is the same disease as the gate findings, one level up: something is
credited with more than it delivers, and no check exists that would notice.

I am registering this as **F54** — not as a request to fix the five, but to
answer the question they raise together: *what stops the sixth?* An RFC status
convention that distinguishes "core complete" from "user-reachable," or a check
that flags a `pub` ui-logic view-model with no `forskscope-ui` consumer, would
both work. Deciding that is M4-C3 or post-v1 scope, not yours today.

**You surfaced this by doing the work carefully three times in one slice.** That
is worth more than the individual corrections.

## 5. Answers to the requested review focus

### 5.1 The delete/narrow posture — right, and F52 is the one to pull forward

Keep the posture. Deleting an unreached layer is almost always better than
wiring it through to justify its existence, and you applied the handoff's steer
("prefer (2) unless you can name what (1) buys") honestly rather than as
license.

Of the three, **F52 is the one that should not wait long**, for the reason you
identified: it is the actual fix for F39's bypass sites, and unlike F51 and F53
it has a *user-visible* cost today — every save/IO failure reaches the user as
raw `CoreError` `Display` text, in English, regardless of locale. That is error
quality, not tidiness.

But not in M4. M4 is closing; F52 is a UI workstream with real design in it.
Post-v1 or M4-C3 if that slice materialises — and F39's narrowed G-006 wording
should point at F52 so the next reader learns why the gate is narrow rather than
only that it is.

F51 and F53 are genuinely deletable and can wait for whoever next touches those
files.

### 5.2 F31's reasoning generalises — and there is at least one more export

The rule you derived is right and worth stating as policy: **a schema version
protects a read path; where none exists, it protects nothing.** Tracing
`restore_from_manifest` to an in-memory `BatchManifest` rather than a parsed
file is exactly the check that distinguishes this from cargo-culting
versioning.

Applied to the rest of the codebase, the other write-only export is **patch
export** — `patch::to_unified` / `patch_from_directories` produce `.patch` files
users keep and feed to `patch -p1` or `git apply`. Same shape: no reader in this
codebase, an external consumer with its own stable format. It needs no schema
envelope for the same reason, and unified diff is a fixed external format
anyway, so the argument is even stronger there.

So: no change needed, but say so once in the module docs rather than leaving the
next auditor to re-derive it. Cheap now that the reasoning exists.

### 5.3 F16 — "all accurate" is plausible here, and your method is why

A first systematic audit turning up almost nothing would normally make me
suspicious. Two things make it credible:

- **You audited against `crates/forskscope-ui/src/`, not core or ui-logic** —
  the correct axis, and the one that would catch exactly the failure this slice
  kept finding. Auditing against core would have "confirmed" F25's preset
  claim, F48's decorations, and F53's font ranges, all wrongly.
- The README was already conservative, and the three-way bullet already carried
  the qualifier the handoff asked others to match.

I spot-checked the claim most likely to be core-backed-but-UI-unreached —
compare profiles, given F25 — and it is genuinely reachable in the Settings
modal. That is the check I would have expected to break the result, and it
holds.

No second pass warranted. Record the method in the F16 entry, though: *audited
against the UI crate specifically, because core-complete does not imply
user-reachable*. That sentence is what makes the audit repeatable, and it is
F54's lesson in one line.

## 6. Notable quality observations

- Deleting the whole `source` job rather than just its archive steps, after
  establishing that `GITHUB_REF_NAME` made its `outputs.version` indirection
  unnecessary. That is the difference between removing a feature and removing
  its scaffolding too.
- Tracing *why* two preset sets exist (RFC-028's selector deferred post-v1)
  instead of treating divergence as drift to be merged.
- Refusing to translate `describe_block`'s two literal arms in isolation,
  because a partial fix with an unchanged gate is the specific failure mode the
  handoff named. Declining to do the easy half was the right call.
- Verifying RFC-062's four acceptance criteria against the implementing commit
  rather than trusting `rfcs/README.md`'s existing "Shipped" annotation — and
  resolving the RFC's own open question against what actually shipped.
- Leaving `CHANGELOG.md`, past reviews and the F24 comment as historical record
  while updating only the current-state claims.

## 7. Recommended next action

1. **F50's dedicated slice first** — `main` is red; nothing else matters until
   `cargo audit` is green.
2. **N1 and N2** (§3) — two lines; fold into F50's slice or the next one.
3. §5.2's one-line note on patch export; §5.3's method sentence in F16.
4. **F54** is registered for M4-C3/post-v1 — the systemic question, not the five
   instances.
5. Owner: the Windows floor and the stale `MaxVersionTested` (review 057 §4.2).
