# Threat Model and Security Notes

This document records the security posture of ForskScope at **v0.171.1**, the
data flows that carry risk, the controls in place, and the known residual
concerns. It is a living document; update it when a new data flow is added.

**Review stamp.** Last full revision 2026-09-24, against commit `2bb7ca8`
(the previous stamp was v0.165.0, 2026-08-01). That revision corrected the
`.xlsx` sections, the script-evaluation claim, the transport dependency chain,
and added the write path (§7), script evaluation (§9) and distribution (§8).
**Sections 2, 3, 4, 5 and 6 were not re-audited in that revision**; they are
as they stood at v0.165.x–v0.170.2. Where this document says a property was
"verified", the date and the method are stated next to it.

---

## Fundamental position: local-only, no external network service

ForskScope has no application-authored external network workflow. It does not
phone home, load remote resources, upload files, or provide a remote API.
Dioxus desktop uses a loopback WebSocket transport between the embedded WebView
and the native host process; that framework transport is accepted and constrained
below. The threat surface is therefore limited to:

1. **Local file I/O** — reading files the user points it at.
2. **Child process execution** — external tools launched by the user.
3. **Settings persistence** — a single JSON file in the platform config dir.
4. **Batch-copy manifest** — a JSON file in the platform data dir.
5. **Background task safety** — async load+diff tasks writing back to UI state.
6. **Local WebView transport** — Dioxus desktop loopback WebSocket IPC.
7. **Parsing user-supplied `.xlsx` archives** — the one third-party parser that
   reads file content (below, under "Enabled third-party parser").
8. **Saving files** — the only operation that can destroy user data (§7).
9. **Script evaluation in the WebView** — `document::eval` (§9).
10. **Distribution** — how a build reaches a user (§8).

There is no product/user authentication surface, no session tokens, no cookies,
no persisted user secrets, and no user-account data.

---

## Data flows and controls (v0.165.0)

### 1. File load and diff (`open_compare`, `reload_tab`)

**Flow:** user picks two paths → `load_path()` reads bytes → `classify()` sniffs
8 KB for NUL bytes → text decoded → `compute_diff()` → `MergeSession` built →
written to `Signal<Vec<CompareTab>>` via a `spawn_blocking` task.

**Controls:**
- File I/O runs in `tokio::task::spawn_blocking`, off the UI thread. The app
  remains responsive regardless of file size.
- Each load captures a process-local `(CompareTabId, LoadGeneration)` token.
  Completion resolves the live tab by ID, then requires the exact generation
  and `TabState::Loading` before writing either success or failure. Closing a
  tab, replacing its vector position, or starting a newer reload invalidates an
  obsolete task; its result is silently dropped.
- `load_path` uses `allow_missing: true` — a missing file is a valid one-sided
  input, not an error.
- Binary comparison is gated by `enable_binary_comparison` (default off). If
  off, `load_and_diff` returns an `Err` string before any diff is computed; the
  tab shows `TabState::Error`. This prevents silent production of meaningless
  hex diffs.
- Text vs. binary cross-comparison (one side text, other binary) is blocked with
  a clear error message.
- `.xlsx` files are **parsed** since v0.169.0 (`d492557`, RFC-085): a pair of
  workbooks goes through `sheets-diff` 2.5.0 (`calamine 0.36.1`,
  `quick-xml 0.41.0`, `zip 8.6.0`) under the bounds set in
  `crates/forskscope-core/src/xlsx.rs`. A comparison that cannot finish — a
  corrupt workbook, or one that reaches a size bound — is shown as an error,
  never as an identical result. What the parser defends against and what it
  does not is under "Enabled third-party parser" below.
- No user-supplied path is ever executed as a shell command or passed through
  `sh -c`. Paths are opened with `std::fs::File::open`, not via a shell.

**Residual concerns:**
- Path traversal: `load_path` follows symlinks. A symlink farm could cause
  ForskScope to read files outside the user's intended scope — but the user
  supplies the paths explicitly, so this is within normal OS file-permission
  semantics. No escalation beyond the user's own permissions is possible.
