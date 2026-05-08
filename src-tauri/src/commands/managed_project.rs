use crate::models::{CreateProjectInstanceInput, ValidationResult, Workspace, WorkspaceSourceType};
use crate::services::{ManagedProjectService, WorkspaceList, WorkspaceService};
use std::path::PathBuf;

#[tauri::command]
pub async fn validate_project_instance(
    input: CreateProjectInstanceInput,
) -> Result<ValidationResult, String> {
    Ok(ManagedProjectService::validate_input(&input))
}

#[tauri::command]
pub async fn create_project_instance(
    input: CreateProjectInstanceInput,
) -> Result<Workspace, String> {
    let workspace = ManagedProjectService::create_project_instance(input)?;
    WorkspaceService::save_workspace(&workspace)?;

    let mut list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });
    list.add_workspace(&workspace)?;

    Ok(workspace)
}

#[tauri::command]
pub async fn load_project_instance(root_path: String) -> Result<Workspace, String> {
    let root = PathBuf::from(root_path);
    let base = find_existing_workspace(&root)?;
    let manifest = ManagedProjectService::load_manifest(&root)?;
    let workspace = ManagedProjectService::workspace_from_manifest(&manifest, base.as_ref());

    if base.is_some() {
        WorkspaceService::save_workspace(&workspace)?;
    }

    Ok(workspace)
}

#[tauri::command]
pub async fn repair_project_instance(root_path: String) -> Result<Workspace, String> {
    let root = PathBuf::from(root_path);
    let base = find_existing_workspace(&root)?;
    let workspace = ManagedProjectService::repair_project_instance(&root, base.as_ref())?;
    persist_existing_workspace(&workspace)?;
    Ok(workspace)
}

#[tauri::command]
pub async fn rebuild_project_links(root_path: String) -> Result<Workspace, String> {
    let root = PathBuf::from(root_path);
    let base = find_existing_workspace(&root)?;
    let workspace = ManagedProjectService::rebuild_project_links(&root, base.as_ref())?;
    persist_existing_workspace(&workspace)?;
    Ok(workspace)
}

fn persist_existing_workspace(workspace: &Workspace) -> Result<(), String> {
    WorkspaceService::save_workspace(workspace)?;

    let mut list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });
    list.add_workspace(workspace)?;
    Ok(())
}

fn find_existing_workspace(root_path: &PathBuf) -> Result<Option<Workspace>, String> {
    let list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });

    for workspace_ref in list.workspaces {
        let Ok(workspace) = WorkspaceService::load_workspace(&workspace_ref.config_path) else {
            continue;
        };
        if workspace.source_type == WorkspaceSourceType::ManagedProject
            && workspace.root_path == *root_path
        {
            return Ok(Some(workspace));
        }
    }

    Ok(None)
}
