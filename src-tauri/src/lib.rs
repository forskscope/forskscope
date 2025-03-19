use tauri::Manager;

mod core;

use core::handlers::{diff_chars, diff_filepaths, list_dir, open_with_file_manager, ready, save};

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
        .invoke_handler(tauri::generate_handler![
            diff_filepaths,
            diff_chars,
            list_dir,
            open_with_file_manager,
            ready,
            save
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
