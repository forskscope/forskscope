# RFC-077 Developer Handoff: Mergetool Save Target

## 1. Summary

Separate compared right/remote input from the actual save destination. Prepare
the merge target's fingerprint and encoding atomically with the comparison
result, then route save/save-as/overwrite/reload exclusively through that
target model.

## 2. Scope followed

In scope:

- typed startup and compare requests;
- `SaveTargetSnapshot` and launch mode;
- target preparation for existing/missing/unsupported paths;
- save, Save As, overwrite, and reload integration;
- existing/missing/changed/replaced merged-target tests;
- Git/JJ and GTK-checklist documentation.

Out of scope:

- base-aware conflict UI, Git invocation, Git exit-code certification, or
  persistence of mergetool sessions.

RFC-075 must land first. Do not recreate index-only async guards in this work.

## 3. Files changed

Expected areas:

- `crates/forskscope-ui/src/main.rs`
- `crates/forskscope-ui/src/app.rs`
- `crates/forskscope-ui/src/state/compare.rs`
- `crates/forskscope-ui/src/state/tab.rs`
- `crates/forskscope-ui/src/ui/view/diff_actions.rs`
- diff header/status presentation
- core loading/save helpers only when required by the accepted model
- integration tests for prepared compare/save target
- README, CLI, Git integration, merging, and GTK checklist docs

## 4. Design decisions and assumptions

- `right_path/right_doc` always mean the compared right/remote input.
- `save_target` alone supplies output path, expected fingerprint, and encoding.
- Existing merged targets are inspected independently; their content is not
  substituted into the two-way comparison.
- Unsupported targets block rather than force overwrite.
- Save As changes only the save target after successful write.
- Prepared comparison plus target snapshot commits under RFC-075's load token.

Recommended patches:

1. Request/target model and preparation tests.
2. Normal compare migration proving unchanged behavior.
3. Mergetool startup and save-path migration.
4. External-change cases, presentation, and docs.

## 5. Tests and gates run

No implementation gates have been run for this design handoff. Required:

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Targeted evidence must name tests for:

- existing merged target and `.bak` bytes;
- missing merged target;
- external merged-target mutation;
- target replaced by directory;
- normal two-argument compare regression;
- reload and Save As identity.

## 6. Generated artifacts

None expected before RFC-078. Tests create only temporary local files and must
clean them through `tempfile` ownership.

## 7. Known limitations

- The mode remains a two-way local-vs-remote workflow, not graphical diff3.
- Git determines resolution from the merged file; normal window exit remains 0.
- Windows replacement semantics require real runtime evidence in RFC-078.

## 8. Recommended next step

After RFC-075 acceptance, implement the request/target model with preparation
tests. Request an architecture checkpoint before migrating save behavior.

