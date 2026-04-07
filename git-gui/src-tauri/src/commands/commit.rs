use git2::{Repository, Signature};
use std::path::PathBuf;
use crate::state::AppState;
use crate::commands::status::{FileStatus, StatusType};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitSummary {
    pub oid: String,
    pub message: String,
    pub author: AuthorInfo,
    pub timestamp: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitDetail {
    pub oid: String,
    pub message: String,
    pub author: AuthorInfo,
    pub committer: AuthorInfo,
    pub parents: Vec<String>,
    pub changed_files: Vec<FileStatus>,
    pub timestamp: i64,
}

#[tauri::command]
pub fn create_commit(state: tauri::State<AppState>, message: String, amend: bool) -> Result<CommitSummary, String> {
    let repo_path = state.current_repo_path.lock().map_err(|_| "Failed to lock repo_path".to_string())?;
    let repo_path_buf = PathBuf::from(repo_path.clone().ok_or_else(|| "No repository opened".to_string())?);
    
    let repo = Repository::open(&repo_path_buf)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    // 獲取簽名 (user.name, user.email)
    let signature = get_signature(&repo)?;

    // 準備索引
    let mut index = repo.index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    
    index.write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    let tree_id = index.write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    
    let tree = repo.find_tree(tree_id)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    let oid = if amend {
        // Amend 模式：修改最後一個提交
        let head = repo.head()
            .map_err(|e| format!("Failed to get HEAD: {}", e))?;
        
        let parent_commit = repo.find_commit(head.target().unwrap())
            .map_err(|e| format!("Failed to find parent commit: {}", e))?;

        // 使用 amend 方法 - 正確參數順序：update_ref, author, committer, message_encoding, tree, message
        let new_oid = parent_commit.amend(
            Some("HEAD"),           // update_ref
            Some(&signature),       // author
            Some(&signature),       // committer
            None,                   // message_encoding (Option<&str>)
            Some(&tree),            // tree (Option<&Tree>)
            Some(message.as_str()), // message (Option<&str>)
        ).map_err(|e| format!("Failed to amend commit: {}", e))?;

        new_oid
    } else {
        // 普通提交
        let parents_vec: Vec<git2::Commit>;
        let parents_refs: Vec<&git2::Commit>;
        
        if let Ok(head) = repo.head() {
            if let Some(target) = head.target() {
                let parent = repo.find_commit(target)
                    .map_err(|e| format!("Failed to find parent: {}", e))?;
                parents_vec = vec![parent];
                parents_refs = parents_vec.iter().collect();
            } else {
                parents_vec = vec![];
                parents_refs = vec![];
            }
        } else {
            parents_vec = vec![];
            parents_refs = vec![];
        }

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parents_refs.as_slice(),
        ).map_err(|e| format!("Failed to create commit: {}", e))?
    };

    let commit = repo.find_commit(oid)
        .map_err(|e| format!("Failed to find new commit: {}", e))?;

    Ok(CommitSummary {
        oid: oid.to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: AuthorInfo {
            name: signature.name().unwrap_or("").to_string(),
            email: signature.email().unwrap_or("").to_string(),
        },
        timestamp: commit.time().seconds(),
    })
}

fn get_signature(repo: &Repository) -> Result<Signature, String> {
    // 嘗試從 config 獲取
    let config = repo.config()
        .map_err(|e| format!("Failed to get config: {}", e))?;
    
    let name = config.get_string("user.name")
        .unwrap_or_else(|_| "Unknown".to_string());
    
    let email = config.get_string("user.email")
        .unwrap_or_else(|_| "unknown@example.com".to_string());

    Signature::now(&name, &email)
        .map_err(|e| format!("Failed to create signature: {}", e))
}

#[tauri::command]
pub fn get_commit_detail(state: tauri::State<AppState>, oid: String) -> Result<CommitDetail, String> {
    let repo_path = state.current_repo_path.lock().map_err(|_| "Failed to lock repo_path".to_string())?;
    let repo_path_buf = PathBuf::from(repo_path.clone().ok_or_else(|| "No repository opened".to_string())?);
    
    let repo = Repository::open(&repo_path_buf)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let commit_oid = git2::Oid::from_str(&oid)
        .map_err(|e| format!("Invalid OID: {}", e))?;

    let commit = repo.find_commit(commit_oid)
        .map_err(|e| format!("Commit not found: {}", e))?;

    // 獲取作者
    let author = commit.author();
    let author_info = AuthorInfo {
        name: author.name().unwrap_or("").to_string(),
        email: author.email().unwrap_or("").to_string(),
    };

    // 獲取提交者
    let committer = commit.committer();
    let committer_info = AuthorInfo {
        name: committer.name().unwrap_or("").to_string(),
        email: committer.email().unwrap_or("").to_string(),
    };

    // 獲取父提交
    let parents: Vec<String> = commit.parents()
        .filter_map(|p| Some(p.id().to_string()))
        .collect();

    // 獲取變更的文件
    let mut changed_files = Vec::new();
    
    if commit.parent_count() > 0 {
        let parent = commit.parent(0)
            .map_err(|e| format!("Failed to get parent: {}", e))?;
        
        let parent_tree = parent.tree()
            .map_err(|e| format!("Failed to get parent tree: {}", e))?;
        
        let current_tree = commit.tree()
            .map_err(|e| format!("Failed to get current tree: {}", e))?;

        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true);
        
        let diff = repo.diff_tree_to_tree(
            Some(&parent_tree),
            Some(&current_tree),
            Some(&mut opts),
        ).map_err(|e| format!("Failed to create diff: {}", e))?;

        diff.foreach(
            &mut |delta, _hunks| {
                let file = delta.new_file();
                let path = file.path().map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                
                let status_type = match delta.status() {
                    git2::Delta::Added => StatusType::Added,
                    git2::Delta::Deleted => StatusType::Deleted,
                    git2::Delta::Modified => StatusType::Modified,
                    git2::Delta::Renamed => StatusType::Renamed,
                    git2::Delta::Typechange => StatusType::Modified,
                    _ => StatusType::Modified,
                };

                changed_files.push(FileStatus {
                    path,
                    old_path: None,
                    status_type,
                });
                true
            },
            None,
            None,
            None,
        ).map_err(|e| format!("Failed to process diff: {}", e))?;
    } else {
        // 初始提交，所有文件都是新增的
        let tree = commit.tree()
            .map_err(|e| format!("Failed to get tree: {}", e))?;
        
        tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            changed_files.push(FileStatus {
                path: entry.name().unwrap_or("unknown").to_string(),
                old_path: None,
                status_type: StatusType::Added,
            });
            git2::TreeWalkResult::Ok
        }).map_err(|e| format!("Failed to walk tree: {}", e))?;
    }

    Ok(CommitDetail {
        oid: commit.id().to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: author_info,
        committer: committer_info,
        parents,
        changed_files,
        timestamp: commit.time().seconds(),
    })
}
