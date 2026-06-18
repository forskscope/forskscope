# RFC 063: Trust, Clarity, and Calm UI Hardening

**Status.** Proposed
**Tracks.** Non-defect UX hardening: empty/first-run states, control density and
hit targets, labeled vs icon-only actions, destructive-modal focus policy,
severity-based notice/toast policy, plain-language settings with progressive
disclosure, and trust-marker clarity.
**Touches.** `crates/forskscope-ui/src/ui/*` broadly (explorer, diff, modals,
statusbar, settings, search, dir_pane), `crates/forskscope-ui/assets/main.css`
(density tokens, control sizing), `crates/forskscope-ui/src/i18n.rs` (wording),
new small components.

## Summary

The UI/UX architect review produced a large set of recommendations that are
**legitimate design improvements but not defects**: the app works correctly as
shipped, but it reads more like a compact developer/operator tool than a calm,
self-explanatory one. This RFC collects those recommendations into a single
coherent "hardening" effort so they are triaged and sequenced deliberately
rather than applied piecemeal.

Crucially, this RFC also records where the reviewer's recommendations conflict
with ForskScope's own product constitution (the non-goals / Don't list and the
"less is more" principle), and makes an explicit maintainer decision on each
such case rather than accepting the review wholesale. ForskScope's audience is
Unix/Linux developers and operators; recommendations calibrated for a general
non-technical audience are adopted only where they also serve that audience.

## Motivation

Two reviews (handoff-based and source-based) converged on the same themes:
icon-only controls, small hit targets, missing empty/first-run states,
ambiguous trust markers, dense settings, and toast/error policy. None of these
cause incorrect behaviour, but together they raise the cost of first use and
the chance of a mistaken click. Capturing them as one RFC keeps the work
coherent and lets each item be accepted, downscoped, or rejected with a recorded
rationale.

## Items

Each item carries a disposition: **Adopt**, **Adopt (downscoped)**, or
**Reject (with rationale)**.

### C1 — Empty and first-run states · Adopt

No primary workspace should render as a blank area.

- **Empty directory pane:** "This folder is empty. Choose another folder, or go
  up one level." with the relevant actions.
- **No compare picks:** a quiet hint, "Choose one item on each side to
  compare."
- **First open (no CLI args):** a brief three-step orientation — choose
  left, choose right, compare — plus a one-line local-first statement. Dismissed
  permanently after the first successful comparison.

Note: a full in-app help/onboarding *system* remains owned by RFC-030 and is not
duplicated here. C1 is limited to inline empty-state copy and a single
dismissible first-run hint.

### C2 — Control density and hit targets · Adopt (downscoped)

Introduce density tokens and a default "comfortable" size for **file-writing and
safety-critical controls** (Save, Save As, Compare, Copy, Overwrite, Discard).
These must never be tiny.

- Adopt density CSS variables (`--control-h`, `--row-h`, `--icon-button-size`,
  `--control-pad-x`) with a comfortable default.
- **Downscope:** the review's full two-density model with a user-facing
  "Compact mode" toggle is deferred. ForskScope's audience is keyboard- and
  density-tolerant; a settings toggle is added only if a user need is
  demonstrated. The default sizes are raised; the toggle is out of scope for
  this RFC.

### C3 — Labeled vs icon-only actions · Adopt (downscoped)

No file-writing action may be an arrow-only icon.

