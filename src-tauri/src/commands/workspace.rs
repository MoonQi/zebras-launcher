use crate::models::Workspace;
use crate::services::{ProjectScanner, WorkspaceService, WorkspaceList, WorkspaceRef};
use std::path::PathBuf;

#[tauri::command]
pub async fn create_workspace(name: String, folders: Vec<String>) -> Result<Workspace, String> {
    if folders.is_empty() {
        return Err("至少需要选择一个文件夹".to_string());
    }

    // 使用第一个文件夹作为 root_path（仅作为显示用途）
    let root_path = PathBuf::from(&folders[0]);

    if !root_path.exists() || !root_path.is_dir() {
        return Err("指定的路径不存在或不是目录".to_string());
    }

    let mut workspace = Workspace::new(name, root_path.clone());

    // 添加所有选择的文件夹
    for folder in folders {
        let path = PathBuf::from(&folder);
        if path.exists() && path.is_dir() {
            workspace.add_folder(folder);
        }
    }

    // 自动扫描所有文件夹中的项目
    workspace.projects = ProjectScanner::scan_folders(&workspace.folders, 3);

    // 保存工作区到用户目录
    WorkspaceService::save_workspace(&workspace)?;

    // 添加到工作区列表
    let mut list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });
    list.add_workspace(&workspace)?;

    Ok(workspace)
}

#[tauri::command]
pub async fn load_workspace(workspace_path: String) -> Result<Workspace, String> {
    let path = PathBuf::from(&workspace_path);

    // 尝试加载工作区
    match WorkspaceService::load_workspace(&path) {
        Ok(workspace) => Ok(workspace),
        Err(_) => {
            // 如果加载失败，尝试从工作区列表中找到对应的工作区 ID
            // 并使用新路径加载
            let list = WorkspaceList::load()?;

            for ws_ref in list.workspaces.iter() {
                if ws_ref.config_path == path {
                    // 找到对应的工作区，使用 ID 构建新路径
                    let new_path = WorkspaceService::get_config_path(&ws_ref.id)?;
                    return WorkspaceService::load_workspace(&new_path);
                }
            }

            Err("工作区文件不存在".to_string())
        }
    }
}

#[tauri::command]
pub async fn scan_workspace_projects(folders: Vec<String>) -> Result<Vec<crate::models::ProjectInfo>, String> {
    if folders.is_empty() {
        return Err("工作区文件夹列表为空".to_string());
    }

    Ok(ProjectScanner::scan_folders(&folders, 3))
}

#[tauri::command]
pub async fn add_workspace_folder(mut workspace: Workspace, folder_path: String) -> Result<Workspace, String> {
    let path = PathBuf::from(&folder_path);

    if !path.exists() || !path.is_dir() {
        return Err("指定的路径不存在或不是目录".to_string());
    }

    workspace.add_folder(folder_path);

    // 重新扫描所有文件夹
    workspace.projects = ProjectScanner::scan_folders(&workspace.folders, 3);

    // 保存工作区
    WorkspaceService::save_workspace(&workspace)?;

    Ok(workspace)
}

#[tauri::command]
pub async fn remove_workspace_folder(mut workspace: Workspace, folder_path: String) -> Result<Workspace, String> {
    workspace.remove_folder(&folder_path);

    // 重新扫描所有文件夹
    workspace.projects = ProjectScanner::scan_folders(&workspace.folders, 3);

    // 保存工作区
    WorkspaceService::save_workspace(&workspace)?;

    Ok(workspace)
}

#[tauri::command]
pub async fn save_workspace(workspace: Workspace) -> Result<(), String> {
    WorkspaceService::save_workspace(&workspace)
}

#[tauri::command]
pub async fn delete_workspace(workspace_id: String, _root_path: String) -> Result<(), String> {
    // 删除工作区配置文件（在用户目录下）
    WorkspaceService::delete_workspace(&workspace_id)?;

    // 从工作区列表移除
    let mut list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });
    list.remove_workspace(&workspace_id)?;

    Ok(())
}

#[tauri::command]
pub async fn get_workspace_list() -> Result<Vec<WorkspaceRef>, String> {
    let list = WorkspaceList::load()?;
    Ok(list.workspaces)
}

#[tauri::command]
pub async fn update_project_enabled(
    mut workspace: Workspace,
    project_id: String,
    enabled: bool,
) -> Result<Workspace, String> {
    // 查找并更新项目的 enabled 状态
    if let Some(project) = workspace.projects.iter_mut().find(|p| p.id == project_id) {
        project.enabled = Some(enabled);
    } else {
        return Err("未找到指定的项目".to_string());
    }

    // 保存工作区
    WorkspaceService::save_workspace(&workspace)?;

    Ok(workspace)
}