- Very large files: the diff engine has a deadline policy (RFC-012) that may
  produce approximate results; a warning banner is shown. Whether a crash or
  panic path exists from oversized input is unverified in general — no fuzzing
  or property-based testing exists in this repository. **One was found and fixed
  (F120, 2026-09-24):** the Inline diff toggle called
  `forskscope_core::diff::refine_pair` on every changed line pair with no length
  bound, and a 400,000-character pair aborted the process ("memory allocation of
  518405760016 bytes failed" — measured by calling the function directly,
  **against `similar` 3.2.0, not the 3.1.1 the workspace locks; not re-run on
  3.1.1**; the timings first quoted here, 11 s at 100,000 and 0.4 s at 20,000,
  were 3.2.0's too and are superseded below). `refine_pair` now returns `None`
  for a side over `MAX_INLINE_CHARS_PER_SIDE` (2,000), the row shows a **Long
  line** badge, and the toggle is disabled for files over 512 KiB. **Not
  bounded:** the aggregate. On the locked `similar` 3.1.1 a pair costs about
  225 ms at the 2,000-character limit (release build; 45 ms at 1,000, 11 ms at
  500), the view renders every changed pair, and 250 pairs of 1,900 characters
  took 49 s from the toggle to the first paint (F124, measured end to end). That
  is a freeze, not a crash.

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

### 4. Settings and session persistence

**Flow:** `forskscope-core::persist::schema` is the sole owner of the on-disk
settings/session schema. `SettingsRepository`/`SessionRepository`
(`persist::schema::settings`/`session`) read and write an explicit path —
`forskscope-ui` resolves that path to the platform config directory
(`dirs_next::config_dir()`) as `settings.json`/`session.json` and never
serializes either document itself. At startup, `resolve_and_commit` loads the
file, classifies it (`PersistenceLoad`: `Missing`/`Current`/`MigratedLegacy`/
`FutureVersion`/`Corrupt`), and commits a legacy migration immediately if one
applies, through the same temp-file-then-rename primitive `save.rs` uses —
visibility-atomic (a reader never sees a partial file), not a power-loss
guarantee (F9/N2; no `fsync`/`sync_all` anywhere in `forskscope-core`).

**Controls:**
- Reads and writes use standard serde-json through a versioned envelope
  (`schema_name`, `schema_version`, `payload`); no `unsafe` or raw pointer
  manipulation.
- A v2 payload's required fields (`theme`, `language`, `tabs`, etc.) have no
  `#[serde(default)]` — a payload missing one is `Corrupt`, not silently
  defaulted. This is deliberate: architecture-audit finding B2 existed
  *because* the previous `app_json_settings::ConfigManager` path collapsed
  every unrecognized shape into defaults.
- A future-schema-version or corrupt file is left byte-for-byte untouched on
  disk and the run's writes are disabled (`write_disabled`) until the user
  takes an explicit recovery action — a blocking dialog, not a toast, since a
  write-disabled session that silently discards edits is a data-loss surface
  in its own right (RFC-076 "User-facing behavior").
- A legacy (pre-schema-v2) file is migrated and its original bytes preserved
  as a non-overwriting `<name>.pre-v2.bak` sibling before the new envelope is
  written (`ensure_pre_v2_backup`); the migrated file is re-read with
  `verify_unchanged` immediately before that commit, refusing to overwrite if
  the file changed underneath the migration (review 037 N1).
- The recovery dialog's explicit "Reset and back up" action (offered only for
  `CorruptPreserved`, never for a future-version file, which may be valid to a
  newer build) backs up the corrupt original under a distinct
  `<name>.reset.bak` name before writing — `reset_with_backup`, gated by the
  same `verify_unchanged` stale-caller guard as a migration commit.
- Settings and session resolve and report independently: if both are
  simultaneously write-disabled, both dialogs are shown in sequence
  (`pending_recovery` queue), never one silently dropped by the other.

**Residual concerns:** none beyond normal file-system write failures, which
disable further writes for the run and are surfaced to the user rather than
retried silently. This closes architecture-audit finding B2; see RFC-076
(Versioned Runtime Settings and Session Persistence, `rfcs/done/`) for the
full design and implementation record.

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

### 6. Publishing credential: Microsoft Store client secret (CI, not the application)

**This is not an application data flow** — it does not run on a user's
machine and ships in no release artifact. It is recorded here because it is
this project's highest-impact credential: its compromise lets someone submit
a package to the Microsoft Store under ForskScope's own Store listing.

**Flow:** `.github/workflows/store-submit.yml`'s `publish` job (RFC-079)
authenticates to the Microsoft Store submission API with an Entra ID
client-credentials flow — tenant ID, client ID, and client secret — then
creates, uploads to, and commits a Store submission.

