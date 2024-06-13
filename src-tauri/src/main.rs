// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    // let s = "Wow This\nis perhaps a\ncat.\n".split("\n").collect::<Vec<&str>>();
    // let t = "This\nis not a\ncat.\nOh...\n".split("\n").collect::<Vec<&str>>();
    // let d = similar::TextDiff::configure().diff_slices(s.as_slice(), t.as_slice());
    // d.ops().iter().for_each(|x| println!("{:?}|{}|{}", x.tag(), s[x.old_range()].join("\n"), t[x.new_range()].join("\n")));

    // println!("{:?}", d.ops()  (|x| println!("{:?}", x.tag())));
    
    // d.iter_all_changes().into_iter().for_each(|x| println!("{:?}", x));
    // let diff = similar::TextDiff::configure().diff_lines(S, T);
    // print!("{}", diff.unified_diff()
    // .header("old_file", "new_file"));

    tauri::Builder::default()
        
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.set_zoom(1.20).unwrap();
                window.open_devtools();
                window.close_devtools();
            }
            Ok(())
        })

        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
