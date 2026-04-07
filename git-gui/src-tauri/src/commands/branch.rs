use tauri::State;
use crate::state::AppState;
use crate::error::{AppError, Result};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use git2::{Repository, BranchType, ObjectType};

/// Commit summary information (re-exported from repo module concept)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub upstream: Option<String>,
    pub ahead: i32,
    pub behind: i32,
    pub last_commit: CommitSummary,
}

/// List of branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchList {
    pub local: Vec<BranchInfo>,
    pub remote: Vec<BranchInfo>,
    pub current: Option<String>,
}

/// Merge result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "conflicts")]
pub enum MergeResult {
    FastForward,
    Merged { conflicts: Vec<String> },
    UpToDate,
}

/// Rebase result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseResult {
    pub success: bool,
    pub commits_rebased: u32,
    pub error_message: Option<String>,
}

/// Tag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    pub oid: String,
    pub tag_type: String, // "commit", "tag", etc.
    pub message: Option<String>,
    pub tagger: Option<String>,
    pub timestamp: Option<i64>,
}

/// Helper function to get the current repository
fn get_repo(state: &State<AppState>) -> Result<Repository> {
    let current_repo = state.current_repo_path.lock().unwrap();
    match &*current_repo {
        Some(path) => {
            let repo_path = PathBuf::from(path);
            if !repo_path.exists() {
                return Err(AppError::NoRepository);
            }
            Ok(Repository::open(&repo_path)?)
        }
        None => Err(AppError::NoRepository),
    }
}

/// Helper function to get commit summary from a git2::Commit
fn commit_summary_from_commit(commit: &git2::Commit) -> CommitSummary {
    CommitSummary {
        oid: commit.id().to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: commit.author().name().unwrap_or("Unknown").to_string(),
        timestamp: commit.time().seconds(),
    }
}

/// Helper function to get branch info from a git2::Branch
fn get_branch_info(branch: &git2::Branch, repo: &Repository) -> Result<BranchInfo> {
    let name = branch
        .name()?
        .unwrap_or("unknown")
        .to_string();
    
    // Get upstream
    let upstream = branch.upstream().ok().and_then(|upstream_branch| {
        upstream_branch.name().ok().flatten().map(|s| s.to_string())
    });
    
    // Get ahead/behind counts
    let (ahead, behind) = get_ahead_behind(branch, repo);
    
    // Get last commit
    let last_commit = branch.get().peel_to_commit().ok().map(|commit| {
        commit_summary_from_commit(&commit)
    }).unwrap_or(CommitSummary {
        oid: "".to_string(),
        message: "".to_string(),
        author: "".to_string(),
        timestamp: 0,
    });
    
    Ok(BranchInfo {
        name,
        upstream,
        ahead,
        behind,
        last_commit,
    })
}

