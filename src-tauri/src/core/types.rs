use similar::DiffTag;

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinesDiff {
    pub diff_kind: DiffTag,
    pub lines_count: usize,
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub replace_diff_lines: Option<Vec<ReplaceLineDiff>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceLineDiff {
    pub chars_diff: Vec<ReplaceCharsDiff>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceCharsDiff {
    pub diff_kind: Option<DiffTag>,
    pub old_str: String,
    pub new_str: String,
}

// #[derive(Serialize)]
// pub struct ListDirReponse {
//     pub current_dir: String,
//     pub dirs: Vec<String>,
//     pub files: Vec<String>,
// }
