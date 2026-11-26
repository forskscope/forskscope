# Handoff 115 — RFC-102 stage A: the list, and Windows measured

**From:** architect. **Date:** 2026-11-24. **Release:** `0.201.0`.
**RFC:** `rfcs/accepted/102-messages-that-need-attention-are-heard.md`,
accepted by the owner on 2026-11-24, warnings included. **Measurement and
a list only. No product change.** It runs alongside handoff 114.

## 1. The list, for the owner

**Every call site** that raises an **error** or **warning** notice
(`Notice::error`, `Notice::warning`, `store.notify`, and any other path).
For each:
- where (file:line);
- the message in English and Japanese, as shown today;
- **when it happens,** in one plain sentence;
- **whether it can arrive while the user is typing** (a text field
  focused).

**Then the success and info notices,** listed the same way but shorter,
so the owner sees what stays passive.

**Send it as its own reply,**
`.git-exclude/review-request/203a-focus-taking-messages.md`. I pass it to
the owner together with handoff 114's wording list.

## 2. Windows: do live regions reach UI Automation?

**A scratch CI run on `windows-latest`:**
- it hosts F226's five shapes (`examples/announce_probe.rs`) in WebView2;
- **a UI Automation listener** (`IUIAutomation::AddAutomationEventHandler`
  for `UIA_LiveRegionChangedEventId`, plus focus-changed for a control)
  records, for each shape, whether the event fires;
- **three runs.**

**Send me the job file before pushing it.** A scratch CI run leaves a
public trace, so it needs my go-ahead (the standing rule). Also say:
- the branch name;
- that it's push-triggered on that branch only;
- that the branch is deleted afterwards.

**Report a table like reply 195's:** shape (rows), against
`LiveRegionChanged` seen, focus events seen, and whether it varied.

## Rules

- **No product change.**
- **The example stays out of every release build** (reply 195's
  reasoning).
- **The register is mine.**

**Reply in** `.git-exclude/review-request/203-rfc-102-stage-a.md`, after
`203a` and the measurement.

## Round 2 (2026-11-25, review 238)

**Go ahead with the Windows job,** with two changes:
- **`set_focus()` before each `invoke()`,** and at least one focus event
  per shape, or the run is invalid;
- **branch from today's `main`.**

Reply in `.git-exclude/review-request/203-rfc-102-stage-a.md`.
