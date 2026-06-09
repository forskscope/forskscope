# Editor Risk Decision Record

## Decision

ForskScope will continue with Dioxus as the adopted migration target for now, while treating the editor as an isolated adapter and keeping the Rust core as the source of product truth.

## Reason

The app is not merely a file viewer. It aims to become a functional diff/merge tool comparable in spirit to WinMerge for Unix/Linux workers.

The risky area is a practical text editor surface:

- two synchronized panes;
- hunk decorations;
- inline character highlights;
- manual text edits;
- selection and clipboard;
- search;
- keyboard navigation;
- undo/redo;
- save safety.

A pure native Rust UI approach such as Iced remains attractive, but the text editor surface could become a serious technical burden if the project needs WinMerge-like editing behavior. Dioxus allows the project to use a mature web-editor style surface while still moving most application logic into Rust.

## Constraint

Dioxus must not become the architecture. It is the presentation/runtime layer.

The architecture is:

```text
Rust core owns truth
Dioxus owns application UI
Editor adapter owns editor integration
Save logic owns write safety
```

## Red Lines

The following are not acceptable:

- DOM/editor content as the only source of file state;
- save directly from editor text without core save preflight;
- merge operations implemented only as visual editor mutations;
- unversioned bridge messages;
- remote editor assets loaded at runtime;
- raw file text injected as HTML.

## Future Option

Iced remains an excellent possible future UI backend after the core model stabilizes. The migration must preserve that option by keeping the core independent from Dioxus and editor details.
