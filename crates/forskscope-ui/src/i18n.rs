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
        "Open tabs" => "開いているタブ",
        "Settings" => "設定",
        "Compare selected files" => "選択したファイルを比較",
        "Compare selected directories" => "選択したディレクトリを比較",
        "Select a file or directory on each side to compare" => {
            "両側でファイルまたはディレクトリを選択してください"
        }
        "Comparing…" => "比較中…",
        "Identical" => "同一",
        "Identical — contents compared" => "同一 — 中身を比較済み",
        "Only on the left" => "左側にのみ存在",
        "Only on the right" => "右側にのみ存在",
        "Only on the left (empty folder)" => "左側にのみ存在（空のフォルダー）",
        "Only on the right (empty folder)" => "右側にのみ存在（空のフォルダー）",
        "Directory contents not compared — use Deep Compare" => {
            "ディレクトリの中身は比較されていません — ディープ比較を使用してください"
        }
        "Names and sizes match; contents not compared" => {
            "名前とサイズは一致しています。中身は比較されていません"
        }
        "Size matches; contents not compared" => "サイズは一致しています。中身は比較されていません",
        "Comparison failed" => "比較に失敗しました",
        "Verify" => "検証",
        "Read every byte to find out for certain." => {
            "確実に判定するため、すべてのバイトを読み込みます。"
        }
        "Filter" => "フィルター",
        "Filter (on)" => "フィルター（適用中）",
        "Move together" => "一緒に移動",
        "When one pane moves to a folder, the other moves to the matching folder, if it has one." => {
            "片方のペインでフォルダーを移動すると、もう片方も対応するフォルダーに移動します（ある場合）。"
        }
        "Filter by name…" => "名前でフィルター…",
        "Hide binary" => "バイナリを非表示",
        "Hide identical" => "同一ファイルを非表示",
        "Clear filter" => "フィルターをクリア",
        "Choose a file or folder on each side to compare" => {
            "左右のファイルまたはフォルダーを選択して比較"
        }
        "Choose a file or folder on the right" => {
            "右側のファイルまたはフォルダーを選択してください"
        }
        "Choose a file or folder on the left" => "左側のファイルまたはフォルダーを選択してください",
        "Compare files or folders" => "ファイルまたはフォルダーを比較",
        "Choose a folder for each side, then select items to compare." => {
            "左右にフォルダーを選択し、比較するアイテムを選んでください。"
        }
        "Nothing leaves this computer." => "ファイルはこのコンピューターの外に出ません。",
        "Home folder access needed" => "ホームフォルダーへのアクセスが必要です",
        "ForskScope needs access to your home folder to browse and compare your files." => {
            "ForskScope がファイルを閲覧・比較するには、ホームフォルダーへのアクセスが必要です。"
        }
        "To restore it, run:" => "復元するには、次を実行してください:",
        "Or use a permissions manager such as Flatseal. Then restart ForskScope." => {
            "または Flatseal などの権限管理アプリを使用してください。その後 ForskScope を再起動してください。"
        }
        "Compare" => "比較",
        "··· {n} unchanged lines — click to expand ···" => {
            "··· {n} 行変更なし — クリックして展開 ···"
        }
        "Show {n} unchanged lines" => "{n} 行の変更なしを表示",
        "Deleted" => "削除",
        "Inserted" => "挿入",
        "Changed" => "変更",
        "Use" => "適用",
        "Use this change (apply left to right)" => "この変更を使用（左→右に適用）",
        "Applied" => "適用済み",
        "Previous match (Shift+Enter)" => "前の一致 (Shift+Enter)",
        "Next match (Enter / F3)" => "次の一致 (Enter / F3)",
        "Search within diff" => "差分内検索",
        "Previous match" => "前の一致",
        "Next match" => "次の一致",
        "Close search bar" => "検索バーを閉じる",
        "match" => "件一致",
        "matches" => "件一致",
        "File changed on disk" => "ディスク上のファイルが変更されました",
        "Save As" => "名前を付けて保存",
        "Reload files" => "ファイルを再読み込み",
        "Swap sides" => "左右を入れ替え",
        "Close comparison" => "比較を閉じる",
        "Copy to right" => "右にコピー",
        "Copy to left" => "左にコピー",
        "Copy this file?" => "このファイルをコピーしますか？",
        "The destination exists. A .bak backup will be created first." => {
            "コピー先が存在します。先に .bak バックアップを作成します。"
        }
        "A manifest will be saved so copies can be reviewed or reversed." => {
            "マニフェストを保存し、コピーの確認や元に戻す操作が可能になります。"
        }
        "Copy result" => "コピー結果",
        "Copy finished" => "コピー完了",
        "succeeded" => "成功",
        "Manifest saved:" => "マニフェスト保存済み:",
        "Errors:" => "エラー:",
        "Copy file" => "ファイルをコピー",
        "Copied" => "コピー完了",
        "failed" => "失敗",
        "Left pane" => "左ペイン",
        "Right pane" => "右ペイン",
        "Left" => "左",
        "Right" => "右",
        "Scanning…" => "スキャン中…",
        "different" => "差分あり",
        "equal" => "同一",
        "left only" => "左のみ",
        "right only" => "右のみ",
        "unreadable" => "読み取り不可",
        "Unreadable" => "読み取り不可",
        "Symlink not followed" => "シンボリックリンク（リンク先は未比較）",
        "checking" => "確認中",
        "Copy" => "コピー",
        "Back" => "戻る",
        "Forward" => "進む",
        "Home folder" => "ホームフォルダー",
        "Open folder…" => "フォルダーを開く…",
        "Edit path" => "パスを編集",
        "click the name" => "名前をクリック",
        "More" => "その他",
        "Keyboard shortcuts (Ctrl+/)" => "キーボードショートカット (Ctrl+/)",
        "File comparison" => "ファイル比較",
        "F7 — Previous change" => "F7 — 前の変更",
        "F8 — Next change" => "F8 — 次の変更",
        "Undo last merge action (Ctrl+Z)" => "最後のマージ操作を元に戻す (Ctrl+Z)",
        "Save merge result (Ctrl+S)" => "マージ結果を保存 (Ctrl+S)",
        "Reload both files from disk" => "両ファイルをディスクから再読み込み",
        "Reload files from disk" => "ディスクからファイルを再読み込み",
        "Search within diff (Ctrl+F)" => "差分内検索 (Ctrl+F)",
        "Open search bar" => "検索バーを開く",
        "Toggle character-level inline diff" => "文字レベルの差分表示を切り替え",
        "Toggle word wrap" => "折り返しを切り替え",
        "Toggle ignore whitespace" => "空白無視を切り替え",
        "Toggle ignore case" => "大文字小文字無視を切り替え",
        "Diff algorithm" => "差分アルゴリズム",
        "Export unified-diff patch file" => "unified diff パッチファイルをエクスポート",
        "Export patch" => "パッチをエクスポート",
        "Batch copy" => "一括コピー",
        "files" => "ファイル",
        "Close" => "閉じる",
        "Cannot compare: one file is binary and the other is text. Compare text with text, or binary with binary." => {
            "比較できません: 一方がバイナリ、もう一方がテキストです。テキスト同士またはバイナリ同士で比較してください。"
        }
        "Loading" => "読み込み中",
        "loading" => "読み込み中",
        "Could not open" => "開けませんでした",
        "Check that the file exists and you have read permission." => {
            "ファイルが存在し、読み取り権限があることを確認してください。"
        }
        // F209 (handoff 088): a startup command line naming one file and
        // one folder is refused in-window - two whole-sentence shapes,
        // one per which side is the file ({left}/{right}, already
        // quoted, substituted after translation, not before). The key
        // itself carries no `\"` - xtask i18n's own key extraction is
        // line-based and not escape-aware, so an escaped quote in the
        // key truncates it and breaks the used/translated match.
        "A file can only be compared with a file, and a folder with a folder. {left} is a file; {right} is a folder." => {
            "ファイルはファイルと、フォルダーはフォルダーとしか比較できません。{left} はファイルで、{right} はフォルダーです。"
        }
        "A file can only be compared with a file, and a folder with a folder. {left} is a folder; {right} is a file." => {
            "ファイルはファイルと、フォルダーはフォルダーとしか比較できません。{left} はフォルダーで、{right} はファイルです。"
        }
        "Git's merge tool compares files only. {name} is a folder." => {
            "Git のマージツールはファイルのみを比較します。{name} はフォルダーです。"
        }
        "comparison" => "比較",
        "merge" => "マージ",
        "Previous / next change" => "前 / 次の変更",
        "Apply focused change (left → right)" => "フォーカス中の変更を適用（左→右）",
        "Undo last merge" => "最後のマージを元に戻す",
        "Redo last undone merge" => "元に戻したマージをやり直す",
        "Save merge result" => "マージ結果を保存",
        "Open / close inline search" => "差分内検索を開く / 閉じる",
        "Next / previous search match" => "次 / 前の一致",
        "Move focus in explorer list" => "エクスプローラー内のフォーカス移動",
        "Expand or collapse the focused folder; does nothing on a file" => {
            "フォーカスしたフォルダーを展開/折りたたみ。ファイルでは何も起きません"
        }
        "Select file as comparison candidate" => "ファイルを比較候補として選択",
        "Switch focused pane (left ↔ right)" => "フォーカスペインを切り替え（左↔右）",
        "Go up one directory (focused pane)" => "1階層上に移動（フォーカス中のペイン）",
        "Go up one directory" => "1階層上に移動",
        "Go to home directory (focused pane)" => "ホームディレクトリに移動（フォーカス中のペイン）",
        "Open a folder (focused pane)" => "フォルダーを開く（フォーカス中のペイン）",
        "Back / forward directory history (focused pane)" => {
            "ディレクトリ履歴の前 / 次（フォーカス中のペイン）"
        }
        "Close the active comparison tab" => "アクティブな比較タブを閉じる",
        "This keyboard reference" => "キーボードショートカット一覧",
        "Close modal / search bar" => "モーダル / 検索バーを閉じる",
        "Close tab" => "タブを閉じる",
        "No comparison." => "比較がありません。",
        "has unsaved changes. Discard them and close?" => {
            "未保存の変更があります。破棄して閉じますか？"
        }
        "From" => "変更前",
        "To" => "変更後",
        "Existing files will receive a .bak backup." => {
            "既存ファイルは.bakバックアップが作成されます。"
        }
        "Version" => "バージョン",
        "Rust" => "Rust",
        "OS" => "OS",
        "Arch" => "アーキテクチャ",
        "CPUs" => "CPU数",
        "Files stay on this computer. ForskScope does not upload them." => {
            "ファイルはこのコンピューターに留まります。ForskScopeはアップロードしません。"
        }
        "Local only" => "ローカルのみ",
        "Portable" => "ポータブル",
        "Portable (read-only): changes are not saved" => {
            "ポータブル（読み取り専用）：変更は保存されません"
        }
        "Private: nothing is saved" => "プライベート：何も保存されません",
        "Mode" => "モード",
        "Data location" => "データの保存先",
        "No matches" => "一致なし",
        "Deep compare" => "深度比較",
        "Different" => "差分あり",
        "All" => "すべて",
        "Equal only" => "同一のみ",
        "Keyboard shortcuts" => "キーボードショートカット",
        "Diff view" => "差分ビュー",
        "Navigation" => "ナビゲーション",
        "Tabs" => "タブ",
        "Next tab, from anywhere (wraps, includes Explorer)" => {
            "次のタブへ（どこからでも、循環、エクスプローラーを含む）"
        }
        "Previous tab, from anywhere (wraps, includes Explorer)" => {
            "前のタブへ（どこからでも、循環、エクスプローラーを含む）"
        }
        "Previous / next tab (tab list focused)" => "前 / 次のタブ（タブ一覧にフォーカス時）",
        "First / last tab (tab list focused)" => "最初 / 最後のタブ（タブ一覧にフォーカス時）",
        "Close the focused tab (tab list focused)" => {
            "フォーカス中のタブを閉じる（タブ一覧にフォーカス時）"
        }
        "App" => "アプリ",
        "Search…" => "検索…",
        "o, class, tmp  (comma separated, no dot needed)" => {
            "o, class, tmp（カンマ区切り、ドット不要）"
        }
        "target, node_modules, *.cache  (* wildcard allowed)" => {
            "target, node_modules, *.cache（* ワイルドカード可）"
        }
        "Ignore file extensions" => "除外ファイル拡張子",
        "Ignore directory names" => "除外ディレクトリ名",
        "Hide hidden files" => "隠しファイルを非表示にする",
        "Also excludes hidden files and folders from comparisons — recursive compare, Deep Compare, and the Explorer's quick folder check. Off by default, so .gitignore, .env, and similar files are included." => {
            "隠しファイルやフォルダーは、比較(再帰比較・ディープ比較・エクスプローラーのクイックフォルダー確認)からも除外されます。デフォルトではオフになっており、.gitignore や .env などのファイルも比較対象に含まれます。"
        }
        "Delete profile" => "プロファイルを削除",
        "0 (show all)" => "0（全表示）",
        "3 (default)" => "3（デフォルト）",
        "Left / Old" => "左 / 旧",
        "Right / New" => "右 / 新",
        "Undo" => "元に戻す",
        "Redo" => "やり直す",
        "Save" => "保存",
        "Save target" => "保存先",
        "More ▼" => "詳細 ▼",
        "Less ▲" => "簡略 ▲",
        "Inline diff" => "文字単位差分",
        "Wrap" => "折り返し",
        "on" => "オン",
        "off" => "オフ",
        "Ignore whitespace" => "空白無視",
        "Ignore case" => "大小文字無視",
        "Ignore line-ending differences (CRLF/LF)" => "改行コードの違いを無視 (CRLF/LF)",
        "Context lines" => "コンテキスト行数",
        "Explorer layout" => "エクスプローラーレイアウト",
        "Aligned" => "整列",
        "Compact (no spacer rows, default)" => "コンパクト（空白行なし、デフォルト）",
        "Remember Explorer directories" => "エクスプローラーのディレクトリを記憶する",
        "When on, the Explorer reopens the last directory shown in each pane. When off, it always starts at your home directory." => {
            "オンの場合、エクスプローラーは各ペインで最後に表示したディレクトリを再度開きます。オフの場合は常にホームディレクトリから開始します。"
        }
        "Enable binary comparison" => "バイナリ比較を有効にする",
        "When off, binary files cannot be compared and are shown as non-actionable in the Explorer." => {
            "オフの場合、バイナリファイルは比較できず、エクスプローラーで操作不可として表示されます。"
        }
        "Binary comparison is off. Enable it in Settings → Advanced." => {
            "バイナリ比較はオフです。設定 → 詳細設定で有効にできます。"
        }
        "Binary file. Binary comparison is off — enable it in Settings → Advanced." => {
            "バイナリファイルです。バイナリ比較はオフです — 設定 → 詳細設定で有効にできます。"
        }
        "Spreadsheet comparison is read-only." => "スプレッドシート比較は読み取り専用です。",
        "Advanced" => "詳細設定",
        "Hide advanced" => "詳細設定を隠す",
        "Compare profiles" => "比較プロファイル",
        "Active compare profile" => "使用する比較プロファイル",
        "+ New profile" => "+ 新規プロファイル",
        "Profile name" => "プロファイル名",
        "Add" => "追加",
        "More options" => "詳細オプション",
        "Character-level highlighting" => "文字単位のハイライト",
        "When I turn it on" => "オンにしたとき",
        "Automatically for small changes" => "小さな変更は自動的に",
        "Never" => "常にオフ",
        "Files are identical" => "ファイルは同一です",
        "Files are identical — nothing to export." => {
            "ファイルは同一です — エクスポートする内容がありません。"
        }
        "unsaved" => "未保存",
        "Theme" => "テーマ",
        "Colour vision" => "色覚",
        "Standard" => "標準",
        "Red–green safe (blue and orange)" => "赤緑の区別に配慮（青とオレンジ）",
        "Language" => "言語",
        "Diff font family" => "差分フォントファミリー",
        "Monospace (default)" => "等幅（デフォルト）",
        "Sans-serif" => "サンセリフ",
        "Serif" => "セリフ",
        "Diff font size" => "差分フォントサイズ",
        "Saved." => "保存しました。",
        "Saved, but the requested encoding was not recognized — used UTF-8 instead." => {
            "保存しましたが、指定されたエンコーディングが認識されなかったため、UTF-8を使用しました。"
        }
        "Reloaded." => "再読み込みしました。",
        "The target file was modified after it was loaded. Overwrite anyway?" => {
            "ファイルが読み込み後に変更されました。上書きしますか？"
        }
        "Path" => "パス",
        "Reload files?" => "ファイルを再読み込みしますか？",
        "Unsaved merge changes will be discarded." => "未保存のマージ変更は破棄されます。",
        "Discard and Reload" => "破棄して再読み込み",
        "Swap sides?" => "左右を入れ替えますか？",
        "Unsaved merge changes will be discarded when sides are swapped." => {
            "左右入替時に未保存のマージ変更は破棄されます。"
        }
        "Discard and Swap" => "破棄して入替",
        "Change diff options" => "差分オプションの変更",
        "Change diff options?" => "差分オプションを変更しますか？",
        "Unsaved merge changes will be discarded when diff options change." => {
            "差分オプション変更時に未保存のマージ変更は破棄されます。"
        }
        "Discard and Change" => "破棄して変更",
        "Change encoding" => "エンコーディングの変更",
        "Change encoding?" => "エンコーディングを変更しますか？",
        "Unsaved merge changes will be discarded when the encoding changes." => {
            "エンコーディング変更時に未保存のマージ変更は破棄されます。"
        }
        "Encoding — re-decode if this looks wrong" => {
            "エンコーディング — 文字化けしている場合は選び直してください"
        }
        "Right-side text encoding" => "右側のテキストエンコーディング",
        "Close comparison?" => "比較を閉じますか？",
        "Discard and close" => "破棄して閉じる",
        "Copy diagnostics" => "診断情報をコピー",
        "About ForskScope" => "ForskScope について",
        "Copied." => "コピーしました。",
        "Check for updates" => "更新を確認",
        "Checking…" => "確認中…",
        "You have the latest version ({version})." => "最新バージョンです（{version}）。",
        "{version} is available." => "{version} が利用可能です。",
        "This build is newer than the latest release ({version})." => {
            "このビルドは最新リリース（{version}）より新しいバージョンです。"
        }
        "Could not check for updates: no network connection." => {
            "更新を確認できませんでした: ネットワーク接続がありません。"
        }
        "Could not check for updates: rate-limited, try again later." => {
            "更新を確認できませんでした: レート制限されています。後でもう一度お試しください。"
        }
        "Could not check for updates: unexpected response (HTTP {code})." => {
            "更新を確認できませんでした: 予期しない応答です（HTTP {code}）。"
        }
        "Could not check for updates: the reply could not be read." => {
            "更新を確認できませんでした: 応答を読み取れませんでした。"
        }
        "Open release page" => "リリースページを開く",
        "Update with your AUR helper" => "AUR ヘルパーで更新してください",
        "Update through the Microsoft Store" => "Microsoft Store から更新してください",
        "Copy all" => "すべてコピー",
        "Overwrite" => "上書き",
        "Cancel" => "キャンセル",
        "Merge/save unavailable for this file type." => {
            "このファイル形式では結合・保存できません。"
        }
        "Large file — inline diff disabled and deadline shortened." => {
            "大きなファイル — インライン差分を無効化し、処理時間を短縮しました。"
        }
        "Diff timed out — result may be approximate." => {
            "差分処理がタイムアウトしました — 結果は概算の可能性があります。"
        }
        "Some rows share an alignment key, so rows may be paired wrongly." => {
            "一部の行が同じ整列キーを持つため、行が誤って対応付けられている可能性があります。"
        }
        "A sheet was too large to align rows and was compared position by position, so an inserted row may appear as many changes." => {
            "シートが大きすぎて行を整列できず、位置ごとに比較しました。挿入された行が多数の変更として表示される場合があります。"
        }
        "Several sheets could have been renamed, so none was matched; they appear as added and removed." => {
            "名前が変更された可能性のあるシートが複数あり、どれも対応付けられませんでした。追加と削除として表示されています。"
        }
        "Not compared (not a worksheet):" => "比較されていません（ワークシートではありません）:",
        "The comparison reported a warning:" => "比較で警告が報告されました:",
        "Sheets:" => "シート:",
        "Some hunks were too large for character-level diff." => {
            "一部のハンクが大きすぎるため、文字単位の差分を省略しました。"
        }
        "Binary file — read-only comparison (hex preview)." => {
            "バイナリファイル — 読み取り専用比較（16進プレビュー）。"
        }
        "Both files not found — read-only." => "両ファイルが見つかりません — 読み取り専用。",
        "One side is missing — read-only." => "片方が存在しません — 読み取り専用。",
        "File type not supported for merge — read-only." => {
            "このファイル形式は結合に対応していません — 読み取り専用。"
        }
        // RFC-076 patch 6: settings/session recovery dialogs.
        "Settings file is from a newer version" => "設定ファイルは新しいバージョン向けです",
        "This settings file uses" => "この設定ファイルは",
        "Settings file could not be read" => "設定ファイルを読み込めませんでした",
        "The settings file is preserved but could not be parsed" => {
            "設定ファイルは保持されていますが、解析できませんでした"
        }
        "Settings could not be upgraded" => "設定をアップグレードできませんでした",
        "Your settings were read and are in use for this session, but they could not be saved in the new format" => {
            "設定は読み込まれ今回のセッションで使用されていますが、新しい形式で保存できませんでした"
        }
        "Session file is from a newer version" => "セッションファイルは新しいバージョン向けです",
        "This session file uses" => "このセッションファイルは",
        "Session file could not be read" => "セッションファイルを読み込めませんでした",
        "The session file is preserved but could not be parsed" => {
            "セッションファイルは保持されていますが、解析できませんでした"
        }
        "Session could not be upgraded" => "セッションをアップグレードできませんでした",
        "Your session was read and is in use for this run, but it could not be saved in the new format" => {
            "セッションは読み込まれ今回の実行で使用されていますが、新しい形式で保存できませんでした"
        }
        "schema version" => "スキーマバージョン",
        "which this version of ForskScope does not understand. The file has not been modified." => {
            "を使用しており、このバージョンのForskScopeでは認識できません。ファイルは変更されていません。"
        }
        "Changes you make this session will not be saved." => {
            "このセッション中の変更は保存されません。"
        }
        "Changes you make this session will not be saved unless you reset it." => {
            "リセットしない限り、このセッション中の変更は保存されません。"
        }
        "Changes will not be saved until this is resolved." => {
            "この問題が解決するまで変更は保存されません。"
        }
        "Exit" => "終了",
        "Continue with defaults" => "デフォルトのまま続行",
        "Continue without saving" => "保存せずに続行",
        "Reset and back up" => "リセットしてバックアップ",
        // RFC-077 patch 5: mergetool result line.
        "Result:" => "結果:",
        // RFC-077 patch 5: Save As to an existing destination.
        "Overwrite existing file?" => "既存のファイルを上書きしますか？",
        "A file already exists at this path." => "このパスには既にファイルが存在します。",
        // F109: Windows WebView2 Runtime missing-at-startup check.
        "forskscope: the Microsoft Edge WebView2 Runtime is not installed" => {
            "forskscope: Microsoft Edge WebView2 ランタイムがインストールされていません"
        }
        "ForskScope needs the Microsoft Edge WebView2 Runtime, which is not installed on this computer. Open the download page?" => {
            "ForskScope には Microsoft Edge WebView2 ランタイムが必要ですが、このコンピューターにはインストールされていません。ダウンロードページを開きますか？"
        }
        // F117: a spreadsheet comparison that failed or hit the size bound.
        "Could not compare the spreadsheets" => "スプレッドシートを比較できませんでした",
        // F120: a changed line pair too long for character-level highlighting.
        "Too long for character-level highlighting" => {
            "長すぎるため文字単位のハイライトを行いません"
        }
        "Long line" => "長い行",
        "Character-level highlighting is not available for this comparison." => {
            "この比較では文字単位のハイライトは利用できません。"
        }
        // F122: the large-file prompt says what the product does.
        "File is large" => "ファイルが大きいです",
        "Diff anyway" => "それでも比較する",
        "One or both files exceed the recommended diff limit ({n} MiB). Diffing may be slow. The line diff stops after 5 seconds and says so if its result is approximate, and character-level highlighting is switched off." => {
            "一方または両方のファイルが推奨される比較サイズ（{n} MiB）を超えています。比較に時間がかかることがあります。行単位の差分は5秒で打ち切られ、結果が近似の場合はその旨が表示されます。文字単位のハイライトは無効になります。"
        }
        "One or both workbooks exceed the recommended comparison size ({n} MiB). Comparing them may be slow. The comparison is exact, or it is refused if a workbook is too large to compare; it is never approximate." => {
            "一方または両方のブックが推奨される比較サイズ（{n} MiB）を超えています。比較に時間がかかることがあります。比較は正確に行われるか、大きすぎる場合は拒否されます。近似の結果が表示されることはありません。"
        }

        // RFC-092 (handoff 084): the export dialog, the toolbar button, the
        // "Open" toast action, and §0's two refusal messages.
        "Export report" => "レポートをエクスポート",
        "Export report…" => "レポートをエクスポート…",
        "Export a comparison report" => "比較レポートをエクスポートします",
        "Export" => "エクスポート",
        "Format" => "形式",
        "HTML report" => "HTML レポート",
        "Unified diff (.patch)" => "統合diff（.patch）",
        "More options ▸" => "詳細オプション ▸",
        "Paths" => "パス",
        "File names only" => "ファイル名のみ",
        "Paths relative to the compared folders" => "比較したフォルダーからの相対パス",
        "Full paths" => "完全なパス",
        "Report saved." => "レポートを保存しました。",
        "Open" => "開く",
        "Reports cover text comparisons for now." => {
            "レポートは現在、テキスト比較のみに対応しています。"
        }
        "Save or undo your changes first: a report describes the saved files." => {
            "先に変更を保存するか取り消してください。レポートは保存済みのファイルの内容を記述します。"
        }
        "A file changed since it was compared. Compare again, then export." => {
            "比較した後にファイルが変更されました。再度比較してからエクスポートしてください。"
        }
        "Export patch…" => "パッチをエクスポート…",
        "Export a unified-diff patch file" => "統合diff形式のパッチファイルをエクスポートします",
        "Finish comparing, or Cancel, before exporting." => {
            "エクスポートする前に、比較を完了するかキャンセルしてください。"
        }
        "Stop comparing — entries not yet compared stay as they are, and can still be exported." => {
            "比較を停止します。まだ比較されていない項目はそのまま残り、エクスポートすることもできます。"
        }
        "Reading and hashing files" => "ファイルの読み込みとハッシュ計算",
        "Working…" => "処理中…",
        "Could not build the patch" => "パッチを作成できませんでした",
        "Could not save the report" => "レポートを保存できませんでした",
        "Could not save the patch" => "パッチを保存できませんでした",

        // RFC-092: the HTML report itself (forskscope-report/src/report/html.rs).
        // Reached through a translator closure, not a literal t(lang, ...)
        // call site here, so cargo xtask i18n cannot see these keys - kept
        // in sync by hand against html.rs's own tr() calls.
        "Comparison report" => "比較レポート",
        "Generated" => "生成日時",
        "What was compared" => "比較対象",
        // Deliberately unchanged - a universal technical term, not an
        // untranslated gap. Review 174 §5's "assert nothing comes back
        // untranslated" test checks this key explicitly for that reason,
        // not just its presence here.
        "SHA-256" => "SHA-256",
        "How" => "方法",
        "Compare profile" => "比較プロファイル",
        "Default" => "デフォルト",
        "Custom" => "カスタム",
        "Summary" => "概要",
        "The changes" => "変更内容",
        "Notes" => "注記",
        "Files" => "ファイル",
        "Status" => "ステータス",
        "Left size" => "左のサイズ",
        "Right size" => "右のサイズ",
        "No differences — the two files are identical." => "差分なし — 2つのファイルは同一です。",
        // Review 174 §4: whole-sentence summary templates
        // (report::html::summary_template_key) - each of the 18 shapes
        // that function can produce, keyed by its exact English text
        // (with English's own singular/plural already baked in - Japanese
        // needs no such distinction, so both the n=1 and n>1 English keys
        // for a given (d>0, a>0) shape map to the same Japanese template).
        // {n}/{d}/{a} are substituted after translation, not before.
        "{n} change: {d} line removed, {a} line added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} change: {d} line removed, {a} lines added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} change: {d} lines removed, {a} line added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} change: {d} lines removed, {a} lines added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} changes: {d} line removed, {a} line added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} changes: {d} line removed, {a} lines added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} changes: {d} lines removed, {a} line added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} changes: {d} lines removed, {a} lines added." => {
            "{n} か所の変更（削除 {d} 行、追加 {a} 行）。"
        }
        "{n} change: {d} line removed." => "{n} か所の変更（削除 {d} 行）。",
        "{n} change: {d} lines removed." => "{n} か所の変更（削除 {d} 行）。",
        "{n} changes: {d} line removed." => "{n} か所の変更（削除 {d} 行）。",
        "{n} changes: {d} lines removed." => "{n} か所の変更（削除 {d} 行）。",
        "{n} change: {a} line added." => "{n} か所の変更（追加 {a} 行）。",
        "{n} change: {a} lines added." => "{n} か所の変更（追加 {a} 行）。",
        "{n} changes: {a} line added." => "{n} か所の変更（追加 {a} 行）。",
        "{n} changes: {a} lines added." => "{n} か所の変更（追加 {a} 行）。",
        "{n} change." => "{n} か所の変更。",
        "{n} changes." => "{n} か所の変更。",
        "Whitespace" => "空白",
        "Case" => "大文字・小文字",
        "Newlines" => "改行",
        "Algorithm" => "アルゴリズム",
        "ignored" => "無視",
        // review 175 §2: the placeholder keeps `<code>sha256sum</code>` out
        // of the translated text - it is substituted in by
        // `report::html::checksum_caveat` after escaping, so the Japanese
        // text itself must carry no backticks of its own.
        "Run {cmd} on a file to confirm it is the file this report describes. This does not prove the report itself is unaltered." => {
            "ファイルに対して{cmd}を実行すると、このレポートが説明しているファイルであることを確認できます。ただし、レポート自体が改ざんされていないことは証明しません。"
        }
        // review 175 §3: the folder report's main counts line falls back
        // to this when every count is zero, rather than printing "X: 0"
        // for each.
        "No files." => "ファイルはありません。",
        // review 174 §3: report::html's per-row status words now come from
        // forskscope_compare::dir::status_key, the same keys status_label
        // (deep_compare.rs) reads - "Different"/"Identical"/"Only on the
        // left"/"Only on the right"/"Only on the left (empty folder)"/
        // "Only on the right (empty folder)"/"Symlink not followed"/
        // "Unreadable" all already have entries (pre-existing, shared with
        // the live view). "Not compared" (RecStatus::Computing once
        // nothing is comparing it any more - review 174 §2a) is new here.
        "Not compared" => "未比較",
        "Cancelled — compare again to see the folders." => {
            "キャンセルしました — フォルダーを表示するには再度比較してください。"
        }
        "Compare again, then export." => "再度比較してからエクスポートしてください。",
        // Review 183 §3.2: report::html::cap_omission_template_key's four
        // shapes — `n` (files omitted) and `max_files` (the cap actually
        // applied, which `--max-files` can change) pluralize
        // independently in English; Japanese needs no such distinction,
        // so all four map to the same template. {n}/{max_files}/
        // {max_size} are substituted after translation, not before.
        "{n} changed file's content is not shown in this report (the cap of {max_files} file or {max_size} of change text was reached):" => {
            "変更された{n}個のファイルの内容はこのレポートには表示されません（{max_files}ファイルまたは{max_size}の変更テキストという上限に達しました）："
        }
        "{n} changed file's content is not shown in this report (the cap of {max_files} files or {max_size} of change text was reached):" => {
            "変更された{n}個のファイルの内容はこのレポートには表示されません（{max_files}ファイルまたは{max_size}の変更テキストという上限に達しました）："
        }
        "{n} changed files' content is not shown in this report (the cap of {max_files} file or {max_size} of change text was reached):" => {
            "変更された{n}個のファイルの内容はこのレポートには表示されません（{max_files}ファイルまたは{max_size}の変更テキストという上限に達しました）："
        }
        "{n} changed files' content is not shown in this report (the cap of {max_files} files or {max_size} of change text was reached):" => {
            "変更された{n}個のファイルの内容はこのレポートには表示されません（{max_files}ファイルまたは{max_size}の変更テキストという上限に達しました）："
        }
        // report::file's own warning strings (distinct wording from
        // diff.rs's live-UI versions of the same facts - report/file.rs
        // predates this handoff and is out of its scope to reword).
        "Large file — inline diff disabled." => {
            "大きなファイルのため、文字単位の差分表示を無効にしました。"
        }
        "Some hunks were too large for inline diff." => {
            "一部のハンクは大きすぎたため、文字単位の差分表示を省略しました。"
        }

        _ => return None,
    };
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::path::PathBuf;

    use forskscope_compare::diff::{DiffOptions, compute_diff};
    use forskscope_compare::dir::{RecEntry, RecStatus};
    use forskscope_report::report::{
        ChangeHunk, DirComparisonReport, FileChecksum, FileComparisonReport, ReportOptions,
        change_hunks_from_diff, render_dir_report, render_file_report,
    };

    /// Review 174 §5: "Japanese through a stand-in translator is not
    /// evidence of criterion 7. Render in a UI-crate test with the real
    /// t(Lang::Ja, …). Assert that no string the renderer requests comes
    /// back untranslated." Records every key `render_file_report`/
    /// `render_dir_report` actually requests through `tr`, then — since
    /// `ja()` is this module's own private function, reachable here but
    /// not from forskscope-report — asserts each one has a real entry in it.
    /// Checking `ja(key).is_some()` rather than comparing translated text
    /// to the key is deliberate: a deliberately-identical entry (`"SHA-256"
    /// => "SHA-256"`, a universal technical term) must count as translated,
    /// not as a gap that merely produces the same text as a coincidence.
    /// A renderer call, boxed so its concrete closure type (and the
    /// translator type it takes) doesn't need spelling out at every call
    /// site.
    type RenderCall<'a> = Box<dyn FnOnce(&dyn Fn(&str) -> String) -> String + 'a>;

    fn requested_ja_keys(render: RenderCall<'_>) -> Vec<String> {
        let requested: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let tr = |s: &str| -> String {
            requested.borrow_mut().push(s.to_string());
            t(Lang::Ja, s)
        };
        let _ = render(&tr);
        requested.into_inner()
    }

    fn assert_every_requested_key_has_a_real_ja_entry(keys: &[String]) {
        let mut missing: Vec<&str> = keys
            .iter()
            .map(String::as_str)
            .filter(|k| ja(k).is_none())
            .collect();
        missing.sort_unstable();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "these keys the renderer requested have no ja() entry at all - they fall through \
             to the raw English key, not a real translation: {missing:?}"
        );
    }

    fn comprehensive_file_report() -> FileComparisonReport {
        let diff = compute_diff(
            "one\ntwo\nthree\n",
            "one\nTWO\nthree\nfour\n",
            DiffOptions {
                ignore_case: true,
                ..DiffOptions::default()
            },
        );
        let mut report = FileComparisonReport::from_diff(
            &diff,
            Some(&PathBuf::from("/left/report.txt")),
            Some(&PathBuf::from("/right/report.txt")),
            ReportOptions::default(),
        )
        .with_checksums(
            Some(FileChecksum::of_bytes(b"left")),
            Some(FileChecksum::of_bytes(b"right")),
        );
        // A real, pre-existing warning string (not caller-pushed free
        // text like an encoding label, which is inherently untranslatable
        // dynamic content, not something this test is about).
        report.push_note("Large file — inline diff disabled.");
        report
    }

    fn comprehensive_dir_report() -> DirComparisonReport {
        let entries = vec![
            RecEntry {
                rel_path: PathBuf::from("equal.txt"),
                status: RecStatus::Equal,
                left_size: Some(1),
                right_size: Some(1),
            },
            RecEntry {
                rel_path: PathBuf::from("changed.txt"),
                status: RecStatus::Changed,
                left_size: Some(2),
                right_size: Some(3),
            },
            RecEntry {
                rel_path: PathBuf::from("left_only.txt"),
                status: RecStatus::LeftOnly,
                left_size: Some(1),
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("right_only.txt"),
                status: RecStatus::RightOnly,
                left_size: None,
                right_size: Some(1),
            },
            RecEntry {
                rel_path: PathBuf::from("a_link"),
                status: RecStatus::Symlink,
                left_size: None,
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("unreadable.txt"),
                status: RecStatus::Unreadable,
                left_size: None,
                right_size: None,
            },
            RecEntry {
                rel_path: PathBuf::from("cancelled.txt"),
                status: RecStatus::Computing,
                left_size: Some(1),
                right_size: Some(1),
            },
        ];
        let diff_options = DiffOptions {
            ignore_case: true,
            ..DiffOptions::default()
        };
        let mut report = DirComparisonReport::from_entries(
            &entries,
            Some(&PathBuf::from("/left")),
            Some(&PathBuf::from("/right")),
            Some(&diff_options),
            ReportOptions::default(),
        );
        let changed_row = report
            .file_rows
            .iter_mut()
            .find(|r| r.path == "changed.txt")
            .unwrap();
        changed_row.left_checksum = Some(FileChecksum::of_bytes(b"old"));
        changed_row.right_checksum = Some(FileChecksum::of_bytes(b"new"));
        let d = compute_diff("old\n", "new\n", DiffOptions::default());
        let hunks: Vec<ChangeHunk> = change_hunks_from_diff(&d, 3);
        changed_row.change_hunks = Some(hunks);
        // Exercise the cap-omission message too, for real.
        report.change_hunks_omitted.push("huge.txt".into());
        report
    }

    #[test]
    fn every_key_a_file_report_requests_in_japanese_has_a_real_translation() {
        let report = comprehensive_file_report();
        let keys = requested_ja_keys(Box::new(|tr| {
            render_file_report(&report, "0.194.0", "now", false, tr)
        }));
        assert!(
            !keys.is_empty(),
            "the renderer requested nothing - fixture is too thin"
        );
        assert_every_requested_key_has_a_real_ja_entry(&keys);
    }

    #[test]
    fn every_key_a_dir_report_requests_in_japanese_has_a_real_translation() {
        let report = comprehensive_dir_report();
        let keys = requested_ja_keys(Box::new(|tr| {
            render_dir_report(&report, "0.194.0", "now", false, tr)
        }));
        assert!(
            !keys.is_empty(),
            "the renderer requested nothing - fixture is too thin"
        );
        assert_every_requested_key_has_a_real_ja_entry(&keys);
    }

    /// Falsification: a key this test does *not* cover (because nothing
    /// requests it) must not silently pass - confirms `ja(k).is_none()`
    /// really does detect a gap, not just that the fixture is comprehensive
    /// by accident.
    #[test]
    fn a_genuinely_untranslated_key_is_detected_as_missing() {
        assert!(ja("a string that is definitely not in the ja table").is_none());
    }

    /// Review 185 §1: `report::html::cap_omission_template_key`'s four
    /// shapes (`n`/`max_files` each singular or plural) - checked against
    /// the real `ja()` table above, not a test-local copy of the
    /// translations (the defect review 185 found in the previous version
    /// of this test, which lived in `forskscope-report` and could never
    /// fail against a real missing entry since its own translator closure
    /// held its own copy of all four strings).
    #[test]
    fn every_cap_omission_shape_in_japanese_has_a_real_translation() {
        for omitted_count in [1usize, 2] {
            for max_files in [1usize, 50] {
                let entries: Vec<RecEntry> = (0..omitted_count)
                    .map(|i| RecEntry {
                        rel_path: PathBuf::from(format!("huge{i}.txt")),
                        status: RecStatus::Changed,
                        left_size: Some(1),
                        right_size: Some(1),
                    })
                    .collect();
                let options = ReportOptions {
                    max_files_with_content: max_files,
                    ..ReportOptions::default()
                };
                let mut report =
                    DirComparisonReport::from_entries(&entries, None, None, None, options);
                for i in 0..omitted_count {
                    report.change_hunks_omitted.push(format!("huge{i}.txt"));
                }
                let keys = requested_ja_keys(Box::new(|tr| {
                    render_dir_report(&report, "0.194.0", "now", false, tr)
                }));
                assert!(
                    !keys.is_empty(),
                    "n={omitted_count}, max_files={max_files}: the renderer requested nothing"
                );
                assert_every_requested_key_has_a_real_ja_entry(&keys);
            }
        }
    }

    /// Review 175 §2: the checksum caveat sentence, rendered through the
    /// real `t(Lang::Ja, …)`, must carry `<code>sha256sum</code>` and no
    /// literal backtick in either language - not just in a mock-translator
    /// unit test of `checksum_caveat` itself, but in what the real app
    /// would actually produce.
    #[test]
    fn the_checksum_caveat_uses_code_markup_and_no_backtick_in_both_languages() {
        let report = comprehensive_file_report();
        for lang in [Lang::En, Lang::Ja] {
            let html = render_file_report(&report, "0.194.0", "now", false, &|s| t(lang, s));
            assert!(
                html.contains("<code>sha256sum</code>"),
                "{lang:?}: missing <code>sha256sum</code> markup"
            );
            assert!(
                !html.contains('`'),
                "{lang:?}: a literal backtick leaked into the rendered page"
            );
        }
    }
}