**Controls:**
- The secret (`STORE_CLIENT_SECRET`, alongside `STORE_TENANT_ID` and
  `STORE_CLIENT_ID`) lives only in the `store-publish` GitHub Environment,
  never a plain repository secret. Only a job that explicitly references
  that environment can read it; `build_and_validate` (the job that builds
  and installs the package) does not, and never requests it, so a
  validation failure never even touches the credential. The application ID
  the API calls also need is **not** among these — it is the public Store
  ID, tracked as `store_id` in `store-listing/en-us/identity.toml` (F105),
  not a credential this environment gates.
- **What the environment does *not* narrow, unlike `aur-publish`
  (RFC-081):** a Store dry run still authenticates and reads Partner
  Center, because Partner Center has no anonymous read the way the AUR's
  git remote does. The `store-publish` environment therefore gates the
  dry-run and real paths equally — a rehearsal is not credential-free here.
- The Entra ID app registration is the project's existing one (RFC-079 §9
  Q4, closed 2026-09-08) — scoped to whatever Partner Center permissions
  submission requires, not broader.
- A dedicated key was not created for this credential the way RFC-081's AUR
  key was; the same caveat applies in reverse here by construction — Entra
  ID app registrations are not shared with any other credential in this
  project, so there is no separate "dedicated vs. personal" question to
  raise.

**Residual concerns:**
- **Compromise impact:** whoever holds this secret can submit arbitrary
  packages to the Store under ForskScope's listing. Certification is
  Microsoft's own backstop against a malicious *binary*, but a submission
  that passes certification would still reach users as an official update.
  This is the reasoning for the environment gate above, not a claim that
  gate is sufficient on its own — a compromised secret with no required
  reviewer on the environment could still submit before anyone notices.
- **Expiry:** Entra ID client secrets expire at 24 months at the most, often
  less under tenant policy, and a lapsed secret breaks submission silently
  at whatever release happens to land after expiry — `store-submit.ps1`
  reports the auth failure with expiry named as the likely cause, but that
  is a loud failure *at* expiry, not a warning *before* it.
  **`STORE_CLIENT_SECRET` expires: `<owner fills in when the secret is
  created>`.** Nothing in this repository can supply that date — it is
  known only inside Partner Center at creation time — so this line is the
  place it must be recorded once it is, and the place to check before
  assuming a submission failure is a code problem.

### 7. Saving: the write path

This is the one place the tool can destroy user data, and it carries the
strongest controls in the codebase. Everything below is `crates/forskscope-core/
src/save.rs` unless stated, checked against the code and against a probe run
on 2026-09-24 (Linux, `umask 022`; **Windows and macOS behaviour was not
probed**).

**Flow:** the UI builds a `SaveRequest` (target, content, encoding, BOM,
`TargetPrecondition`, `BackupPolicy`) → `save_text` → precondition check →
encode → optional `.bak` copy → commit. Settings and session files reuse the
same `atomic_replace` primitive (§4).

