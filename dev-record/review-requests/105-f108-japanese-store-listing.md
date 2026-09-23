# Review Request 105 — F108: Japanese in the Windows package and the Store listing

Handoff: `dev-record/handoffs/035-f108-japanese-store-listing.md`
Commit: `5a1bf30` (pushed to `main`). CI run `35167057084` confirmed green.

## The text: checked against the running app, nothing needed correcting

Verified each claim in §2's text against the code before committing it
verbatim, per the handoff's "if any claim is false, fix the text and say
which one":

- F7/F8 move between changes, Enter applies — `keyboard.rs:97-98,107`
  (`Key::F7 => A::MoveFocus(-1)`, `Key::F8 => A::MoveFocus(1)`,
  `Key::Enter => A::ApplyFocusedHunk`).
- Myers and Histogram are both real `DiffAlgorithmSetting` variants,
  reachable through the built-in compare profiles
  (`state/settings.rs:91-93`).
- `ignore_whitespace`/`ignore_case` are real, wired settings fields
  (`state/settings.rs:51-52`, `ui/view/diff.rs:319-320`).
- Every Japanese term in the listing matches `i18n.rs`'s own vocabulary
  exactly: 変更 (`"Changed" => "変更"`), エクスプローラー
  (`"Explorer" => "エクスプローラー"`), 設定 (`"Settings" => "設定"`),
  言語 (`"Language" => "言語"`), 大小文字無視
  (`"Ignore case" => "大小文字無視"`).

No claim needed correcting. `ja-jp/listing.toml` was committed as written.

## Live verification, not just visual inspection

Handoff §"Verification" item 4 asked to confirm F7/F8/Enter "by use," not
by reading the code alone. Launched the real binary with Language set to
日本語 and drove it with real key events (`wtype`), screenshotting each
step:

1. Baseline: focus counter reads `1 / 3`, hunk 1 (`old-line`/`new-line`)
   highlighted.
2. **F8** pressed: counter advances to `2 / 3`, focus moves to hunk 2
   (`delete-line`) — confirmed by screenshot, not assumed.
3. **Enter** pressed: hunk 2 applies — `delete-line` appears on the right
   pane where it was previously absent, a ✓ replaces the apply button,
   the tab gets an unsaved-change dot and the status bar reads `未保存`,
   and focus auto-advances to `3 / 3`. All exactly the documented
   behavior, observed live.

Both screenshotted views match what `ja-jp/screenshots/` ships.

## The manifest: CRLF preserved, packs and validates for real

`packaging/windows/AppxManifest.xml` gained exactly one line,
`<Resource Language="ja-JP" />`, after `en-US`:

```
before: 35 lines, 35 CRLF-terminated
after:  36 lines, 36 CRLF-terminated
```

**`git diff --check` reports the new line as trailing whitespace** (exit
2) — expected, and not fixed: this file has no `text eol=crlf`
`.gitattributes` entry, so git flags the `\r` on *any* newly-added line to
this file regardless of content, which is inherent to preserving the
established CRLF convention the handoff explicitly asked to keep. Fixing
it would mean stripping the CRLF, the one thing not to do here.

