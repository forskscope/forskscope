# Threat Model and Security Notes

This document records the security posture of ForskScope at v0.152.0, the
data flows that carry risk, the controls in place, and the known residual
concerns. It is a living document; update it when a new data flow is added.

---

## Fundamental position: local-only, no network

ForskScope has no network code. It does not phone home, load remote resources,
or accept inbound connections. The threat surface is therefore limited to:

1. **Local file I/O** — reading files the user points it at.
2. **Child process execution** — external tools launched by the user.
3. **Settings persistence** — a single JSON file in the platform config dir.
4. **Batch-copy manifest** — a JSON file in the platform data dir.
5. **Background task safety** — async load+diff tasks writing back to UI state.

There is no authentication surface, no session tokens, no cookies, no
cryptographic material to protect, and no user-account data.

---

## Data flows and controls (v0.152.0)

### 1. File load and diff (`open_compare`, `reload_tab`)

**Flow:** user picks two paths → `load_path()` reads bytes → `classify()` sniffs
8 KB for NUL bytes → text decoded → `compute_diff()` → `MergeSession` built →
written to `Signal<Vec<CompareTab>>` via a `spawn_blocking` task.

**Controls:**
- File I/O runs in `tokio::task::spawn_blocking`, off the UI thread. The app
  remains responsive regardless of file size.
- The write-back guard checks `tabs.get_mut(index)` (tab still present) **and**
  `tab.state == TabState::Loading` (not already resolved or closed) before
  writing. A stale task completing after a tab is closed is silently dropped.
- `load_path` uses `allow_missing: true` — a missing file is a valid one-sided
  input, not an error.
- Binary comparison is gated by `enable_binary_comparison` (default off). If
  off, `load_and_diff` returns an `Err` string before any diff is computed; the
  tab shows `TabState::Error`. This prevents silent production of meaningless
  hex diffs.
- Text vs. binary cross-comparison (one side text, other binary) is blocked with
  a clear error message.
- No user-supplied path is ever executed as a shell command or passed through
  `sh -c`. Paths are opened with `std::fs::File::open`, not via a shell.

**Residual concerns:**
- Path traversal: `load_path` follows symlinks. A symlink farm could cause
  ForskScope to read files outside the user's intended scope — but the user
  supplies the paths explicitly, so this is within normal OS file-permission
  semantics. No escalation beyond the user's own permissions is possible.
- Very large files: the diff engine has a deadline policy (RFC-012) that may
  produce approximate results; a warning banner is shown. No crash or panic
  path exists from oversized input (fuzzing confirmed in test suite).

### 2. Directory listing and binary sniff (`list_dir`, `classify`)

**Flow:** `DirectoryTree` calls `list_dir()` which calls `classify()` per file
(reads up to 8 KB per file) to populate `FileEntry::is_binary`.

In the Explorer UI, `classify()` is also called lazily per rendered row via
`binary_cache: Signal<HashMap<PathBuf, bool>>`.

**Controls:**
- `binary_cache` is cleared in the `use_effect` hooks that fire on `left_dir`
  and `right_dir` changes, so stale results from a previous directory do not
  persist across navigation. *(Fixed in this release.)*
- The filter-bar "Hide binary" path also uses `binary_cache`, so `classify()` is
  called at most once per unique path per session. *(Fixed in this release.)*
- `classify()` reads exactly 8 KB (`SAMPLE_LEN`) and immediately closes the
  file handle. No unbounded read.
- `list_dir` errors on individual entries are silently skipped (`continue`) —
  a file that cannot be stat'd is omitted from the listing rather than causing
  a panic or error dialog.

**Residual concerns:**
- The cache is a `HashMap<PathBuf, bool>` with no eviction other than the
  directory-change clear. In a session where the user repeatedly expands and
  collapses large trees without changing directories, the map grows with every
  unique path visited. For typical directory sizes (thousands of files) this is
  negligible. If a session covers millions of unique paths, memory pressure could
  become meaningful; no mitigation exists today beyond the dir-change clear.
  Tracked for future work.

### 3. Batch-copy manifest

**Flow:** `batch_copy()` writes a JSON manifest to
`dirs_next::data_dir() / "forskscope/manifests/<op-id>.json"`.

**Controls:**
- `dirs_next::data_dir()` returns `None` on some platforms; the code checks for
  `None` and skips writing the manifest (the copy still proceeds). No panic.
- The manifest directory is created with standard `std::fs::create_dir_all`;
  the resulting file is readable only by the user's own OS account (default
  `umask`).
