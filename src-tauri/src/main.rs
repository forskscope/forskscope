// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, path::Path};

use patch_hygge_lib::types::StartupParam;

fn main() {
    patch_hygge_lib::run(startup_param())
}

fn startup_param() -> StartupParam {
    let args: Vec<String> = env::args().collect();

    let old_filepath = if 2 <= args.len() {
        let s = args[1].as_str();
        if Path::new(s).is_file() {
            Some(s.to_owned())
        } else {
            None
        }
    } else {
        None
    };
    let new_filepath = if 3 <= args.len() {
        let s = args[2].as_str();
        if Path::new(s).is_file() {
            Some(s.to_owned())
        } else {
            None
        }
    } else {
        None
    };

    StartupParam {
        old_filepath,
        new_filepath,
    }
}
