# RFC 057: Settings Dialog Layout Refinements

**Status.** Implemented (v0.36.0)

**Tracks.** Settings dialog UX. Two focused layout changes: relocating the
About/info action into the Settings header, and hiding the new-profile
form behind progressive disclosure.

**Touches.** `crates/forskscope-ui-dioxus/src/ui/header.rs`,
`crates/forskscope-ui-dioxus/src/ui/settings.rs`,
`crates/forskscope-ui-dioxus/src/ui/modals.rs` (About modal trigger).

## Summary

Two small but worthwhile refinements to the Settings dialog:

1. **Relocate the "i" (About/info) button** from the global app header
   into the **Settings dialog header**. About/diagnostics is a low-
   frequency action; it does not need a permanent slot in the main
   header, and grouping it with Settings is where users look for "app
   information".

2. **Hide the "New profile" form by default.** The compare-profile
   creation form (name field, option checkboxes, algorithm selector, Add
   button) is currently always visible, adding clutter to the profile
   list. Replace it with a **"New profile" button** that reveals the form
   on demand (progressive disclosure), collapsing it again after a
   profile is added or the action is cancelled.

## Motivation

The global header currently carries brand, Explorer, Settings, About
(`ℹ`), and Keyboard (`?`) controls. The About action is rarely used
(version/diagnostics lookup) and competes for attention with the
high-frequency controls. Moving it into Settings declutters the main
header.

The always-visible new-profile form violates the "less is more"
principle: most sessions never create a profile, yet every visit to
Settings shows the full creation form. Progressive disclosure keeps the
common case (reading/selecting existing profiles) clean.

## Goals

- Remove the `ℹ` About button from the global header.
- Add an About/info affordance to the Settings dialog header (an `ℹ`
  button or an "About" entry), opening the existing About modal.
- Replace the always-on new-profile form with a "New profile" button that
  toggles the form's visibility.
- Collapse the form after a successful add.
- Preserve the existing `?` Keyboard reference button in the global
  header (unchanged).

## Non-Goals

- Redesigning the About modal contents (version, build, platform, copy
  diagnostics) — unchanged.
- Restructuring Settings into a multi-pane/sidebar layout. The dialog
  stays a single scrollable panel.
- Changing profile data model or matching behavior.

## External Design

### Settings header with About

```text
┌ Settings ─────────────────────────────── ℹ  ✕ ┐
│ Theme        ( ) Dark  ( ) Light  ( ) Night    │
│ Language     ( ) English  ( ) 日本語           │
│ Diff font    [ 14 ]                            │
│ Context      [ 3 ▾ ]                            │
│ ...                                            │
└────────────────────────────────────────────────┘
```

The `ℹ` in the Settings header opens the About modal (which renders above
or replaces the Settings modal per the existing modal-host behavior).

### Profile list with progressive disclosure

Collapsed (default):

```text
Compare profiles
  ● Exact (default)
  ○ Ignore whitespace
  ○ Ignore case
  ○ Histogram
  [ + New profile ]
```

Expanded (after clicking "New profile"):

```text
Compare profiles
  ● Exact (default)
  ○ Ignore whitespace
  ...
  ┌ New profile ───────────────────────────┐
  │ Name        [ my-profile             ]  │
  │ [x] Ignore whitespace  [ ] Ignore case  │
  │ Algorithm   [ Histogram ▾ ]             │
  │ [ Add ]  [ Cancel ]                     │
  └─────────────────────────────────────────┘
```

- "New profile" toggles a local `show_new_profile` UI signal.
- "Add" creates the profile (existing `add_profile`) and collapses the
  form.
- "Cancel" collapses the form without adding.

### State ownership

The form-visibility flag is **UI state** (a Dioxus signal local to the
Settings component), not application/session state — it resets each time
the dialog opens, which is the desired behavior (always start collapsed).

## Alternatives Considered

- **Keep About in the header but move it next to `?`.** Rejected: the
  request is specifically to group it with Settings, and that is the more
  conventional location for "app information".
- **Inline-expand the new-profile form as the last list row instead of a
  separate disclosed panel.** Equivalent UX; the detailed design may pick
  whichever renders cleaner. The RFC requires only that the form be hidden
  until requested.

## Testing

These are UI-layout changes with no core logic, so verification is
primarily manual / visual, plus:

- The About button no longer exists in the header component; the Settings
  header renders an About trigger that sets the About modal.
- The new-profile form is absent from the initial Settings render and
  present after the toggle (testable on the component's rendered output if
  a render harness exists; otherwise a manual check).
- `add_profile` still produces a persisted profile and the form collapses.

## Open Questions

- Whether the Settings-header About opens the About modal *over* Settings
  or replaces it. Proposed: open over Settings (stacked), returning to
  Settings on close, since the user came from Settings.
