# Handoff 035 — F108: Japanese in the Windows package and the Store listing

**From:** architect. **Date:** 2026-09-17. **Release:** 0.172.0.
**Register:** F108 in `ROADMAP.md`.

## Why

ForskScope ships a complete Japanese UI, and `cargo xtask i18n` enforces it.
But `packaging/windows/AppxManifest.xml` declares only
`<Resource Language="en-US" />`, so the Microsoft Store lists the app as
English only.

The owner decided on 2026-09-17:
- declare Japanese in the package;
- publish a Japanese Store listing;
- have **the architect write the text and the dev team commit it**.

Publishing the listing itself stays manual and belongs to the owner, as RFC-079
Q5 decided.

While scoping this, I found that `en-us/listing.toml` advertises **adjustable
density**, a control the app does not have (review 106). It also names only
the dark and light themes, although Night exists. Both listings are corrected
here.

## 1. The manifest

In `packaging/windows/AppxManifest.xml`, add `ja-JP` **after** `en-US`. The
first entry is the default language.

```xml
  <Resources>
    <Resource Language="en-US" />
    <Resource Language="ja-JP" />
  </Resources>
```

**The file uses CRLF line endings on every line.** Keep them: the new line
must end in CRLF as well. Check with `grep -c $'\r$'` against `wc -l` before
and after.

Do not change `DisplayName` or `Description`. They are single OS-facing
strings, not localized resources, and nothing here adds a `resources.pri`.

## 2. `packaging/windows/store-listing/ja-jp/listing.toml`

Commit this text as written. It was checked against the code:
- F7 and F8 move between changes (`keyboard.rs:97-98`).
- Enter applies the focused change (`:107`).
- Myers and Histogram are both reachable through the built-in compare
  profiles.
- The ignore-whitespace, ignore-case and encoding controls exist.
- The terms follow the app's own Japanese in `i18n.rs`: 変更, エクスプローラー,
  設定, 言語, 大小文字無視.

**If any claim is false against the running app, fix the text and say which one
in your reply. Do not keep a false claim.**

```toml
# Microsoft Store listing content for ForskScope, ja-jp market.
#
# F108 (owner decision 2026-09-17): text written by the architect, committed
# by the dev team. Publishing it to Partner Center stays a manual step
# (RFC-079 §9 Q5); store-submit.ps1 never reads this file.
#
# Keep claims in step with ../en-us/listing.toml. See ../README.md.

[product]
title = "ForskScope"
publisher_display_name = "nabbisen"

# Partner Center's short/"about" field.
short_description = """
ファイルを左右に並べて比較・マージでき、フォルダー同士もまとめて比較できる、\
ローカル完結型の差分ツールです。アカウント不要、アップロードなし、テレメトリなし。
"""

# Partner Center's full description field, up to 10,000 characters.
long_description = """
ForskScope は 2 つのファイルを左右に並べて表示し、変更箇所を行単位・文字単位で\
強調します。F7 / F8 で変更を移動し、Enter で左の変更を右へ適用できます。\
エクスプローラー画面では 2 つのフォルダーを同時に比較し、一致するファイル、\
異なるファイル、片側にしかないファイルをひと目で確認できます。

すべての処理はお使いのコンピューター上で行われます。ネットワーク通信も\
アカウントもテレメトリもなく、ファイルが外部に送られることはありません。

主な機能:
- 左右に並べた差分表示と、変更ごとの適用（F7 / F8 で移動、Enter で適用）
- 変更された行の中の、文字単位の強調表示
- フォルダー全体を比較できる 2 ペインのエクスプローラー
- Git の difftool / mergetool として利用可能
- 複数の差分アルゴリズム（Myers、Histogram）と、空白の違いや大小文字を無視する比較
- 文字コードの自動判別と手動指定（UTF-16、BOM にも対応）
- ダーク、ライト、ナイトのテーマと、文字サイズの調整
- 日本語と英語の画面表示（初回は英語で起動します。［設定］→［言語］で日本語に切り替えられます）
"""

# Partner Center allows up to 7 search terms.
search_terms = [
  "差分",
  "比較",
  "ファイル比較",
  "フォルダー比較",
  "マージ",
  "diff",
  "git difftool",
]

[screenshots]
# Captured against the real, running application with Language set to
# 日本語. See the en-us file's note on why these are not native Windows
# captures.
files = [
  "screenshots/01-diff-view.png",
  "screenshots/02-explorer-view.png",
]
```

