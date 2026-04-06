// Prevent the unused import warning for commands module
#[allow(unused_imports)]
pub mod commands;
pub mod error;
pub mod state;

use tauri::{Manager, RunEvent};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::repo::open_repository,
            commands::repo::get_current_repository,
            commands::repo::close_repository,
        ])
        .run(|_app_handle, event| {
            if let RunEvent::Exit = event {
                // Cleanup on exit
            }
        })
        .expect("error while running tauri application");
}