**Real Windows CI validation** (handoff's item 1): no release or Store
credential exists to trigger a real dispatch against this exact commit's
tag, so — matching the same scratch-branch/scratch-tag technique used for
RFC-079's own original rehearsal — a branch off this commit
(`f108-rehearsal`, deleted after use) had `AppxManifest.xml`'s `Version`
temporarily set to `0.0.1.0` (never a real release number) and was tagged
`0.0.1` (also deleted after use). Dispatched
`store-submit.yml -f tag=0.0.1 -f dry_run=true`, run `35167492914`. The
**credential-free** `Build and validate` job — `makeappx pack`, unpack,
identity/asset checks, and the real install+launch check — passed
completely:

```
OK: manifest Version matches the released tag (0.0.1.0)
OK: Identity Name matches the tracked Store identity
OK: Identity Publisher matches the tracked Store identity
OK: PublisherDisplayName matches the tracked Store identity
OK: package contains forskscope.exe
OK: package contains Assets\StoreLogo.png
OK: package contains Assets\Square150x150Logo.png
OK: package contains Assets\Square44x44Logo.png
OK: package contains Assets\Wide310x150Logo.png
OK: package installed
OK: forskscope.exe is running (PID 8344) after installing and launching the signed validation copy
```

The credential-gated `Submit to the Microsoft Store` job failed, as
expected and required — **no real submission was attempted**, matching
the explicit instruction. `main`'s `AppxManifest.xml` was confirmed
unchanged (`Version="0.171.1.0"`) both before and after the dispatch; the
scratch branch and tag are deleted, locally and on `origin`.

## TOML parsing and screenshots

```
en-us: 5 search terms, 2 screenshots
ja-jp: 7 search terms, 2 screenshots
```
Both parsed cleanly with Python's `tomllib`; every listed screenshot file
exists at its declared path; both markets stay within Partner Center's
7-term limit. Both `01-diff-view.png`/`02-explorer-view.png` pairs are
2569×1426 — identical dimensions to the English pair, captured the same
way (`niri msg action screenshot-window`, maximized column, not `import`).

**Screenshot content, disclosed**: the compared directories/files use only
repository fixtures — `tests/fixtures/text/left_all_hunk_kinds.txt` /
`right_all_hunk_kinds.txt` (the same pair `render_check.py` itself uses)
and the repository's own `README.md`, staged under
`.git-exclude/tmp/store-screenshots-ja/` — no personal file or folder
names appear anywhere in either screenshot. The window's path breadcrumb
in the Explorer screenshot does show this sandbox's own absolute path
(`/home/<user>/Desktop/...`), an unavoidable artifact of any local
capture rather than a chosen personal name; no unrelated personal
directory (a prior screenshot round's `Nextcloud`/`orbok-qa`-style
listing) appears, since the Explorer was pointed directly at the two
fixture directories rather than a real home-directory listing.

## The two other doc edits

`en-us/listing.toml`: `- Dark and light themes, adjustable density and
font size` → `- Dark, light and night themes, and adjustable font size`;
added the closing `- English and Japanese interface (starts in English;
switch in Settings → Language)` bullet. Nothing else changed — confirmed
by diff.

`store-listing/README.md`: documents both markets, states
`identity.toml` stays `en-us/`-only (language-neutral, read by
`store-submit.ps1`/`store-validate.ps1` from that exact path), and that a
further market gets `listing.toml` + `screenshots/` only.

## Scope

Held to exactly what §"Scope" named:
`AppxManifest.xml`, `store-listing/ja-jp/` (new),
`store-listing/en-us/listing.toml`, `store-listing/README.md`. Out,
untouched: Partner Center (no real submission attempted), `store-submit.ps1`/
`store-validate.ps1` (run, not edited), first-launch language detection,
`CHANGELOG.md`/`ROADMAP.md`, RFC-080, and F107.

## Gates

`git diff --check` — the one expected, disclosed CRLF finding above, not
otherwise clean-required per the handoff's own framing. `cargo xtask
version-sync` — passed (`version sync passed for v0.171.1`, the manifest
being one of its checked files). `mdbook build docs` — clean. No workflow
file touched, so `actionlint` doesn't apply to this change; ran it anyway
(clean). Also ran the project's full standard set as a matter of course
even though this handoff touches no Rust: `cargo fmt --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
(736/29/16/5/133/133/152/5/6, all unchanged and green), `cargo xtask
css/i18n/rfc-sync/audit-deps/ui-logic-connectivity/ui-logic-docs` — all
clean. CI run `35167057084` for `5a1bf30` confirmed green. Live Windows
Store-workflow dispatch `35167492914` confirmed the manifest packs and
validates for real, per the handoff's explicit ask.
