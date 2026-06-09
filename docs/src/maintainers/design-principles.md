# Design Principles

## Less is more

The default layout shows only what the current workflow step needs.  A user
opening the app for the first time should see exactly enough to start
comparing: two directory panes and a Compare button.  A user in the diff view
should see the file differences, navigation, and a Save button — nothing more
until they ask for it.

Advanced controls (inline character diff, redo, future compare profiles) are
behind a one-click disclosure.  They are available but not front-loaded.

This layering is documented in the non-goals policy under D-001 ("Don't show
every possible pane by default") and informs every UI decision.

## One job, trusted

ForskScope has a defined boundary: local two-pane diff and merge.  It does not
aspire to be a Git GUI, IDE, cloud service, file synchronizer, or plugin
platform.  The non-goals policy (see `rfcs/notes/`) enumerates all the things
it deliberately does not do.

A narrow scope makes the app trustworthy.  A tool that does one thing
predictably is safer in a merge workflow than one that might also be running a
sync or touching a remote server.

## Model-backed correctness

Every merge action goes through a transaction log in the Rust core (`MergeSession`).
The canonical result text is computed from that model.  The UI never infers
merge state from DOM content, component state, or rendered text.

This distinction is the single most important architectural difference from
v0.22.x, where the diff model was mutated in-place on the frontend.

## Local-first and private

No accounts, no telemetry, no cloud upload.  The local-first stance is not a
feature to market; it is the default because users compare private source code,
credentials in config files, internal documents, and production logs.  Trust
should not require opting out.
