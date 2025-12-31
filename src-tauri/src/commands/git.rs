use crate::models::{GitPullResult, GitStatus};
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
