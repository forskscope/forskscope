# Diff Options

The default comparison is exact: every character difference, including
whitespace and case, is significant.

Planned user-visible compare profiles (RFC-028):

| Option | Effect |
|---|---|
| Ignore whitespace | Whitespace-only changes treated as equal. |
| Ignore case | Case differences treated as equal. |
| Histogram algorithm | Git-style histogram diff for more readable boundaries. |

In v0.23.0, options are set through the `DiffOptions` struct in
`forskscope-core`. A compare-profiles UI is planned for a later release.
