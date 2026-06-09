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

When enabled, lines that differ only in leading/trailing spaces, indentation, or
blank lines between them are treated as equal and do not appear as changes.

Useful for:
- Comparing files reformatted by an auto-formatter
- Reviewing content changes in YAML/JSON where indentation was adjusted
- Diffing Python code after a `black` run

Toggle: **Ignore WS: off/on** in the advanced toolbar.

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
| `0`   | Collapse all equal hunks — show only the changed lines and no context |
| `3`   | Default — 3 lines of context above and below each change |
| `5`   | More context, useful for reviewing function-level changes |
| `10`  | Maximum context, useful for auditing intent in long stable sections |

Set in Settings → "Context lines".

---

## Inline (character-level) diff

For **replace** hunks (a line was modified rather than purely added or deleted),
ForskScope can show exactly which characters changed within the line.

Toggle: **Inline: off/on** in the advanced toolbar.

The inline diff is computed lazily — it only runs when you enable it.
For very long lines, inline diff may be skipped (a notice appears in the toolbar).
The threshold is configurable in the core (default: 16 KB per hunk).

---

## Compare profiles

A compare profile is a named set of diff options. ForskScope ships with four:

| Profile | Algorithm | Ignore WS | Ignore case |
|---------|-----------|-----------|-------------|
| Exact (default) | Myers | No | No |
| Ignore whitespace | Myers | Yes | No |
| Ignore case | Myers | No | Yes |
| Histogram | Histogram | No | No |

You can create your own profiles in Settings → "Compare profiles". The active
profile is applied when you open a new comparison. Existing tabs are unaffected
until you toggle options on them directly.
