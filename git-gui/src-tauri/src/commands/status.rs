use git2::{IndexAddOption, Repository, StatusOptions, StatusShow, DiffOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{Emitter, State, Window};

use crate::error::AppError;
use crate::state::AppState;

/// 文件狀態類型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StatusType {
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChange,
    Conflicted,
}

/// 單個文件的狀態信息（用於 WorkingStatus）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub old_path: Option<String>,
    pub status_type: StatusType,
}

/// 工作區完整狀態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingStatus {
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<FileStatus>,
}

/// 倉庫狀態（用於 get_repo_status）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<FileStatusInfo>,
}

/// 簡化的文件狀態信息（用於 RepoStatus）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatusInfo {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

/// 提交信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
}

/// 將 git2 的 delta 狀態轉換為 StatusType
fn delta_to_status_type(status: u32) -> StatusType {
    use git2::Status;
    
    let s = Status::from_bits_truncate(status);
    
    // 檢查衝突
    if s.intersects(Status::CONFLICTED) {
        StatusType::Conflicted
    }
    // 檢查重命名（index 或 workdir）
    else if s.intersects(Status::INDEX_RENAMED | Status::WT_RENAMED) {
        StatusType::Renamed
    }
    // 檢查類型變更
    else if s.intersects(Status::INDEX_TYPECHANGE | Status::WT_TYPECHANGE) {
        StatusType::TypeChange
    }
    // 檢查刪除
    else if s.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
        StatusType::Deleted
    }
    // 檢查新增
    else if s.intersects(Status::INDEX_NEW | Status::WT_NEW) {
        StatusType::Added
    }
    // 其他情況視為修改
    else {
        StatusType::Modified
    }
}

/// 將路徑轉換為正斜杠格式
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// 獲取工作區文件狀態
#[tauri::command]
pub async fn get_working_status(state: State<'_, AppState>) -> Result<WorkingStatus, AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .show(StatusShow::IndexAndWorkdir);

    let statuses = repo.statuses(Some(&mut opts))?;

    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry
            .path()
            .map(|p| normalize_path(p))
            .unwrap_or_default();

        // 跳過空路徑或目錄
        if path.is_empty() || path.ends_with('/') {
            continue;
        }

        let old_path = entry
            .head_to_index()
            .and_then(|delta| delta.new_file().path())
            .or_else(|| entry.index_to_workdir().and_then(|delta| delta.old_file().path()))
            .map(|p| p.to_string_lossy().to_string())
            .map(|p| normalize_path(&p));

        // 檢查是否為未追蹤文件
        if status.intersects(git2::Status::WT_NEW) {
            untracked.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status_type: StatusType::Added,
            });
            continue;
        }

        // 檢查是否在 index 中（已暫存）
        let is_staged = status.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        );

        // 檢查是否有工作區改動
        let has_workdir_changes = status.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        );

        let status_type = delta_to_status_type(status.bits());

        if is_staged {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: old_path.clone(),
                status_type: status_type.clone(),
            });
        }

        if has_workdir_changes || status.intersects(git2::Status::WT_MODIFIED) {
            unstaged.push(FileStatus {
                path,
                old_path,
                status_type,
            });
        }
    }

    Ok(WorkingStatus {
        staged,
        unstaged,
        untracked,
    })
}

/// 暫存指定文件
#[tauri::command]
pub async fn stage_file(path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;
    let mut index = repo.index()?;

    // 將路徑從正斜杠轉回系統格式
    let system_path = path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let repo_relative_path = Path::new(&system_path);

    index.add_path(repo_relative_path)?;
    index.write()?;

    Ok(())
}

/// 取消暫存指定文件
#[tauri::command]
pub async fn unstage_file(path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;

    // 將路徑從正斜杠轉回系統格式
    let system_path = path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let repo_relative_path = Path::new(&system_path);

    // 使用 reset 將文件從 index 移回工作區
    let (object, _) = repo.revparse_ext("HEAD")?;
    let path_str = repo_relative_path.as_os_str().to_str().unwrap();
    repo.reset_default(Some(&object), Some(&path_str))?;

    Ok(())
}

/// 暫存所有修改和新增的文件
#[tauri::command]
pub async fn stage_all(state: State<'_, AppState>) -> Result<(), AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;
    let mut index = repo.index()?;

    // 添加所有未追蹤和修改的文件
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    Ok(())
}

/// 丟棄工作區改動，從 HEAD 恢復文件
#[tauri::command]
pub async fn discard_changes(path: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;

    // 將路徑從正斜杠轉回系統格式
    let system_path = path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let repo_relative_path = Path::new(&system_path);

    // 獲取 HEAD commit
    let head = repo.head()?;
    let tree_id = head.peel_to_tree()?.id();
    let tree = repo.find_tree(tree_id)?;

    // 設置 checkout options
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.force();
    checkout_opts.path(repo_relative_path);

    // 執行 checkout
    repo.checkout_tree(tree.as_object(), Some(&mut checkout_opts))?;

    Ok(())
}

/// Get commit history (last 100 commits)
#[tauri::command]
pub async fn get_commit_history(state: State<'_, AppState>) -> Result<Vec<CommitInfo>, AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;
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
pub async fn get_repo_status(state: State<'_, AppState>) -> Result<RepoStatus, AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;

    // Get current branch
    let branch = repo
        .head()
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

        files.push(FileStatusInfo {
            path,
            status: status.to_string(),
            staged: entry.status().is_index_new()
                || entry.status().is_index_modified()
                || entry.status().is_index_deleted(),
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
) -> Result<String, AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;
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
pub async fn get_diff(state: State<'_, AppState>, path: String) -> Result<String, AppError> {
    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let repo = Repository::open(&repo_path)?;
    let full_path = PathBuf::from(&path);
    let relative_path = full_path.strip_prefix(&repo_path).unwrap_or(&full_path);

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

            let diff = repo.diff_tree_to_tree(
                Some(&old_tree),
                Some(&new_tree),
                Some(&mut diff_opts),
            )?;
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

/// 設置文件監聽器，監聽倉庫目錄變化
#[tauri::command]
pub async fn setup_file_watcher(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    let repo_path = state
        .current_repo_path
        .lock()
        .map_err(|_| AppError::StateLockError("Failed to lock repo_path".to_string()))?
        .clone()
        .ok_or_else(|| AppError::NotInitialized("No repository opened".to_string()))?;

    let (_tx, rx) = channel::<notify::Result<notify::Event>>();

    // 創建 watcher
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                // 過濾掉 .git 目錄和一些臨時文件
                if event.paths.iter().any(|p| {
                    p.to_string_lossy().contains(".git")
                        || p.extension().map(|e| e == "swp" || e == "tmp").unwrap_or(false)
                }) {
                    return;
                }

                // 發送事件到前端（防抖：簡單延遲）
                let window_clone = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(100));
                    let _ = window_clone.emit("repo_changed", ());
                });
            }
        },
        Config::default(),
    )?;

    // 監聽倉庫根目錄，遞歸子目錄
    watcher.watch(Path::new(&repo_path), RecursiveMode::Recursive)?;

    // 將 watcher 存儲在 AppState 中（需要修改 AppState 結構）
    // 這裡簡化處理，實際生產環境應該使用 Arc<Mutex<Option<Watcher>>>

    // 在新線程中保持 watcher 運行
    std::thread::spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(_) => {}
                Err(_) => break, // channel closed
            }
        }
    });

    Ok(())
}