/// Helper function to get ahead/behind counts for a branch
fn get_ahead_behind(branch: &git2::Branch, repo: &Repository) -> (i32, i32) {
    if let Ok(upstream) = branch.upstream() {
        let local_oid = match branch.get().target() {
            Some(oid) => oid,
            None => return (0, 0),
        };
        
        let upstream_oid = match upstream.get().target() {
            Some(oid) => oid,
            None => return (0, 0),
        };
        
        match repo.graph_ahead_behind(local_oid, upstream_oid) {
            Ok((ahead, behind)) => (ahead as i32, behind as i32),
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    }
}

/// Check if working directory is clean
fn is_working_directory_clean(repo: &Repository) -> bool {
    let mut status_opts = git2::StatusOptions::new();
    status_opts.include_untracked(true);
    
    match repo.statuses(Some(&mut status_opts)) {
        Ok(statuses) => statuses.is_empty(),
        Err(_) => false,
    }
}

/// Get all branches (local and remote)
#[tauri::command]
pub fn get_branches(state: State<AppState>) -> Result<BranchList> {
    let repo = get_repo(&state)?;
    
    let mut local_branches = Vec::new();
    let mut remote_branches = Vec::new();
    
    // Get current branch name
    let current = repo.head().ok().and_then(|head| {
        head.shorthand().map(|s| s.to_string())
    });
    
    // Iterate over local branches
    let mut branches = repo.branches(Some(BranchType::Local))?;
    while let Some(branch_result) = branches.next() {
        let (branch, _) = branch_result?;
        if let Ok(info) = get_branch_info(&branch, &repo) {
            local_branches.push(info);
        }
    }
    
    // Iterate over remote branches
    let mut remote_branch_iter = repo.branches(Some(BranchType::Remote))?;
    while let Some(branch_result) = remote_branch_iter.next() {
        let (branch, _) = branch_result?;
        if let Ok(info) = get_branch_info(&branch, &repo) {
            remote_branches.push(info);
        }
    }
    
    Ok(BranchList {
        local: local_branches,
        remote: remote_branches,
        current,
    })
}

/// Create a new branch
#[tauri::command]
pub fn create_branch(
    state: State<AppState>,
    name: String,
    from_ref: Option<String>,
) -> Result<BranchInfo> {
    let repo = get_repo(&state)?;
    
    // Determine the starting point
    let from_oid = if let Some(ref_name) = from_ref {
        // Try to resolve the reference
        let obj = repo.revparse_single(&ref_name)?;
        obj.peel_to_commit()?.id()
    } else {
        // Use HEAD
        let head = repo.head()?;
        head.target().ok_or_else(|| {
            AppError::Git(git2::Error::from_str("HEAD is not a valid commit"))
        })?
    };
    
    // Create the branch
    let from_commit = repo.find_commit(from_oid)?;
    let branch = repo.branch(&name, &from_commit, false)?;
    
    // Get branch info
    get_branch_info(&branch, &repo)
}

/// Checkout a branch
#[tauri::command]
pub fn checkout_branch(state: State<AppState>, name: String) -> Result<()> {
    let repo = get_repo(&state)?;
    
    // Check if working directory is clean
    if !is_working_directory_clean(&repo) {
        return Err(AppError::Git(git2::Error::from_str(
            "Working directory is not clean. Please commit or stash changes before switching branches."
        )));
    }
    
    // Find the branch
    let branch = repo.find_branch(&name, BranchType::Local)?;
    
    // Get the commit object
    let _object = branch.get().peel_to_tree()?;
    
    // Checkout using checkout_head
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.safe();
    
    repo.checkout_head(Some(&mut checkout_opts))?;
    
    // Set HEAD to the branch
    repo.set_head(branch.get().name().unwrap_or("refs/heads/master"))?;
    
    Ok(())
}

/// Delete a branch
#[tauri::command]
pub fn delete_branch(
    state: State<AppState>,
    name: String,
    force: bool,
) -> Result<()> {
    let repo = get_repo(&state)?;
    
    // Find the branch
    let mut branch = repo.find_branch(&name, BranchType::Local)?;
    
    // Check if this is the current branch
    if let Ok(head) = repo.head() {
        if let Some(head_name) = head.shorthand() {
            if head_name == name {
                return Err(AppError::Git(git2::Error::from_str(
                    "Cannot delete the currently checked out branch"
                )));
            }
        }
    }
    
    // If not forced, check if the branch is fully merged
    if !force {
        let branch_commit = branch.get().peel_to_commit()?;
        
        // Get the merge base with main/master
        let merge_base = if let Ok(master) = repo.find_branch("main", BranchType::Local) {
            master.get().peel_to_commit().ok()
        } else if let Ok(master) = repo.find_branch("master", BranchType::Local) {
            master.get().peel_to_commit().ok()
        } else {
            None
        };
        
        if let Some(base_commit) = merge_base {
            // Check if branch commit is reachable from master (i.e., merged)
            let mut revwalk = repo.revwalk()?;
            revwalk.push(base_commit.id())?;
            
            let is_merged = revwalk.any(|oid| oid.unwrap() == branch_commit.id());
            
            if !is_merged {
                return Err(AppError::Git(git2::Error::from_str(
                    "Branch is not fully merged. Use force=true to delete anyway."
                )));
            }
        }
    }
    
    // Delete the branch
    branch.delete()?;
    
    Ok(())
}

/// Merge a branch into the current branch
#[tauri::command]
pub fn merge_branch(state: State<AppState>, name: String) -> Result<MergeResult> {
    let repo = get_repo(&state)?;
    
    // Check if working directory is clean
    if !is_working_directory_clean(&repo) {
        return Err(AppError::Git(git2::Error::from_str(
            "Working directory is not clean. Please commit or stash changes before merging."
        )));
    }
    
    // Find the branch to merge
    let branch = repo.find_branch(&name, BranchType::Local)?;
    let branch_commit = branch.get().peel_to_commit()?;
    
    // Get the current HEAD commit
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    
    // Check if already up to date
    if head_commit.id() == branch_commit.id() {
        return Ok(MergeResult::UpToDate);
    }
    
    // Check for fast-forward possibility
    let merge_base = repo.merge_base(head_commit.id(), branch_commit.id())?;
    
    if merge_base == head_commit.id() {
        // Fast-forward is possible
        let tree = branch_commit.tree()?;
        
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.safe();
        
        repo.checkout_tree(&tree.as_object(), Some(&mut checkout_opts))?;
        head.set_target(branch_commit.id(), "Fast-forward merge")?;
        
        return Ok(MergeResult::FastForward);
    }
    
    // Perform a normal merge
    let branch_ann = repo.find_annotated_commit(branch_commit.id())?;
    
    repo.merge(&[&branch_ann], None, None)?;
    
    // Check for conflicts
    let mut conflicts = Vec::new();
    if let Ok(mut conflict_iter) = repo.index()?.conflicts() {
        while let Some(conflict_result) = conflict_iter.next() {
            let conflict = conflict_result?;
            if let Some(entry) = conflict.our {
                if let Some(path_bytes) = &entry.path {
                    if !path_bytes.is_empty() {
                        if let Ok(path_str) = std::str::from_utf8(path_bytes) {
                            conflicts.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    if !conflicts.is_empty() {
        // There are conflicts, leave the repo in merge state
        return Ok(MergeResult::Merged { conflicts });
    }
    
    // No conflicts, complete the merge
    let index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    
    // Create merge commit
    let signature = repo.signature()?;
    let parent_ids = vec![head_commit.id(), branch_commit.id()];
    let parents: Vec<git2::Commit> = parent_ids.iter()
        .filter_map(|id| repo.find_commit(*id).ok())
        .collect();
    let parents_refs: Vec<&git2::Commit> = parents.iter().collect();
    
    let commit_id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &format!("Merge branch '{}'", name),
        &tree,
        &parents_refs,
    )?;
    
    // Clean up merge state
    repo.cleanup_state()?;
    
    Ok(MergeResult::Merged { conflicts: vec![] })
}

/// Rebase current branch onto another branch
#[tauri::command]
pub fn rebase_branch(state: State<AppState>, onto: String) -> Result<RebaseResult> {
    let repo = get_repo(&state)?;
    
    // Check if working directory is clean
    if !is_working_directory_clean(&repo) {
        return Err(AppError::Git(git2::Error::from_str(
            "Working directory is not clean. Please commit or stash changes before rebasing."
        )));
    }
    
    // Find the target branch to rebase onto
    let onto_branch = repo.find_branch(&onto, BranchType::Local)?;
    let onto_commit = onto_branch.get().peel_to_commit()?;
    
    // Get the upstream branch (or use onto if no upstream is set)
    let upstream = repo.head()?.peel_to_commit()?;
    
    // Find merge base
    let merge_base = repo.merge_base(upstream.id(), onto_commit.id())?;
    let base_commit = repo.find_commit(merge_base)?;
    
    // Initialize rebase
    let mut rebase_opts = git2::RebaseOptions::new();
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.safe();
    rebase_opts.checkout_options(checkout_opts);
    
    let upstream_ann = repo.find_annotated_commit(upstream.id())?;
    let base_ann = repo.find_annotated_commit(base_commit.id())?;
    let onto_ann = repo.find_annotated_commit(onto_commit.id())?;
    
    let mut rebase = repo.rebase(
        Some(&upstream_ann),
        Some(&base_ann),
        Some(&onto_ann),
        Some(&mut rebase_opts),
    )?;
    
    let mut commits_rebased = 0u32;
    
    // Perform the rebase
    while let Some(op) = rebase.next() {
        match op {
            Ok(_operation) => {
                // Sign off on the operation (auto-commit)
                let signature = repo.signature()?;
                rebase.commit(None, &signature, None)?;
                commits_rebased += 1;
            }
            Err(e) => {
                rebase.abort()?;
                return Ok(RebaseResult {
                    success: false,
                    commits_rebased,
                    error_message: Some(e.to_string()),
                });
            }
        }
    }
    
    // Finish the rebase
    rebase.finish(None)?;
    
    Ok(RebaseResult {
        success: true,
        commits_rebased,
        error_message: None,
    })
}

/// Cherry-pick a commit
#[tauri::command]
pub fn cherry_pick(state: State<AppState>, oid: String) -> Result<()> {
    let repo = get_repo(&state)?;
    
    // Check if working directory is clean
    if !is_working_directory_clean(&repo) {
        return Err(AppError::Git(git2::Error::from_str(
            "Working directory is not clean. Please commit or stash changes before cherry-picking."
        )));
    }
    
    // Parse the OID
    let commit_oid = git2::Oid::from_str(&oid)?;
    let commit = repo.find_commit(commit_oid)?;
    
    // Perform cherry-pick
    let mut opts = git2::CherrypickOptions::new();
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.safe();
    opts.mainline(0);
    
    repo.cherrypick(&commit, Some(&mut opts))?;
    
    // Check for conflicts
    if repo.state() == git2::RepositoryState::CherryPick {
        // There might be conflicts, leave the repo in cherry-pick state
        // User needs to resolve conflicts and commit manually
        return Ok(());
    }
    
    // If no conflicts, complete the cherry-pick
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    
    let signature = repo.signature()?;
    let parent = repo.head()?.peel_to_commit()?;
    
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        commit.message().unwrap_or("Cherry-picked commit"),
        &tree,
        &[&parent],
    )?;
    
    // Clean up state
    repo.cleanup_state()?;
    
    Ok(())
}

/// Get all tags
#[tauri::command]
pub fn get_tags(state: State<AppState>) -> Result<Vec<TagInfo>> {
    let repo = get_repo(&state)?;
    
    let mut tags = Vec::new();
    
    let tag_names = repo.tag_names(None)?;
    
    for tag_name_opt in tag_names.iter() {
        if let Some(tag_name) = tag_name_opt {
            // Try to resolve the tag
            let obj = repo.revparse_single(&format!("refs/tags/{}", tag_name))?;
            
            let tag_info = if let Ok(tag) = obj.into_tag() {
                // Annotated tag
                let target = tag.target()?;
                let tag_type = match target.kind() {
                    Some(ObjectType::Commit) => "commit".to_string(),
                    Some(ObjectType::Blob) => "blob".to_string(),
                    Some(ObjectType::Tree) => "tree".to_string(),
                    Some(ObjectType::Tag) => "tag".to_string(),
                    _ => "unknown".to_string(),
                };
                
                TagInfo {
                    name: tag_name.to_string(),
                    oid: target.id().to_string(),
                    tag_type,
                    message: tag.message().map(|s| s.to_string()),
                    tagger: tag.tagger().and_then(|sig| {
                        sig.name().map(|n| n.to_string())
                    }),
                    timestamp: tag.tagger().map(|sig| sig.when().seconds()),
                }
            } else {
                // Lightweight tag (just points to an object)
                let obj = repo.revparse_single(&format!("refs/tags/{}", tag_name))?;
                let obj_type = match obj.kind() {
                    Some(ObjectType::Commit) => "commit".to_string(),
                    Some(ObjectType::Blob) => "blob".to_string(),
                    Some(ObjectType::Tree) => "tree".to_string(),
                    _ => "unknown".to_string(),
                };
                
                TagInfo {
                    name: tag_name.to_string(),
                    oid: obj.id().to_string(),
                    tag_type: obj_type,
                    message: None,
                    tagger: None,
                    timestamp: None,
                }
            };
            
            tags.push(tag_info);
        }
    }
    
    Ok(tags)
}

/// Create a new tag
#[tauri::command]
pub fn create_tag(
    state: State<AppState>,
    name: String,
    oid: Option<String>,
    message: Option<String>,
) -> Result<()> {
    let repo = get_repo(&state)?;
    
    // Determine what to tag
    let target_obj = if let Some(oid_str) = oid {
        let oid = git2::Oid::from_str(&oid_str)?;
        repo.find_object(oid, None)?
    } else {
        // Default to HEAD
        let head = repo.head()?;
        let obj = head.peel_to_commit()?;
        repo.find_object(obj.id(), None)?
    };
    
    let signature = repo.signature()?;
    
    if let Some(msg) = message {
        // Create annotated tag
        repo.tag(&name, &target_obj, &signature, &msg, false)?;
    } else {
        // Create lightweight tag
        repo.tag_lightweight(&name, &target_obj, false)?;
    }
    
    Ok(())
}