**The parenthesis in the last bullet is intentional.** The owner decided on
2026-09-17 that the app keeps starting in English, so the listing tells
Japanese users how to switch. Keep that parenthesis.

## 3. Screenshots

Add `packaging/windows/store-listing/ja-jp/screenshots/01-diff-view.png` and
`02-explorer-view.png`. They should show the same two views as the English
ones, with **Settings → Language set to 日本語**.

- Capture with `niri msg action screenshot-window`, not `import`.
- Use repository fixtures for the file and folder contents, never paths or
  names from a personal home directory.
- Use the same window size as the English screenshots.

## 4. Correct `packaging/windows/store-listing/en-us/listing.toml`

In `long_description`:
- Replace `- Dark and light themes, adjustable density and font size` with
  `- Dark, light and night themes, and adjustable font size`.
- Add a final bullet: `- English and Japanese interface (starts in English;
  switch in Settings → Language)`.

Change nothing else. The two listings must make the same claims.

## 5. `packaging/windows/store-listing/README.md`

- The bullet list describes `en-us/` only. Say that `ja-jp/` exists, with
  `listing.toml` and `screenshots/`.
- **`identity.toml` stays in `en-us/` only.** It is language-neutral
  identity, and `store-submit.ps1` and `store-validate.ps1` read it from that
  exact path. Do not copy it into `ja-jp/`.
- Replace *"Only one market (`en-us`) exists today, matching `AppxManifest.xml`'s
  single `<Resource Language="en-US" />`. A second market gets its own
  `<lang>/` sibling directory with the same three pieces."* with text that
  says:
  - two markets exist, matching the manifest's two `Resource` entries;
  - a further market gets `listing.toml` and `screenshots/`, not
    `identity.toml`;
  - the listings must make the same claims.

## Verification

1. **The manifest still packs and validates.** On Windows CI, dispatch the
   Store workflow's credential-free validation job against this commit, and
   give the run ID. If no such dispatch exists, say so, and show `makeappx
   pack` and `store-validate.ps1` output from whatever Windows run you can
   reach.
   - Do **not** run a real submission. That is the owner's manual step.
2. **CRLF is intact.** Give the counts from §1 before and after.
3. **Both TOML files parse**, for example with Python's `tomllib`. Print each
   file's number of search terms and number of screenshots, and confirm every
   listed screenshot file exists.
4. **Every claim in the Japanese text is true in the running app.** Launch it,
   switch the language to 日本語, and confirm each of these by use:
   - F7 and F8 move between changes, and Enter applies;
   - both views match the screenshots.

   Report anything that isn't as described.

## Scope

- **In:**
  - `AppxManifest.xml`
  - `store-listing/ja-jp/` (new)
  - `store-listing/en-us/listing.toml`
  - `store-listing/README.md`
- **Out:**
  - Partner Center: the owner publishes by hand.
  - `store-submit.ps1` and `store-validate.ps1`, beyond running them.
  - First-launch language detection (owner decision pending).
  - `CHANGELOG.md` and `ROADMAP.md`: the architect maintains them.
  - RFC-080 and F107, which come in the 0.172.0 handoff after 0.171.0 is
    published.

## Gates

- `git diff --check`, with the manifest's CRLF expected and noted.
- `cargo xtask version-sync`, because the manifest is one of its files.
- `mdbook build docs`.
- `actionlint`, if a workflow is touched. None should be.
- A green CI run for the final commit, with its run ID.

Reply in `dev-record/review-requests/105-f108-japanese-store-listing.md`.
