use crate::models::{GitBranch, GitPullResult, GitStatus, GitSwitchResult};
use crate::services::GitManager;

#[tauri::command]
pub async fn is_git_repo(path: String) -> bool {
    GitManager::is_git_repo(&path)
}

#[tauri::command]
pub async fn get_git_status(path: String) -> Result<GitStatus, String> {
    GitManager::new().get_status(path).await
}

#[tauri::command]
pub async fn git_fetch(path: String) -> Result<GitStatus, String> {
    GitManager::new().fetch(path).await
}

#[tauri::command]
pub async fn git_pull(path: String) -> Result<GitPullResult, String> {
    GitManager::new().pull(path).await
}

#[tauri::command]
pub async fn list_git_branches(path: String, fetch_first: bool) -> Result<Vec<GitBranch>, String> {
    GitManager::new().list_branches(path, fetch_first).await
}

#[tauri::command]
pub async fn git_switch_branch(
    path: String,
    branch_name: String,
    is_remote: bool,
) -> Result<GitSwitchResult, String> {
    GitManager::new()
        .switch_branch(path, branch_name, is_remote)
        .await
}
