use tauri::Manager;

mod core;

use core::handlers::{
    binary_comparison_only, diff_chars, diff_filepaths, dir_digest_diff, file_digest_diff,
    list_dir, open_with_file_manager, path_separator, ready, save,
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
            binary_comparison_only,
            diff_filepaths,
            diff_chars,
            dir_digest_diff,
            file_digest_diff,
            list_dir,
            open_with_file_manager,
            path_separator,
            ready,
            save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
