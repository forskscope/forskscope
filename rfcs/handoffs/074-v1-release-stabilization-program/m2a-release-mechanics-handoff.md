# RFC-074 M2-A Developer Handoff: Release Mechanics

**Governing RFC.** [RFC-074](../../proposed/074-v1-release-stabilization-program.md)
**Milestone.** M2-A — release-mechanics slice, ahead of M2-B (RFC-076)
**Register items.** F19, F20, F21, F22, plus review-032 findings N5 and N6

This handoff directs execution of one slice. It does not redefine RFC-074. If
implementation evidence contradicts a decision below, amend RFC-074 first, then
update this handoff to match.

## 1. Summary

Three consecutive defects have now been found in release mechanics, and all
three share one shape: something was configured plausibly, never exercised, and
credited as working.

- The release workflow's tag trigger required a `v` prefix this project never
  produces, so it had never fired (found at R0).
- The roadmap's numbering rule contradicted `release.md`'s content-driven
  scheme, and R0's post-release bump applied it automatically, pre-committing
  the next release to a minor level before its scope existed.
- `generate_release_notes: true` summarises pull requests; this project commits
  directly to `main`, so it emits only a compare link and never consults the
  CHANGELOG. Every release would have shipped empty notes.

R0 was scoped to fix the first. M2-A treats the release pipeline as one unit and
fixes the rest, so the pattern stops rather than recurring a fourth time.

This slice changes no product behaviour and closes no audit blocker. B2, B3, and
B4 remain open; the v1/public release decision remains **No-Go**.

## 2. Scope followed

In scope:

- CI-composed release notes sourced from the CHANGELOG (F22);
- release policy and procedure documentation in `release.md` (F19, F21);
- threat-model currency: audit-history attribution and a stale heading (F20);
- two small `xtask` corrections carried from review 032 (N5, N6).

Out of scope — do not include:

- any RFC-076 persistence work; that is M2-B and follows this slice;
- any product, UI, diff, merge, or save behaviour change;
- automating the draft-to-published transition (see §4.2 — this is deliberate);
- F18, the `xtask` formatting-gate gap, which belongs to M4;
- refactoring `xtask/src/main.rs` beyond the two targeted corrections;
- cutting a release. M2-A's end-to-end evidence arrives at M2-B's release cut.

## 3. Files changed

Expected areas:

- `.github/workflows/release.yml` — checkout, notes extraction, `body_path`
- `docs/src/maintainers/release.md` — re-release policy, cycle rules, publish step
- `docs/src/maintainers/threat-model.md` — audit history, section heading
- `xtask/src/main.rs` — two corrections, no behaviour change

## 4. Design decisions and assumptions

### 4.1 Release-notes composition (F22)

Replace `generate_release_notes: true` with a `body_path` built from the
CHANGELOG section matching the tag.

**Step order matters.** The `Create GitHub Release` job currently has no
`actions/checkout`; it only downloads artifacts into the working directory.
`actions/checkout` cleans the workspace by default, so it must run **before**
`actions/download-artifact`, or the artifacts are wiped and the release
publishes with no files attached. This is the single most likely way to break
this job.

**Match the heading by exact prefix, not regex.** The obvious
`$0 ~ "^## \\[" ver "\\]"` treats the dots in a version as regex wildcards, so
`0.166.0` would also match `0X166X0`. Construct the literal header and use
`index($0, hdr) == 1`:

```sh
awk -v ver="${GITHUB_REF_NAME}" '
  BEGIN { hdr = "## [" ver "]" }
  index($0, hdr) == 1 { f = 1; next }
  f && /^## \[/ { exit }
  f { print }
' CHANGELOG.md > release-notes.md
```

**Fail closed on content, not bytes.** If the extraction yields no actual
content, the job must fail rather than publish a draft with empty notes:

```sh
grep -q '[^[:space:]]' release-notes.md || { echo "::error::CHANGELOG section for ${GITHUB_REF_NAME} is missing or empty"; exit 1; }
```

This wording is deliberate and supersedes an earlier revision of this handoff
that specified `test -s`. A byte-length test is insufficient: a CHANGELOG
section that exists as a heading with no body extracts to a single newline —
one byte — which passes `test -s`. Because the compare link is appended after
the guard, such a section composes to a blank line plus `**Full Changelog**: …`,
which is exactly the bare compare link F22 exists to eliminate. The defect
reproduces inside its own fix. The post-release bump opens an empty section by
design and `version-sync` asserts only that the heading exists, so this is
reachable in normal operation, not a contrived case.

