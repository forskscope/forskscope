# Diff Options

ForskScope exposes several options that change how differences are detected
and displayed. Most can be toggled per-tab in the diff toolbar (More ▼) or
set as defaults via a **compare profile** in Settings.

---

## Diff algorithms

The algorithm controls how the comparison engine finds the minimum edit
distance between two files.

| Algorithm | Characteristics |
|-----------|----------------|
| **Myers** (default) | Fast, widely supported, good for most source code. Tends to produce small diffs but can show unintuitive alignments on repetitive content. |
| **Patience** | Slower but often more readable. Anchors on unique lines first, which avoids the "staircase" effect in repetitive code. Good for refactored files. |
| **Histogram** | A refinement of Patience, typically produces the most human-readable diffs for both prose and structured code. Recommended for mixed-content files. |

**Switching:** dropdown in the advanced toolbar, or choose an algorithm in a compare profile so it applies automatically when you open a comparison.

---

## Ignore whitespace

When enabled, lines that differ only in whitespace (indentation, trailing spaces,
or spaces inside the line) are treated as equal and do not appear as changes. An
added or removed blank line still appears as a change.

Useful for:
- Comparing files reformatted by an auto-formatter
- Reviewing content changes in YAML/JSON where indentation was adjusted
- Diffing Python code after a `black` run

Toggle: **Ignore whitespace: off/on** in the advanced toolbar.

---

## Ignore case

When enabled, uppercase and lowercase differences are invisible to the diff engine.
The line `Hello World` and `hello world` are treated as equal.

Useful for:
- Case-insensitive file comparisons (Windows path strings, SQL keywords)
- Reviewing prose where capitalisation changed without semantic meaning

Toggle: **Ignore case: off/on** in the advanced toolbar.

---

## Combining options

Ignore whitespace and ignore case can be combined. A line like `  Hello  ` and
`HELLO` would be equal under both options simultaneously.

---

## Context lines

Controls how many equal lines are shown around each change before the rest
collapses. A collapsible divider (···) marks the hidden region; click it to expand.

| Value | Effect |
|-------|--------|
| `0`   | **Never collapse** — show the entire file, every equal line included |
| `3`   | Default — 3 lines of context above and below each change |
| `5`   | More context, useful for reviewing function-level changes |
| `10`  | Maximum context, useful for auditing intent in long stable sections |

Set in Settings → Advanced → "Context lines".

---

## Inline (character-level) diff

For **replace** hunks (a line was modified rather than purely added or deleted),
ForskScope can show exactly which characters changed within the line.

Toggle: **Inline diff: off/on** in the advanced toolbar.

A compare profile's **More options ▸** sets when character-level highlighting
turns on for a new comparison opened under that profile:

- **When I turn it on** (default) — off until you click the toggle, as above.
- **Automatically for small changes** — on from the moment a new tab opens.
- **Never** — the toggle itself is disabled, with the tooltip "Character-level
  highlighting is not available for this comparison." (the same generic
  tooltip shown whenever the toggle is unavailable, for any reason — it does
  not name the profile as the specific cause); this comparison never shows
  character-level highlighting.

Opening a tab is the only thing a profile's setting affects — an already-open
tab keeps whatever the toggle is set to until you change it yourself.

**Only a long, genuinely different pair is skipped, and it says so.** The limit
is **2,000 characters**, applied to what's actually different: ForskScope
trims the common prefix and suffix first, so a long line changed in one spot
— a minified line with one value edited — stays cheap and gets highlighted
however long the line is. A pair still over the limit after trimming is shown
as plain text with a **Long line** badge, and the toolbar warns "Some hunks
were too large for character-level diff." — so a line without highlighting is
either identical character for character or carries that badge, never
silently unexamined. The cost of refining a pair grows with the square of the
length of what differs — measured in a release build on `similar` 3.2.0,
about 0.1 ms for a 200-character difference, 1.1–1.4 ms at 1,000 and 4.3 ms at
2,000 — and the view refines every changed pair it shows, so a file with
hundreds of lines that are each broadly different near the limit pauses the
window when you turn the toggle on (250 such pairs at 1,900 characters took
1.3 s to appear; 1,000 pairs, 4.9 s). A file with the same number of long
lines each changed in only one place does not pause at all.

**Large files have it switched off.** For files over 512 KiB ForskScope shows
"Large file — inline diff disabled." and the **Inline diff** button is
disabled. See [Known limitations](../users/known-limitations.md#inline-diff-skips-lines-that-differ-broadly-and-is-off-for-large-files).

---

## Compare profiles

A compare profile is a named set of diff options. ForskScope ships with four:

| Profile | Algorithm | Ignore whitespace | Ignore case |
|---------|-----------|-----------|-------------|
| Exact (default) | Myers | No | No |
| Ignore whitespace | Myers | Yes | No |
| Ignore case | Myers | No | Yes |
| Histogram | Histogram | No | No |

You can create your own profiles in Settings → "Compare profiles". The active
profile is applied when you open a new comparison. Existing tabs are unaffected
until you toggle options on them directly.

A new profile's visible fields are a name, ignore whitespace, ignore case,
ignore line-ending differences, and the algorithm. **More options ▸**, collapsed
by default, adds a setting for when character-level highlighting turns on — see
[Inline (character-level) diff](#inline-character-level-diff) below.
