//! Minimal localization (RFC-009 §locale).
//!
//! Keys are the English source strings; English returns the key, Japanese
//! maps known keys. Unknown keys fall back to the key itself, so missing
//! translations degrade gracefully rather than showing blanks.

use crate::state::Lang;

/// Translate `key` for `lang`.
pub fn t(lang: Lang, key: &str) -> String {
    match lang {
        Lang::En => key.to_string(),
        Lang::Ja => ja(key).unwrap_or(key).to_string(),
    }
}

fn ja(key: &str) -> Option<&'static str> {
    let v = match key {
        "Explorer" => "エクスプローラー",
        "Open Files" => "ファイルを開く",
        "Settings" => "設定",
        "Compare" => "比較",
        "Search…" => "検索…",
        "o, class, tmp  (comma separated, no dot needed)" => "o, class, tmp（カンマ区切り、ドット不要）",
        "target, node_modules, *.cache  (* wildcard allowed)" => "target, node_modules, *.cache（* ワイルドカード可）",
        "Ignore file extensions" => "除外ファイル拡張子",
        "Ignore directory names" => "除外ディレクトリ名",
        "Delete profile" => "プロファイルを削除",
        "0 (show all)" => "0（全表示）",
        "3 (default)" => "3（デフォルト）",
        "Left / Old" => "左 / 旧",
        "Right / New" => "右 / 新",
        "Use as Left" => "左に設定",
        "Use as Right" => "右に設定",
        "Select left, then right, then Compare." => "左・右を選んでから比較してください。",
        "List" => "表示",
        "Previous change" => "前の差分",
        "Next change" => "次の差分",
        "Apply ▶" => "適用 ▶",
        "Undo" => "元に戻す",
        "Redo" => "やり直す",
        "Save" => "保存",
        "Save As" => "名前を付けて保存",
        "More ▼" => "詳細 ▼",
        "Less ▲" => "簡略 ▲",
        "Inline diff" => "文字単位差分",
        "Wrap" => "折り返し",
        "on" => "オン",
        "off" => "オフ",
        "Swap sides" => "左右入替",
        "Ignore WS" => "空白無視",
        "Ignore case" => "大小文字無視",
        "Context lines" => "コンテキスト行数",
        "Compare profiles" => "比較プロファイル",
        "+ New profile" => "+ 新規プロファイル",
        "Profile name" => "プロファイル名",
        "Add" => "追加",
        "Files are identical" => "ファイルは同一です",
        "unsaved" => "未保存",
        "changes" => "件の差分",
        "Theme" => "テーマ",
        "Language" => "言語",
        "Diff font size" => "差分フォントサイズ",
        "Close" => "閉じる",
        "Saved." => "保存しました。",
        "Reloaded." => "再読み込みしました。",
        "File changed on disk" => "ファイルがディスク上で変更されました",
        "The target file was modified after it was loaded. Overwrite anyway?" =>
            "ファイルが読み込み後に変更されました。上書きしますか？",
        "Save As" => "名前を付けて保存",
        "Path" => "パス",
        "Reload files?" => "ファイルを再読み込みしますか？",
        "Unsaved merge changes will be discarded." => "未保存のマージ変更は破棄されます。",
        "Discard and Reload" => "破棄して再読み込み",
        "Swap sides?" => "左右を入れ替えますか？",
        "Unsaved merge changes will be discarded when sides are swapped." =>
            "左右入替時に未保存のマージ変更は破棄されます。",
        "Discard and Swap" => "破棄して入替",
        "Close comparison?" => "比較を閉じますか？",
        "Discard and close" => "破棄して閉じる",
        "Copy file?" => "ファイルをコピーしますか？",
        "Copy diagnostics" => "診断情報をコピー",
        "Copied." => "コピーしました。",
        "Export patch" => "パッチをエクスポート",
        "Copy" => "コピー",
        "Copy all" => "すべてコピー",
        "Overwrite" => "上書き",
        "Cancel" => "キャンセル",
        "Merge/save unavailable for this file type." => "このファイル形式では結合・保存できません。",
        "Large file — inline diff disabled and deadline shortened." => "大きなファイル — インライン差分を無効化し、処理時間を短縮しました。",
        "Diff timed out — result may be approximate." => "差分処理がタイムアウトしました — 結果は概算の可能性があります。",
        "Some hunks were too large for character-level diff." => "一部のハンクが大きすぎるため、文字単位の差分を省略しました。",
        "Binary file — read-only comparison (hex preview)." => "バイナリファイル — 読み取り専用比較（16進プレビュー）。",
        "Spreadsheet — read-only comparison." => "スプレッドシート — 読み取り専用比較。",
        "One side is missing — read-only." => "片方が存在しません — 読み取り専用。",
        "File type not supported for merge — read-only." => "このファイル形式は結合に対応していません — 読み取り専用。",
        _ => return None,
    };
    Some(v)
}
