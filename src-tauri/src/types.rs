use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartupParam {
    pub old_filepath: Option<String>,
    pub new_filepath: Option<String>,
}
