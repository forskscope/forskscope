use tauri::Manager;

mod core;

use core::handlers::{
    diff_chars, diff_filepaths, list_dir, open_with_file_manager, ready, save, validate_filepath,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            diff_filepaths,
            diff_chars,
            list_dir,
            open_with_file_manager,
            ready,
            save,
            validate_filepath
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
