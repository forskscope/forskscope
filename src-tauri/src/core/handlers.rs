// use tauri::Manager;

use super::diff::lines_diff;
use super::file::filepath_content;
use super::types::LinesDiff;

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

#[tauri::command]
pub fn diff_filepaths(old: &str, new: &str) -> Vec<LinesDiff> {
    let old_content = filepath_content(old);
    let new_content = filepath_content(new);
    lines_diff(old_content.as_str(), new_content.as_str())
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

// #[tauri::command]
// pub fn list_dir(current_dir: &str) -> Result<ListDirReponse, String> {
//     let current_dir = if current_dir.is_empty() {
//         std::env::current_dir()
//             .expect("Failed to get current directory")
//             .into_os_string()
//             .into_string()
//             .expect("Failed to get os specific path")
//     } else {
//         current_dir.to_owned()
//     };
//     let mut dirs = Vec::<String>::new();
//     let mut files = Vec::<String>::new();

//     let read = std::fs::read_dir(current_dir.as_str());
//     // todo: return error to frontend
//     if let Err(_) = read {
//         return Err(format!("Invalid path: {}", current_dir.as_str()));
//     }
//     for x in read.unwrap() {
//         match x {
//             Ok(dir_entry) => {
//                 let name = dir_entry.file_name().to_string_lossy().to_string();
//                 match dir_entry.file_type() {
//                     Ok(file_type) => {
//                         if file_type.is_dir() {
//                             dirs.push(name)
//                         } else {
//                             files.push(name)
//                         }
//                     }
//                     _ => {}
//                 }
//             }
//             // todo
//             Err(err) => println!("Failed to get dir/file info due to {}", err),
//         }
//     }
//     Ok(ListDirReponse {
//         current_dir: current_dir,
//         dirs: dirs,
//         files: files,
//     })
// }
