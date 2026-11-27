// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = cubo_desktop_lib::run() {
        eprintln!("Error fatal en la aplicación: {err:?}");
        std::process::exit(1);
    }
}
