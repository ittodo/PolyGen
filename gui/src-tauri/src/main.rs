// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::run_generate,
            commands::run_migrate,
            commands::get_polygen_version,
            commands::read_file,
            commands::write_file,
            commands::parse_imports,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
