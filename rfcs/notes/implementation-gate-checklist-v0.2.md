# Implementation Gate Checklist v0.2

## Gate A — Before Dioxus UI Work Expands

- [ ] `forskscope-core` crate exists or equivalent core module boundary exists.
- [ ] Core has no Dioxus/Tauri/Svelte dependency.
- [ ] Text file diff can run from tests or CLI.
- [ ] Binary classification can run from tests or CLI.
- [ ] Structured errors are introduced for new core APIs.
- [ ] Parity fixture directory exists.

## Gate B — Before Editor Bridge Work

- [ ] `ComparisonSession` model exists.
- [ ] `DiffHunk` and hunk identity are defined.
- [ ] Dirty state is model-backed.
- [ ] Save preflight API exists, even if not fully implemented.
- [ ] Undo/redo transaction model is drafted in code.
- [ ] Encoding/newline metadata exists in file model.

## Gate C — Before Save Is Enabled in UI

- [ ] Save preflight detects external modification.
- [ ] Save preflight detects binary/non-saveable sessions.
- [ ] Encoding warnings are returned as structured values.
- [ ] Backup or atomic write policy is implemented.
- [ ] Dirty close dialog is implemented.
- [ ] Save tests pass for UTF-8 and at least one non-UTF fixture if supported.

## Gate D — Before Large Directory/File Release

- [ ] Background job model exists.
- [ ] Directory comparison is cancellable.
- [ ] Inline diff is lazy or bounded.
- [ ] Large-file warning mode exists.
- [ ] Performance diagnostics record diff timings.
- [ ] UI remains responsive during synthetic large fixture smoke test.

## Gate E — Before Release Candidate

- [ ] Core tests pass.
- [ ] Parity tests pass or intentional-change records exist.
- [ ] Editor bridge protocol tests pass.
- [ ] Manual QA checklist completed.
- [ ] Diagnostics summary works.
- [ ] Linux package smoke test completed.
- [ ] Windows/macOS smoke tests completed if artifacts are published.