**Controls:**
- **The temp file is created unpredictably.** `atomic_replace` and
  `persist_noclobber` create their temp file with `tempfile::Builder` in the
  target's directory: a random name opened exclusively, never a predictable
  sibling. The earlier `.{name}.fsk-tmp` scheme let a pre-created symlink at
  that name redirect the write (CWE-59/CWE-378, F89/RFC-082 §D5; the reasoning
  is in `save.rs`'s doc comment on `atomic_replace`). A dedicated
  attack-regression test,
  `atomic_replace_does_not_follow_a_pre_created_symlink_at_the_old_predictable_temp_path`
  (`tests/save_tests.rs`), reproduces the attack and asserts the victim file is
  untouched and the target stays a regular file.
- **The commit is a rename.** A reader sees the old file or the new one, never
  a torn write. This is *visibility* atomicity, not durability: no `fsync` is
  called on the file or its directory (F9/N2).
- **A save that must not overwrite cannot.** `MustBeAbsent` checks with
  `symlink_metadata`, not `exists()`, so a dangling symlink counts as present and
  a genuine read failure propagates instead of reading as "absent" (review 046
  N1); the commit is `persist_noclobber`, and if the platform cannot provide it
  the error propagates — there is no fallback to an overwriting write (RFC-077).
- **A lossy save is refused before anything is touched.** If the target encoding
  cannot represent the content, `save_text` returns `CoreError::Encode` *before*
  the backup step, because the backup clobbers an existing `<name>.bak` (F87).
- **A stale save is refused.** `MustMatch` compares the on-disk fingerprint with
  the one captured at load (missing / changed / replaced are all conflicts).
- **`.xlsx` is never written.** Spreadsheet comparison is read-only.

- **The backup is never written through a link** (F121 A, 2026-09-24).
  `BackupPolicy::SiblingBak` (the default) removes an existing `<name>.bak` — a
  file, or a symlink itself, never its target — and creates the backup with
  `create_new` (`O_EXCL`), which refuses any existing entry, including a link
  re-created in the gap: that fails the save instead of following it. Before
  this, `fs::copy` opened the backup path with `O_TRUNC` and wrote through a
  symlink (the F89 class on a predictable name): with `doc.txt.bak ->
  victim.txt` beside `doc.txt`, `victim.txt` ended up holding `doc.txt`'s
  previous content. Regression test:
  `backup_does_not_write_through_a_pre_created_symlink_at_the_bak_path`. The
  backup keeps the source's mode.
- **A save keeps an existing file's permission bits** (F121 B, unix). The
  replacement is created `0666 & ~umask` (F38: right for a *new* file) and, when
  the target exists, gets the target's mode — including setuid, setgid and
  sticky — before the rename. Before this a `0600` file was `0644` after a
  save.
- **The precondition is checked again inside the commit** (F121 C): after the
  temp file is written and immediately before the rename, so the window is the
  gap between that check and `rename(2)` rather than the whole span of encoding,
  backup and temp write.

**What the guarantee does not cover.** Observed on Linux (`umask 022`) unless
marked; **Windows and macOS were not probed**, so none of the below is claimed
for them, and the mode carry is `cfg(unix)` code that has type-checked for the
Windows target but never run on macOS.
- **Ownership, ACLs and extended attributes are lost on every save.** The
  replacement is a new file: it is owned by the saving user and takes the
  directory's default group; POSIX ACLs and xattrs (including SELinux contexts,
  and on macOS quarantine and Finder tags) are not carried. Probe: a file with
  `user.f121=keepme` had no xattr after a save. Carrying them needs privileges
  or a dependency, and was not attempted; the mode is carried because it is the
  one that decides who can read the file.
- **Saving through a symlink replaces the link.** Probe: after a save to
  `link.txt -> real.txt`, `link.txt` was a regular file and `real.txt` was
  unchanged. Safe against write-through, and not what a user editing "the file
  the link points to" expects.
- **A hard link is split.** Probe: after a save to `a.txt`, its hard link
  `b.txt` still held the old content.
- **`MustMatch` and `Force` are still check-then-rename, narrowed but not
  closed.** A path has no compare-and-swap. After F121 C a process that writes
  the target between the final check and the rename is still overwritten — that
  gap is microseconds rather than the whole save. The check compares length and
  modification time (`check_external_state`), so an edit that preserves both is
  not seen. `Force` has no precondition by definition. Only `MustBeAbsent` has a
  race-free commit. A real fix (a lock or a platform primitive) is a design
  decision that was not made.
- **A conflict found by the final check can leave a fresher `.bak`.** The backup
  runs before the commit, so when the final check refuses the save the backup has
  already replaced the previous one — with the file's *current* content, which is
  the data the refusal protected.
- **No durability** (above): a power loss after a save can lose it, because
  nothing is flushed.

### 8. Distribution

How a build reaches a user is part of the threat surface: the project's
publishing credentials (§6 for the Store, and the AUR key below) let someone
substitute a build. Checked 2026-09-24.

| Channel | What the user receives | What vouches for it |
|---|---|---|
| Microsoft Store | An MSIX the project submits **unsigned**; Microsoft signs it after certification. No code-signing certificate of this project's own exists (`packaging/windows/README.md`, RFC-079 §2a). | Microsoft's certification and signature. |
| GitHub release, Windows zip | `forskscope.exe` in a zip. **No signing step exists in any workflow** (no `signtool`, GPG, cosign or attestation anywhere under `.github/`). | Only the release's own digest. |
| GitHub release, macOS DMG | Not signed with an Apple Developer ID and not notarized (`installation.md`). | Only the release's own digest. |
| GitHub release, Linux tarball | Unsigned. | Only the release's own digest. |
| AUR | A `PKGBUILD` (and `.SRCINFO`) pushed by `aur-publish.yml`. `sha256sums=('SKIP')` in the repository; the real source hash is computed at publish time from GitHub's own archive of the tag. | The workflow, then the AUR account holding the SSH key. |