- Manifest content is derived entirely from the in-memory copy plan; no
  user-supplied strings are interpolated into JSON without serde serialization.

**Residual concerns:** none beyond normal file-system write failures, which are
handled as `Ok(None)` in the result path.

### 4. Settings persistence

**Flow:** `AppSettings` is serialized to JSON by `app_json_settings::ConfigManager`
into the platform config directory (`dirs_next::config_dir()`) as `settings.json`.

**Controls:**
- Reads and writes use standard serde-json; no `unsafe` or raw pointer
  manipulation.
- `#[serde(default)]` on all new fields means that a settings file written by an
  older version will deserialize cleanly without panicking.
- The file is written synchronously from the settings dialog on every change.
  No background thread; no race with the UI signal reads.

**Residual concerns:** none.

### 5. External tool launch (`core::external_tool`)

**Flow:** the external-tool module can construct a `std::process::Command` from
a `CommandDefinition` with an allowlist of argument templates.

**Controls:**
- No shell execution (`sh -c`, `bash -c`). All arguments are passed directly as
  `OsStr` elements to `Command::arg()`.
- The argument template expander (`expand_args`) substitutes only known
  placeholders (`{path}`, `{line}`, etc.) with values derived from file paths
  and line numbers — no user-supplied arbitrary strings are expanded into
  arguments.
- The tool itself is not invoked automatically; it requires explicit user action
  (a button click or keyboard shortcut).

**Residual concerns:** none beyond the user choosing to launch an untrusted
external tool, which is outside ForskScope's control.

---

## What ForskScope deliberately does not do

The following properties are guaranteed by the absence of code, not by
defensive programming:

- **No network requests** — no `reqwest`, `hyper`, `ureq`, or any async HTTP
  crate in the dependency tree. Verified by `cargo tree`.
- **No telemetry or analytics** — no beacon calls, no usage counters written
  to any remote endpoint.
- **No code execution from diff content** — diffs are rendered as text with
  HTML-escaped content inside Dioxus RSX; there is no `innerHTML` injection
  or `eval()` surface.
- **No privilege escalation** — ForskScope runs as the invoking user. It never
  calls `sudo`, `pkexec`, or platform privilege APIs.
- **No plugin loading** — no dynamic library loading, no WASM sandbox, no
  scripting engine. All behavior is compiled-in.

---

## Dependency surface

Key crates touching file I/O or process execution:

| Crate | Version | Role | Risk note |
|---|---|---|---|
| `similar` | 3.1.1 | Diff computation | Pure computation; no I/O |
| `encoding_rs` | * | Text decoding | No I/O; operates on in-memory bytes |
| `chardetng` | * | Encoding detection | No I/O |
| `tokio` | 1 | Async runtime + `spawn_blocking` | Standard; no network features enabled |
| `app_json_settings` | 2.0.3 | Settings persistence | Local JSON file only |
| `dirs_next` | * | Platform dirs | Read-only path resolution |
| `rfd` | 0.17 | File picker dialog | OS dialog; no custom code |
| `dioxus` | 0.7.9 | UI framework | Desktop WebKit; no remote URLs loaded |
| `sheets-diff` | 1.1.4 | XLSX diff | Known panic risk on malformed XLSX (RFC-058); mitigated by Result API wrapper |

### Known third-party risk: `sheets-diff` panic on malformed XLSX

`sheets-diff` v1.1.4 can panic on certain malformed `.xlsx` inputs. ForskScope
wraps the call in a `catch_unwind` boundary in `core::xlsx` (RFC-058). A
malformed file will produce an `Err` result shown as a diff error, not a crash.
This is a residual risk until the upstream crate is fixed or replaced.

---

## Audit history

| Version | Change | Security impact |
|---|---|---|
| v0.145.x | Batch copy, modal keyboard guard | `stop_propagation` prevents modal escape via keyboard |
| v0.147.0 | Per-pane horizontal scroll | Layout only; no data-flow change |
| v0.148.0 | Async compare (`spawn_blocking`) | Stale-tab guard prevents write to closed tab |
| v0.149.0 | Binary comparison off by default | Prevents misleading hex diff; `enable_binary_comparison` gate |
| v0.150.0 | Filter bar with `classify()` calls | **Risk introduced:** bare `classify()` in filter loop bypassed cache |
| v0.151.0 | Compact view mode | Tree rendering path only; no new data flows |
| v0.152.0 | Targets label; font family | UI only; no new data flows |
| v0.152.0 (this audit) | **Fix:** `binary_cache` cleared on dir change; filter loop uses cache | Eliminates stale binary detection and redundant file I/O per render frame |
