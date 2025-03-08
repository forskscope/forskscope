// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;

use patch_hygge_lib::types::StartupParam;

fn main() {
    let args: Vec<String> = env::args().collect();

    let old_filepath = if 2 <= args.len() {
        Some(args[1].to_owned())
    } else {
        None
    };
    let new_filepath = if 3 <= args.len() {
        Some(args[2].to_owned())
    } else {
        None
    };

    let startup_param = StartupParam {
        old_filepath,
        new_filepath,
    };
    patch_hygge_lib::run(startup_param)
}