**Controls:**
- A release is created as a **draft** by the tag-triggered `release.yml`; it does
  nothing user-visible until the owner publishes it, and both the Store and AUR
  workflows trigger on *publication*, never on the tag push. A published version
  is immutable by policy (`release.md`).
- The AUR key lives in the `aur-publish` GitHub Environment; only `PKGBUILD` and
  a regenerated `.SRCINFO` are pushed, after the package has been built,
  installed and run through `namcap`; the AUR host key is pinned in the
  workflow, not fetched.
- Each release asset carries a SHA-256 digest that GitHub computes on upload
  (`gh release view <tag> --json assets`).

**Residual concerns:**
- **Integrity and origin are the same anchor.** The only digest a Windows-zip,
  DMG or tarball user can compare against is published by the same GitHub
  account that publishes the file; nothing independent signs a build. Anyone who
  can replace a release asset can replace its digest. The digest is on the
  release's asset list, **not in the release notes**, which `installation.md`
  ("listed with their digests in each release's notes") had said.
- **Unsigned binaries teach users to click through warnings.** macOS users are
  told to clear the quarantine attribute (`installation.md`), which removes the
  operating system's check for that build entirely.
- **The AUR hash is derived, not verified.** Because the source hash is computed
  from the tag's own archive at publish time, it detects a later change to that
  archive, not a substituted one.
- **Compromise of the AUR key, the Store credential (§6), or the GitHub
  repository each reaches users as an official update.** §6 records the Store
  case; this document does not record whether the `aur-publish` environment
  requires a reviewer, and this revision did not check.

### 9. Script evaluation in the WebView (`document::eval`)

`forskscope-ui` calls `dioxus::document::eval` at **11 sites** (counted
2026-09-24). Nothing renders untrusted HTML (no `dangerous_inner_html`, no
`innerHTML`); this is the only way application code makes the WebView run
JavaScript it composed at runtime.

| Sites | Argument | Runtime data? |
|---|---|---|
| `app.rs` (×4), `modals.rs`, `dir_pane.rs` | A fixed string (focus an element, click a button, scroll to top). | None. |
| `diff.rs` (scroll-sync installer) | `format!` with the tab index. | An integer (`usize`). |
| `diff_actions.rs` (scroll to a hunk) | `format!` with `h-{hunk_id}`. | An integer (`u64`). |
| `app.rs` (window title) | `format!("document.title = {:?}", title)` | **A string derived from the file names the user opened.** |
| `search.rs` (scroll to a match) | `format!("…getElementById({id:?})…")` | A string, but `h-{u64}` built in `forskscope-ui-logic`. |
| `about.rs` (copy diagnostics) | `format!("navigator.clipboard?.writeText({:?})", d)` | The platform report (`PlatformInfo::to_report`): version, OS, architecture, CPU count. No file data. |

**The one that matters** is the window title, because file names are chosen by
whoever made the file — including a third party whose directory the user opens.
Its argument is quoted with Rust's `{:?}` (`Debug`), which is not a JavaScript
escaper; it works because the two languages' string-literal escapes largely
coincide. That was **tested, not assumed** (2026-09-24): 21 strings —
quotes, backslashes, CR/LF, tab, U+2028/2029, other control characters, an emoji,
combining marks, a zero-width joiner, a bidi override, a BOM, `</script><img
onerror=…>`, a template-literal `` `${…}` ``, a literal that tries to close the
string (`";alert(1);//`), a single quote, private-use and non-characters,
Japanese — were formatted with `{:?}` and evaluated in a JavaScript engine in
both sloppy and strict mode. **Every one round-tripped to the identical
string**, except one: a NUL followed by an ASCII digit is written `\01`, a legacy
octal escape (a different character in sloppy mode, a `SyntaxError` in strict
mode). A NUL cannot occur in a path on Linux, macOS or Windows, so this is not
reachable from a file name.

**What was not established:**
- The result is an experiment over the classes above, not a proof, and the
  project has no fuzzing.
- `Debug`'s output format is **not a documented, stable contract**. Nothing in
  the repository pins this behaviour with a test, so a change in a future Rust
  release would not be noticed here.
- The tab title and clipboard write are inert sinks (`document.title`, a text
  clipboard write), which limits what a successful injection could do; that
  reasoning was not tested.
- The two non-string interpolations are integers by construction and were not
  examined further.

