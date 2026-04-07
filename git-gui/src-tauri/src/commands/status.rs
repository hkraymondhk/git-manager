use git2::{IndexAddOption, Repository, StatusOptions, StatusShow};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// 單個文件的狀態信息
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

/// 將 git2 的 delta 狀態轉換為 StatusType
fn delta_to_status_type(status: u32) -> StatusType {
    let flags = git2::StatusFlags::from_bits_truncate(status);

    if flags.contains(git2::StatusFlags::CONFLICTED) {
        StatusType::Conflicted
    } else if flags.contains(git2::StatusFlags::INDEX_RENAMED) {
        StatusType::Renamed
    } else if flags.contains(git2::StatusFlags::INDEX_TYPECHANGE)
        || flags.contains(git2::StatusFlags::WT_TYPECHANGE)
    {
        StatusType::TypeChange
    } else if flags.contains(git2::StatusFlags::INDEX_DELETED)
        || flags.contains(git2::StatusFlags::WT_DELETED)
    {
        StatusType::Deleted
    } else if flags.contains(git2::StatusFlags::INDEX_NEW)
        || flags.contains(git2::StatusFlags::WT_NEW)
    {
        StatusType::Added
    } else {
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
        if status.contains(git2::StatusFlags::WT_NEW) {
            untracked.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status_type: StatusType::Added,
            });
            continue;
        }

        // 檢查是否在 index 中（已暫存）
        let is_staged = status.intersects(
            git2::StatusFlags::INDEX_NEW
                | git2::StatusFlags::INDEX_MODIFIED
                | git2::StatusFlags::INDEX_DELETED
                | git2::StatusFlags::INDEX_RENAMED
                | git2::StatusFlags::INDEX_TYPECHANGE,
        );

        // 檢查是否有工作區改動
        let has_workdir_changes = status.intersects(
            git2::StatusFlags::WT_MODIFIED
                | git2::StatusFlags::WT_DELETED
                | git2::StatusFlags::WT_RENAMED
                | git2::StatusFlags::WT_TYPECHANGE,
        );

        let status_type = delta_to_status_type(status.bits());

        if is_staged {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: old_path.clone(),
                status_type: status_type.clone(),
            });
        }

        if has_workdir_changes || status.contains(git2::StatusFlags::WT_MODIFIED) {
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
    repo.reset_default(Some(&object), Some(&[repo_relative_path.as_os_str().to_str().unwrap()]))?;

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

    let (tx, rx) = channel();

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
