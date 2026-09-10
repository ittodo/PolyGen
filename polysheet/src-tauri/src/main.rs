#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

const VELOPACK_HOOKS: [&str; 4] = [
    "--veloapp-install",
    "--veloapp-obsolete",
    "--veloapp-updated",
    "--veloapp-uninstall",
];

fn is_velopack_hook(argument: &str) -> bool {
    VELOPACK_HOOKS.contains(&argument)
}

fn main() {
    if std::env::args()
        .skip(1)
        .any(|argument| is_velopack_hook(&argument))
    {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::create_unsaved_project,
            commands::create_project,
            commands::open_project,
            commands::attach_schema,
            commands::save_project_as,
            commands::project_summary,
            commands::bind_data_sheet,
            commands::add_calculation_sheet,
            commands::get_rows_chunk,
            commands::get_sheet_json,
            commands::get_row_json,
            commands::get_row_draft,
            commands::compare_rows,
            commands::get_sheet_formulas,
            commands::get_calculation_sheet,
            commands::apply_calculation_sheet,
            commands::insert_calculation_row,
            commands::insert_calculation_column,
            commands::apply_sheet_json,
            commands::apply_row_json,
            commands::insert_row_json,
            commands::apply_cell_edits,
            commands::apply_row_order,
            commands::confirm_row_identity,
            commands::get_normalization_preview,
            commands::save_project,
            commands::list_git_refs,
            commands::diff_project,
            commands::preview_merge,
            commands::apply_pending_merge,
            commands::stage_sheets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PolySheet");
}

#[cfg(test)]
mod tests {
    use super::{is_velopack_hook, VELOPACK_HOOKS};

    #[test]
    fn recognizes_all_velopack_fast_exit_hooks() {
        for hook in VELOPACK_HOOKS {
            assert!(is_velopack_hook(hook));
        }
        assert!(!is_velopack_hook("--project"));
    }
}