`version-sync` already asserts the CHANGELOG section exists during preflight, so
this is a second, cheap guard at the point of use. Silent empty notes are the
exact failure being removed; do not let the fallback reintroduce them.

**Preserve the compare link.** Append it after the extracted section so the
information the old auto-generation provided is not lost.

### 4.2 Publishing stays manual — deliberately

Do not automate the draft-to-published transition, and do not add a
`workflow_dispatch` publish job in this slice.

Draft state is the owner's approval gate and the inspection window. R0 is the
proof: its first release run went red on the Windows job, and nothing reached
users precisely because publication had not happened. Automating the flip would
make the tag push the irreversible public act and delete that control.

The division to document and honour is three verbs: **CI builds and creates the
draft; CI composes the notes; a human publishes.** Only the third is manual, and
it is manual on purpose.

### 4.3 Release policy documentation (F19, F21)

`release.md` currently has no re-release or immutability policy at all, despite
that policy having governed R0's tag re-cut, and it documents the publish step
only as "inspect the draft release artifacts before publishing" with no command.
Add:

**Publication and immutability.**

> A version is **published** once its GitHub Release leaves draft state. Before
> that point the tag may be re-cut: delete the remote tag, re-tag the corrected
> commit, and record the re-cut in that version's CHANGELOG entry. After that
> point the version is immutable — supersede it with a new patch version. Never
> re-cut a tag whose release has left draft, even to fix a broken build.

**Version level.** `release.md` is authoritative and content-driven. The commit
after a release bumps to the next **patch** level, which satisfies the version
invariant while claiming nothing about content. At release time the accumulated
content decides the level, and promotion from patch to minor is confirmed by the
owner with the content visible. Record why: the level cannot be known before the
content is, and an earlier roadmap rule that fixed it mechanically pre-committed
a release before its scope existed.

**Publish step.** Document it as an explicit owner action with the command, and
note that it is the approval gate rather than a formality.

**Caution.** `gh release delete --cleanup-tag` deletes the remote tag as well as
the release. The flag reads as release-scoped and is not.

Note one asymmetry rather than papering over it: `version-sync`'s check keys on
**tag existence**, while this policy defines published as **out of draft**. That
is intentional — a pushed tag is the practical collision point for a version
number, and it is the earlier and safer of the two lines. State it explicitly so
a future reader does not treat it as a contradiction to be "fixed".

### 4.4 Threat-model currency (F20)

- Add a `v0.165.0` audit-history row for the RFC-075 load-token guard. RFC-075's
  own security section states it prevents integrity failures in which content
  from one path pair is displayed or saved under another tab identity, so it
  belongs in the history of security-relevant changes.
- The existing `v0.148.0` row still asserts "Stale-tab guard prevents write to
  closed tab", which audit finding B1 established was insufficient. Do not
  rewrite the historical row — it records what was believed then. Make the new
  row explicitly supersede it.
- The section heading still reads `## Data flows and controls (v0.164.0)` while
  the document version line was updated to `v0.165.0`. Align it.

### 4.5 `xtask` corrections (N5, N6)

- `git_lines`'s doc comment claims it returns "trimmed non-empty stdout lines";
  it neither trims nor filters. Behaviour is correct because `git tag` emits
  neither, so fix the comment to describe the code, or make the code match the
  comment. Either is acceptable; a comment that misstates a contract is not.
- The failure message says "already published", but the check triggers on any
  local tag, and a tag never pushed is not published. Reword to
  `version {version} is already tagged; bump it`, consistent with §4.3's
  distinction between tagged and published.

Both are small. Do not let them grow into a refactor — `xtask/src/main.rs` is at
498 ELOC against a 500 hard threshold, and F18 (its formatting-gate gap) is
M4's, not this slice's.

## 5. Tests and gates run

No implementation commands have been run for this handoff. Required observed
evidence:

```sh
cargo fmt --check
cargo xtask version-sync
cargo xtask css --check
cargo xtask i18n
cargo xtask audit-deps
cargo audit
cargo test -p forskscope-core -p forskscope-ui-logic
cargo clippy -p forskscope-core -p forskscope-ui-logic -- -D warnings
cargo test --workspace
cargo clippy --workspace -- -D warnings
git diff --check
```

This slice changes no Rust product code, so the suites are regression evidence.
Record observed counts rather than asserting they are unchanged.

