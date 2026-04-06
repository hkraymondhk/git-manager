use tauri::State;
use crate::state::AppState;
use crate::error::{AppError, Result};
use std::path::PathBuf;

#[tauri::command]
pub fn open_repository(path: String, state: State<AppState>) -> Result<bool> {
    let repo_path = PathBuf::from(&path);
    
    if !repo_path.exists() {
        return Err(AppError::InvalidPath(path));
    }
    
    // Verify it's a git repository
    git2::Repository::open(&repo_path)?;
    
    let mut current_repo = state.current_repo_path.lock().unwrap();
    *current_repo = Some(path);
    
    Ok(true)
}

#[tauri::command]
pub fn get_current_repository(state: State<AppState>) -> Result<Option<String>> {
    let current_repo = state.current_repo_path.lock().unwrap();
    Ok(current_repo.clone())
}

#[tauri::command]
pub fn close_repository(state: State<AppState>) -> Result<bool> {
    let mut current_repo = state.current_repo_path.lock().unwrap();
    *current_repo = None;
    Ok(true)
}
