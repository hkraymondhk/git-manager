use crate::error::{AppError, Result};
use crate::state::AppState;
use git2::{Repository, StatusOptions, DiffOptions};

use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<FileStatus>,
}

/// Get commit history (last 100 commits)
#[tauri::command]
pub async fn get_commit_history(state: State<'_, AppState>) -> Result<Vec<CommitInfo>> {
    let repo_path = state.current_repo_path.lock().unwrap();
    let repo_path = repo_path.as_ref().ok_or(AppError::NoRepository)?;
    
    let repo = Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    
    let commits: Vec<CommitInfo> = revwalk
        .filter_map(|oid| oid.ok())
        .filter_map(|oid| {
            repo.find_commit(oid).ok().map(|commit| {
                let time = commit.time();
                CommitInfo {
                    id: oid.to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    email: commit.author().email().unwrap_or("").to_string(),
                    timestamp: time.seconds(),
                }
            })
        })
        .take(100)
        .collect();
    
    Ok(commits)
}

/// Get repository status (branch, ahead/behind, changed files)
#[tauri::command]
pub async fn get_repo_status(state: State<'_, AppState>) -> Result<RepoStatus> {
    let repo_path = state.current_repo_path.lock().unwrap();
    let repo_path = repo_path.as_ref().ok_or(AppError::NoRepository)?;
    
    let repo = Repository::open(repo_path)?;
    
    // Get current branch
    let branch = repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));
    
    // Get ahead/behind count
    let (ahead, behind) = if let Some(branch_name) = &branch {
        match repo.find_branch(branch_name, git2::BranchType::Local) {
            Ok(local_branch) => {
                if let Ok(upstream) = local_branch.upstream() {
                    let local_oid = local_branch.get().target();
                    let upstream_oid = upstream.get().target();
                    if let (Some(local), Some(upstream)) = (local_oid, upstream_oid) {
                        repo.graph_ahead_behind(local, upstream).unwrap_or((0, 0))
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                }
            }
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    };
    
    // Get file statuses
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut files = Vec::new();
    
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("unknown").to_string();
        let status = if entry.status().is_wt_new() {
            "untracked"
        } else if entry.status().is_index_new() {
            "staged"
        } else if entry.status().is_wt_modified() {
            "modified"
        } else if entry.status().is_index_modified() {
            "staged"
        } else if entry.status().is_wt_deleted() {
            "deleted"
        } else if entry.status().is_index_deleted() {
            "staged"
        } else {
            "changed"
        };
        
        files.push(FileStatus {
            path,
            status: status.to_string(),
            staged: entry.status().is_index_new() || entry.status().is_index_modified() || entry.status().is_index_deleted(),
        });
    }
    
    Ok(RepoStatus {
        branch,
        ahead,
        behind,
        files,
    })
}


/// Create a commit
#[tauri::command]
pub async fn create_commit(
    state: State<'_, AppState>,
    message: String,
) -> Result<String> {
    let repo_path = state.current_repo_path.lock().unwrap();
    let repo_path = repo_path.as_ref().ok_or(AppError::NoRepository)?;
    
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;
    
    if index.is_empty() {
        return Err(AppError::Git(git2::Error::from_str("Nothing to commit")));
    }
    
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    
    let signature = repo.signature()?;
    
    let parents = if let Ok(head) = repo.head() {
        vec![repo.find_commit(head.target().unwrap())?]
    } else {
        vec![]
    };
    
    let commit = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &message,
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )?;
    
    Ok(commit.to_string())
}

/// Get diff for a file
#[tauri::command]
pub async fn get_diff(state: State<'_, AppState>, path: String) -> Result<String> {
    let repo_path = state.current_repo_path.lock().unwrap();
    let repo_path = repo_path.as_ref().ok_or(AppError::NoRepository)?;
    
    let repo = Repository::open(repo_path)?;
    let full_path = PathBuf::from(&path);
    let relative_path = full_path.strip_prefix(repo_path).unwrap_or(&full_path);
    
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(relative_path);
    
    let diff = repo.diff_index_to_workdir(None, Some(&mut diff_opts))?;
    
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = match line.origin() {
            '+' | '>' | 'B' => "+",
            '-' | '<' => "-",
            ' ' | '=' | '|' => " ",
            _ => "",
        };
        let content = String::from_utf8_lossy(line.content());
        diff_text.push_str(origin);
        diff_text.push_str(&content);
        if !content.ends_with('\n') {
            diff_text.push('\n');
        }
        true
    })?;
    
    if diff_text.is_empty() {
        // Try to get staged diff
        if let Ok(head) = repo.head() {
            let commit = repo.find_commit(head.target().unwrap())?;
            let old_tree = commit.tree()?;
            let new_tree = repo.find_tree(repo.index()?.write_tree()?)?;
            
            let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut diff_opts))?;
            diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                let origin = match line.origin() {
                    '+' | '>' | 'B' => "+",
                    '-' | '<' => "-",
                    ' ' | '=' | '|' => " ",
                    _ => "",
                };
                let content = String::from_utf8_lossy(line.content());
                diff_text.push_str(origin);
                diff_text.push_str(&content);
                if !content.ends_with('\n') {
                    diff_text.push('\n');
                }
                true
            })?;
        }
    }
    
    Ok(diff_text)
}

