// use tauri::Manager;

use std::path::Path;
use std::process::Command;

use tauri::State;

use crate::types::StartupParam;

use super::diff::{chars_diffs, lines_diffs};
use super::file::{self, file_manager_command, filepath_content};
use super::types::{CharsDiffResponse, DiffResponse, LinesDiff, ListDirReponse};

// #[tauri::command]
// pub fn startup_args(app_handle: tauri::AppHandle) -> Vec<String> {
//     app_handle
//         .env()
//         .args_os
//         .into_iter()
//         // first arg is executable themself
//         .skip(1)
//         .map(|x| {
//             x.to_os_string()
//                 .into_string()
//                 .unwrap_or_else(|x| x.to_string_lossy().into_owned())
//         })
//         .collect()
// }

#[tauri::command(async)]
pub async fn diff_filepaths(old: &str, new: &str) -> Result<DiffResponse, ()> {
    let old_read = filepath_content(old);
    let new_read = filepath_content(new);
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

// #[tauri::command]
// pub fn diff_contents(old: &str, new: &str) -> Vec<LinesDiff> {
//     lines_diff(old, new)
// }

// #[tauri::command]
// pub fn path_join(path1: &str, path2: &str) -> String {
//     let p1 = Path::new(path1);
//     let p2 = Path::new(path2);
//     p1.join(p2)
//         .canonicalize()
//         .expect(format!("Failed to canonicalize combined {} and {}", path1, path2).as_str())
//         .into_os_string()
//         .into_string()
//         .unwrap_or_else(|x| x.to_string_lossy().into_owned())
// }

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

#[tauri::command]
pub fn ready(state: State<StartupParam>) -> Result<StartupParam, String> {
    let startup_param = StartupParam {
        old_filepath: state.old_filepath.clone(),
        new_filepath: state.new_filepath.clone(),
    };

    // clean up startup param
    std::mem::drop(state);

    Ok(startup_param)
}
