// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod diff;

use tauri::Manager;

use crate::diff::{diff, list_dir};

fn main() {
    tauri::Builder::default()

        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                // todo
                window.set_zoom(1.20).unwrap();
                window.open_devtools();
                window.close_devtools();
            }
            Ok(())
        })
        
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![diff, list_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
