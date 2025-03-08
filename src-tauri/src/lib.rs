use tauri::Manager;
use types::StartupParam;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod core;
pub mod types;

use core::handlers::{diff_chars, diff_filepaths, list_dir, open_with_file_manager, ready};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(startup_param: StartupParam) {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            Ok(())
        })
        .manage(startup_param)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            diff_filepaths,
            diff_chars,
            list_dir,
            open_with_file_manager,
            ready
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
