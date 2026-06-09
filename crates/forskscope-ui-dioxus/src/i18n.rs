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
        "Inline diff" => "文字単位差分",
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
        "Overwrite" => "上書き",
        "Cancel" => "キャンセル",
        "Merge/save unavailable for this file type." => "このファイル形式では結合・保存できません。",
        _ => return None,
    };
    Some(v)
}
