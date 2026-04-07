use git2::{Repository, Revwalk, Sort};
use std::path::PathBuf;
use crate::state::AppState;
use crate::commands::commit::{CommitSummary, AuthorInfo};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogOptions {
    pub branch: Option<String>,   // None = HEAD
    pub path_filter: Option<String>, // 文件歷史過濾
    pub limit: usize,             // 默認 200
    pub offset: usize,            // 分頁
    pub search: Option<String>,   // 搜索 message/author
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphData {
    pub commits: Vec<GraphCommit>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphCommit {
    pub summary: CommitSummary,
    pub lane: usize,
    pub branches: Vec<String>, // 指向該 commit 的分支名
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Connection {
    pub to_oid: String,
    pub from_lane: usize,
    pub to_lane: usize,
}

#[tauri::command]
pub fn get_commit_log(state: tauri::State<AppState>, options: LogOptions) -> Result<Vec<CommitSummary>, String> {
    let repo_path = state.current_repo_path.lock().map_err(|_| "Failed to lock repo_path".to_string())?;
    let repo_path_buf = PathBuf::from(repo_path.clone().ok_or_else(|| "No repository opened".to_string())?);
    
    let repo = Repository::open(&repo_path_buf)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    // 確定起始點
    let start_oid = match &options.branch {
        Some(branch_name) => {
            // 嘗試解析分支名
            let refname = if branch_name.starts_with("refs/") {
                branch_name.clone()
            } else {
                format!("refs/heads/{}", branch_name)
            };
            
            repo.find_reference(&refname)
                .or_else(|_| repo.find_reference(&format!("refs/remotes/origin/{}", branch_name)))
                .map_err(|e| format!("Branch '{}' not found: {}", branch_name, e))?
                .target()
                .ok_or_else(|| format!("Branch '{}' is not a direct reference", branch_name))?
        }
        None => {
            // 使用 HEAD
            repo.head()
                .map_err(|e| format!("Failed to get HEAD: {}", e))?
                .target()
                .ok_or_else(|| "HEAD is detached and has no target".to_string())?
        }
    };

    // 創建 revwalk
    let mut revwalk = repo.revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    revwalk.push(start_oid)
        .map_err(|e| format!("Failed to push start OID: {}", e))?;

    // 設置排序方式
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
        .map_err(|e| format!("Failed to set sorting: {}", e))?;

    // 如果有 path_filter，需要特殊處理
    if options.path_filter.is_some() {
        revwalk.simplify_first_parent()
            .map_err(|e| format!("Failed to simplify: {}", e))?;
    }

    let mut results = Vec::new();
    let mut skipped = 0;

    for oid_result in revwalk {
        let oid = oid_result.map_err(|e| format!("Failed to walk: {}", e))?;
        
        // 處理 offset
        if skipped < options.offset {
            skipped += 1;
            continue;
        }

        let commit = repo.find_commit(oid)
            .map_err(|e| format!("Failed to find commit: {}", e))?;

        // 處理 search 過濾
        if let Some(search_term) = &options.search {
            let message = commit.message().unwrap_or("");
            let author_name = commit.author().name().unwrap_or("").to_string();
            let author_email = commit.author().email().unwrap_or("").to_string();
            
            let search_lower = search_term.to_lowercase();
            if !message.to_lowercase().contains(&search_lower)
                && !author_name.to_lowercase().contains(&search_lower)
                && !author_email.to_lowercase().contains(&search_lower)
            {
                continue;
            }
        }

        let author = commit.author();
        results.push(CommitSummary {
            oid: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: AuthorInfo {
                name: author.name().unwrap_or("").to_string(),
                email: author.email().unwrap_or("").to_string(),
            },
            timestamp: commit.time().seconds(),
        });

        // 檢查是否達到限制
        if results.len() >= options.limit {
            break;
        }
    }

    Ok(results)
}

#[tauri::command]
pub fn get_graph_data(state: tauri::State<AppState>, limit: usize) -> Result<GraphData, String> {
    let repo_path = state.current_repo_path.lock().map_err(|_| "Failed to lock repo_path".to_string())?;
    let repo_path_buf = PathBuf::from(repo_path.clone().ok_or_else(|| "No repository opened".to_string())?);
    
    let repo = Repository::open(&repo_path_buf)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    // 獲取所有分支
    let mut branches_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for branch in repo.branches(Some(git2::BranchType::Local))
        .map_err(|e| format!("Failed to list branches: {}", e))? 
    {
        let (branch_ref, _branch_type) = branch.map_err(|e| format!("Failed to get branch: {}", e))?;
        let branch_name = branch_ref.name()
            .unwrap_or(None)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        if let Some(target) = branch_ref.get().target() {
            branches_map.entry(target.to_string())
                .or_insert_with(Vec::new)
                .push(branch_name.trim_start_matches("refs/heads/").to_string());
        }
    }

    // 創建 revwalk
    let mut revwalk = repo.revwalk()
        .map_err(|e| format!("Failed to create revwalk: {}", e))?;

    // 推入所有分支的 HEAD
    for branch in repo.branches(Some(git2::BranchType::Local))
        .map_err(|e| format!("Failed to list branches: {}", e))? 
    {
        let (branch_ref, _) = branch.map_err(|e| format!("Failed to get branch: {}", e))?;
        if let Some(target) = branch_ref.get().target() {
            let _ = revwalk.push(target);
        }
    }

    // 如果沒有分支，使用 HEAD
    if revwalk.next().is_none() {
        if let Ok(head) = repo.head() {
            if let Some(target) = head.target() {
                revwalk.push(target)
                    .map_err(|e| format!("Failed to push HEAD: {}", e))?;
            }
        }
    }

    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
        .map_err(|e| format!("Failed to set sorting: {}", e))?;

    // 收集提交並計算 lane
    let mut commits_data: Vec<(git2::Oid, CommitSummary, Vec<String>)> = Vec::new();
    let mut oid_to_lane: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut next_lane = 0;

    for oid_result in revwalk.take(limit) {
        let oid = oid_result.map_err(|e| format!("Failed to walk: {}", e))?;
        
        let commit = repo.find_commit(oid)
            .map_err(|e| format!("Failed to find commit: {}", e))?;

        let author = commit.author();
        let summary = CommitSummary {
            oid: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: AuthorInfo {
                name: author.name().unwrap_or("").to_string(),
                email: author.email().unwrap_or("").to_string(),
            },
            timestamp: commit.time().seconds(),
        };

        // 獲取分支名
        let branches = branches_map.get(&oid.to_string())
            .cloned()
            .unwrap_or_default();

        commits_data.push((oid, summary, branches));
    }

    // 計算 lane 分配
    // 簡單的策略：每個父關係嘗試復用 lane
    let mut graph_commits = Vec::new();

    for (oid, summary, branches) in commits_data.iter() {
        let commit = repo.find_commit(*oid)
            .map_err(|e| format!("Failed to find commit: {}", e))?;

        // 決定這個提交的 lane
        let lane = if let Some(parent) = commit.parents().next() {
            let parent_oid = parent.id().to_string();
            if let Some(&parent_lane) = oid_to_lane.get(&parent_oid) {
                parent_lane
            } else {
                // 分配新 lane
                let new_lane = next_lane;
                next_lane += 1;
                new_lane
            }
        } else {
            // 初始提交
            let new_lane = next_lane;
            next_lane += 1;
            new_lane
        };

        oid_to_lane.insert(summary.oid.clone(), lane);

        // 構建連接
        let mut connections = Vec::new();
        for parent in commit.parents() {
            let parent_oid = parent.id().to_string();
            let to_lane = oid_to_lane.get(&parent_oid).copied().unwrap_or(lane);
            
            connections.push(Connection {
                to_oid: parent_oid,
                from_lane: lane,
                to_lane,
            });
        }

        graph_commits.push(GraphCommit {
            summary: summary.clone(),
            lane,
            branches: branches.clone(),
            connections,
        });
    }

    Ok(GraphData {
        commits: graph_commits,
    })
}
