# Handoff 116 — RFC-102 stage B: errors and warnings take focus; passive notices are announced

**From:** architect. **Date:** 2026-11-25. **Release:** `0.201.0`.
**RFC:** `rfcs/accepted/102-messages-that-need-attention-are-heard.md`.
Read §1–§3, both amendments of 2026-11-25:
- the large-file notice becomes info;
- passive notices go through one permanent live region.

**Start after handoff 114's round 3 is sent,** so every message is already
in the catalog.

## Evidence plan first

- **markup tests** for the roles, names and descriptions;
- **focus tests** through the real event path;
- **Orca Level 2** with F226's harness (private buses, audio blocked) for
  "an error is spoken";
- **one Windows scratch run** for "a passive notice fires
  `LiveRegionChanged`", reusing 203's job. **Send me its diff from 203's
  version first.**

**Send the plan as the first section of your reply.**

## Do

1. **Severities** (the owner's answers, 2026-11-25):
   - errors #1–#19 and warning #22 take focus;
   - **the large-file notice (#20, #21) becomes `Info`.**
2. **The focus-taking toast** (RFC-102 §2):
   - `role="alertdialog"`, labelled by the message;
   - focus moves to *Dismiss*, or to its action, described by the
     message;
   - Escape or *Dismiss* closes it and **returns focus** where it was;
   - **inside a dialog,** focus returns into the dialog;
   - **typed text is kept.**
3. **One permanent live region** (RFC-102 §3, amended):
   - visually hidden, `role="status"`, mounted once with the app;
   - **each passive notice clears it, then sets it** (shape C);
   - **the visible toast no longer carries `role="status"`,** so nothing
     is announced twice.
4. **`Notice` carries a `TranslatableMessage`,** and the toast and the
   live region render it through `TranslatedText`, so user data in a
   notice is isolated (review 239 §2.1).
5. **The guide:** the accessibility page and the known limitations,
   saying:
   - **errors and warnings are spoken everywhere,** because they take
     focus;
   - **other notices are spoken on Windows, not yet on Linux.**

## Verification

- **RFC-102's verification points 2–5.**
- **The Windows run** (above): a passive notice through the permanent
  region fires `LiveRegionChanged` **3 times in 3,** with the focus
  control valid.
- **Test names diffed.**
- **RFC-098's gate green.** The toast's occlusion allowance is unchanged.

## Gates

**The standard set,** including:
- `cargo clippy --manifest-path xtask/Cargo.toml`;
- `i18n`;
- both audits.

Reply in `.git-exclude/review-request/204-rfc-102-stage-b.md`.

## Round 2 (2026-11-25, review 240)

- **Orca Level 2 through the existing private-bus harness**
  (`atspi-private-bus.sh`, as `f226-run-orca-probe.sh` does): an error is
  spoken; on Linux, a passive notice isn't. Three runs each.
- **The Windows job:**
  - one launch per run;
  - the listener on the root element, filtered by process ID,
    registered before each launch;
  - the control is that focus stays inside the Settings dialog.
  
  **Go-ahead with exactly these changes.**
- **The guide's Windows claim** stands only if the run shows 3 of 3.

Reply in `.git-exclude/review-request/204b-rfc-102-stage-b.md`.

## Round 3 (2026-11-25, review 241)

**Orca: approved.** The Windows run's process filter was my mistake:
WebView2's events come from `msedgewebview2.exe`. One more attempt, at
most two pushes, **modelled on 203:**
- the listener on the app's window element, with no process filter;
- a test-only delay before the passive notice fires;
- the control is a `FocusChanged` event from the window's subtree;
- three launches.

**If it's still inconclusive, stop.** The guide's current wording stands.

Reply in `.git-exclude/review-request/204c-windows-passive.md`.

## Round 4 (2026-11-25, review 242): F241, measure first

**The method is settled.** Measure the four cases, three runs each:

| Case | Dialog |
|---|---|
| passive notice | none |
| passive notice | Settings open |
| error | none |
| error | Settings open |

- **Linux:** Orca through `atspi-private-bus.sh`, as in 204b.
- **Windows:** 204c's working job, two plan variants, at most two pushes.
  **Go-ahead given.**

**No product change.** If the error with a dialog open isn't spoken on a
platform, say so: the guide's "everywhere" will name the exception.

Reply in `.git-exclude/review-request/204d-notices-and-dialogs.md`.

## Closed (2026-11-25, review 243)

**Complete.** RFC-102 stage B is done. F241 narrows to the Windows passive notice, for `0.202.0`.
