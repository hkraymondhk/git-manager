use tauri::{State, EventTarget, Emitter};
use crate::state::AppState;
use crate::error::{AppError, Result};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Commit summary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub path: String,
    pub current_branch: Option<String>,
    pub is_bare: bool,
    pub head_commit: Option<CommitSummary>,
}

/// Open an existing repository
#[tauri::command]
pub fn open_repository(path: String, state: State<AppState>) -> Result<RepoInfo> {
    let repo_path = PathBuf::from(&path);
    
    if !repo_path.exists() {
        return Err(AppError::InvalidPath(path));
    }
    
    let repo = git2::Repository::open(&repo_path)?;
    
    // Get current branch
    let current_branch = repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));
    
    // Check if bare
    let is_bare = repo.is_bare();
    
    // Get HEAD commit summary
    let head_commit = repo.head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| CommitSummary {
            oid: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            timestamp: commit.time().seconds(),
        });
    
    let repo_info = RepoInfo {
        path: path.clone(),
        current_branch,
        is_bare,
        head_commit,
    };
    
    // Save to AppState
    let mut current_repo = state.current_repo_path.lock().unwrap();
    *current_repo = Some(path);
    
    Ok(repo_info)
}

/// Initialize a new repository
#[tauri::command]
pub fn init_repository(path: String) -> Result<RepoInfo> {
    let repo_path = PathBuf::from(&path);
    
    // Create parent directories if they don't exist
    if let Some(parent) = repo_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let repo = git2::Repository::init(&repo_path)?;
    
    let repo_info = RepoInfo {
        path: path.clone(),
        current_branch: None,
        is_bare: false,
        head_commit: None,
    };
    
    Ok(repo_info)
}

/// Clone a remote repository with progress reporting
#[tauri::command]
pub async fn clone_repository(
    url: String,
    path: String,
    app_handle: tauri::AppHandle,
) -> Result<RepoInfo> {
    let repo_path = PathBuf::from(&path);
    
    // Create parent directories if they don't exist
    if let Some(parent) = repo_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let mut builder = git2::build::RepoBuilder::new();
    
    // Configure fetch options with progress callback
    let mut fetch_opts = git2::FetchOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    
    callbacks.transfer_progress(|progress| {
        if progress.total_objects() > 0 {
            let percent = (progress.received_objects() as f32 / progress.total_objects() as f32 * 100.0) as u8;
            let _ = app_handle.emit(
                "clone_progress",
                serde_json::json!({
                    "stage": "fetching",
                    "received": progress.received_objects(),
                    "total": progress.total_objects(),
                    "percent": percent,
                })
            );
        }
        true // continue
    });
    
    fetch_opts.remote_callbacks(callbacks);
    builder.fetch_options(fetch_opts);
    
    let repo = builder.clone(&url, &repo_path)?;
    
    // Emit completion event
    let _ = app_handle.emit(
        "clone_progress",
        serde_json::json!({
            "stage": "complete",
            "message": "Clone completed successfully",
        })
    );
    
    // Get current branch
    let current_branch = repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));
    
    // Get HEAD commit summary
    let head_commit = repo.head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| CommitSummary {
            oid: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            timestamp: commit.time().seconds(),
        });
    
    let repo_info = RepoInfo {
        path: path.clone(),
        current_branch,
        is_bare: repo.is_bare(),
        head_commit,
    };
    
    Ok(repo_info)
}

/// Get current repository info from AppState
#[tauri::command]
pub fn get_repo_info(state: State<AppState>) -> Result<Option<RepoInfo>> {
    let current_repo = state.current_repo_path.lock().unwrap();
    
    match &*current_repo {
        Some(path) => {
            let repo_path = PathBuf::from(path);
            
            if !repo_path.exists() {
                return Ok(None);
            }
            
            let repo = git2::Repository::open(&repo_path)?;
            
            // Get current branch
            let current_branch = repo.head()
                .ok()
                .and_then(|head| head.shorthand().map(|s| s.to_string()));
            
            // Check if bare
            let is_bare = repo.is_bare();
            
            // Get HEAD commit summary
            let head_commit = repo.head()
                .ok()
                .and_then(|head| head.peel_to_commit().ok())
                .map(|commit| CommitSummary {
                    oid: commit.id().to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    timestamp: commit.time().seconds(),
                });
            
            Ok(Some(RepoInfo {
                path: path.clone(),
                current_branch,
                is_bare,
                head_commit,
            }))
        }
        None => Ok(None),
    }
}
