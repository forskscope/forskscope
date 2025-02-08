// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod core;

#[cfg(debug_assertions)]
use tauri::Manager;

use core::handlers::diff_filepaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![diff_filepaths])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
