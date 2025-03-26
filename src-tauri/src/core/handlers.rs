// use tauri::Manager;

use std::path::Path;
use std::process::Command;

use tauri::Manager;

use super::diff::{self, chars_diffs, lines_diffs, startup_compare_set_item};
use super::file::{self, file_manager_command, filepaths_content};
use super::types::{CharsDiffResponse, CompareSet, DiffResponse, LinesDiff, ListDirReponse};

#[tauri::command]
pub fn ready(app_handle: tauri::AppHandle) -> CompareSet {
    let mut args = app_handle
        .env()
        .args_os
        .into_iter()
        // first arg is executable themself
        .skip(1);

    let old = startup_compare_set_item(&args.next());
    let new = startup_compare_set_item(&args.next());
    CompareSet { old, new }
}

#[tauri::command]
pub fn binary_comparison_only(filepath: &str) -> Result<bool, String> {
    Ok(diff::binary_comparison_only(filepath))
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
