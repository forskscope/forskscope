// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

/// app entry point
fn main() {
    mitigate_nvidia_gbm_failure_on_linux_webkitgtk();

    forskscope_lib::run()
}

/// migigate nvidia drive does not support linux webkitgtk around gbm / dma-buf
fn mitigate_nvidia_gbm_failure_on_linux_webkitgtk() {
    if !cfg!(target_os = "linux") {
        return;
    }

    let nvidia_driver_is_active = match Command::new("sh")
        .arg("-c")
        .arg("lspci -k | grep -EA3 'VGA|3D|Display'")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("NVIDIA") && stdout.contains("Kernel driver in use: nvidia")
        }
        Err(_) => false,
    };
    if !nvidia_driver_is_active {
        return;
    }

    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

        #[cfg(debug_assertions)]
        println!("WEBKIT_DISABLE_DMABUF_RENDERER set to 1 by application (NVIDIA Linux detected).");
    }
}
