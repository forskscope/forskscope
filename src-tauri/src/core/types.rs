use similar::DiffTag;

// use serde::{Deserialize, Serialize};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinesDiff {
    pub diff_kind: DiffTag,
    pub lines_count: usize,
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub replace_detail: Option<ReplaceDetailLinesDiff>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceDetailLinesDiff {
    pub old_lines: Vec<Vec<ReplaceDiffChars>>,
    pub new_lines: Vec<Vec<ReplaceDiffChars>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceDiffChars {
    pub diff_kind: DiffTag,
    pub chars: String,
}

// #[derive(Serialize)]
// pub struct ListDirReponse {
//     pub current_dir: String,
//     pub dirs: Vec<String>,
//     pub files: Vec<String>,
// }
