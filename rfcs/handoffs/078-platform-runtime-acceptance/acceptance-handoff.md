# RFC-078 QA Handoff: Platform Runtime Acceptance

## 1. Summary

Execute and retain the platform/runtime matrix against exact release-candidate
artifacts after RFC-075–077 and integrated gates pass. Artifact build success
does not count as runtime acceptance.

## 2. Scope followed

In scope:

- artifact hashes and source commit identity;
- Linux Wayland/X11, Windows, and macOS matrix execution;
- Compare/Explorer/layout/save/persistence/mergetool cases P01–P12;
- platform-specific save and package behavior;
- advisory dispositions and explicit waiver records;
- concise committed evidence records.

Out of scope:

- testing before correctness workstreams finish;
- storing private host data, credentials, certificates, or user documents;
- interpreting missing evidence as a pass.

## 3. Files changed

Expected evidence/documentation files:

- `docs/src/maintainers/release-evidence/vX.Y.Z-rcN/README.md`
- `artifacts.md` and one record per matrix environment
- `matrix-plan.md` pinning exact versions, owner roles, access, and case scope
- `advisories.md`
- `docs/src/SUMMARY.md`
- corrections to platform prerequisites/minimum versions
- RFC/status/handoff updates after observed results

Implementation fixes discovered by QA require their own reviewable patches and
invalidate affected artifact evidence.

## 4. Design decisions and assumptions

- Evidence is bound to SHA-256 plus source commit.
- Required case states are Pass, Fail, Blocked, or Waived; Waived is not Pass.
- Correctness/data-safety failures cannot be waived for v1.
- Sanitized Markdown summaries are durable; raw screenshots/logs remain outside
  the repo unless privacy-reviewed.
- Rebuilt artifacts require affected cases to be rerun.
- M5 does not start until every matrix row has an exact environment and
  confirmed executor role; unavailable hosts rebaseline the schedule.

## 5. Tests and gates run

None have been run for this future release candidate. Before matrix execution,
observe RFC-074 Gate C. During execution, record every command and manual case
in the relevant evidence file.

At minimum rerun on the source commit:

```sh
cargo fmt --check
cargo xtask css --check
cargo xtask version-sync
cargo xtask i18n
cargo xtask audit-deps
cargo audit
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## 6. Generated artifacts

Expected release-candidate artifacts:

- source `.tar.gz`;
- Linux x86_64 binary archive;
- macOS aarch64 DMG;
- Windows x64 zip;
- SHA-256 manifest.

Record artifacts; do not commit binary packages into the source repository.

## 7. Known limitations

- Manual visual/accessibility checks remain necessary for v1.
- Host access may delay the calendar but does not relax the matrix.
- Signing/notarization may be deferred only with owner-approved limitations;
  functional launch evidence remains required.

## 8. Recommended next step

Before Milestone M5, commit and approve `matrix-plan.md`. Then create the
release-candidate evidence directory, hash all artifacts, and execute Linux
Wayland first to catch WebKitGTK regressions before parallel Windows/macOS
testing.
