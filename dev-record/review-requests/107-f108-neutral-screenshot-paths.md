# Review Request 107 — F108 follow-up: neutral screenshot paths

Handoff: review 108 §3 (`dev-record/reviews/108-f108-japanese-store-listing-review.md`).
Commit: `dc01adf` (pushed to `main`). CI run `35168798003` confirmed green.

## What changed

Both `en-us/screenshots/02-explorer-view.png` and
`ja-jp/screenshots/02-explorer-view.png` showed this sandbox's real home
directory and this repository's internal scratch folder in the
breadcrumb. Re-staged the same fixture content
(`tests/fixtures/text/left_all_hunk_kinds.txt`/`right_all_hunk_kinds.txt`,
`README.md`, `left_deletions.txt`/`right_insertions.txt` — unchanged from
before) under `/tmp/forskscope-demo/{old,new}_dir` and re-captured both
languages against it.

## Confirmed by looking at the result

- **English** (`en-us/screenshots/02-explorer-view.png`): breadcrumb on
  both panes reads `tmp / forskscope-demo / old_dir` and
  `tmp / forskscope-demo / new_dir`. No home directory, no `.git-exclude`,
  no username.
- **Japanese** (`ja-jp/screenshots/02-explorer-view.png`): same neutral
  path, full Japanese UI (エクスプローラー, 左のファイルまたはフォルダーを選択して比較, 比較,
  ローカルのみ).
- **Both diff-view screenshots checked too**, per §3 item 3: neither
  `01-diff-view.png` shows a path — the tab header and file labels read
  only `.../old_config.toml` / `.../new_config.toml` (English) and
  `.../sample.txt` (Japanese), matching what review 108 §2 already
  confirmed for the Japanese one.

Same window size (2569×1426, unchanged) and capture method
(`niri msg action screenshot-window`, maximized column) as every prior
screenshot in this listing. English UI used for the `en-us` capture.

## Scope

Exactly the two files review 108 §3 named:
`en-us/screenshots/02-explorer-view.png`,
`ja-jp/screenshots/02-explorer-view.png`. Nothing else touched — confirmed
by `git status`/`git diff --stat` showing only these two binary
replacements, same dimensions, before committing.

## Gates

`cargo xtask version-sync`, `git diff --check`, `mdbook build docs` — all
clean (no text file changed, so nothing else applies). CI run
`35168798003` for `dc01adf` confirmed green.