- Copy/apply/overwrite actions get written labels ("Copy to right", "Use this
  change", "Replace right with left"). The copy-direction labels are defined in
  RFC-062; this item adopts the same wording for the diff apply control.
- **Navigation-only icons may remain icons** (Back, Forward, Up, Home, folder
  picker, edit-path), provided each has a visible tooltip, an accessible label,
  a keyboard focus style, and a help-modal entry. This is the downscope:
  ForskScope is a keyboard-first tool and a fully label-only navigation bar
  fights the "less is more" principle. Icons + accessible names is the
  standard, labels for write actions.

### C4 — Destructive-modal focus policy · Adopt

For any modal whose primary action writes or destroys (Overwrite, Discard,
Copy/Replace), **Cancel is the default-focused button**. The destructive action
requires a deliberate click or keyboard navigation. Verbs name the result
("Save anyway", "Close without saving", "Replace right with left"), never "OK".
Where a backup is made, the modal says so.

Note: `OverwriteModal` already autofocuses Cancel today; this item generalizes
the rule to all destructive modals and audits each one.

### C5 — Severity-based notice/toast policy · Adopt

Define a `NoticeKind` (Success / Info / Warning / Error) and a duration policy:

| Kind | Duration | Dismissal |
|---|---|---|
| Success | ~3–4 s | auto + click |
| Info | ~5 s | auto + click |
| Warning | persistent | click |
| Error | persistent | click |

File-safety warnings (external change, encoding fallback, copy failure) must
also appear **inline near the affected view**, not only as a toast. Today all
toasts are persistent click-to-dismiss; this item adds auto-dismiss for
success/info only.

### C6 — Plain-language settings with progressive disclosure · Adopt (downscoped)

Reorganize Settings into Appearance / Compare behaviour / Advanced. Plain-label
the common compare options; keep algorithm names (Histogram, Myers, Patience)
and patch-export defaults under Advanced.

- **Downscope:** the underlying profile model and algorithm names are not
  renamed in code or in the profile data (RFC-028 owns the profile model);
  this is a presentation-layer relabel + grouping only. Power users keep access
  to the precise names under Advanced. This respects Don't-list D-012 ("don't
  require users to understand internal algorithms" *in normal UI*) without
  hiding capability from the audience that wants it.

### C7 — "Local only" trust marker clarity · Adopt

Add a lock glyph and a tooltip: `🔒 Local only`, tooltip "Files stay on this
computer. ForskScope does not upload them." Show the longer statement once in
the first-run state (C1). On narrow layouts, `🔒 Local` with the tooltip.

### C8 — Identical-files presentation · Reject (keep current) 

The review recommended defaulting identical files to a compact "No differences
found" summary that hides content behind a "Show content" button.

**Decision: reject.** This was changed deliberately in the v0.144.x line, in
response to direct maintainer smoke-test feedback, *from* a content-hiding
message *to* showing full content with a green "Files are identical" notice at
top. Reverting would undo a considered decision. The current behaviour (content
visible + clear top notice) is retained. Recorded here so the question is
closed and not re-litigated.

### C9 — Directory-report statistics exactness · Adopt

Compute each category explicitly (changed / equal / left-only / right-only /
computing / symlink) rather than deriving `equal = total − diff`, which can fold
computing/other states into the equal count. Counts are trust anchors and must
be exact; show "computing" separately until complete. (Source review P1-3.)

### C10 — Friendly error mapping · Adopt

Route core errors through the existing UI error-conversion layer everywhere;
some action handlers still format raw OS error strings. User-facing messages
should state what failed, which path, the likely cause, and the next action.
The plain-language message patterns from the review (binary/text mismatch,
missing files, external change) are adopted. (Source review P1-7; handoff §6.)

## Explicitly rejected / deferred from the reviews

- **Review-mode-before-Merge-mode gating (handoff P2-1):** Reject. Adds a mode
  gate before merge controls for a tool whose users came to merge; fights "less
  is more" and adds friction for the core audience. Keyboard shortcuts already
  let experts work fast; the dirty-marker + save-guard already protect against
  accidental save.
- **Full onboarding system / checklists (handoff P2-3):** Defer to RFC-030
  (already owns onboarding/help). C1 provides only the minimal inline hint.
- **Compact-mode toggle (handoff P0-2 full form):** Defer (see C2).

## Non-goals

- No new comparison features. This RFC is clarity and calm only.
- No change to merge/save/diff correctness.
- No duplication of the onboarding system (RFC-030) or profile model (RFC-028).

## Acceptance criteria

- No primary workspace renders blank; empty states carry the next safe action.
- Safety-critical controls meet the comfortable size; write actions are
  labeled.
- All destructive modals focus Cancel by default and name the outcome.
- Success/info notices auto-dismiss; warnings/errors persist and also appear
  inline.
- Settings present plain labels with algorithm detail under Advanced.
- "Local only" is self-explanatory via glyph + tooltip + first-run statement.
- Directory-report counts are computed per category and exact.
- The identical-files view is unchanged (C8 decision recorded).

## Cross-references

- RFC-028 — profiles/compare options (label, don't rename, the model).
- RFC-030 — onboarding/help system (owns the full help surface).
- RFC-062 — copy direction wording and destructive-modal focus (shared).
- non-goals / Don't list — D-001 (calm default layout), D-004 (progressive
  disclosure), D-011 (ergonomics over screenshots), D-012 (no algorithm jargon
  in normal UI), D-014 (quiet app).

## Open questions

- Sequencing within this RFC: C4 (destructive-modal focus) and C10 (friendly
  errors) are the highest trust-value items and could ship first; C1/C2/C3/C7
  are the discoverability cluster; C5/C6/C9 are refinements. Should this RFC be
  split into "trust" (C4, C10) and "clarity" (rest) sub-efforts for staged
  delivery?
