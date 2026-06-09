# File Type Support

| Kind | Detected by | Diff | Merge / Save |
|---|---|---|---|
| Text (any encoding) | No NUL bytes in first 8 KB | Line diff + inline | ✓ |
| Binary | NUL byte found | Hex preview diff | — |
| Excel `.xlsx` | `.xlsx` extension | Derived text diff | — |
| Missing | Path not found | One-sided diff | — |

Encoding detection uses `chardetng` and `encoding_rs`. The detected encoding is
shown in the status bar and preserved on save — a Shift_JIS file saves as
Shift_JIS by default.
