// Prevent the unused import warning for commands module
#[allow(unused_imports)]
pub mod commands;
pub mod error;
pub mod state;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::repo::open_repository,
            commands::repo::get_current_repository,
            commands::repo::close_repository,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