**The notes extraction must be proven locally before it can be trusted in CI.**
Run the exact `awk` against the committed CHANGELOG and show that:

- `ver=0.165.0` reproduces that section in full, stopping at the `0.164.0`
  heading and including neither heading, and passes the guard;
- a heading-only section — one that exists but has no body, such as a freshly
  opened post-release section — **fails** the guard. This is the case a
  byte-length test lets through, so demonstrate it explicitly. In the current
  tree `ver=0.165.1` is exactly this case;
- a version with no section at all yields empty output and fails the guard;
- a deliberately wildcard-shaped version such as `0X165X0` yields empty output,
  proving the prefix match is literal rather than regex.

Four invocations cover these; a heading-only section and the current
in-development section are the same case whenever the post-release section has
not yet accumulated entries.

Expect one pre-existing `git diff --check` hit on
`packaging/windows/AppxManifest.xml` if that file is touched: it uses CRLF
throughout, and git flags the `\r` only on lines inside a diff hunk. Not a
defect, and not introduced here.

## 6. Generated artifacts

None. This slice cuts no release. Its end-to-end evidence — a real release whose
notes come from the CHANGELOG — arrives at M2-B's release cut.

## 7. Known limitations

- The notes extraction is proven locally in this slice and end-to-end only at
  the next real cut. That is the accepted verification model: content review
  now, live evidence at the cut.
- Publishing remains manual, so a release can still sit in draft indefinitely.
  That is the intended control, not a gap.
- F18 (`xtask` outside `cargo fmt --check`) is untouched and remains M4's.
- The slice closes no audit blocker. B2, B3, B4 remain open.

## 8. Recommended next step

Land M2-A as its own reviewed change before starting M2-B (RFC-076). Keeping
them separate matters: RFC-076 rewrites the production persistence path, and a
release-mechanics change entangled with it would be reviewed under the wrong
kind of attention.

After M2-A is accepted, begin RFC-076 per
`../076-versioned-runtime-persistence/implementation-handoff.md`, whose design
review pause after its first patch still applies. Its persistence adapters must
never install legacy persisted IDs as runtime `CompareTabId` values.

## 9. Acceptance criteria

- The release job checks out the repository **before** downloading artifacts,
  and a dry inspection confirms artifacts are still attached.
- `generate_release_notes` is gone; the release body comes from `body_path`.
- The extraction matches headings by literal prefix, and the four local cases in
  §5 are demonstrated with recorded output.
- An absent CHANGELOG section fails the job with a visible error.
- The compare link survives in the composed body.
- `release.md` documents publication and immutability, the content-driven level
  rule with the post-release patch default, the publish step as an explicit
  owner action, the `--cleanup-tag` caution, and the tagged-versus-published
  asymmetry.
- The threat model carries a `v0.165.0` row for the RFC-075 guard that
  supersedes the `v0.148.0` claim without rewriting it, and its section heading
  matches the document version.
- Both `xtask` corrections are applied and the file stays under 500 ELOC.
- All gates in §5 pass with recorded output.

## 10. Prohibited shortcuts

- Automating the draft-to-published transition, or adding a publish trigger.
- Letting an empty or fallback-generated body substitute for a real CHANGELOG
  section.
- Rewriting the historical `v0.148.0` threat-model row instead of superseding it.
- Bundling any RFC-076 work, or F18, into this slice.
- Refactoring `xtask/src/main.rs` beyond the two named corrections.
- Cutting or publishing a release to "prove" the notes change.
- Reporting gate results that were not observed in this workstream.

## 11. Compatibility and security constraints

- `0.165.0` is published and immutable. Nothing in this slice may alter it.
- No dependency is added, removed, or version-changed.
- `cargo audit` and `cargo xtask audit-deps` must still pass with the reviewed
  `.cargo/audit.toml` exceptions intact.
- The threat-model edit records a historical attribution and a shipped
  mitigation; it must not overstate either.
- No real user paths, host names, or secrets in documentation or evidence.

## 12. Required review-request content

Submit under `.git-exclude/review-request/` with:

1. implementation summary;
2. addressed items (F19–F22, N5, N6);
3. changed files;
4. important implementation decisions, especially the checkout-before-download
   ordering and the literal-prefix match;
5. any difference from this handoff or from RFC-074;
6. executed gates with observed output, including all four notes-extraction
   cases;
7. unresolved issues and known limitations;
8. requested review focus.
