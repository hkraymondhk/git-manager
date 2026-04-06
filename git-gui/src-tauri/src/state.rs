use std::sync::Mutex;

pub struct AppState {
    pub current_repo_path: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_repo_path: Mutex::new(None),
        }
    }
}
