# F138 fixture — `oom-artifact-515b.bin`

512 bytes, all zero except the CFB (OLE2) header fields a compound-file parser
reads before anything else:

| Offset | Bytes | Value | Meaning |
|---|---|---|---|
| 0 | 8 | `D0 CF 11 E0 A1 B1 1A E1` | CFB signature |
| 26 | 2 (LE) | `0x0003` | format version |
| 30 | 2 (LE) | `0x0009` | sector shift → 512-byte sectors |
| 32 | 2 (LE) | `0x0006` | mini-sector shift (required, or the header parse errors early) |
| 44 | 4 (LE) | `0x8A000000` | **FAT length**: read unchecked and multiplied by 4 bytes/entry → a
  9,261,023,232-byte (9.26 GB) allocation request |
| 48 | 4 (LE) | `0xFFFFFFFE` | directory start = `ENDOFCHAIN` |
| 68 | 4 (LE) | `0xFFFFFFFE` | DIFAT start = `ENDOFCHAIN`, so the DIFAT-walk loop is skipped |

Everything else is zero. The header parse requires all 512 bytes to be present
— hence the file's size, not a rounder number.

`tests/f138_decline_before_delegating.rs` **rebuilds these bytes itself** and
asserts they equal this committed file, so the fixture cannot silently drift
from what this table documents, before using it to exercise the regression.

**What it is for:** `calamine` 0.36.1's CFB reader (`cfb.rs`) passed the FAT
length above straight to `Vec::with_capacity` with no bound check against the
file's actual size — this file is 512 bytes; the allocation it names is 9.26
GB. `sheets-diff` 3.2.0 fixed this by declining any input that does not begin
with the ZIP magic bytes before the parser ever sees it (real encrypted
`.xlsx` files, which are legitimately CFB containers, are still carved out by
a byte-scan, never a parse). This fixture is what proves that: opened under a
capped address space, it must return an ordinary `Err`, not abort the process.
The defect was live in every `sheets-diff` release through 3.1.0, including
what `0.172.0` shipped.
