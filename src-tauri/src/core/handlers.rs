// use tauri::Manager;

use serde::Serialize;
use std::path::Path;
use std::process::Command;

use tauri::Manager;

use super::diff::{chars_diffs, lines_diffs};
use super::file::{self, arg_to_filepath, file_manager_command, filepaths_content};
use super::types::{CharsDiffResponse, DiffResponse, LinesDiff, ListDirReponse};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupParam {
    pub old_filepath: Option<String>,
    pub new_filepath: Option<String>,
}

#[tauri::command]
pub fn ready(app_handle: tauri::AppHandle) -> StartupParam {
    let mut args = app_handle
        .env()
        .args_os
        .into_iter()
        // first arg is executable themself
        .skip(1);

    let old_filepath = arg_to_filepath(&args.next());
    let new_filepath = arg_to_filepath(&args.next());

    StartupParam {
        old_filepath,
        new_filepath,
    }
}

#[tauri::command(async)]
pub async fn diff_filepaths(old: &str, new: &str) -> Result<DiffResponse, String> {
    let (old_read, new_read) = match filepaths_content(old, new) {
        Ok(read_contents) => (&read_contents[0].clone(), &read_contents[1].clone()),
        Err(err) => return Err(err),
    };

    let lines_diffs = lines_diffs(old_read.content.as_str(), new_read.content.as_str());

    Ok(DiffResponse {
        old_charset: old_read.charset.to_owned(),
        new_charset: new_read.charset.to_owned(),
        lines_diffs,
    })
}

#[tauri::command(async)]
pub async fn diff_chars(lines_diffs: Vec<LinesDiff>) -> Result<CharsDiffResponse, ()> {
    let diffs = chars_diffs(&lines_diffs);
    Ok(CharsDiffResponse { diffs })
}

#[tauri::command]
pub fn save(filepath: &str, content: &str, charset: &str) -> Result<(), String> {
    match file::save(filepath, content, charset) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
pub fn list_dir(current_dir: &str) -> Result<ListDirReponse, String> {
    file::list_dir(current_dir)
}

#[tauri::command]
pub fn open_with_file_manager(dirpath: &str) -> Result<(), String> {
    let dirpath = Path::new(dirpath)
        .canonicalize()
        .expect(format!("Failed to get path {}", dirpath).as_str());

    let command = file_manager_command();

    Command::new(command)
        .arg(dirpath)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
