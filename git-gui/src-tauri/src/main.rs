// Prevent the unused import warning for commands module
#[allow(unused_imports)]
pub mod commands;
pub mod error;
pub mod state;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            // Repository management commands
            commands::repo::open_repository,
            commands::repo::init_repository,
            commands::repo::clone_repository,
            commands::repo::get_repo_info,
            // Git operations commands
            commands::status::get_commit_history,
            commands::status::get_repo_status,
            commands::status::stage_file,
            commands::status::unstage_file,
            commands::status::create_commit,
            commands::status::get_diff,
            commands::status::discard_changes,
            // Working status commands
            commands::status::get_working_status,
            commands::status::stage_all,
            commands::status::setup_file_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
