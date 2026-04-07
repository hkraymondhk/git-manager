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
            commands::git_ops::get_commit_history,
            commands::git_ops::get_repo_status,
            commands::git_ops::stage_file,
            commands::git_ops::unstage_file,
            commands::git_ops::create_commit,
            commands::git_ops::get_diff,
            commands::git_ops::discard_changes,
            // Working status commands
            commands::status::get_working_status,
            commands::status::stage_file,
            commands::status::unstage_file,
            commands::status::stage_all,
            commands::status::discard_changes,
            commands::status::setup_file_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
