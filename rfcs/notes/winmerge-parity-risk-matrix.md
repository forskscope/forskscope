# WinMerge Parity Risk Matrix

This matrix does not say that ForskScope must clone WinMerge. It identifies product behaviors users may expect when they hear “diff and merge tool,” especially Unix/Linux users who lack WinMerge.

| Capability | v0.4 Position | Risk | RFC |
|---|---|---:|---|
| Two-pane text diff | Required | Medium | 006, 024, 032, 035 |
| Hunk navigation | Required | Medium | 014, 035 |
| Copy left to right / right to left | Required | Medium | 006, 015, 032 |
| Direct text editing | Required for serious adoption | High | 032, 040 |
| Synchronized scrolling | Required | High | 035 |
| Inline character highlights | Required | Medium | 024, 035 |
| Undo/redo across merge operations | Required | High | 015, 032 |
| Save safety and backup | Required | High | 007, 023, 036 |
| Directory comparison | Required | Medium | 008, 037 |
| Directory merge | Important but dangerous | High | 022, 037 |
| Three-way merge | Important after v1 baseline | High | 033, 034 |
| Conflict resolution workflow | Required for 3-way | High | 034 |
| External modification detection | Required | High | 036 |
| Patch export/apply | Useful | Medium | 039 |
| VCS context | Useful but bounded | Medium | 038 |
| Full IDE/editor feature set | Non-goal | Extreme | 041 |

## Guiding Principle

ForskScope should prioritize trustworthy merge operations over broad editor features. A smaller, safer feature set is better than a visually impressive editor that can silently corrupt or overwrite user work.
