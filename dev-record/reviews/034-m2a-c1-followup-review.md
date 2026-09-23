# RFC-074 M2-A — C1 follow-up review

**Review date:** 2026-08-02
**Request:** `dev-record/review-requests/030-m2a-c1-followup.md`
**Baseline:** `fe9940e` (`release: fail-closed on empty CHANGELOG content, not byte length (C1)`)
**Responds to:** review 033, mandatory correction C1
**Review mode:** Independent verification against the repository and the GitHub
API; no implementation changes made.

## 1. Verdict

**Approved.** C1 is closed.

The correction is exactly one line, in exactly the right place, and the failure
mode it was meant to remove is demonstrably gone. With C1 closed, **M2-A as a
whole is Approved** — review 033's conditional approval had this as its only
mandatory item.

One non-blocking improvement is registered (§3). N1 from review 033 remains open
and unchanged, tracked as F23.

The slice still changes no product behaviour, cuts no release, and closes no
audit blocker. B2, B3, and B4 remain open; v1/public release stays **No-Go**.

## 2. Verification

The change is a single line, with no collateral edits:

```diff
-test -s release-notes.md || { echo "::error::no CHANGELOG section for ${GITHUB_REF_NAME}"; exit 1; }
+grep -q '[^[:space:]]' release-notes.md || { echo "::error::CHANGELOG section for ${GITHUB_REF_NAME} is missing or empty"; exit 1; }
```

`git diff --stat HEAD~1 HEAD` confirms `release.yml` at `2 +-`. The other two
files in the commit are the architect's own amendments (ROADMAP F23 registration
and the handoff §4.1/§5 rewrite), correctly attributed as not authored by this
patch.

All extraction cases re-run independently against the committed CHANGELOG with
the committed guard:

| Version | Bytes | Guard |
|---|---|---|
| `0.165.0` — real section with content | 5850 | PASS |
| `0.165.1` — heading-only; **C1's reproduction case** | 1 | FAIL-CLOSED |
| `9.9.9` — no matching heading | 0 | FAIL-CLOSED |
| `0X165X0` — wildcard-shaped adversarial input | 0 | FAIL-CLOSED |

The second row is the one that matters: under `test -s` it passed at one byte
and would have composed a blank line plus the compare link. It now fails closed.

CI run `30741678224` — `success` on `fe9940e4`. Gates re-run here:
`cargo fmt --check`, `cargo xtask version-sync` (`v0.165.1`),
`cargo test -p forskscope-core -p forskscope-ui-logic` (8 suites, 943), and
`cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings` all pass.

### Note on the case count

The request says "all five required extraction cases" above four blocks. That is
correct, not an omission: the handoff's case 2 (the current in-development
section) and its amended case 5 (a heading-only section) are the *same
invocation* in this repository, because `0.165.1` is currently heading-only. The
request labels it as C1's reproduction case, which is the right handling.

The inconsistency is in the handoff, not the work — my amendment left case 2
worded as though `0.165.1` should yield content while case 5 requires it to
fail. The handoff §5 wording is being corrected so a future reader does not
inherit a contradictory case list.

## 3. Non-blocking finding

### N1 — The guard is correct but fires in the last job of six

This answers the request's review question directly: yes, one thing in the
notes-composition step is worth re-examining, and it is placement rather than
logic.

The release workflow's dependency graph is:

```text
preflight (Release gates)
  └── source
        ├── linux
        ├── macos
        └── windows
              └── release (Create GitHub Release)  ← the guard lives here
```

An empty CHANGELOG section is detectable from the repository alone, at
preflight, in about four minutes. As positioned, it is instead detected after
the source archive plus all three platform builds have completed — roughly
twelve to fifteen minutes of the observed R0 timings — and by then the tag
already exists at a commit with an empty section, so recovery requires a tag
re-cut under the publication policy rather than a simple re-run.

**Recommended change:** extend `cargo xtask version-sync`'s **release mode**
(`expected_version` present) to require that the CHANGELOG section for that
version contains non-whitespace content. The preflight job already calls
`cargo xtask version-sync "${GITHUB_REF_NAME}"`, so this fails fast before any
artifact is built.

The mode restriction is essential and must not be skipped: **dev mode (no
argument) must keep accepting an empty section**, because the post-release bump
deliberately opens one and `ci.yml` runs `version-sync` on every push. Requiring
content unconditionally would break CI immediately after every release.

This is pleasingly symmetric with the existing design — the published-tag check
is dev-mode only, this would be release-mode only — and it slots naturally
beside the existing `## [{version}]` heading assertion, which currently runs in
both modes and checks presence only.

Keep the job-level `grep -q` guard as well. Defence in depth is warranted at the
point of use, and it costs nothing.

**Registered as F24 against M4**, alongside F18 and F23 as the release-tooling
cluster. Deliberately not folded into M2-B: that milestone rewrites the
production persistence path, and entangling release-mechanics work with it would
repeat the mistake M2-A was created to avoid. If M2-B's CHANGELOG section is
written as work lands — which the post-release open-section practice now
encourages — the empty case will not arise at that cut anyway. This is insurance,
not a prerequisite.

## 4. Carried forward unchanged

- **F23 / review 033 N1** — `release.yml`'s YAML validity is still unproven. No
  parser was available to the implementer or to this review. The first thing
  that exercises the file is M2-B's cut, where a parse error means no release.
  `actionlint` in CI closes it structurally.
- End-to-end evidence for the whole notes-composition change still arrives at
  M2-B's cut. Nothing in M2-A has executed against a real tag.

## 5. Recommended next action

1. Treat C1 as closed and M2-A as Approved.
2. Begin M2-B (RFC-076) against
   `rfcs/handoffs/076-versioned-runtime-persistence/implementation-handoff.md`.
   Its design-review pause after the first patch still applies, and its
   persistence adapters must never install legacy persisted IDs as runtime
   `CompareTabId` values.
3. Before M2-B's release cut, settle F23. F24 is optional but cheap.
