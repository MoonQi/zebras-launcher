use crate::models::ProjectInfo;
use crate::services::ProjectScanner;
use std::path::PathBuf;

#[tauri::command]
pub async fn get_project_details(project_path: String) -> Result<ProjectInfo, String> {
    let path = PathBuf::from(project_path);
    ProjectScanner::rescan_project(&path)
}

#[tauri::command]
pub async fn rescan_project(project_path: String) -> Result<ProjectInfo, String> {
    let path = PathBuf::from(project_path);
    ProjectScanner::rescan_project(&path)
}

#[tauri::command]
pub async fn is_zebras_project(project_path: String) -> Result<bool, String> {
    let path = PathBuf::from(project_path);
    Ok(ProjectScanner::is_zebras_project(&path))
}
