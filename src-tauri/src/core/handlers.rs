// use tauri::Manager;

use std::path::{Path, MAIN_SEPARATOR};
use std::process::Command;

use tauri::Manager;

use super::diff::{self, chars_diffs, lines_diffs, startup_compare_set_item};
use super::file::{self, file_manager_command, filepaths_content};
use super::types::{CharsDiffResponse, CompareSet, LinesDiff, LinesDiffResponse, ListDirResponse};

#[tauri::command]
/// app starter to collect frontend startup info
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
/// check if path is file (excluding symlink)
pub fn is_file(filepath: &str) -> Result<bool, String> {
    let path = Path::new(filepath);
    let metadata = path.metadata().expect("Failed to get file metadata");

    if !metadata.is_symlink() {
        Ok(metadata.is_file())
    } else {
        Err("May be symlink".to_owned())
    }
}

#[tauri::command(async)]
/// collect diff around content to file paths
pub async fn diff_filepaths(old: &str, new: &str) -> Result<LinesDiffResponse, String> {
    let (old_read, new_read) = match filepaths_content(old, new) {
        Ok(read_contents) => (&read_contents[0].clone(), &read_contents[1].clone()),
        Err(err) => return Err(err),
    };

    let diffs = lines_diffs(old_read.content.as_str(), new_read.content.as_str());

    Ok(LinesDiffResponse {
        old_charset: old_read.charset.to_owned(),
        new_charset: new_read.charset.to_owned(),
        diffs,
    })
}

#[tauri::command(async)]
/// collect diff on chars
pub async fn diff_chars(lines_diffs: Vec<LinesDiff>) -> Result<CharsDiffResponse, ()> {
    let diffs = chars_diffs(&lines_diffs);
    Ok(CharsDiffResponse { diffs })
}

#[tauri::command]
/// list directory to draw files and dirs
pub fn list_dir(current_dir: &str) -> Result<ListDirResponse, String> {
    file::list_dir(current_dir)
}

#[tauri::command]
// todo: remove ?
pub fn path_separator() -> Result<char, String> {
    Ok(MAIN_SEPARATOR)
}

#[tauri::command]
/// check if file is able to be compared via binary only
pub fn binary_comparison_only(filepath: &str) -> Result<bool, String> {
    Ok(diff::binary_comparison_only(filepath))
}

#[tauri::command]
/// collect file digest diff to be shown in explorer
pub fn file_digest_diff(filename: &str, old_dir: &str, new_dir: &str) -> Result<bool, String> {
    diff::file_digest_diff(filename, old_dir, new_dir)
}

#[tauri::command]
/// collect directory digest diff to be shown in explorer
pub fn dir_digest_diff(dirname: &str, old_dir: &str, new_dir: &str) -> Result<bool, String> {
    diff::dir_digest_diff(dirname, old_dir, new_dir)
}

#[tauri::command]
/// save text into file
pub fn save(filepath: &str, content: &str, charset: &str) -> Result<(), String> {
    match file::save(filepath, content, charset) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
/// open file manager with directory path specifid
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
