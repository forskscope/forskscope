// use tauri::Manager;

use std::path::Path;
use std::process::Command;
use std::time::UNIX_EPOCH;

use chrono::{Local, TimeZone};

use super::diff::{chars_diffs, lines_diffs};
use super::file::{
    comma_separated_bytes_size, file_manager_command, filepath_content, human_readable_size,
};
use super::types::{CharsDiffResponse, DiffResponse, FileAttr, LinesDiff, ListDirReponse};

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
    let target_dir = if current_dir.is_empty() {
        std::env::current_dir().expect("Failed to get current directory")
    } else {
        Path::new(current_dir)
            .canonicalize()
            .expect(format!("Failed to canonicalize path: {}", current_dir).as_str())
    };
    let mut dirs = Vec::<String>::new();
    let mut files = Vec::<FileAttr>::new();

    let read = match std::fs::read_dir(target_dir.as_path()) {
        Ok(x) => x,
        Err(err) => {
            return Err(format!("Invalid path: {} ({})", current_dir, err));
        }
    };
    for x in read {
        match x {
            Ok(dir_entry) => {
                let name = dir_entry.file_name().to_string_lossy().to_string();
                match dir_entry.metadata() {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            dirs.push(name)
                        } else {
                            let modified = metadata
                                .modified()
                                .unwrap()
                                .duration_since(UNIX_EPOCH)
                                .unwrap();
                            let local_timestamp = Local.timestamp_nanos(modified.as_nanos() as i64);
                            let last_modified =
                                local_timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                            files.push(FileAttr {
                                name,
                                bytes_size: format!(
                                    "{} bytes",
                                    comma_separated_bytes_size(metadata.len())
                                ),
                                human_readable_size: human_readable_size(metadata.len()),
                                last_modified,
                            })
                        }
                    }
                    _ => {}
                }
            }
            // todo
            Err(err) => println!("Failed to get dir/file info due to {}", err),
        }
    }
    Ok(ListDirReponse {
        current_dir: target_dir.to_str().unwrap().to_owned(),
        dirs: dirs,
        files: files,
    })
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
