# Implementation Checklist

## Before Coding

- [ ] Approve RFC-042 roadmap (originally numbered RFC-000).
- [ ] Confirm Dioxus version and desktop backend assumptions.
- [ ] Confirm `similar` v3 pinned version.
- [ ] Confirm editor component candidate and adapter POC scope.

## M1 Core

- [ ] Create workspace crates.
- [ ] Move file loading into `forskscope-core`.
- [ ] Add domain error model.
- [ ] Add golden fixtures.

## M2 Diff

- [ ] Upgrade diff dependency.
- [ ] Define normalized diff model.
- [ ] Add Unicode/newline tests.
- [ ] Add large-file threshold behavior.

## M3/M4 UI and Editor

- [ ] Create Dioxus shell.
- [ ] Implement command dispatcher.
- [ ] Mount editor proof of concept.
- [ ] Implement revision-checked editor events.

## M5/M6 Workspaces

- [ ] Implement explorer workspace.
- [ ] Implement diff/merge workspace.
- [ ] Implement hunk navigation.
- [ ] Implement merge transaction history.

## M7/M8 Safety and Jobs

- [ ] Implement dirty state.
- [ ] Implement save plan and conflict checks.
- [ ] Implement backup policy.
- [ ] Implement background digest jobs.

## M9/M10 Release

- [ ] Implement settings persistence.
- [ ] Add diagnostics panel.
- [ ] Run platform smoke tests.
- [ ] Produce release candidate artifacts.
