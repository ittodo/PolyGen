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
            commands::validate_schema,
            commands::goto_definition,
            commands::find_references,
            commands::get_hover_info,
            commands::get_completions,
            commands::get_document_symbols,
            commands::prepare_rename,
            commands::rename_symbol,
            commands::get_schema_visualization,
            commands::get_recent_projects,
            commands::add_recent_project,
            commands::remove_recent_project,
            commands::clear_recent_projects,
            // Template editor commands
            commands::list_template_languages,
            commands::list_template_files,
            commands::read_template_file,
            commands::write_template_file,
            commands::create_new_language,
            commands::create_new_language_v2,
            commands::delete_template_file,
            commands::preview_template,
            commands::validate_rhai_script,
            commands::read_language_config,
            commands::write_language_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
