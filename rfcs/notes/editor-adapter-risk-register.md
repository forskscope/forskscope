# Editor Adapter Risk Register

## R1 — DOM State Diverges from Core State

### Cause
The editor accepts edits, decorations, or normalization internally that are not reflected in `forskscope-core`.

### Mitigation
All editor changes must become typed operations. The core validates and acknowledges operations. The editor view is updated from acknowledged model state.

## R2 — Line Mapping Breaks During Edit

### Cause
Diff hunks and visual decorations are anchored by stale line numbers.

### Mitigation
Use durable spans and recompute decorations from canonical document revision IDs. Never mutate decoration state as the primary record.

## R3 — Large File Performance Collapse

### Cause
Full document diff, full decoration rebuild, or full DOM repaint after every edit.

### Mitigation
Use thresholds, virtualization, throttling, and explicit large-file mode. Long-running diff jobs must be cancellable.

## R4 — IME and Selection Bugs

### Cause
Linux/Wayland/WebView/editor-specific text input behavior.

### Mitigation
Adopt a web editor with mature IME behavior, test common IME flows, and avoid replacing composition text from Rust-side updates during composition.

## R5 — JS Bridge Security Drift

### Cause
The editor adapter gradually exposes generic filesystem or command execution abilities to JavaScript.

### Mitigation
The bridge must expose only editor events and editor commands. File access remains Rust-side and capability checked.

## R6 — Hidden Formatting Changes

### Cause
The editor normalizes newlines, tabs, final newline, encoding, or BOM without explicit policy.

### Mitigation
Encoding and newline policy must be applied only by the core and surfaced to users when conversion is needed.