---

## What ForskScope deliberately does not do

The following properties are guaranteed by the absence of application code, not
by defensive programming:

- **No external network requests** — no `reqwest`, `hyper`, `ureq`, or app
  feature opens remote HTTP endpoints. Dioxus desktop's loopback WebSocket
  transport is the reviewed exception.
- **No telemetry or analytics** — no beacon calls, no usage counters written
  to any remote endpoint.
- **No code execution from diff content** — diffs are rendered as text with
  HTML-escaped content inside Dioxus RSX. Checked 2026-09-24: there is no
  `dangerous_inner_html` and no `innerHTML` anywhere under `crates/`.
  **This is not a claim that there is no `eval` surface.** The application
  does evaluate JavaScript in its WebView (`document::eval`, 11 call sites),
  and three of them interpolate runtime strings. §9 records what they are and
  what was and was not examined.
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
| `serde` / `serde_json` | 1.0.228 / 1.0.150 | RFC-076 settings/session schema v2: envelope parsing and payload (de)serialization in `forskscope-core` | Local serialization only; parses/writes local settings and session JSON, never network input; introduces no data flow |
| `tokio` | 1 | Async runtime + `spawn_blocking` | Standard; no network features enabled |
| `app_json_settings` | 2.4.1 | Settings persistence (production call sites being converged onto RFC-076 core repositories; see `persist::v2`) | Local JSON file only |
| `dirs_next` | * | Platform dirs | Read-only path resolution |
| `rfd` | 0.17 | File picker dialog | OS dialog; no custom code |
| `dioxus` | 0.7.9 | UI framework | Default features disabled; no devtools |
| `dioxus-desktop` | 0.7.9 | Desktop WebView host | Uses authenticated loopback WebSocket IPC between WebView and host |
| `tungstenite` / `native-tls` | 0.28 / 0.2 | Dioxus desktop transport dependency | Accepted only via `dioxus-desktop`; no app-authored remote connections |
| `quick-xml` | 0.39.4 | Wayland protocol code generation through GTK/Dioxus stack | Build-time/proc-macro path; not reachable from user-supplied files. Carries the two advisories ignored in `.cargo/audit.toml` |
| `sheets-diff` | 2.5.0 | `.xlsx` structural comparison (RFC-085, re-enabled in v0.169.0); this project's own crate | **Parses user-supplied workbooks.** Bounded by `CellBounds` and `Limits::hardened()`; see "Enabled third-party parser". Immediate dependent: `forskscope-core` only (`audit-deps` asserts it) |
| `calamine` | 0.36.1 | Workbook reader under `sheets-diff` | **Parses user-supplied XML and archives.** Materialises a whole sheet before any bound counts a cell. Immediate dependent: `sheets-diff` only |
| `quick-xml` | 0.41.0 | XML parsing under `calamine` | **Reachable from user-supplied files.** Not covered by the `.cargo/audit.toml` ignore list (which names 0.39 only) |
| `zip` | 8.6.0 | Archive reading under `calamine` | **Reachable from user-supplied files.** Compressed size is bounded (50 MiB); expansion is not |
| `rustls` | 0.23.45 | TLS library compiled into `tungstenite` (v0.171.0: RUSTSEC-2026-0285) | Framework transport only, see "Accepted local WebView transport"; not reachable from file content |
| `tempfile` | 3.27.0 | RFC-077: promoted from a `forskscope-core` dev-dependency to a normal one for the Git mergetool save target's no-clobber commit (`save::persist_noclobber`, using `NamedTempFile::persist_noclobber`) | Local filesystem only — creates a same-directory temp file and commits or discards it; no network data flow; re-audited with `cargo xtask audit-deps` and `cargo audit` after promotion, no new advisories |

### Accepted local WebView transport

Dioxus desktop depends on `tungstenite` and `native-tls` because its WebView
runtime uses a loopback WebSocket channel for edit and event transport between
the embedded WebView and the native host process. This is not an application
feature for network file access, telemetry, remote API calls, or user-visible
sync.

ForskScope disables Dioxus default features and depends on `dioxus-desktop`
directly without its default `devtools` feature. `wry/devtools` is enabled only
to provide the WebView method surface required by `dioxus-desktop` release
builds; `dioxus-devtools` remains inactive and ForskScope removes the default
Dioxus menu bar so the framework devtools toggle is not exposed as product UI.
`tungstenite` 0.28.0 compiles in **both** TLS backends, so there are two
reviewed residual paths (checked 2026-09-24 with `cargo tree -i`):

```text
native-tls -> tungstenite -> dioxus-desktop -> forskscope-ui
rustls     -> tungstenite -> dioxus-desktop -> forskscope-ui
```

(`rustls-webpki` sits under `rustls` on the same path.) v0.171.0 updated
`rustls` from 0.23.41 to 0.23.45 for RUSTSEC-2026-0285, published 2026-09-14: the
old version accepted some TLS 1.3 handshake messages that should have been
encrypted when a peer sent them in plaintext. The handshake stayed
authenticated. The path is framework transport, not file content.

The release gate `cargo xtask audit-deps` asserts that `dioxus-devtools` is not
active, that `tungstenite`/`native-tls` remain limited to the reviewed
`dioxus-desktop` path, and that common external HTTP client/server crates
(`reqwest`, `hyper`, `ureq`) are absent. Since F121 D (2026-09-24) it also asserts that `rustls`'s only immediate dependent
is `tungstenite` and `rustls-webpki`'s is `rustls`. **It now queries every
target's graph** (`cargo tree --target all`): before, it saw only the host's,
and `rustls` — compiled for the Windows and macOS builds that ship — was not
in the Linux graph at all, so a crate that appeared only in a shipped-platform
build would have passed every assertion. If a future dependency introduces
another network-capable path, update this threat model under S-001 before
release.

### Enabled third-party parser: `.xlsx`

**This replaces the section that said the parser was disabled.** That text was
true from v0.165.0 until `d492557` (v0.169.0, 2026-09-05) and was wrong for the
three releases after it. The lift is recorded, with its four conditions, in the
2026-09-24 amendment to `rfcs/done/058-spreadsheet-xlsx-structural-diff.md`.

**What it parses.** Two user-selected `.xlsx` files: a ZIP container of XML
parts (sheets, shared strings, workbook metadata), through
`sheets-diff -> calamine -> quick-xml, zip`. Formula text is read as text and
never evaluated. `.xlsx` is read-only in every path.

**What defends it:**
- The dependency chain (`quick-xml 0.41.0`, `zip 8.6.0`) carries no advisory
  known to `cargo audit` today; `audit.yml` re-checks daily. `quick-xml 0.39`
  remains only through `wayland-scanner`, and `cargo xtask audit-deps` asserts
  both facts (the immediate dependents of `sheets-diff`, `calamine`, `zip` and
  both `quick-xml` versions).
- **Bounds** (`CellBounds` and `Limits::hardened()` in `xlsx.rs`): input file
  50 MiB, checked before any read; 256 sheets; 1,000,000 differences;
  **2,000,000 cells compared and 4,000,000 cells read**. The cell bounds were
  measured (release build, two identical single-sheet numeric workbooks,
  unbounded): 1.6 s and 0.97 GB at 1,000,000 coordinates, 2.7 s and 1.9 GB at
  2,000,000, 7.1 s and 4.8 GB at 5,000,000 — about 1 KB of peak memory per
  coordinate. `Limits::hardened()`'s own 5,000,000 would admit ~4.8 GB and was
  not used.
- **A bound that is reached is an error.** The comparison stops and the tab shows
  it. Until F117, an error in this path was turned into two empty documents and
  displayed as "identical".
- **Cancellation**, polled every 50,000 cells in `sheets-diff`'s read and compare
  loops, is tested mid-comparison and falsified.
- `AlignmentMode` is `Positional`, the default and the cheapest.

**What it does not defend against:**
- **A 5 KB workbook can abort the process (measured 2026-09-24, F123).**
  `calamine` builds a dense range over the bounding box of the *populated* cells
  before `sheets-diff` counts a cell, at about 31 bytes per box cell. A workbook
  with one cell at `A1` and one far away — hostile or an accidental stray
  cell — costs memory in proportion to the area between them: 100M cells took
  3.13 GB and 1.2 s before F117's bound refused it; 300M took 9.38 GB and 3.1 s;
  Excel's maximum sheet (1,048,576 × 16,384) makes the allocator request 512 GiB
  and **aborts the process** (`memory allocation of 549755813888 bytes failed`),
  losing unsaved work in other tabs. The full table is in the RFC-058
  amendment. A declared `<dimension>` with no far cell is harmless. **Open:** the
  fix belongs in `sheets-diff` (stream cells into its sparse map, bound and poll
  inside the loop); there is no in-crate route, since `forskscope-core` has no
  direct `calamine` access.
- **A densely populated sheet costs its parse before refusal.** 20M populated
  cells in a 51.5 MB file (under the 50 MiB input bound) were refused after
  5.9 s at 2.58 GB.
- **An uninterruptible parse.** Cancelling 1 ms into the 300M-area case returned
  after 3.33 s, its full uncancelled duration.
- **Advisories published later** against `quick-xml`, `zip` or `calamine`.

The reviewed `quick-xml 0.39` advisory exceptions are recorded in
`.cargo/audit.toml`; they do not cover `0.41.0`, which `cargo audit` reports on
its own merits.

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
| v0.165.0 | Async compare load-token guard (RFC-075) — **supersedes the v0.148.0 row's claim**, which audit finding B1 established was insufficient | Stable `CompareTabId` + per-load `LoadGeneration` prevent integrity failures in which content from one user-selected path pair is displayed or saved under another tab identity; a stale-tab check alone did not guard against a newer reload on the same tab |
| v0.165.0 | XLSX parser path removed; `.xlsx` comparison fails closed | Removes runtime user-supplied workbook XML exposure to vulnerable `quick-xml` path |
| v0.165.0 | Dioxus desktop dependency policy reviewed | Accepts loopback WebSocket IPC only; `cargo xtask audit-deps` enforces no devtools and reviewed network-capable paths |
| v0.165.0 | Release UI build compatibility with `dioxus-desktop`/`wry` | Enables `wry/devtools` method surface without `dioxus-devtools`; removes default Dioxus menu bar |
| v0.165.0 | Release archive and CI gates aligned | Archive layout, version sync, i18n coverage, audit policy, and dependency paths are enforced before release artifact creation |
| v0.165.1 | Versioned settings/session persistence (RFC-076) — closes audit finding B2 | Core owns a schema-versioned envelope; a future-version or corrupt file is preserved untouched and reported via a blocking recovery dialog rather than silently collapsed to defaults; legacy migration and explicit reset both create a non-overwriting backup before any write |
| v0.169.0 | `.xlsx` comparison re-enabled (`d492557`, RFC-085) on `sheets-diff` 2.5.0 | **Re-opens a parser to user-supplied archives** (`calamine 0.36.1`, `quick-xml 0.41.0`, `zip 8.6.0`); the suspension's lifting was not recorded here or in RFC-058 until 2026-09-24. `audit-deps` changed from asserting the chain absent to asserting it present |
| v0.170.2 | Microsoft Store submission automation (RFC-079) — publishing credential recorded | New CI-only data flow (§6): `store-submit.yml`'s `publish` job holds an Entra ID client secret in the `store-publish` GitHub Environment, gated the same way RFC-081's AUR key is, except a Store dry run cannot be credential-free (no anonymous Partner Center read exists) |
| v0.171.0 | `rustls` 0.23.41 → 0.23.45 (RUSTSEC-2026-0285, published 2026-09-14) | Framework WebSocket transport only; a peer could send some TLS 1.3 handshake messages in plaintext, and the handshake stayed authenticated |
| v0.171.1 | F117: `.xlsx` cell bounds set (2,000,000 compared / 4,000,000 read); an uncomparable workbook pair is an error, not "identical" | Closes the unbounded comparison RFC-058 condition 4 required be bounded; removes a path that displayed a failed or refused comparison as a match |
| v0.171.1 | Threat-model revision (F116): write path (§7), distribution (§8), script evaluation (§9) added; `.xlsx` and transport sections corrected | Records three write-path behaviours the F89 fix did not cover (backup symlink write-through, permission widening, non-atomic `MustMatch`), all observed on Linux and **not fixed** |
| v0.172.0 | F120: character-level refinement bounded (`MAX_INLINE_CHARS_PER_SIDE` = 2,000), skipped pairs shown, Inline toggle disabled for files over 512 KiB | Closes a file-content-triggered process abort reachable from a user toggle; the aggregate cost of many near-limit pairs is not bounded |
| v0.173.0 | F121: backup no longer written through a symlink; a save keeps an existing file's mode; precondition re-checked before the rename; `audit-deps` asserts the `rustls` path and queries all targets | Closes the F89 class on the `.bak` path and a private-file exposure; narrows (does not close) the `MustMatch`/`Force` race; extends a dependency gate from the host graph to the shipped platforms' |
